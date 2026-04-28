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

const SAMPLE_RATE: u32 = 48_000;
const FRAME_MS: u32 = 20;
const FRAME_SAMPLES: usize = (SAMPLE_RATE as usize) * (FRAME_MS as usize) / 1000; // 960
const BITRATE_BPS: i32 = 32_000;
// libopus default warm-up at 48 kHz; required by RFC 7845 §4.2 so decoders
// know how many leading samples to discard.
const PRE_SKIP: u16 = 312;
// Ogg logical-stream serial — arbitrary 32-bit value, "RuVO" in ASCII.
const SERIAL: u32 = 0x5275_564f;
// Encoded packet upper bound: 4000 bytes is the max permitted by libopus
// (`opus_encode` returns at most this for a single 20 ms frame at any bitrate).
const MAX_PACKET_BYTES: usize = 4000;

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

/// Encode a 48 kHz mono float WAV at `wav_path` to an Ogg-Opus file at
/// `opus_path`. Streaming — memory use is bounded regardless of audio length.
pub fn encode_wav_to_opus(wav_path: &Path, opus_path: &Path) -> Result<()> {
    let mut reader = hound::WavReader::open(wav_path)?;
    let spec = reader.spec();

    if spec.sample_rate != SAMPLE_RATE {
        return Err(AudioError::UnsupportedFormat(format!(
            "expected sample rate {SAMPLE_RATE}, got {}",
            spec.sample_rate
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

    let total_samples = reader.duration() as usize;
    let total_frames = total_samples.div_ceil(FRAME_SAMPLES);

    let mut encoder = Encoder::new(SAMPLE_RATE, Channels::Mono, Application::Voip)?;
    encoder.set_bitrate(Bitrate::Bits(BITRATE_BPS))?;

    let file = BufWriter::new(File::create(opus_path)?);
    let mut writer = PacketWriter::new(file);

    writer.write_packet(build_opus_head(), SERIAL, PacketWriteEndInfo::EndPage, 0)?;
    writer.write_packet(build_opus_tags(), SERIAL, PacketWriteEndInfo::EndPage, 0)?;

    let mut encoded = vec![0u8; MAX_PACKET_BYTES];
    let mut frame_buf = vec![0f32; FRAME_SAMPLES];
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
        absgp += FRAME_SAMPLES as u64;
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

fn build_opus_head() -> Vec<u8> {
    let mut buf = Vec::with_capacity(19);
    buf.extend_from_slice(b"OpusHead");
    buf.push(1); // version
    buf.push(1); // channel count (mono)
    buf.write_u16::<LittleEndian>(PRE_SKIP).unwrap();
    buf.write_u32::<LittleEndian>(SAMPLE_RATE).unwrap(); // input sample rate
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

    /// Write a 1 s 48 kHz mono float WAV containing a 440 Hz sine, encode it,
    /// and assert the result is a non-empty Ogg stream.
    #[test]
    fn encode_short_wav_produces_valid_opus() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("in.wav");
        let opus_path = dir.path().join("out.opus");

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&wav_path, spec).expect("create wav");
        let total = SAMPLE_RATE as usize; // 1 second
        for i in 0..total {
            let t = i as f32 / SAMPLE_RATE as f32;
            let s = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.25;
            writer.write_sample(s).expect("write sample");
        }
        writer.finalize().expect("finalize wav");

        encode_wav_to_opus(&wav_path, &opus_path).expect("encode");

        let bytes = std::fs::read(&opus_path).expect("read opus");
        assert!(bytes.len() > 1000, "opus too small: {}", bytes.len());
        assert_eq!(&bytes[..4], b"OggS", "not an Ogg stream");
    }

    #[test]
    fn rejects_wrong_sample_rate() {
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

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(&wav_path, spec).expect("create wav");
        for i in 0..SAMPLE_RATE as usize {
            let t = i as f32 / SAMPLE_RATE as f32;
            writer
                .write_sample((2.0 * std::f32::consts::PI * 220.0 * t).sin() * 0.2)
                .expect("write");
        }
        writer.finalize().expect("finalize");

        let opus_path = replace_wav_with_opus(&wav_path).expect("replace");
        assert!(opus_path.exists(), "opus file missing");
        assert!(!wav_path.exists(), "source wav should be gone");
        assert_eq!(opus_path.extension().and_then(|e| e.to_str()), Some("opus"));
    }
}
