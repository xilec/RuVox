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
    use crate::storage::test_util::write_sine_wav;
    use test_case::test_case;

    /// Rates enumerated as per-rate `#[test_case]` rows below. Kept in sync with
    /// the production `SUPPORTED_SAMPLE_RATES` constant by
    /// `test_case_rates_match_supported_sample_rates`.
    const TEST_CASE_RATES: [u32; 5] = [8_000, 12_000, 16_000, 24_000, 48_000];

    /// Read the `input_sample_rate` field out of an encoded Ogg-Opus file's
    /// OpusHead packet. The packet is: "OggS" page header (27+ bytes) then
    /// segment table, then payload starting with "OpusHead"; the rate is a
    /// little-endian u32 at offset +12 from that magic (bytes 12-15 of the
    /// 19-byte OpusHead payload, RFC 7845 §5.1).
    fn read_opus_head_rate(opus_path: &Path) -> u32 {
        let bytes = std::fs::read(opus_path).expect("read opus");
        assert_eq!(&bytes[..4], b"OggS", "not an Ogg stream");
        let head_off = bytes
            .windows(8)
            .position(|w| w == b"OpusHead")
            .expect("OpusHead present");
        u32::from_le_bytes([
            bytes[head_off + 12],
            bytes[head_off + 13],
            bytes[head_off + 14],
            bytes[head_off + 15],
        ])
    }

    /// Every Opus-native sample rate (RFC 6716 §2, `SUPPORTED_SAMPLE_RATES`)
    /// must round-trip through the encoder: a non-empty Ogg stream whose
    /// OpusHead records the input rate. One `#[test_case]` per rate so a
    /// failing rate is a named case, not a loop iteration swallowed by the
    /// first `assert`.
    #[test_case(8_000; "8kHz")]
    #[test_case(12_000; "12kHz")]
    #[test_case(16_000; "16kHz")]
    #[test_case(24_000; "24kHz")]
    #[test_case(48_000; "48kHz")]
    fn encode_wav_produces_valid_opus_at_supported_rate(rate: u32) {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("in.wav");
        let opus_path = dir.path().join("out.opus");

        write_sine_wav(&wav_path, rate, 440.0, 0.25);
        encode_wav_to_opus(&wav_path, &opus_path)
            .unwrap_or_else(|e| panic!("encode failed at {rate} Hz: {e}"));

        // 1 s at 32 kbps VOIP yields >1700 bytes even at 8/12 kHz; the
        // pre-refactor 48 kHz test asserted > 1000, keep that bar so a
        // header-only or truncated stream can't slip through.
        let bytes = std::fs::read(&opus_path).expect("read opus");
        assert!(
            bytes.len() > 1000,
            "opus too small at {rate} Hz: {}",
            bytes.len()
        );

        let head_rate = read_opus_head_rate(&opus_path);
        assert_eq!(
            head_rate, rate,
            "OpusHead input_sample_rate mismatch at {rate} Hz"
        );
    }

    /// Compile-/run-time guard: the literal rates enumerated as `#[test_case]`
    /// rows above must equal the production `SUPPORTED_SAMPLE_RATES` set, so a
    /// newly supported rate cannot land without its own per-rate case.
    #[test]
    fn test_case_rates_match_supported_sample_rates() {
        let mut cases = TEST_CASE_RATES;
        let mut supported = SUPPORTED_SAMPLE_RATES;
        cases.sort_unstable();
        supported.sort_unstable();
        assert_eq!(
            cases.as_slice(),
            supported.as_slice(),
            "add a #[test_case] row for every SUPPORTED_SAMPLE_RATES entry"
        );
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

    /// Non-mono input must be rejected up front — `encode_wav_to_opus` checks
    /// `spec.channels` before touching the encoder, it does not downmix.
    #[test]
    fn rejects_stereo_wav() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("in.wav");
        let opus_path = dir.path().join("out.opus");

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&wav_path, spec).expect("create wav");
        for _ in 0..1000 {
            writer.write_sample(0.0f32).expect("write sample (L)");
            writer.write_sample(0.0f32).expect("write sample (R)");
        }
        writer.finalize().expect("finalize");

        let err = encode_wav_to_opus(&wav_path, &opus_path).expect_err("should reject stereo wav");
        match err {
            AudioError::UnsupportedFormat(_) => {}
            other => panic!("expected UnsupportedFormat, got {other:?}"),
        }
    }

    /// Non-float sample formats must be rejected up front — `encode_wav_to_opus`
    /// checks `spec.sample_format`/`bits_per_sample` before touching the
    /// encoder, it does not convert integer PCM to float.
    #[test]
    fn rejects_non_float_sample_format() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("in.wav");
        let opus_path = dir.path().join("out.opus");

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&wav_path, spec).expect("create wav");
        for _ in 0..1000 {
            writer.write_sample(0i16).expect("write sample");
        }
        writer.finalize().expect("finalize");

        let err = encode_wav_to_opus(&wav_path, &opus_path)
            .expect_err("should reject 16-bit int PCM wav");
        match err {
            AudioError::UnsupportedFormat(_) => {}
            other => panic!("expected UnsupportedFormat, got {other:?}"),
        }
    }

    #[test]
    fn replace_wav_with_opus_removes_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("clip.wav");

        write_sine_wav(&wav_path, 48_000, 220.0, 0.25);

        let opus_path = replace_wav_with_opus(&wav_path).expect("replace");
        assert!(opus_path.exists(), "opus file missing");
        assert!(!wav_path.exists(), "source wav should be gone");
        assert_eq!(opus_path.extension().and_then(|e| e.to_str()), Some("opus"));
    }
}
