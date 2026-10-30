//! ONNX synthesis: text encoder, duration predictor, distilled sampler, and
//! the causal chunked vocoder.
//!
//! The graphs and the schedule mirror the upstream Python reference
//! (`teratts.py`): eight Euler steps with the distilled CFG-3 sampler, 3 072
//! samples per compressed frame, a 16-frame vocoder chunk with 20 frames of
//! left context, and trimming to the predicted duration. The one deliberate
//! difference is the initial latent: it comes from a seeded Rust PRNG, not from
//! numpy's `default_rng`, so a full-audio comparison against Python uses an
//! injected latent (see the parity test) instead of comparing two noise draws.

use std::f32::consts::PI;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use ort::session::Session;
use ort::value::Tensor;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use tracing::debug;

use crate::bundle::Bundle;
use crate::error::{Error, Result};
use crate::styles::Styles;

/// Samples the model produces per compressed latent frame.
pub const SAMPLES_PER_FRAME: usize = 3_072;
/// Output sample rate.
pub const SAMPLE_RATE: u32 = 44_100;
/// Frames per vocoder call.
pub const VOCODER_CHUNK_FRAMES: usize = 16;
/// Frames of left context for each vocoder call.
pub const VOCODER_CONTEXT_FRAMES: usize = 20;
/// The model's own speed factor, applied on top of `duration_scale`.
pub const SPEED: f32 = 1.05;
/// Latent channels.
const LATENT_CHANNELS: usize = 144;

/// Sampling and threading knobs for one synthesis call.
#[derive(Debug, Clone, Copy)]
pub struct SynthOptions {
    /// Multiplier on the predicted duration; the measured optimum is 0.9.
    pub duration_scale: f32,
    /// CFG guidance value baked into the distilled sampler call.
    pub guidance: f32,
    /// Seed for the initial latent.
    pub seed: u64,
}

impl Default for SynthOptions {
    fn default() -> Self {
        Self {
            duration_scale: 0.9,
            guidance: 3.0,
            seed: 1234,
        }
    }
}

/// The four sessions, each behind a mutex (ONNX Runtime sessions are not
/// `Sync` for concurrent `run`).
pub struct Sessions {
    text_encoder: Mutex<Session>,
    duration_predictor: Mutex<Session>,
    sampler: Mutex<Session>,
    vocoder: Mutex<Session>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn open_session(path: &Path, threads: usize) -> Result<Session> {
    Session::builder()
        .map_err(|e| Error::Synthesis(format!("session builder: {e}")))?
        .with_intra_threads(threads)
        .map_err(|e| Error::Synthesis(format!("with_intra_threads({threads}): {e}")))?
        .commit_from_file(path)
        .map_err(|e| Error::BundleCorrupt(format!("cannot open {}: {e}", path.display())))
}

impl Sessions {
    /// Open the four graphs with `threads` intra-op threads each.
    pub fn open(bundle: &Bundle, threads: usize) -> Result<Self> {
        Ok(Self {
            text_encoder: Mutex::new(open_session(
                &bundle.file("models/text_encoder.onnx"),
                threads,
            )?),
            duration_predictor: Mutex::new(open_session(
                &bundle.file("models/duration_predictor.onnx"),
                threads,
            )?),
            sampler: Mutex::new(open_session(
                &bundle.file("models/sampler_distilled_cfg3_8step.onnx"),
                threads,
            )?),
            vocoder: Mutex::new(open_session(&bundle.file("models/vocoder.onnx"), threads)?),
        })
    }
}

fn take_f32(outputs: &ort::session::SessionOutputs, name: &str) -> Result<(Vec<i64>, Vec<f32>)> {
    let value = outputs
        .get(name)
        .ok_or_else(|| Error::Synthesis(format!("model output {name} missing")))?;
    let (shape, data) = value
        .try_extract_tensor::<f32>()
        .map_err(|e| Error::Synthesis(format!("cannot read output {name}: {e}")))?;
    Ok((shape.to_vec(), data.to_vec()))
}

/// Run only the duration predictor; returns raw seconds (no `duration_scale`).
///
/// Used both by synthesis and by the chunk validator / timestamp estimator,
/// which call it on word prefixes.
pub fn predict_raw(
    sessions: &Sessions,
    text_ids: Vec<i64>,
    text_mask: Vec<f32>,
    style_dp: &[f32],
) -> Result<f64> {
    let length = text_ids.len();
    if length == 0 {
        return Err(Error::InvalidText("text produced no tokens".into()));
    }
    let ids = Tensor::from_array((vec![1, length], text_ids))
        .map_err(|e| Error::Synthesis(format!("text_ids tensor: {e}")))?;
    let mask = Tensor::from_array((vec![1, 1, length], text_mask))
        .map_err(|e| Error::Synthesis(format!("text_mask tensor: {e}")))?;
    let style = Tensor::from_array((vec![1, 8, 16], style_dp.to_vec()))
        .map_err(|e| Error::Synthesis(format!("style_dp tensor: {e}")))?;
    let mut session = lock(&sessions.duration_predictor);
    let outputs = session
        .run(ort::inputs!["text_ids" => ids, "style_dp" => style, "text_mask" => mask])
        .map_err(|e| Error::Synthesis(format!("duration predictor: {e}")))?;
    let (_, raw) = take_f32(&outputs, "output")?;
    let seconds = raw.first().copied().unwrap_or(0.0) as f64;
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err(Error::Synthesis(format!(
            "duration predictor returned {seconds}"
        )));
    }
    Ok(seconds)
}

