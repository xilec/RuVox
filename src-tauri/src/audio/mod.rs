//! WAV → Ogg-Opus streaming encoder, plus the Opus → WAV decode used by
//! audio export (#252).
//!
//! Encoder inputs are mono 32-bit-float WAV files. Output is a valid
//! Ogg-Opus stream at 32 kbps VOIP, 20 ms frames. The implementation is
//! streaming — samples are read from the WAV in frame-sized chunks and fed
//! straight to the encoder, so memory use stays constant regardless of
//! audio length.
//!
//! libopus only accepts the five Opus-native rates (RFC 6716 §2: 8/12/16/24/48
//! kHz) as an encoder input rate, so a WAV at any other rate (e.g. Piper's
//! 22050 Hz, or 44100 Hz) is linear-resampled to the nearest native rate
//! before encoding. The native path (Silero ttsd's 48 kHz) is passed through
//! untouched, keeping that common case streaming.
//!
//! See `tmp/opus_compare/` for the prototype this was ported from and the
//! benchmarks that motivated the choice of `opus = "0.3"` (FFI to C libopus)
//! over the pure-Rust `opus-rs` (issue #19).

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use byteorder::{LittleEndian, WriteBytesExt};
use ogg::{PacketReader, PacketWriteEndInfo, PacketWriter};
use opus::{Application, Bitrate, Channels, Decoder, Encoder};
use thiserror::Error;

const FRAME_MS: u32 = 20;
const BITRATE_BPS: i32 = 32_000;
// Ogg logical-stream serial — arbitrary 32-bit value, "RuVO" in ASCII.
const SERIAL: u32 = 0x5275_564f;
// Encoded packet upper bound: 4000 bytes is the max permitted by libopus
// (`opus_encode` returns at most this for a single 20 ms frame at any bitrate).
const MAX_PACKET_BYTES: usize = 4000;
// Sample rates Opus accepts natively (RFC 6716 §2). The encoder is wired up
// for whichever of these the input WAV uses. Anything outside this set is
// resampled to the nearest entry (see `nearest_supported_rate`) before encoding.
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
    #[error("ogg error: {0}")]
    Ogg(String),
    #[error("unsupported wav format: {0}")]
    UnsupportedFormat(String),
}

pub type Result<T> = std::result::Result<T, AudioError>;

/// Encode a mono 32-bit-float WAV at `wav_path` to an Ogg-Opus file at
/// `opus_path`. Streaming — memory use is bounded regardless of audio length.
///
/// The WAV's sample rate must be one Opus accepts natively (8/12/16/24/48 kHz)
/// or, if it is off-list (e.g. Piper's 22050 Hz), it is resampled to the
/// nearest native rate first. The resampled (native) rate is what gets
/// recorded in `OpusHead`, so a 22050 Hz clip ends up as a 24 kHz Opus stream.
pub fn encode_wav_to_opus(wav_path: &Path, opus_path: &Path) -> Result<()> {
    let mut reader = hound::WavReader::open(wav_path)?;
    let spec = reader.spec();

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

    let in_rate = spec.sample_rate;
    // libopus only takes native rates; anything else is resampled to the
    // nearest one before encoding (see module docs).
    let encode_rate = if SUPPORTED_SAMPLE_RATES.contains(&in_rate) {
        in_rate
    } else {
        nearest_supported_rate(in_rate)
    };

    let mut encoder = Encoder::new(encode_rate, Channels::Mono, Application::Voip)?;
    encoder.set_bitrate(Bitrate::Bits(BITRATE_BPS))?;

    // Pre-skip is the leading-sample count decoders must discard, expressed in
    // 48 kHz output ticks (RFC 7845 §4.2). Query libopus for its actual
    // lookahead at the chosen rate and convert; if the bind is unavailable
    // for any reason fall back to the libopus default at 48 kHz so files
    // remain decodable, just with a tiny silence offset on lower rates.
    let pre_skip_48k: u32 = encoder
        .get_lookahead()
        .ok()
        .map(|n| (n as u32).saturating_mul(48_000) / encode_rate)
        .unwrap_or(DEFAULT_PRE_SKIP_48K);
    let pre_skip: u16 = pre_skip_48k.min(u16::MAX as u32) as u16;

    let file = BufWriter::new(File::create(opus_path)?);
    let mut writer = PacketWriter::new(file);

    writer.write_packet(
        build_opus_head(encode_rate, pre_skip),
        SERIAL,
        PacketWriteEndInfo::EndPage,
        0,
    )?;
    writer.write_packet(build_opus_tags(), SERIAL, PacketWriteEndInfo::EndPage, 0)?;

    // Native rates stream straight from the WAV reader (constant memory).
    // Off-list rates must be buffered once to resample, then streamed out.
    let mut samples_iter: Box<dyn Iterator<Item = std::result::Result<f32, hound::Error>>> =
        if encode_rate == in_rate {
            Box::new(reader.samples::<f32>())
        } else {
            let samples: Vec<f32> = reader
                .samples::<f32>()
                .collect::<std::result::Result<Vec<f32>, hound::Error>>()?;
            Box::new(
                resample_linear(&samples, in_rate, encode_rate)
                    .into_iter()
                    .map(Ok),
            )
        };

    write_frames(&mut encoder, &mut writer, encode_rate, &mut samples_iter)?;

    let mut file = writer.into_inner();
    file.flush()?;
    Ok(())
}

