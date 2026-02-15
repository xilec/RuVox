#!/usr/bin/env python3
"""
TTS Generation Script - RuVox Pipeline + Silero v5

Usage:
    python scripts/tts_generate.py input.txt              # Output: input.wav
    python scripts/tts_generate.py input.txt output.wav   # Custom output
    python scripts/tts_generate.py input.txt -s baya      # Different speaker
    python scripts/tts_generate.py --text "Привет мир"    # Direct text input
"""

import argparse
import sys
import time
from pathlib import Path

import numpy as np
import scipy.io.wavfile as wavfile
import torch

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from ruvox.pipeline import TTSPipeline

# Available speakers for Silero v5 Russian model
SPEAKERS = ["aidar", "baya", "kseniya", "xenia", "eugene"]
DEFAULT_SPEAKER = "xenia"
DEFAULT_SAMPLE_RATE = 48000


def load_silero_v5():
    """Load Silero TTS v5 Russian model."""
    print("Loading Silero TTS v5...")

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

    result = torch.hub.load(
        repo_or_dir="snakers4/silero-models", model="silero_tts", language="ru", speaker="v5_ru", trust_repo=True
    )

    if isinstance(result, tuple):
        model, _ = result
    else:
        model = result

    if model is None:
        raise RuntimeError("Failed to load Silero v5 model")

    model.to(device)
    print(f"Model loaded on: {device}")

    return model


def split_text_into_chunks(text: str, max_length: int = 900) -> list[str]:
    """Split text into chunks by sentences, respecting max_length."""
    import re

    # Split by sentence endings
    sentences = re.split(r"(?<=[.!?])\s+", text)

    chunks = []
    current_chunk = ""

    for sentence in sentences:
        # If single sentence is too long, split by commas or spaces
        if len(sentence) > max_length:
            if current_chunk:
                chunks.append(current_chunk.strip())
                current_chunk = ""

            # Split long sentence by commas
            parts = re.split(r"(?<=,)\s+", sentence)
            for part in parts:
                if len(part) > max_length:
                    # Last resort: split by spaces
                    words = part.split()
                    sub_chunk = ""
                    for word in words:
                        if len(sub_chunk) + len(word) + 1 > max_length:
                            if sub_chunk:
                                chunks.append(sub_chunk.strip())
                            sub_chunk = word
                        else:
                            sub_chunk = f"{sub_chunk} {word}" if sub_chunk else word
                    if sub_chunk:
                        current_chunk = sub_chunk
                elif len(current_chunk) + len(part) + 1 > max_length:
                    if current_chunk:
                        chunks.append(current_chunk.strip())
                    current_chunk = part
                else:
                    current_chunk = f"{current_chunk} {part}" if current_chunk else part
        elif len(current_chunk) + len(sentence) + 1 > max_length:
            if current_chunk:
                chunks.append(current_chunk.strip())
            current_chunk = sentence
        else:
            current_chunk = f"{current_chunk} {sentence}" if current_chunk else sentence

    if current_chunk:
        chunks.append(current_chunk.strip())

    return [c for c in chunks if c.strip()]


def synthesize(model, text: str, speaker: str = DEFAULT_SPEAKER, sample_rate: int = DEFAULT_SAMPLE_RATE) -> tuple:
    """Generate speech from text. Returns (waveform, sample_rate, duration, inference_time)."""
    start_time = time.time()

    audio = model.apply_tts(text=text, speaker=speaker, sample_rate=sample_rate)

    inference_time = time.time() - start_time

    # Convert to numpy
    if isinstance(audio, torch.Tensor):
        waveform = audio.cpu().numpy()
    else:
        waveform = np.array(audio)

    duration = len(waveform) / sample_rate

    return waveform, sample_rate, duration, inference_time


def synthesize_long_text(
    model, text: str, speaker: str = DEFAULT_SPEAKER, sample_rate: int = DEFAULT_SAMPLE_RATE, quiet: bool = False
) -> tuple:
    """Generate speech from long text by splitting into chunks."""
    chunks = split_text_into_chunks(text)

    if not quiet:
        print(f"Text split into {len(chunks)} chunks")

    all_waveforms = []
    total_inference_time = 0

    for i, chunk in enumerate(chunks):
        if not quiet:
            print(f"  Processing chunk {i + 1}/{len(chunks)} ({len(chunk)} chars)...")

        waveform, sr, duration, inf_time = synthesize(model, chunk, speaker, sample_rate)
        all_waveforms.append(waveform)
        total_inference_time += inf_time

    # Concatenate all waveforms
    combined = np.concatenate(all_waveforms)
    total_duration = len(combined) / sample_rate

    return combined, sample_rate, total_duration, total_inference_time