/// A synthesized waveform at [`SAMPLE_RATE`].
pub struct Waveform {
    pub samples: Vec<f32>,
    pub duration_sec: f64,
}

/// Synthesize one prepared chunk.
///
/// `text_ids`/`text_mask` describe the accented text (tags included);
/// `duration_ids`/`duration_mask` describe the same text with `+` removed, as
/// the upstream reference does.
pub fn synthesize(
    sessions: &Sessions,
    styles: &Styles,
    text_ids: Vec<i64>,
    text_mask: Vec<f32>,
    duration_ids: Vec<i64>,
    duration_mask: Vec<f32>,
    options: &SynthOptions,
) -> Result<Waveform> {
    if !options.duration_scale.is_finite() || options.duration_scale <= 0.0 {
        return Err(Error::Synthesis("duration_scale must be positive".into()));
    }
    let text_emb = run_text_encoder(sessions, styles, text_ids, text_mask)?;

    let raw_duration = predict_raw(sessions, duration_ids, duration_mask, &styles.dp)?;
    let duration_seconds = raw_duration * options.duration_scale as f64 / SPEED as f64;
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(Error::Synthesis(format!(
            "duration predictor returned {duration_seconds}"
        )));
    }
    let frames =
        ((duration_seconds * SAMPLE_RATE as f64 / SAMPLES_PER_FRAME as f64).ceil() as usize).max(1);
    debug!(raw_duration, duration_seconds, frames, "latent geometry");

    let latent = initial_latent(options.seed, frames);
    let latent_mask = vec![1.0f32; frames];
    let latent = run_sampler(
        sessions,
        styles,
        text_emb,
        latent,
        latent_mask,
        options.guidance,
    )?;

    let maximum_samples = (duration_seconds * SAMPLE_RATE as f64).round() as usize;
    let samples = run_vocoder(sessions, latent, frames, maximum_samples)?;
    let duration_sec = samples.len() as f64 / SAMPLE_RATE as f64;
    Ok(Waveform {
        samples,
        duration_sec,
    })
}

/// Draw the initial latent `[1, 144, frames]` from a seeded PRNG.
fn initial_latent(seed: u64, frames: usize) -> Vec<f32> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut latent = Vec::with_capacity(LATENT_CHANNELS * frames);
    while latent.len() < LATENT_CHANNELS * frames {
        // Box-Muller: two uniforms per pair of normals, no extra dependency.
        let u1 = rng.random::<f32>().max(f32::MIN_POSITIVE);
        let u2 = rng.random::<f32>();
        let radius = (-2.0 * u1.ln()).sqrt();
        let angle = 2.0 * PI * u2;
        latent.push(radius * angle.cos());
        if latent.len() < LATENT_CHANNELS * frames {
            latent.push(radius * angle.sin());
        }
    }
    latent
}