/// Pick the Opus-native rate (one of [`SUPPORTED_SAMPLE_RATES`]) closest to
/// `rate`. Used to decide what an off-list WAV should be resampled to: 22050
/// Hz → 24000 Hz, 44100 Hz → 48000 Hz, 32000 Hz → 24000 Hz, etc.
fn nearest_supported_rate(rate: u32) -> u32 {
    *SUPPORTED_SAMPLE_RATES
        .iter()
        .min_by_key(|&&r| (r as i64 - rate as i64).abs())
        .expect("SUPPORTED_SAMPLE_RATES is non-empty")
}

/// Linear-interpolation resample of `input` (mono float, `in_rate` Hz) to
/// `out_rate` Hz. Returns a fresh buffer. Used to bring off-list WAV rates
/// (e.g. Piper's 22050 Hz) onto an Opus-native rate before encoding — the
/// resampling grid changes but pitch and duration are preserved (output
/// length scales with `out_rate/in_rate`). Endpoints past the last input
/// sample are held constant so a trailing frame stays well-formed.
fn resample_linear(input: &[f32], in_rate: u32, out_rate: u32) -> Vec<f32> {
    if in_rate == out_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = out_rate as f64 / in_rate as f64;
    let out_len = (input.len() as f64 * ratio).ceil() as usize;
    let last = input[input.len() - 1];
    let mut out = Vec::with_capacity(out_len);
    for k in 0..out_len {
        let p = k as f64 / ratio;
        let i = p.floor() as usize;
        let frac = p - i as f64;
        let a = input[i] as f64;
        let b = input.get(i + 1).copied().unwrap_or(last) as f64;
        out.push((a * (1.0 - frac) + b * frac) as f32);
    }
    out
}