def save_audio(waveform, sample_rate: int, filepath: Path):
    """Save waveform to WAV file."""
    # Normalize to int16 range
    # Check if it's float data (typically in range [-1, 1] or slightly beyond)
    if waveform.dtype in (np.float32, np.float64):
        # Clip to [-1, 1] and scale to int16 range
        waveform_clipped = np.clip(waveform, -1.0, 1.0)
        waveform_int16 = (waveform_clipped * 32767).astype("int16")
    else:
        waveform_int16 = waveform.astype("int16")
    wavfile.write(filepath, sample_rate, waveform_int16)


def process_text(text: str, show_warnings: bool = True) -> str:
    """Process text through RuVox pipeline."""
    pipeline = TTSPipeline()
    result = pipeline.process(text)

    if show_warnings:
        pipeline.print_warnings()

    return result


def main():
    parser = argparse.ArgumentParser(
        description="Generate speech from text using RuVox + Silero v5",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s document.txt                    # Process file, output to document.wav
  %(prog)s document.txt speech.wav         # Custom output filename
  %(prog)s document.txt -s baya            # Use different speaker voice
  %(prog)s --text "Привет, мир!"           # Direct text input
  %(prog)s document.txt --no-preprocess    # Skip preprocessing (raw text)

Available speakers: aidar, baya, kseniya, xenia (default), eugene
        """,
    )

    parser.add_argument("input", nargs="?", help="Input text file")
    parser.add_argument("output", nargs="?", help="Output WAV file (default: input with .wav extension)")
    parser.add_argument("--text", "-t", help="Direct text input instead of file")
    parser.add_argument(
        "--speaker", "-s", default=DEFAULT_SPEAKER, choices=SPEAKERS, help=f"Speaker voice (default: {DEFAULT_SPEAKER})"
    )
    parser.add_argument(
        "--sample-rate",
        "-r",
        type=int,
        default=DEFAULT_SAMPLE_RATE,
        help=f"Sample rate in Hz (default: {DEFAULT_SAMPLE_RATE})",
    )
    parser.add_argument("--no-preprocess", action="store_true", help="Skip RuVox preprocessing")
    parser.add_argument("--show-text", action="store_true", help="Print processed text before synthesis")
    parser.add_argument("--quiet", "-q", action="store_true", help="Suppress informational output")

    args = parser.parse_args()

    # Get input text
    if args.text:
        original_text = args.text
        output_path = Path(args.output) if args.output else Path("output.wav")
    elif args.input:
        input_path = Path(args.input)
        if not input_path.exists():
            print(f"Error: File not found: {input_path}", file=sys.stderr)
            sys.exit(1)
        original_text = input_path.read_text(encoding="utf-8")
        output_path = Path(args.output) if args.output else input_path.with_suffix(".wav")
    else:
        parser.print_help()
        sys.exit(1)

    # Preprocess text
    if args.no_preprocess:
        text = original_text
        if not args.quiet:
            print("Skipping preprocessing (--no-preprocess)")
    else:
        if not args.quiet:
            print("Preprocessing text with RuVox pipeline...")
        text = process_text(original_text, show_warnings=not args.quiet)

    if args.show_text:
        print("\n--- Processed text ---")
        print(text)
        print("--- End of text ---\n")

    if not text.strip():
        print("Error: No text to synthesize", file=sys.stderr)
        sys.exit(1)

    # Load model and synthesize
    model = load_silero_v5()

    if not args.quiet:
        print(f"Synthesizing with speaker '{args.speaker}' at {args.sample_rate} Hz...")

    waveform, sr, duration, inf_time = synthesize_long_text(
        model, text, args.speaker, args.sample_rate, quiet=args.quiet
    )

    # Save audio
    save_audio(waveform, sr, output_path)

    if not args.quiet:
        print(f"\nGenerated: {output_path}")
        print(f"Duration: {duration:.2f}s")
        print(f"Inference time: {inf_time:.2f}s")
        print(f"Real-time factor: {duration / inf_time:.1f}x")


if __name__ == "__main__":
    main()