/// Run the text encoder for one chunk. Kept separate so the parity test can
/// compare the embedding without duplicating the tensor plumbing.
fn run_text_encoder(
    sessions: &Sessions,
    styles: &Styles,
    text_ids: Vec<i64>,
    text_mask: Vec<f32>,
) -> Result<(Vec<i64>, Vec<f32>)> {
    let length = text_ids.len();
    let ids = Tensor::from_array((vec![1, length], text_ids))
        .map_err(|e| Error::Synthesis(format!("text_ids tensor: {e}")))?;
    let mask = Tensor::from_array((vec![1, 1, length], text_mask))
        .map_err(|e| Error::Synthesis(format!("text_mask tensor: {e}")))?;
    let style_ttl = Tensor::from_array((vec![1, 50, 256], styles.ttl.clone()))
        .map_err(|e| Error::Synthesis(format!("style_ttl tensor: {e}")))?;
    let mut session = lock(&sessions.text_encoder);
    let outputs = session
        .run(ort::inputs!["text_ids" => ids, "style_ttl" => style_ttl, "text_mask" => mask])
        .map_err(|e| Error::Synthesis(format!("text encoder: {e}")))?;
    take_f32(&outputs, "output")
}

fn run_sampler(
    sessions: &Sessions,
    styles: &Styles,
    (emb_shape, text_emb): (Vec<i64>, Vec<f32>),
    latent: Vec<f32>,
    latent_mask: Vec<f32>,
    guidance: f32,
) -> Result<(Vec<i64>, Vec<f32>)> {
    let frames = latent.len() / LATENT_CHANNELS;
    let text_frames = *emb_shape.last().unwrap_or(&0) as usize;
    let latent_t = Tensor::from_array((vec![1, LATENT_CHANNELS, frames], latent))
        .map_err(|e| Error::Synthesis(format!("initial_latent tensor: {e}")))?;
    let emb_t = Tensor::from_array((vec![1, 256, text_frames], text_emb))
        .map_err(|e| Error::Synthesis(format!("text_emb tensor: {e}")))?;
    let style_t = Tensor::from_array((vec![1, 50, 256], styles.ttl.clone()))
        .map_err(|e| Error::Synthesis(format!("style_ttl tensor: {e}")))?;
    let latent_mask_t = Tensor::from_array((vec![1, 1, frames], latent_mask))
        .map_err(|e| Error::Synthesis(format!("latent_mask tensor: {e}")))?;
    let text_mask_t = Tensor::from_array((vec![1, 1, text_frames], vec![1.0f32; text_frames]))
        .map_err(|e| Error::Synthesis(format!("text_mask tensor: {e}")))?;
    let guidance_t = Tensor::from_array((vec![1], vec![guidance]))
        .map_err(|e| Error::Synthesis(format!("guidance tensor: {e}")))?;
    let mut session = lock(&sessions.sampler);
    let outputs = session
        .run(ort::inputs![
            "initial_latent" => latent_t,
            "text_emb" => emb_t,
            "style_ttl" => style_t,
            "latent_mask" => latent_mask_t,
            "text_mask" => text_mask_t,
            "guidance" => guidance_t
        ])
        .map_err(|e| Error::Synthesis(format!("sampler: {e}")))?;
    take_f32(&outputs, "latent")
}

