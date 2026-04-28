//! WAV → Ogg-Opus streaming encoder.
//!
//! Inputs are 48 kHz mono 32-bit float WAV files (the format ttsd writes via
//! Silero). Output is a valid Ogg-Opus stream at 32 kbps VOIP, 20 ms frames.
//! The implementation is streaming — samples are read from the WAV in 960-
//! sample chunks and fed straight to the encoder, so memory use stays
//! constant regardless of audio length.
//!
//! See `tmp/opus_compare/` for the prototype this was ported from and the
//! benchmarks that motivated the choice of `opus = "0.3"` (FFI to C libopus)
//! over the pure-Rust `opus-rs` (issue #19).

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use byteorder::{LittleEndian, WriteBytesExt};
use ogg::{PacketWriteEndInfo, PacketWriter};
use opus::{Application, Bitrate, Channels, Encoder};
use thiserror::Error;

const FRAME_MS: u32 = 20;
const BITRATE_BPS: i32 = 32_000;
// Ogg logical-stream serial — arbitrary 32-bit value, "RuVO" in ASCII.
const SERIAL: u32 = 0x5275_564f;
// Encoded packet upper bound: 4000 bytes is the max permitted by libopus
// (`opus_encode` returns at most this for a single 20 ms frame at any bitrate).
const MAX_PACKET_BYTES: usize = 4000;
// Sample rates Opus accepts natively (RFC 6716 §2). The encoder is wired up
// for whichever of these the input WAV uses.
const SUPPORTED_SAMPLE_RATES: [u32; 5] = [8_000, 12_000, 16_000, 24_000, 48_000];
// Granule position (RFC 7845 §4.1) is always reported in 48 kHz output ticks
// regardless of the input rate, so one 20 ms frame advances the granule by
// exactly 960 ticks.
const GRANULE_PER_FRAME: u64 = 48_000 * FRAME_MS as u64 / 1000;
// Fallback when `Encoder::get_lookahead()` is unavailable — the libopus
// default at 48 kHz / 20 ms / VOIP, expressed in 48 kHz output samples.
const DEFAULT_PRE_SKIP_48K: u32 = 312;

