"""Pydantic v2 models for the ttsd JSON protocol (Layer 3 of docs/ipc-contract.md).

Design note: we use separate response classes (OkWarmup, OkSynthesize, OkShutdown) instead
of a single OkResponse with optional fields. This keeps response types self-documenting and
makes handler return types explicit. The trade-off is a few extra classes, which is acceptable
for a small, stable protocol.

The discriminated Request union uses Literal["cmd"] as the discriminator so Pydantic
selects the correct model from a single model_validate_json() call.
"""

from typing import Annotated, Literal

from pydantic import BaseModel, Field, TypeAdapter

# ---------------------------------------------------------------------------
# Shared sub-types
# ---------------------------------------------------------------------------


class CharMappingEntry(BaseModel):
    """Single entry in the optional char_mapping array passed with SynthesizeRequest."""

    norm_start: int
    norm_end: int
    orig_start: int
    orig_end: int


class WordTimestamp(BaseModel):
    """Word-level timestamp returned inside OkSynthesize."""

    word: str
    start: float
    end: float
    original_pos: tuple[int, int]


# ---------------------------------------------------------------------------
# Request models
# ---------------------------------------------------------------------------


class WarmupRequest(BaseModel):
    """Load the Silero model. Sent once at startup."""

    cmd: Literal["warmup"]


class SynthesizeRequest(BaseModel):
    """Synthesize speech from normalized text and write WAV to out_wav path."""

    cmd: Literal["synthesize"]
    text: str
    speaker: str
    sample_rate: int
    out_wav: str
    char_mapping: list[CharMappingEntry] | None = None


class ShutdownRequest(BaseModel):
    """Request graceful shutdown; ttsd exits after sending the response."""

    cmd: Literal["shutdown"]


# Discriminated union: Pydantic reads the "cmd" field to pick the right model.
Request = Annotated[
    WarmupRequest | SynthesizeRequest | ShutdownRequest,
    Field(discriminator="cmd"),
]

# TypeAdapter is required to call model_validate_json on an Annotated (non-class) type.
RequestAdapter: TypeAdapter[Request] = TypeAdapter(Request)


# ---------------------------------------------------------------------------
# Response models
# ---------------------------------------------------------------------------


class OkWarmup(BaseModel):
    """Successful response to a warmup request."""

    ok: Literal[True] = True
    version: str


class OkSynthesize(BaseModel):
    """Successful response to a synthesize request."""

    ok: Literal[True] = True
    timestamps: list[WordTimestamp]
    duration_sec: float


class OkShutdown(BaseModel):
    """Successful response to a shutdown request."""

    ok: Literal[True] = True


class ErrResponse(BaseModel):
    """Error response for any failed request."""

    ok: Literal[False] = False
    error: Literal["model_not_loaded", "synthesis_failed", "bad_input", "internal"]
    message: str
