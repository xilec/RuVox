"""E2E test for TTS synthesis pipeline.

Tests the full TTS pipeline including model loading and synthesis in
QThreadPool worker threads, which is the exact path used by the application.

The segfault regression being tested:
    When torch was imported for the first time inside a QThreadPool worker
    thread, the BERT TorchScript model (homosolver) would crash with SIGSEGV
    when processing text that contained Russian homographs (words like "один",
    "сорок", "временного"). The fix is to import torch at module level in
    tts_worker.py, ensuring C++ runtime initialization happens in the main
    thread before any Qt worker threads are started.
"""

import subprocess
import sys
from pathlib import Path

import pytest

PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
E2E_WORKER = PROJECT_ROOT / "tmp" / "tts_e2e_worker.py"
MODEL_CACHE = Path.home() / ".cache" / "torch" / "hub" / "snakers4_silero-models_master"
HISTORY_CACHE = Path.home() / ".cache" / "ruvox" / "history.json"

# Skip if model not cached locally (to avoid slow network downloads in CI)
requires_silero_model = pytest.mark.skipif(
    not MODEL_CACHE.exists(),
    reason="Silero model not in local cache (run the app once to download)",
)

# Skip if the problematic history entry is not present
requires_history_entry = pytest.mark.skipif(
    not HISTORY_CACHE.exists(),
    reason="~/.cache/ruvox/history.json not found",
)


def _has_b0d2b20e_entry() -> bool:
    """Check if the specific history entry that triggers the bug exists."""
    if not HISTORY_CACHE.exists():
        return False
    try:
        import json

        with open(HISTORY_CACHE) as f:
            history = json.load(f)
        return any("b0d2b20e" in str(e.get("id", "")) for e in history.get("entries", []))
    except Exception:
        return False


requires_b0d2b20e = pytest.mark.skipif(
    not _has_b0d2b20e_entry(),
    reason="History entry b0d2b20e (Moltbook analytics) not found in cache",
)


class TestTorchModuleLevelImport:
    """Unit tests verifying that torch is imported at module level.

    This is a regression guard: if someone removes the module-level torch
    import from tts_worker.py, these tests will fail before the segfault
    regression test can catch it.
    """

    def test_torch_imported_at_module_level(self):
        """tts_worker module must have torch as a module-level attribute.

        The module-level import ensures PyTorch's C++ runtime initializes
        in the main thread, preventing SIGSEGV in QThreadPool worker threads
        when the BERT TorchScript homosolver model processes homographs.
        """
        import ruvox.ui.services.tts_worker as tts_worker_module

        assert hasattr(tts_worker_module, "torch"), (
            "torch must be imported at module level in tts_worker.py. "
            "Without this, importing torch for the first time inside a "
            "QThreadPool worker causes SIGSEGV in the Silero BERT homosolver."
        )

    def test_torch_is_real_module(self):
        """Verify the torch attribute is the actual torch module, not a stub."""
        import ruvox.ui.services.tts_worker as tts_worker_module

        assert hasattr(tts_worker_module, "torch"), "torch not imported at module level"
        torch_mod = tts_worker_module.torch
        assert hasattr(torch_mod, "tensor"), "torch module missing .tensor — is it a stub?"
        assert hasattr(torch_mod, "no_grad"), "torch module missing .no_grad"


@requires_silero_model
@requires_b0d2b20e
class TestTTSSynthesisE2E:
    """E2E tests that run the full TTS synthesis in a subprocess.

    Uses the exact same code path as the application (TTSWorker → ModelLoadRunnable
    → TTSRunnable via QThreadPool) to detect regressions in the C-level crash.

    Slow: ~2-4 minutes per run (model loading + synthesis).
    """

    def test_synthesis_completes_without_crash(self):
        """Full TTS synthesis must complete successfully without SIGSEGV.

        Regression test for: first-time torch import inside QThreadPool worker
        causing SIGSEGV in Silero BERT homosolver when processing Russian
        homographs (один, сорок, временного, etc.) in the Moltbook analytics text.

        The crash manifested as:
            Fatal Python error: Segmentation fault
            File "<torch_package_0>.models/homosolver.py", line 35 in __call__
        """
        assert E2E_WORKER.exists(), f"E2E worker script not found: {E2E_WORKER}"

        result = subprocess.run(
            [sys.executable, str(E2E_WORKER)],
            capture_output=True,
            text=True,
            timeout=300,  # 5 minutes max
        )

        # Print output for debugging if test fails
        if result.returncode != 0:
            print("\n--- stdout ---")
            print(result.stdout)
            print("--- stderr ---")
            print(result.stderr)

        assert result.returncode == 0, (
            f"TTS synthesis crashed (exit code {result.returncode}). "
            f"This is likely the SIGSEGV regression: torch was imported "
            f"inside a QThreadPool worker thread instead of the main thread.\n"
            f"Last stdout: {result.stdout[-500:]}\n"
            f"Last stderr: {result.stderr[-500:]}"
        )
