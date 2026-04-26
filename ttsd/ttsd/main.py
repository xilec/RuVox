"""Main loop: read JSON request lines from stdin, dispatch, write JSON responses to stdout."""

import logging
import sys
from pathlib import Path

from pydantic import BaseModel, ValidationError

from ttsd import __version__
from ttsd.protocol import (
    ErrResponse,
    OkShutdown,
    OkSynthesize,
    OkWarmup,
    RequestAdapter,
    SynthesizeRequest,
)
from ttsd.silero import SileroEngine

logger = logging.getLogger("ttsd")

_engine = SileroEngine()


def _setup_logging() -> None:
    handler = logging.StreamHandler(stream=sys.stderr)
    handler.setFormatter(logging.Formatter("%(asctime)s %(levelname)s %(name)s: %(message)s"))
    logging.basicConfig(level=logging.INFO, handlers=[handler])


def _write_response(response: BaseModel) -> None:
    sys.stdout.write(response.model_dump_json() + "\n")
    sys.stdout.flush()


def _handle_warmup() -> OkWarmup | ErrResponse:
    """Load the Silero model.

    On failure, returns ErrResponse instead of crashing so ttsd keeps running
    and can accept a retry warmup request.  The Rust backend will emit
    model_error and may retry.
    """
    try:
        _engine.load()
    except Exception as exc:
        logger.exception("warmup failed")
        return ErrResponse(error="model_not_loaded", message=str(exc))
    return OkWarmup(version=__version__)


def _handle_synthesize(req: SynthesizeRequest) -> OkSynthesize | ErrResponse:
    if not _engine.is_loaded():
        return ErrResponse(
            error="model_not_loaded",
            message="Silero model is not loaded; send warmup first",
        )

    if not req.text.strip():
        return ErrResponse(error="bad_input", message="text must not be empty")

    try:
        char_mapping = (
            [entry.model_dump() for entry in req.char_mapping] if req.char_mapping else None
        )
        result = _engine.synthesize(
            text=req.text,
            speaker=req.speaker,
            sample_rate=req.sample_rate,
            out_wav=Path(req.out_wav),
            char_mapping=char_mapping,
        )
    except ValueError as exc:
        return ErrResponse(error="bad_input", message=str(exc))
    except Exception as exc:
        logger.exception("synthesis failed")
        return ErrResponse(error="synthesis_failed", message=str(exc))

    return OkSynthesize(timestamps=result.timestamps, duration_sec=result.duration_sec)


def _handle_shutdown() -> OkShutdown:
    logger.info("shutdown requested")
    return OkShutdown()


def main() -> int:
    _setup_logging()
    logger.info("ttsd starting, version=%s", __version__)
    try:
        for raw_line in sys.stdin:
            line = raw_line.strip()
            if not line:
                continue
            try:
                req = RequestAdapter.validate_json(line)
            except ValidationError as exc:
                _write_response(ErrResponse(error="bad_input", message=str(exc)))
                continue
            if req.cmd == "warmup":
                _write_response(_handle_warmup())
            elif req.cmd == "synthesize":
                _write_response(_handle_synthesize(req))  # type: ignore[arg-type]
            elif req.cmd == "shutdown":
                _write_response(_handle_shutdown())
                return 0
            else:
                _write_response(
                    ErrResponse(error="bad_input", message=f"unknown cmd: {req.cmd}")  # type: ignore[union-attr]
                )
    except KeyboardInterrupt:
        logger.info("interrupted")
        return 0
    except Exception:
        logger.exception("fatal error in ttsd main loop")
        return 1
    return 0
