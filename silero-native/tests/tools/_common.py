"""Shared helpers for the fixture generators in this directory.

The generators run inside the `silero-native/export` uv environment (pinned
torch CPU + onnxruntime); see README.md in this directory.
"""
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]

# The exporter module carries the canonical model resolution (torch hub cache
# -> models.yml download) and package loading. Importing it is side-effect
# free (graph surgery is only installed in its main()).
sys.path.insert(0, str(REPO / "silero-native" / "export"))

from export import load_package, resolve_model_path  # noqa: E402


def load_pack(model_path: str | None = None):
    """Load the v5_ru torch.package and return the dequantized model package.

    References MUST be taken from the dequantized model: `load_package` calls
    `unpack_q_model()` because the quantized homosolver resolves stress
    differently, and production `apply_tts` dequantizes before the first
    inference (the ONNX export was traced after dequantization as well).
    """
    pt_path, _url = resolve_model_path("v5_ru", model_path)
    _tts_model, pack = load_package(pt_path)
    return pack