/// Encode an `f32` sample stream at `encode_rate` (one of the Opus-native
/// rates) into 20 ms Opus frames wrapped in Ogg pages. The last frame is
/// marked `EndStream`; because the stream length isn't known up front, the
/// previously-completed frame is buffered and flushed as a `NormalPacket`
/// when the next frame arrives (or as the terminal `EndStream` page at the
/// end of input). `samples` yields one float per output sample.
fn write_frames(
    encoder: &mut Encoder,
    writer: &mut PacketWriter<BufWriter<File>>,
    encode_rate: u32,
    samples: &mut dyn Iterator<Item = std::result::Result<f32, hound::Error>>,
) -> Result<()> {
    let frame_samples = frame_samples(encode_rate);
    let mut encoded = vec![0u8; MAX_PACKET_BYTES];
    let mut frame_buf = vec![0f32; frame_samples];
    let mut absgp: u64 = 0;
    let mut filled: usize = 0;
    // The most recently completed frame, awaiting its `EndStream` decision.
    let mut pending: Option<(Vec<u8>, u64)> = None;

    loop {
        let s = match samples.next() {
            Some(Ok(s)) => s,
            Some(Err(e)) => return Err(AudioError::Wav(e)),
            None => break,
        };
        frame_buf[filled] = s;
        filled += 1;
        if filled == frame_samples {
            let n = encoder.encode_float(&frame_buf, &mut encoded)?;
            absgp += GRANULE_PER_FRAME;
            let data = encoded[..n].to_vec();
            if let Some((prev, prev_gp)) = pending.take() {
                writer.write_packet(prev, SERIAL, PacketWriteEndInfo::NormalPacket, prev_gp)?;
            }
            pending = Some((data, absgp));
            filled = 0;
        }
    }

    // Flush a trailing partial frame (zero-padded to a full 20 ms).
    if filled > 0 {
        for slot in &mut frame_buf[filled..] {
            *slot = 0.0;
        }
        let n = encoder.encode_float(&frame_buf, &mut encoded)?;
        absgp += GRANULE_PER_FRAME;
        let data = encoded[..n].to_vec();
        if let Some((prev, prev_gp)) = pending.take() {
            writer.write_packet(prev, SERIAL, PacketWriteEndInfo::NormalPacket, prev_gp)?;
        }
        pending = Some((data, absgp));
    }

    if let Some((data, gp)) = pending.take() {
        writer.write_packet(data, SERIAL, PacketWriteEndInfo::EndStream, gp)?;
    }
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

// ── Opus → WAV decode (audio export, #252) ──────────────────────────────────

/// Opus always decodes natively at 48 kHz (RFC 6716 §2) — the export WAV's
/// rate, regardless of the rate the stream was encoded from.
const OPUS_DECODE_RATE: u32 = 48_000;
/// One decoded frame is at most 120 ms (RFC 6716 §2.1.3) — the decode
/// buffer bound.
const MAX_FRAME_SAMPLES_48K: usize = OPUS_DECODE_RATE as usize / 1000 * 120;

/// Decode a stored Ogg-Opus file at `opus_path` to a mono 16-bit PCM WAV at
/// `wav_path` (48 kHz). Used by audio export: the cached Opus original stays
/// untouched, only the exported file is written (#252).
///
/// Honors the RFC 7845 §4.9-4.10 trim rules: the `OpusHead` pre-skip is
/// discarded from the decoded start, and output is capped at the final page
/// granule minus pre-skip, so encoder padding never reaches the file. A
/// failure before the writer opens the target (unreadable source, bad
/// headers) never touches an existing file there; a failure after it does
/// removes the partial output this call wrote — a half-written WAV must not
/// be left at the user-chosen path.
pub fn decode_opus_to_wav(opus_path: &Path, wav_path: &Path) -> Result<()> {
    let created = !wav_path.exists();
    match decode_opus_to_wav_stream(opus_path, wav_path) {
        Ok(()) => Ok(()),
        Err(e) => {
            if created {
                // Best-effort cleanup of our own partial output; the
                // original error is what matters to the caller.
                let _ = fs::remove_file(wav_path);
            }
            Err(e)
        }
    }
}

fn decode_opus_to_wav_stream(opus_path: &Path, wav_path: &Path) -> Result<()> {
    let mut reader = PacketReader::new(BufReader::new(File::open(opus_path)?));
    let mut decoder = Decoder::new(OPUS_DECODE_RATE, Channels::Mono)?;

    let head = reader
        .read_packet()
        .map_err(map_ogg_read_error)?
        .ok_or_else(|| AudioError::Ogg("no OpusHead packet".to_string()))?;
    let (channels, pre_skip) = parse_opus_head(&head.data)?;
    if channels != 1 {
        return Err(AudioError::UnsupportedFormat(format!(
            "expected mono stream, got {channels} channels"
        )));
    }
    // OpusTags carries no samples, but its presence marks a well-formed
    // stream: a file ending after OpusHead must fail (not export a silent
    // 0-sample WAV), and a stream whose second packet is not OpusTags would
    // misalign our header/audio split.
    match reader.read_packet().map_err(map_ogg_read_error)? {
        Some(tags) if tags.data.len() >= 8 && &tags.data[..8] == b"OpusTags" => {}
        Some(_) => return Err(AudioError::Ogg("missing OpusTags packet".to_string())),
        None => return Err(AudioError::Ogg("stream ends after OpusHead".to_string())),
    }

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: OPUS_DECODE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut out = hound::WavWriter::create(wav_path, spec)?;
    let mut frame_buf = vec![0f32; MAX_FRAME_SAMPLES_48K];
    let mut skip_remaining = u64::from(pre_skip);
    // Audible length once the final page's granule is known:
    // granule (48 kHz ticks) minus the discarded pre-skip.
    let mut audible_limit: Option<u64> = None;
    let mut written: u64 = 0;

    while let Some(pkt) = reader.read_packet().map_err(map_ogg_read_error)? {
        let is_last = pkt.last_in_stream();
        let n = decoder.decode_float(&pkt.data, &mut frame_buf, false)?;
        if is_last {
            audible_limit = Some(pkt.absgp_page().saturating_sub(u64::from(pre_skip)));
        }

        let start = skip_remaining.min(n as u64) as usize;
        skip_remaining -= start as u64;
        let end = match audible_limit {
            // Never write past the audible length.
            Some(limit) => (start + limit.saturating_sub(written) as usize).min(n),
            None => n,
        };
        for &s in &frame_buf[start..end] {
            out.write_sample(f32_to_i16(s))?;
        }
        written += (end - start) as u64;
    }

    out.finalize()?;
    Ok(())
}

/// Parse the `OpusHead` fields the decoder needs: channel count and
/// pre-skip (RFC 7845 §5.1 — version at byte 8, channels at 9, pre-skip
/// u16 LE at 10-11).
fn parse_opus_head(data: &[u8]) -> Result<(u8, u16)> {
    if data.len() < 19 || &data[..8] != b"OpusHead" {
        return Err(AudioError::UnsupportedFormat(
            "missing OpusHead packet".to_string(),
        ));
    }
    if data[8] != 1 {
        return Err(AudioError::UnsupportedFormat(format!(
            "unsupported OpusHead version {}",
            data[8]
        )));
    }
    Ok((data[9], u16::from_le_bytes([data[10], data[11]])))
}

/// Convert a decoded float sample to 16-bit PCM: clamp to [-1, 1] and
/// round (`* 32767`), matching the silero-native WAV convention.
fn f32_to_i16(s: f32) -> i16 {
    (s.clamp(-1.0, 1.0) * 32767.0).round() as i16
}

/// Ogg transport errors: real I/O keeps the `Io` variant, structure errors
/// get their own message (`OggReadError` implements only `Debug`).
fn map_ogg_read_error(e: ogg::OggReadError) -> AudioError {
    match e {
        ogg::OggReadError::ReadError(io) => AudioError::Io(io),
        other => AudioError::Ogg(format!("{other:?}")),
    }
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

    /// Write a short, well-formed WAV at `path` matching an arbitrary `spec`
    /// (unlike `write_sine_wav`, which is float/mono-only). Used by the
    /// `rejects_*` negative tests to build inputs `encode_wav_to_opus` must
    /// reject before it ever decodes a sample -- silence is fine since the
    /// content never gets that far.
    fn write_wav_with_spec(path: &Path, spec: hound::WavSpec) {
        let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
        let total_samples = 1000 * spec.channels as usize;
        match spec.sample_format {
            hound::SampleFormat::Float => {
                for _ in 0..total_samples {
                    writer.write_sample(0.0f32).expect("write sample");
                }
            }
            hound::SampleFormat::Int => {
                for _ in 0..total_samples {
                    writer.write_sample(0i16).expect("write sample");
                }
            }
        }
        writer.finalize().expect("finalize");
    }

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

    /// An off-list rate (Piper's 22050 Hz) must NOT be rejected — it is
    /// resampled to the nearest Opus-native rate (24000 Hz) and encoded, with
    /// the resampled rate recorded in `OpusHead`. Regression guard for #206.
    #[test]
    fn resamples_off_list_rate_to_nearest_native() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("in.wav");
        let opus_path = dir.path().join("out.opus");

        write_sine_wav(&wav_path, 22_050, 440.0, 0.25);
        encode_wav_to_opus(&wav_path, &opus_path)
            .unwrap_or_else(|e| panic!("22050 Hz wav should resample+encode, got: {e}"));

        let bytes = std::fs::read(&opus_path).expect("read opus");
        assert!(bytes.len() > 1000, "opus too small: {}", bytes.len());
        assert_eq!(
            read_opus_head_rate(&opus_path),
            24_000,
            "22050 Hz must be resampled to 24000 Hz in OpusHead"
        );
    }

    /// A 44100 Hz WAV must resample to 48000 Hz (the nearest native rate),
    /// exercising the upsample branch of `nearest_supported_rate`.
    #[test]
    fn resamples_44100_to_48000() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("in.wav");
        let opus_path = dir.path().join("out.opus");

        write_sine_wav(&wav_path, 44_100, 440.0, 0.25);
        encode_wav_to_opus(&wav_path, &opus_path)
            .unwrap_or_else(|e| panic!("44100 Hz wav should resample+encode, got: {e}"));

        assert_eq!(
            read_opus_head_rate(&opus_path),
            48_000,
            "44100 Hz must be resampled to 48000 Hz in OpusHead"
        );
    }

    /// `nearest_supported_rate` must pick the closest native rate, exercising
    /// both upsample (22050→24000, 44100→48000, 11025→12000) and downsample
    /// (32000→24000) candidates, and pass native rates through unchanged.
    #[test]
    fn nearest_supported_rate_picks_closest_native() {
        assert_eq!(nearest_supported_rate(8_000), 8_000);
        assert_eq!(nearest_supported_rate(22_050), 24_000);
        assert_eq!(nearest_supported_rate(32_000), 24_000);
        assert_eq!(nearest_supported_rate(44_100), 48_000);
        assert_eq!(nearest_supported_rate(11_025), 12_000);
    }

    /// `resample_linear` must scale output length by `out_rate/in_rate`, keep
    /// the first sample, pass equal rates through untouched, and return an
    /// empty buffer for empty input.
    #[test]
    fn resample_linear_scales_length_and_handles_empty() {
        let input: Vec<f32> = (0..22_050).map(|i| i as f32 / 100.0).collect();

        let out = resample_linear(&input, 22_050, 24_000);
        // Length scales by out/in rate; `ceil` can land one sample over due to
        // float rounding (22050 * 24000/22050 ≈ 24000.000000x).
        assert!(
            out.len() == 24_000 || out.len() == 24_001,
            "length must scale by out/in rate, got {}",
            out.len()
        );
        assert!((out[0] - input[0]).abs() < 1e-3, "first sample preserved");

        // Equal rates pass through unchanged (same length, same content).
        let same = resample_linear(&input, 22_050, 22_050);
        assert_eq!(same.len(), input.len());

        // Empty input stays empty.
        assert!(resample_linear(&[], 22_050, 24_000).is_empty());
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
        write_wav_with_spec(&wav_path, spec);

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
        write_wav_with_spec(&wav_path, spec);

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

    /// Decode is the inverse of the encode side: a sine encoded to Opus at
    /// 24 kHz decodes back as a mono 16-bit PCM WAV at 48 kHz, with the
    /// audible duration preserved (pre-skip discarded, end trim applied —
    /// within one 20 ms frame of the source) and a non-silent signal.
    #[test]
    fn decode_opus_to_wav_round_trips_encoded_sine() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav_path = dir.path().join("src.wav");
        let opus_path = dir.path().join("src.opus");
        let out_path = dir.path().join("out.wav");

        write_sine_wav(&wav_path, 24_000, 440.0, 0.25);
        encode_wav_to_opus(&wav_path, &opus_path).expect("encode");
        decode_opus_to_wav(&opus_path, &out_path).expect("decode");

        let mut reader = hound::WavReader::open(&out_path).expect("open decoded wav");
        let spec = reader.spec();
        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 48_000);
        assert_eq!(spec.bits_per_sample, 16);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);

        let samples: Vec<i16> = reader
            .samples::<i16>()
            .collect::<std::result::Result<_, _>>()
            .expect("read samples");
        let one_frame = 48_000usize * FRAME_MS as usize / 1000;
        assert!(
            samples.len() >= 48_000 - one_frame && samples.len() <= 48_000 + one_frame,
            "audible length must stay within one frame of 1 s, got {}",
            samples.len()
        );
        let peak = samples.iter().map(|s| s.abs()).max().expect("non-empty");
        assert!(
            peak > 3_000,
            "decoded signal must not be silence, peak {peak}"
        );
    }

    /// A file that is not an Ogg stream must fail the decode (not write a
    /// silent or partial WAV) — export maps this to `export.convert_failed`.
    #[test]
    fn decode_opus_to_wav_rejects_non_ogg_input() {
        let dir = tempfile::tempdir().expect("tempdir");
        let opus_path = dir.path().join("garbage.opus");
        let out_path = dir.path().join("out.wav");
        std::fs::write(&opus_path, b"definitely not an ogg stream").expect("write garbage");

        let err = decode_opus_to_wav(&opus_path, &out_path)
            .expect_err("non-ogg input must fail the decode");
        assert!(matches!(err, AudioError::Ogg(_)), "got {err:?}");
    }

    /// A stream that ends right after OpusHead (no OpusTags, no audio
    /// packets) must fail, not "succeed" with a silent 0-sample WAV. The
    /// file is built with the same ogg writer the encoder uses, so the
    /// transport framing is valid.
    #[test]
    fn decode_opus_to_wav_rejects_head_only_stream() {
        let dir = tempfile::tempdir().expect("tempdir");
        let opus_path = dir.path().join("head-only.opus");
        let out_path = dir.path().join("out.wav");

        let file = BufWriter::new(File::create(&opus_path).expect("create ogg"));
        let mut writer = PacketWriter::new(file);
        writer
            .write_packet(
                build_opus_head(24_000, 0),
                SERIAL,
                PacketWriteEndInfo::EndPage,
                0,
            )
            .expect("write OpusHead page");

        let err = decode_opus_to_wav(&opus_path, &out_path)
            .expect_err("head-only stream must fail the decode");
        assert!(matches!(err, AudioError::Ogg(_)), "got {err:?}");
        assert!(!out_path.exists(), "no partial WAV may be left behind");
    }
}
