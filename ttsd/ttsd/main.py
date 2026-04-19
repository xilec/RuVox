"""Main loop: read JSON request lines from stdin, dispatch, write JSON responses to stdout."""

import logging
import sys

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

logger = logging.getLogger("ttsd")


def _setup_logging() -> None:
    handler = logging.StreamHandler(stream=sys.stderr)
    handler.setFormatter(logging.Formatter("%(asctime)s %(levelname)s %(name)s: %(message)s"))
    logging.basicConfig(level=logging.INFO, handlers=[handler])


def _write_response(response: BaseModel) -> None:
    sys.stdout.write(response.model_dump_json() + "\n")
    sys.stdout.flush()


def _handle_warmup() -> OkWarmup:
    # Silero load happens in F7; skeleton just reports ready.
    logger.info("warmup: skeleton (model load deferred to F7)")
    return OkWarmup(version=__version__)


def _handle_synthesize(req: SynthesizeRequest) -> OkSynthesize | ErrResponse:
    # Actual synthesis implemented in F7. Skeleton returns error.
    return ErrResponse(error="model_not_loaded", message="Silero model not loaded yet (F7 pending)")


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
            except ValidationError as e:
                _write_response(ErrResponse(error="bad_input", message=str(e)))
                continue
            if req.cmd == "warmup":
                _write_response(_handle_warmup())
            elif req.cmd == "synthesize":
                _write_response(_handle_synthesize(req))  # type: ignore[arg-type]
            elif req.cmd == "shutdown":
                _write_response(_handle_shutdown())
                return 0
    except KeyboardInterrupt:
        logger.info("interrupted")
        return 0
    except Exception:
        logger.exception("fatal error in ttsd main loop")
        return 1
    return 0