#[inline]
fn frame_samples(sample_rate: u32) -> usize {
    (sample_rate as usize) * (FRAME_MS as usize) / 1000
}

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("wav error: {0}")]
    Wav(#[from] hound::Error),
    #[error("opus error: {0}")]
    Opus(#[from] opus::Error),
    #[error("unsupported wav format: {0}")]
    UnsupportedFormat(String),
}

pub type Result<T> = std::result::Result<T, AudioError>;

/// Encode a mono 32-bit-float WAV at `wav_path` (sample rate must be one of
/// 8/12/16/24/48 kHz — the rates Opus supports natively) to an Ogg-Opus file
/// at `opus_path`. Streaming — memory use is bounded regardless of audio length.
pub fn encode_wav_to_opus(wav_path: &Path, opus_path: &Path) -> Result<()> {
    let mut reader = hound::WavReader::open(wav_path)?;
    let spec = reader.spec();

    if !SUPPORTED_SAMPLE_RATES.contains(&spec.sample_rate) {
        return Err(AudioError::UnsupportedFormat(format!(
            "expected sample rate in {:?} Hz, got {}",
            SUPPORTED_SAMPLE_RATES, spec.sample_rate
        )));
    }
    if spec.channels != 1 {
        return Err(AudioError::UnsupportedFormat(format!(
            "expected mono (1 channel), got {} channels",
            spec.channels
        )));
    }
    if spec.sample_format != hound::SampleFormat::Float || spec.bits_per_sample != 32 {
        return Err(AudioError::UnsupportedFormat(format!(
            "expected 32-bit float PCM, got {:?} {}-bit",
            spec.sample_format, spec.bits_per_sample
        )));
    }

    let sample_rate = spec.sample_rate;
    let frame_samples = frame_samples(sample_rate);

    let mut encoder = Encoder::new(sample_rate, Channels::Mono, Application::Voip)?;
    encoder.set_bitrate(Bitrate::Bits(BITRATE_BPS))?;

    // Pre-skip is the leading-sample count decoders must discard, expressed in
    // 48 kHz output ticks (RFC 7845 §4.2). Query libopus for its actual
    // lookahead at the chosen rate and convert; if the bind is unavailable
    // for any reason fall back to the libopus default at 48 kHz so files
    // remain decodable, just with a tiny silence offset on lower rates.
    let pre_skip_48k: u32 = encoder
        .get_lookahead()
        .ok()
        .map(|n| (n as u32).saturating_mul(48_000) / sample_rate)
        .unwrap_or(DEFAULT_PRE_SKIP_48K);
    let pre_skip: u16 = pre_skip_48k.min(u16::MAX as u32) as u16;

    let total_samples = reader.duration() as usize;
    let total_frames = total_samples.div_ceil(frame_samples);

    let file = BufWriter::new(File::create(opus_path)?);
    let mut writer = PacketWriter::new(file);

    writer.write_packet(
        build_opus_head(sample_rate, pre_skip),
        SERIAL,
        PacketWriteEndInfo::EndPage,
        0,
    )?;
    writer.write_packet(build_opus_tags(), SERIAL, PacketWriteEndInfo::EndPage, 0)?;

    let mut encoded = vec![0u8; MAX_PACKET_BYTES];
    let mut frame_buf = vec![0f32; frame_samples];
    let mut absgp: u64 = 0;
    let mut frame_idx: usize = 0;
    let mut samples = reader.samples::<f32>();

    loop {
        let mut n_read = 0usize;
        for slot in frame_buf.iter_mut() {
            match samples.next() {
                Some(Ok(s)) => {
                    *slot = s;
                    n_read += 1;
                }
                Some(Err(e)) => return Err(AudioError::Wav(e)),
                None => break,
            }
        }
        if n_read == 0 {
            break;
        }
        for slot in &mut frame_buf[n_read..] {
            *slot = 0.0;
        }

        let n = encoder.encode_float(&frame_buf, &mut encoded)?;
        absgp += GRANULE_PER_FRAME;
        frame_idx += 1;

        let end = if frame_idx == total_frames {
            PacketWriteEndInfo::EndStream
        } else {
            PacketWriteEndInfo::NormalPacket
        };
        writer.write_packet(encoded[..n].to_vec(), SERIAL, end, absgp)?;
    }

    let mut file = writer.into_inner();
    file.flush()?;
    Ok(())
}

/// Convenience wrapper: encode `wav_path` to `<wav_path with .opus extension>`,
/// then delete the source `.wav`. Returns the Opus file path.
///
/// On encode failure the source `.wav` is left untouched so the caller can
/// fall back to it.
pub fn replace_wav_with_opus(wav_path: &Path) -> Result<std::path::PathBuf> {
    let opus_path = wav_path.with_extension("opus");
    encode_wav_to_opus(wav_path, &opus_path)?;
    fs::remove_file(wav_path)?;
    Ok(opus_path)
}

fn build_opus_head(input_sample_rate: u32, pre_skip: u16) -> Vec<u8> {
    let mut buf = Vec::with_capacity(19);
    buf.extend_from_slice(b"OpusHead");
    buf.push(1); // version
    buf.push(1); // channel count (mono)
    buf.write_u16::<LittleEndian>(pre_skip).unwrap();
    buf.write_u32::<LittleEndian>(input_sample_rate).unwrap();
    buf.write_i16::<LittleEndian>(0).unwrap(); // output gain Q7.8
    buf.push(0); // mapping family 0 (mono / stereo)
    buf
}

fn build_opus_tags() -> Vec<u8> {
    let vendor = b"RuVox";
    let mut buf = Vec::with_capacity(8 + 4 + vendor.len() + 4);
    buf.extend_from_slice(b"OpusTags");
    buf.write_u32::<LittleEndian>(vendor.len() as u32).unwrap();
    buf.extend_from_slice(vendor);
    buf.write_u32::<LittleEndian>(0).unwrap(); // user comment list length
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write a 1-second mono 32-bit-float sine WAV at `rate`. Used by encode
    /// tests below.
    fn write_sine_wav(path: &Path, rate: u32, freq_hz: f32) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
        for i in 0..rate as usize {
            let t = i as f32 / rate as f32;
            writer
                .write_sample((2.0 * std::f32::consts::PI * freq_hz * t).sin() * 0.25)
                .expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }

    /// Encode a 48 kHz / 1 s sine and confirm the result is a non-empty Ogg
    /// stream that announces 48 kHz in its OpusHead.
    #[test]
    fn encode_48khz_wav_produces_valid_opus() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("in.wav");
        let opus_path = dir.path().join("out.opus");

        write_sine_wav(&wav_path, 48_000, 440.0);
        encode_wav_to_opus(&wav_path, &opus_path).expect("encode");

        let bytes = std::fs::read(&opus_path).expect("read opus");
        assert!(bytes.len() > 1000, "opus too small: {}", bytes.len());
        assert_eq!(&bytes[..4], b"OggS", "not an Ogg stream");
        // OpusHead packet: "OggS" page header (27+) then segment table, then
        // payload starting with "OpusHead". Look for the magic and read the
        // input-sample-rate field at offset +12 from "OpusHead" (bytes 12-15
        // of the 19-byte header).
        let head_off = bytes
            .windows(8)
            .position(|w| w == b"OpusHead")
            .expect("OpusHead present");
        let rate = u32::from_le_bytes([
            bytes[head_off + 12],
            bytes[head_off + 13],
            bytes[head_off + 14],
            bytes[head_off + 15],
        ]);
        assert_eq!(rate, 48_000, "OpusHead input_sample_rate mismatch");
    }

    /// Same as above but at 24 kHz — the rate the prior 48-kHz-only encoder
    /// would have rejected. Verifies the multi-rate path end-to-end.
    #[test]
    fn encode_24khz_wav_produces_valid_opus() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("in.wav");
        let opus_path = dir.path().join("out.opus");

        write_sine_wav(&wav_path, 24_000, 440.0);
        encode_wav_to_opus(&wav_path, &opus_path).expect("encode");

        let bytes = std::fs::read(&opus_path).expect("read opus");
        assert!(bytes.len() > 500, "opus too small: {}", bytes.len());
        assert_eq!(&bytes[..4], b"OggS");
        let head_off = bytes
            .windows(8)
            .position(|w| w == b"OpusHead")
            .expect("OpusHead present");
        let rate = u32::from_le_bytes([
            bytes[head_off + 12],
            bytes[head_off + 13],
            bytes[head_off + 14],
            bytes[head_off + 15],
        ]);
        assert_eq!(rate, 24_000, "OpusHead must record the input rate");
    }

    /// Rates outside the Opus-native set must be rejected up front.
    #[test]
    fn rejects_unsupported_sample_rate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("in.wav");
        let opus_path = dir.path().join("out.opus");

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 22_050,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&wav_path, spec).expect("create wav");
        for _ in 0..1000 {
            writer.write_sample(0.0f32).expect("write sample");
        }
        writer.finalize().expect("finalize");

        let err =
            encode_wav_to_opus(&wav_path, &opus_path).expect_err("should reject 22.05 kHz wav");
        match err {
            AudioError::UnsupportedFormat(_) => {}
            other => panic!("expected UnsupportedFormat, got {other:?}"),
        }
    }

    #[test]
    fn replace_wav_with_opus_removes_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("clip.wav");

        write_sine_wav(&wav_path, 48_000, 220.0);

        let opus_path = replace_wav_with_opus(&wav_path).expect("replace");
        assert!(opus_path.exists(), "opus file missing");
        assert!(!wav_path.exists(), "source wav should be gone");
        assert_eq!(opus_path.extension().and_then(|e| e.to_str()), Some("opus"));
    }
}