fn run_vocoder(
    sessions: &Sessions,
    (latent_shape, latent): (Vec<i64>, Vec<f32>),
    frames: usize,
    maximum_samples: usize,
) -> Result<Vec<f32>> {
    if latent_shape.len() != 3
        || latent_shape[0] != 1
        || latent_shape[1] != LATENT_CHANNELS as i64
        || latent_shape[2] as usize != frames
    {
        return Err(Error::Synthesis(format!(
            "sampler returned latent shape {latent_shape:?}, expected [1, {LATENT_CHANNELS}, {frames}]"
        )));
    }
    let mut samples = Vec::with_capacity(frames * SAMPLES_PER_FRAME);
    let mut emitted = 0usize;
    for start in (0..frames).step_by(VOCODER_CHUNK_FRAMES) {
        let end = (start + VOCODER_CHUNK_FRAMES).min(frames);
        let input_start = start.saturating_sub(VOCODER_CONTEXT_FRAMES);
        let chunk_frames = end - input_start;
        let mut chunk = Vec::with_capacity(LATENT_CHANNELS * chunk_frames);
        for channel in 0..LATENT_CHANNELS {
            let base = channel * frames + input_start;
            chunk.extend_from_slice(&latent[base..base + chunk_frames]);
        }
        let input = Tensor::from_array((vec![1, LATENT_CHANNELS, chunk_frames], chunk))
            .map_err(|e| Error::Synthesis(format!("vocoder input tensor: {e}")))?;
        let decoded = {
            let mut session = lock(&sessions.vocoder);
            let outputs = session
                .run(ort::inputs!["latent" => input])
                .map_err(|e| Error::Synthesis(format!("vocoder: {e}")))?;
            let (shape, data) = take_f32(&outputs, "output")?;
            if shape.len() != 2 || shape[0] != 1 {
                return Err(Error::Synthesis(format!(
                    "vocoder returned shape {shape:?}, expected [1, N]"
                )));
            }
            data
        };
        let discard = (start - input_start) * SAMPLES_PER_FRAME;
        let new_samples = (end - start) * SAMPLES_PER_FRAME;
        let take = new_samples.min(decoded.len().saturating_sub(discard));
        if take != new_samples {
            return Err(Error::Synthesis(format!(
                "vocoder returned {} samples, expected {new_samples}",
                decoded.len().saturating_sub(discard)
            )));
        }
        let remaining = maximum_samples.saturating_sub(emitted);
        let take = take.min(remaining);
        samples.extend_from_slice(&decoded[discard..discard + take]);
        emitted += take;
        if emitted >= maximum_samples {
            break;
        }
    }
    if samples.len() != maximum_samples {
        return Err(Error::Synthesis(format!(
            "vocoder produced {} samples, expected {maximum_samples}",
            samples.len()
        )));
    }
    Ok(samples)
}

/// Encode samples as a mono PCM16 WAV at [`SAMPLE_RATE`].
pub fn encode_wav(samples: &[f32]) -> Result<Vec<u8>> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut buffer, spec)
            .map_err(|e| Error::Synthesis(format!("wav writer: {e}")))?;
        for sample in samples {
            let value = (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16;
            writer
                .write_sample(value)
                .map_err(|e| Error::Synthesis(format!("wav sample: {e}")))?;
        }
        writer
            .finalize()
            .map_err(|e| Error::Synthesis(format!("wav finalize: {e}")))?;
    }
    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latent_has_the_expected_geometry_and_is_seeded() {
        let first = initial_latent(1234, 7);
        let second = initial_latent(1234, 7);
        assert_eq!(first, second);
        assert_eq!(first.len(), LATENT_CHANNELS * 7);
        assert_ne!(first, initial_latent(1235, 7));
    }

    #[test]
    fn latent_values_are_reasonable_normals() {
        let latent = initial_latent(7, 400);
        let mean = latent.iter().sum::<f32>() / latent.len() as f32;
        let variance = latent.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / latent.len() as f32;
        assert!(mean.abs() < 0.05, "mean {mean}");
        assert!((variance - 1.0).abs() < 0.1, "variance {variance}");
    }

    #[test]
    fn wav_header_is_mono_pcm16_at_44k() {
        let wav = encode_wav(&[0.0, 1.0, -1.0]).unwrap();
        let reader = hound::WavReader::new(std::io::Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, 44_100);
        assert_eq!(reader.spec().bits_per_sample, 16);
        assert_eq!(reader.len(), 3);
    }
}

/// Cross-language parity with the upstream Python reference. The fixtures are
/// generated by `export/parity_fixtures.py` from the pinned snapshot and live
/// in `tests/fixtures/`; the test is skipped when `TERATTS_BUNDLE` is unset or
/// the fixtures are absent, so the default suite stays hermetic.
#[cfg(test)]
mod parity {
    use std::path::{Path, PathBuf};

    use super::*;

    fn fixture_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn read_f32(name: &str) -> Vec<f32> {
        let bytes = std::fs::read(fixture_dir().join(name)).expect("fixture");
        bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    }

    fn max_diff(actual: &[f32], expected: &[f32]) -> f32 {
        assert_eq!(actual.len(), expected.len(), "fixture length");
        actual
            .iter()
            .zip(expected)
            .map(|(actual, expected)| (actual - expected).abs())
            .fold(0.0, f32::max)
    }

    #[test]
    fn matches_python_fixtures() {
        let Some(dir) = std::env::var_os("TERATTS_BUNDLE").map(PathBuf::from) else {
            eprintln!("skipping: TERATTS_BUNDLE is not set");
            return;
        };
        let metadata_path = fixture_dir().join("parity.json");
        if !metadata_path.exists() {
            eprintln!("skipping: parity fixtures are not present");
            return;
        }
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&metadata_path).expect("metadata"))
                .expect("metadata JSON");
        let text_ids: Vec<i64> = metadata["text_ids"]
            .as_array()
            .expect("text_ids")
            .iter()
            .map(|value| value.as_i64().expect("token"))
            .collect();
        let text_mask = vec![1.0f32; text_ids.len()];
        let voice = metadata["voice"].as_str().expect("voice");

        let bundle = Bundle::load(&dir).expect("bundle");
        // One thread keeps the reference and the Rust run on the same kernel
        // schedule; CPU kernels are otherwise deterministic but this removes
        // any doubt about intra-op reduction order.
        let sessions = Sessions::open(&bundle, 1).expect("sessions");
        let styles = Styles::load(&dir.join("styles").join(voice)).expect("styles");

        let (emb_shape, text_emb) =
            run_text_encoder(&sessions, &styles, text_ids.clone(), text_mask.clone())
                .expect("text encoder");
        let diff = max_diff(&text_emb, &read_f32("text_emb.bin"));
        assert!(diff < 2e-3, "text embedding max diff {diff}");

        let raw_duration =
            predict_raw(&sessions, text_ids, text_mask, &styles.dp).expect("duration predictor");
        let expected_duration = metadata["raw_duration"].as_f64().expect("raw_duration");
        assert!(
            (raw_duration - expected_duration).abs() < 1e-3,
            "raw duration {raw_duration} vs {expected_duration}"
        );

        let frames = metadata["frames"].as_u64().expect("frames") as usize;
        let latent = read_f32("initial_latent.bin");
        assert_eq!(latent.len(), LATENT_CHANNELS * frames);
        let guidance = metadata["guidance"].as_f64().expect("guidance") as f32;
        let (sampled_shape, sampled_latent) = run_sampler(
            &sessions,
            &styles,
            (emb_shape, text_emb),
            latent,
            vec![1.0f32; frames],
            guidance,
        )
        .expect("sampler");
        let diff = max_diff(&sampled_latent, &read_f32("sampler_latent.bin"));
        assert!(diff < 2e-3, "sampler latent max diff {diff}");

        let maximum_samples = metadata["maximum_samples"]
            .as_u64()
            .expect("maximum_samples") as usize;
        let audio = run_vocoder(
            &sessions,
            (sampled_shape, sampled_latent),
            frames,
            maximum_samples,
        )
        .expect("vocoder");
        let expected_audio = read_f32("audio.bin");
        let diff = max_diff(&audio[..expected_audio.len()], &expected_audio);
        assert!(diff < 5e-3, "audio max diff {diff}");
    }
}
