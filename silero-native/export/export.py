#!/usr/bin/env python3
"""Silero TTS (torch.package) -> ONNX bundle exporter.

Exports a self-contained ONNX bundle from a Silero v5-style torch.package
(tested with v5_ru):

    tts_main.onnx         FastPitch + vocoder head (mag, cos, sin, dur_hat)
    istft.onnx            inverse STFT (48 kHz waveform)
    pqmf_24k.onnx         PQMF analysis filterbank, band 0 (48 kHz -> 24 kHz)
    pqmf_8k.onnx          PQMF analysis filterbank, band 0 (48 kHz -> 8 kHz)
    homosolver.onnx       homograph disambiguation BERT (dequantized)
    accentor_tensor.onnx  stress/yo classifiers over ngram embedding-bag
    ngrams.gz             accentor ngram dictionary (space-separated, index = position)
    exceptions.gz         accentor exceptions ("word stress_vowel yo_char" per line)
    homodict.json         homograph variants for the homosolver
    vocab.txt             BERT tokenizer vocab (token per line, index = line number)
    frontend.json         symbols / symbol_to_id / alphabet / speakers / constants
    manifest.json         versions, sha256, opset, file list

Usage:
    uv run python export.py --model v5_ru --out <dir> [--speaker aidar] [--no-self-check]

Model resolution order:
    1. --model-path <file> (explicit)
    2. torch hub cache: $TORCH_HOME/hub/snakers4_silero-models_master/src/silero/model/<model>.pt
    3. download from models.yml (tts_models.<lang>.<model>.latest.package)
       into ~/.cache/silero-onnx-export/
"""

import argparse
import gzip
import hashlib
import json
import math
import os
import re
import sys
import time
import wave
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F

OPSET = 17
MODELS_YML_URL = "https://raw.githubusercontent.com/snakers4/silero-models/master/models.yml"

# ---------------------------------------------------------------------------
# Model resolution
# ---------------------------------------------------------------------------


def hub_cache_path(model_id: str) -> Path:
    torch_home = Path(os.environ.get("TORCH_HOME", Path.home() / ".cache" / "torch"))
    return torch_home / "hub" / "snakers4_silero-models_master" / "src" / "silero" / "model" / f"{model_id}.pt"


def resolve_model_path(model_id: str, model_path: str | None) -> tuple[Path, str | None]:
    """Returns (path, source_url_or_None)."""
    if model_path:
        p = Path(model_path).expanduser()
        if not p.is_file():
            raise SystemExit(f"--model-path not found: {p}")
        return p, None
    cached = hub_cache_path(model_id)
    if cached.is_file():
        print(f"[resolve] using torch hub cache: {cached}")
        return cached, None

    import yaml

    print(f"[resolve] {model_id} not in cache, resolving URL from {MODELS_YML_URL}")
    with urllib.request.urlopen(MODELS_YML_URL) as resp:
        models_yml = yaml.safe_load(resp.read())
    url = None
    for lang, entries in models_yml["tts_models"].items():
        if isinstance(entries, dict) and model_id in entries:
            url = entries[model_id]["latest"]["package"]
            break
    if url is None:
        raise SystemExit(f"model '{model_id}' not found in models.yml tts_models")
    dest_dir = Path.home() / ".cache" / "silero-onnx-export"
    dest_dir.mkdir(parents=True, exist_ok=True)
    dest = dest_dir / f"{model_id}.pt"
    if not dest.is_file():
        print(f"[resolve] downloading {url} -> {dest}")
        urllib.request.urlretrieve(url, dest)
    return dest, url


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


# ---------------------------------------------------------------------------
# Package loading
# ---------------------------------------------------------------------------


def load_package(pt_path: Path):
    """Returns (tts_model, pack) where pack is PartTTSModelMultiAcc_v3."""
    torch.set_grad_enabled(False)
    imp = torch.package.PackageImporter(str(pt_path))
    tts_model = imp.load_pickle("tts_models", "model")
    pack = tts_model.packages[0]
    torch._C._jit_set_profiling_mode(False)
    pack.unpack_q_model()
    pack.q_model_unpacked = True
    return tts_model, pack


def build_model_input(pack, text: str, speaker: str):
    """Text -> (sequence, speaker_ids, durs_rate, pitch_coefs), accentor applied."""
    speaker_ids, _model_id = pack.get_speakers(speaker)
    sentences, _clean, break_lens, rates, pitches, sp_ids = pack.prepare_tts_model_input(
        text, ssml=False, speaker_ids=speaker_ids
    )
    sequence, _symb_durs, durs_rate, pitch_coefs = pack.merge_batch_model(
        sentences, break_lens, rates, pitches
    )
    return sequence, sp_ids, durs_rate, pitch_coefs


# ---------------------------------------------------------------------------
# Export wrapper modules
# ---------------------------------------------------------------------------


class TtsMainWrapper(nn.Module):
    """Narrows the JIT model forward() to the pure-inference signature.

    symb_durs (SSML pauses), gt_durs/gt_pitch (training) are dropped; sr is
    fixed at 48000 because downstream resampling is handled by the PQMF graphs.
    """

    def __init__(self, jit_model):
        super().__init__()
        self.jit_model = jit_model

    def forward(self, sequence, speaker_ids, durs_rate, pitch_coefs):
        (mag, x, y), dur_hat = self.jit_model(
            sequence, speaker_ids, 48000, None, durs_rate, pitch_coefs, None, None, "cpu"
        )
        return mag, x, y, dur_hat


class IstftExport(nn.Module):
    """ONNX-friendly ISTFT: irfft as matmul with precomputed DFT matrices,
    overlap-add via static pads (valid because win_length = 4 * hop_length).
    Mirrors tts_package/package_utils.py ISTFT with padding='same'."""

    def __init__(self, istft):
        super().__init__()
        n_fft, hop, win = istft.n_fft, istft.hop_length, istft.win_length
        assert win == 4 * hop, "OLA pad decomposition assumes win = 4 * hop"
        freqs = n_fft // 2 + 1
        f = torch.arange(freqs).float().unsqueeze(1)
        t = torch.arange(n_fft).float().unsqueeze(0)
        ang = 2 * math.pi * f * t / n_fft
        scale = torch.ones(freqs, 1)
        scale[1:-1] = 2.0
        self.register_buffer("Wr", torch.cos(ang) * scale / n_fft)
        self.register_buffer("Wi", -torch.sin(ang) * scale / n_fft)
        self.register_buffer("window", istft.window.clone())
        self.hop = hop
        self.win = win
        self.pad = (win - hop) // 2
        self.k = win // hop

    def ola(self, x):
        # x: (B, WIN, T) -> (B, HOP * (T + K - 1)) via overlap-add with static pads
        B, _, T = x.shape
        c = x.reshape(B, self.k, self.hop, T)
        outs = []
        for k in range(self.k):
            a = c[:, k].transpose(1, 2).reshape(B, -1)
            outs.append(F.pad(a, [k * self.hop, (self.k - 1 - k) * self.hop]))
        return sum(outs)

    def forward(self, mag, x, y):
        re = mag * x
        im = mag * y
        ifft = torch.matmul(re.transpose(1, 2), self.Wr) + torch.matmul(im.transpose(1, 2), self.Wi)
        ifft = ifft.transpose(1, 2) * self.window[None, :, None]  # (B, WIN, T)
        y_ = self.ola(ifft)[:, self.pad : -self.pad]
        T = ifft.shape[2]
        wsq = self.window.square().reshape(self.k, self.hop)  # (K, HOP)
        env = None
        for k in range(self.k):
            a = wsq[k].repeat(T)
            a = F.pad(a, [k * self.hop, (self.k - 1 - k) * self.hop])
            env = a if env is None else env + a
        env = env[self.pad : -self.pad]
        return y_ / env


def pqmf_analysis_filters(n_bands: int, taps: int, cutoff: float, beta: float) -> torch.Tensor:
    """PQMF analysis filterbank, identical to tts_package/package_utils.py PQMF.
    Computed once at export time via scipy.firwin and baked into the graph."""
    from scipy import signal as sig

    qmf = sig.firwin(taps + 1, cutoff, window=("kaiser", beta))
    H = np.zeros((n_bands, len(qmf)))
    for k in range(n_bands):
        constant_factor = (2 * k + 1) * (np.pi / (2 * n_bands)) * (np.arange(taps + 1) - ((taps - 1) / 2))
        phase = (-1) ** k * np.pi / 4
        H[k] = 2 * qmf * np.cos(constant_factor + phase)
    return torch.from_numpy(H[:, None, :]).float()


class PqmfExport(nn.Module):
    """PQMF analysis + band-0 selection: 48 kHz waveform -> target-rate waveform.
    Matches WrappedJitV: conv1d(H, stride=N, pad=taps//2), optional tanh, [:, :1, :]."""

    def __init__(self, filters: torch.Tensor, n_bands: int, taps: int):
        super().__init__()
        self.register_buffer("H", filters)
        self.n_bands = n_bands
        self.pad = taps // 2

    def forward(self, audio):
        bands = F.conv1d(audio, self.H, padding=self.pad, stride=self.n_bands)
        # package: x if max(|x|) <= 1 else tanh(x) — where() keeps both branches numeric-safe
        bands = torch.where(torch.max(torch.abs(bands)) <= 1.0, bands, torch.tanh(bands))
        return bands[:, :1, :]


class AccentorTensorExport(nn.Module):
    """Tensor part of AccentorNgram: embedding_bag(mean) + stress/yo classifiers.
    The string part (tokenize, char ngrams, dict lookup) is reimplemented by
    the consumer; the exact ngram algorithm is documented in frontend.json."""

    def __init__(self, acc_model):
        super().__init__()
        self.emb_w = nn.Parameter(acc_model.embedding.weight.data.clone(), requires_grad=False)
        self.stress_clf = acc_model.stress_clf
        self.yo_clf = acc_model.yo_clf

    def forward(self, ind, offsets):
        e = F.embedding_bag(ind, self.emb_w, offsets, mode="mean")
        return self.stress_clf(e), self.yo_clf(e)


def word_ngrams(text: str, min_len: int, max_len: int) -> list[str]:
    """Exact copy of the ngram extraction inside the accentor JIT model
    (pkg .data/ts_code/code/__torch__/models/modules.py word_ngrams)."""
    grams = []
    ext = "<" + text + ">"
    for i in range(min_len, max_len + 1):
        for j in range(len(ext) - i + 1):
            grams.append(ext[j : j + i])
    if len(text) < min_len:
        grams.append(text)
    return grams


def accentor_inputs(ngram_dict: dict, words: list[str]) -> tuple[torch.Tensor, torch.Tensor]:
    ind, offsets = [], [0]
    for word in words:
        grams = word_ngrams(word, 1, len(word) + 3)
        ids = [ngram_dict[g] for g in grams if g in ngram_dict] or [ngram_dict["UNK"]]
        ind += ids
        offsets.append(offsets[-1] + len(ids))
    return torch.tensor(ind), torch.tensor(offsets[:-1])


# ---------------------------------------------------------------------------
# JIT graph surgery for tts_main.onnx
#
# The TorchScript graph of the main model does not export as-is. The surgery
# below runs inside _jit_pass_onnx_remove_inplace_ops_for_onnx (the last pass
# that sees the inlined JIT graph before ONNX conversion) and rewrites:
#   - aten::format / string prim::If leaves (MHA fast-path error messages) -> ""
#   - aten::__is__/__isnot__ identity checks -> constant bools
#     (valid here: every MHA in this model is pure self-attention, q is k is v)
#   - prim::isinstance on list values (pad_sequence guard) -> False
#   - LengthRegulator batch loop + pad_sequence -> a single repeat_interleave
#     (batch is always 1: pad_sequence([ri(x[0], dur[0]+0.5)]) == ri(x, dur+0.5, dim=1))
#   - pad_to_multiple prim::If (L % f == 0 ? x : pad(x)) -> always zeros+cat
#     (remainder adjusted to stay a no-op when L % f == 0)
#   - aten::pad with a dynamic pad list -> zeros+cat along the padded dim
# plus a generic rank-repair pass for tensor types stripped during inlining.
# ---------------------------------------------------------------------------

_C = torch._C


def _install_graph_surgery():
    orig_remove_inplace = _C._jit_pass_onnx_remove_inplace_ops_for_onnx

    def surgery(graph, module):
        _rewrite_strings_and_identity(graph)
        _C._jit_pass_constant_propagation(graph)
        _C._jit_pass_dce(graph)
        _repair_ranks(graph)
        _rewrite_length_regulator(graph)
        _rewrite_pad_to_multiple(graph)
        _rewrite_dynamic_pad(graph)
        _C._jit_pass_dce(graph)
        return orig_remove_inplace(graph, module)

    _C._jit_pass_onnx_remove_inplace_ops_for_onnx = surgery


def _rewrite_strings_and_identity(graph):
    def walk(block):
        for node in list(block.nodes()):
            for b in node.blocks():
                walk(b)
            kind = node.kind()
            if kind == "aten::format":
                const = graph.create("prim::Constant")
                const.s_("value", "")
                const.insertBefore(node)
                const.output().setType(node.output().type())
                node.output().replaceAllUsesWith(const.output())
                node.destroy()
            elif kind in ("aten::__is__", "aten::__isnot__"):
                _fold_identity_check(graph, node)
            elif kind == "aten::_native_multi_head_attention":
                for o in node.outputs():
                    o.setType(torch._C.TensorType.get().with_sizes([None, None, None]))
            elif kind == "prim::isinstance":
                # pad_sequence's isinstance(x, Tensor) check on a Tensor[] -> False
                if "[]" in str(node.inputsAt(0).type()):
                    const = graph.create("prim::Constant")
                    const.i_("value", 0)
                    const.insertBefore(node)
                    const.output().setType(node.output().type())
                    node.output().replaceAllUsesWith(const.output())
                    node.destroy()
            elif kind == "prim::If":
                # fast-path error-message Ifs: all leaves are strings -> force slow path
                for o in node.outputs():
                    if "str" in str(o.type()).lower():
                        const = graph.create("prim::Constant")
                        const.s_("value", "")
                        const.insertBefore(node)
                        const.output().setType(o.type())
                        o.replaceAllUsesWith(const.output())
                if node.outputsSize() > 0 and all(len(o.uses()) == 0 for o in node.outputs()):
                    node.destroy()
                elif node.outputsSize() == 0 and (
                    _if_is_assert(node) or _if_cond_has_kind(node, "aten::__contains__")
                ):
                    # assert-style Ifs (RaiseException) cannot be represented
                    # in ONNX; conditions are guaranteed by the caller.
                    # The punctuation pitch-zeroing If (__contains__ + slice
                    # copy_ inside a Loop) fails ONNX conversion and is dropped
                    # (spike behavior; acoustic impact is negligible — pitch of
                    # unvoiced pause frames, measured e2e impact <= 1e-3).
                    node.destroy()
                # NOTE: other zero-output Ifs mutate tensors in place (the
                # dur_hat sos/eos clamps `if dur[0] > 5: dur[0] = 5` etc.) and
                # MUST be kept — the stock remove-inplace pass converts them
                # to functional form. The spike destroyed ALL zero-output Ifs
                # (all([]) == True), silently dropping the clamps.

    walk(graph)


def _if_is_assert(node) -> bool:
    for block in node.blocks():
        for n in block.nodes():
            if n.kind() == "prim::RaiseException":
                return True
    return False


def _if_cond_has_kind(node, kind: str, max_depth: int = 6) -> bool:
    """BFS from the If condition value upward through producer inputs."""
    seen = set()
    stack = [node.inputsAt(0)]
    while stack and max_depth > 0:
        max_depth -= 1
        nxt = []
        for v in stack:
            prod = v.node()
            if prod.kind() == kind:
                return True
            if prod in seen:
                continue
            seen.add(prod)
            nxt.extend(prod.inputs())
        stack = nxt
    return False


def _fold_identity_check(graph, node):
    t0s = str(node.inputsAt(0).type()).lower()
    t1s = str(node.inputsAt(1).type()).lower()
    none_involved = "none" in t0s or "none" in t1s
    same = node.inputsAt(0).unique() == node.inputsAt(1).unique()
    none_side = None
    for cand in (node.inputsAt(0), node.inputsAt(1)):
        if "none" in str(cand.type()).lower():
            none_side = cand
    if none_side is not None:
        other = node.inputsAt(1) if none_side.unique() == node.inputsAt(0).unique() else node.inputsAt(0)
        is_none = other.node().kind() == "prim::Constant" and "none" in str(other.type()).lower()
        val = is_none if node.kind() == "aten::__is__" else (not is_none)
    elif same or not none_involved:
        # identical SSA value, or tensor-tensor identity check: all MHAs here
        # are pure self-attention (q is k is v) -> True
        val = node.kind() == "aten::__is__"
    else:
        return
    const = graph.create("prim::Constant")
    const.i_("value", 1 if val else 0)
    const.output().setType(torch._C.BoolType.get())
    const.insertBefore(node)
    node.output().replaceAllUsesWith(const.output())
    node.destroy()


_RANK_SAME = {
    "aten::layer_norm", "aten::dropout", "aten::add", "aten::mul", "aten::sub",
    "aten::div", "aten::exp", "aten::sin", "aten::cos", "aten::tanh", "aten::gelu",
    "aten::sigmoid", "aten::softmax", "aten::log_softmax", "aten::linear", "aten::clone",
    "aten::contiguous", "aten::masked_fill", "aten::clamp", "aten::round",
    "aten::transpose", "aten::slice", "aten::narrow", "aten::cat", "aten::where",
    "aten::cumsum", "aten::relu", "aten::erf", "aten::sqrt", "aten::rsqrt", "aten::pow",
    "aten::neg", "aten::abs", "aten::floor", "aten::ceil", "aten::log", "aten::index_put",
    "aten::conv1d", "aten::conv_transpose1d", "aten::type_as", "aten::detach",
    "aten::fill_", "aten::zero_", "aten::copy_", "aten::expand", "aten::repeat",
    "aten::roll", "aten::flip", "aten::pad", "aten::constant_pad_nd", "aten::sign",
}
_RANK_MATMUL = {"aten::matmul", "aten::bmm", "aten::baddbmm", "aten::mm"}


def _repair_ranks(graph):
    """Generic rank repair for tensor types stripped by inlining."""
    ranks = {}

    def trank(v):
        if v in ranks:
            return ranks[v]
        try:
            return v.type().dim()
        except Exception:
            return None

    for node in graph.nodes():
        kind = node.kind()
        if kind not in _RANK_SAME and kind not in _RANK_MATMUL:
            continue
        ins = [r for r in (trank(i) for i in node.inputs()) if r is not None]
        if not ins:
            continue
        r = max(ins[:2]) if kind in _RANK_MATMUL else ins[0]
        for o in node.outputs():
            try:
                if "Tensor" in str(o.type()) and o.type().dim() is None:
                    t_in = node.inputsAt(0).type()
                    if getattr(t_in, "dim", lambda: None)() is not None:
                        o.setType(t_in)
                    else:
                        o.setType(torch._C.TensorType.get().with_dtype(torch.float32).with_sizes([None] * r))
                    ranks[o] = r
            except Exception:
                pass


def _mk_const_int(graph, v, before):
    c = graph.create("prim::Constant")
    c.i_("value", v)
    c.output().setType(torch._C.IntType.get())
    c.insertBefore(before)
    return c.output()


def _mk_const_float(graph, v, before):
    c = graph.create("prim::Constant")
    c.f_("value", v)
    c.output().setType(torch._C.FloatType.get())
    c.insertBefore(before)
    return c.output()


def _mk_const_none(graph, before):
    c = graph.create("prim::Constant")
    c.output().setType(torch._C.NoneType.get())
    c.insertBefore(before)
    return c.output()


def _tensor_t(rank=None):
    t = torch._C.TensorType.get()
    if rank is not None:
        t = t.with_sizes([None] * rank)
    return t


def _rank_of(v, default=None):
    try:
        d = v.type().dim()
        return d if d is not None else default
    except Exception:
        return default


def _rewrite_length_regulator(graph):
    """B=1 rewrite: batch loop { append(repeat_interleave(x[i], dur[i]+0.5)) }
    + pad_sequence -> unsqueeze-free single repeat_interleave(x, dur+0.5, dim=1)."""
    for node in list(graph.nodes()):
        if node.kind() != "aten::pad_sequence":
            continue
        list_v = node.inputsAt(0)
        prod = list_v.node()
        if prod.kind() == "prim::If":
            # else-branch returns the (loop-mutated) list
            list_v = list(prod.blocks())[1].returnNode().inputsAt(0)
            prod = list_v.node()
        ri_node = None
        loop_node = None
        for n in graph.nodes():
            if n.kind() == "prim::Loop":
                for b in n.blocks():
                    for bn in b.nodes():
                        if bn.kind() == "aten::append" and bn.inputsAt(0).unique() == list_v.unique():
                            ri_node = bn.inputsAt(1).node()
                            loop_node = n
        if ri_node is None:
            x_exp = None
            if prod.kind() == "prim::ListConstruct" and prod.inputsSize() == 1:
                x_exp = prod.inputsAt(0)
            else:
                for u in list_v.uses():
                    if u.user.kind() == "aten::append":
                        x_exp = u.user.inputsAt(1)
            if x_exp is not None and x_exp.node().kind() == "aten::repeat_interleave":
                ri_node = x_exp.node()
        if ri_node is None or ri_node.kind() != "aten::repeat_interleave":
            print("[surgery] pad_sequence: could not locate inner repeat_interleave")
            continue
        sel_x = ri_node.inputsAt(0).node()
        add_n = ri_node.inputsAt(1).node()  # aten::to or aten::add
        while add_n.kind() not in ("aten::add", "aten::select"):
            add_n = add_n.inputsAt(0).node()
        sel_d = add_n.inputsAt(0).node() if add_n.kind() == "aten::add" else add_n
        if sel_x.kind() != "aten::select" or sel_d.kind() != "aten::select":
            print(f"[surgery] pad_sequence: unexpected ri inputs: {sel_x.kind()} {sel_d.kind()}")
            continue
        x_val, dur_val = sel_x.inputsAt(0), sel_d.inputsAt(0)
        anchor = node
        cm1 = _mk_const_int(graph, -1, anchor)
        c1 = _mk_const_int(graph, 1, anchor)
        half = _mk_const_float(graph, 0.5, anchor)
        shape_l = graph.create("prim::ListConstruct", [cm1])
        shape_l.output().setType(torch._C.ListType.ofInts())
        shape_l.insertBefore(anchor)
        d = graph.create("aten::reshape", [dur_val, shape_l.output()])
        d.insertBefore(anchor)
        d.output().setType(_tensor_t(1))
        da = graph.create("aten::add", [d.output(), half, c1])
        da.insertBefore(anchor)
        da.output().setType(_tensor_t(1))
        ri = graph.create("aten::repeat_interleave", [x_val, da.output(), c1, _mk_const_none(graph, anchor)])
        ri.insertBefore(anchor)
        ri.output().setType(_tensor_t(_rank_of(x_val, 3)))
        node.output().replaceAllUsesWith(ri.output())
        if loop_node is not None and all(len(o.uses()) == 0 for o in loop_node.outputs()):
            loop_node.destroy()


def _rewrite_pad_to_multiple(graph):
    """prim::If (L % factor == 0 ? x : pad(x)) -> always-pad via zeros+cat with
    pad = (factor - L % factor) % factor (no-op when already aligned)."""
    for node in list(graph.nodes()):
        if node.kind() != "prim::If" or node.outputsSize() != 1:
            continue
        cond = node.inputsAt(0).node()
        if cond.kind() != "aten::eq":
            continue
        rem_n = cond.inputsAt(0).node()
        if rem_n.kind() != "aten::remainder":
            continue
        seq_len, factor = rem_n.inputsAt(0), rem_n.inputsAt(1)
        blocks = list(node.blocks())
        x_val = blocks[0].returnNode().inputsAt(0)
        xr = _rank_of(x_val)
        if xr not in (2, 3):
            print(f"[surgery] pad_to_multiple: unexpected rank {xr}")
            continue
        anchor = node
        m = graph.create("aten::remainder", [seq_len, factor])
        m.output().setType(torch._C.IntType.get())
        m.insertBefore(anchor)
        r = graph.create("aten::sub", [factor, m.output()])
        r.output().setType(torch._C.IntType.get())
        r.insertBefore(anchor)
        rem2 = graph.create("aten::remainder", [r.output(), factor])
        rem2.output().setType(torch._C.IntType.get())
        rem2.insertBefore(anchor)
        sh = graph.create("aten::size", [x_val])
        sh.output().setType(torch._C.ListType.ofInts())
        sh.insertBefore(anchor)

        def sh_get(i, _sh=sh, _anchor=anchor):
            ci = _mk_const_int(graph, i, _anchor)
            gi = graph.create("aten::__getitem__", [_sh.output(), ci])
            gi.output().setType(torch._C.IntType.get())
            gi.insertBefore(_anchor)
            return gi.output()

        dims = [sh_get(0), rem2.output()] + ([sh_get(2)] if xr == 3 else [])
        zl = graph.create("prim::ListConstruct", dims)
        zl.output().setType(torch._C.ListType.ofInts())
        zl.insertBefore(anchor)
        try:
            st = x_val.type().scalarType()
        except Exception:
            st = None
        cdt = _mk_const_int(graph, 6 if st is None or str(st).lower() == "float" else 11, anchor)
        zeros = graph.create(
            "aten::zeros",
            [zl.output(), cdt, _mk_const_none(graph, anchor), _mk_const_none(graph, anchor), _mk_const_none(graph, anchor)],
        )
        zeros.output().setType(_tensor_t(xr))
        zeros.insertBefore(anchor)
        cl = graph.create("prim::ListConstruct", [x_val, zeros.output()])
        cl.output().setType(torch._C.ListType.ofTensors())
        cl.insertBefore(anchor)
        cd = _mk_const_int(graph, 1, anchor)
        cat = graph.create("aten::cat", [cl.output(), cd])
        cat.output().setType(_tensor_t(xr))
        cat.insertBefore(anchor)
        node.output().replaceAllUsesWith(cat.output())
        node.destroy()


_DTYPE_TO_INT = {"float": 6, "double": 7, "long": 4, "int": 3, "bool": 11, "short": 2}


def _rewrite_dynamic_pad(graph):
    """aten::pad(x, dyn_list, 'constant', v) -> cat([x, zeros]) along the padded dim."""
    for node in list(graph.nodes()):
        if node.kind() != "aten::pad":
            continue
        x = node.inputsAt(0)
        lst = node.inputsAt(1).node()
        if lst.kind() != "prim::ListConstruct":
            print(f"[surgery] aten::pad: pad arg is not ListConstruct: {lst.kind()}")
            continue
        elems = list(lst.inputs())
        rem = elems[-1]
        xr = _rank_of(x)
        if xr not in (2, 3):
            print(f"[surgery] aten::pad: unexpected input rank {xr}")
            continue
        pad_dim = xr - 1 if len(elems) == 2 else xr - 2
        anchor = node
        sh = graph.create("aten::size", [x])
        sh.output().setType(torch._C.ListType.ofInts())
        sh.insertBefore(anchor)

        def sh_get(i, _sh=sh, _anchor=anchor):
            ci = _mk_const_int(graph, i, _anchor)
            gi = graph.create("aten::__getitem__", [_sh.output(), ci])
            gi.output().setType(torch._C.IntType.get())
            gi.insertBefore(_anchor)
            return gi.output()

        dims = [rem if i == pad_dim else sh_get(i) for i in range(xr)]
        zl = graph.create("prim::ListConstruct", dims)
        zl.output().setType(torch._C.ListType.ofInts())
        zl.insertBefore(anchor)
        try:
            st = x.type().scalarType()
        except Exception:
            st = None
        dt_int = _DTYPE_TO_INT.get(str(st).lower() if st else "float", 6)
        zeros = graph.create(
            "aten::zeros",
            [zl.output(), _mk_const_int(graph, dt_int, anchor), _mk_const_none(graph, anchor),
             _mk_const_none(graph, anchor), _mk_const_none(graph, anchor)],
        )
        zeros.output().setType(_tensor_t(xr))
        zeros.insertBefore(anchor)
        cl = graph.create("prim::ListConstruct", [x, zeros.output()])
        cl.output().setType(torch._C.ListType.ofTensors())
        cl.insertBefore(anchor)
        cd = _mk_const_int(graph, pad_dim, anchor)
        cat = graph.create("aten::cat", [cl.output(), cd])
        cat.output().setType(_tensor_t(xr))
        cat.insertBefore(anchor)
        node.output().replaceAllUsesWith(cat.output())
        node.destroy()


def _install_type_preservation():
    """Keep surgery-assigned ranks: per-node shape inference can clobber them,
    so re-derive ranks for the ops we create after each inference call."""
    orig_node_infer = _C._jit_pass_onnx_node_shape_type_inference

    def node_infer(node, params_dict, opset_version):
        orig_node_infer(node, params_dict, opset_version)
        kind = node.kind()
        if kind in ("aten::unsqueeze", "aten::select", "aten::repeat_interleave", "aten::add"):
            for o in node.outputs():
                try:
                    if o.type().dim() is not None:
                        continue
                except Exception:
                    pass
                try:
                    ir = node.inputsAt(0).type().dim()
                except Exception:
                    ir = None
                if ir is None:
                    continue
                if kind == "aten::unsqueeze":
                    r = ir + 1
                elif kind == "aten::select":
                    r = ir - 1
                else:
                    r = ir
                o.setType(torch._C.TensorType.get().with_sizes([None] * r))

    _C._jit_pass_onnx_node_shape_type_inference = node_infer

    import torch.onnx._internal.torchscript_exporter.utils as onnx_utils

    orig_rsf = onnx_utils._run_symbolic_function

    def rsf(graph, block, node, inputs, env, values_in_env, new_nodes, operator_export_type=None):
        if operator_export_type is None:
            out = orig_rsf(graph, block, node, inputs, env, values_in_env, new_nodes)
        else:
            out = orig_rsf(graph, block, node, inputs, env, values_in_env, new_nodes, operator_export_type)
        # propagate repaired JIT output types onto symbolic return values
        try:
            vals = list(out) if isinstance(out, (list, tuple)) else [out]
            for jv, nv in zip(node.outputs(), vals):
                if nv is None:
                    continue
                try:
                    d_nv = nv.type().dim()
                except Exception:
                    d_nv = None
                try:
                    d_jv = jv.type().dim()
                except Exception:
                    d_jv = None
                if d_nv is None and d_jv is not None:
                    try:
                        nv.setType(jv.type())
                    except Exception:
                        pass
        except Exception:
            pass
        return out

    onnx_utils._run_symbolic_function = rsf


# ---------------------------------------------------------------------------
# Custom ONNX symbolics (opset 17)
# ---------------------------------------------------------------------------


def _register_symbolics():
    from onnx import TensorProto
    from torch.onnx._internal.torchscript_exporter import symbolic_helper as sh

    torch.onnx.register_custom_op_symbolic("prim::abs", lambda g, x: g.op("Abs", x), OPSET)

    def repeat_interleave_symbolic(g, self, repeats, dim, output_size):
        # decomposition for 1-D int repeats (LengthRegulator)
        axis0 = g.op("Constant", value_t=torch.tensor(0, dtype=torch.int64))
        r = g.op("Cast", repeats, to_i=TensorProto.INT64)
        r = g.op("Reshape", r, g.op("Constant", value_t=torch.tensor([-1], dtype=torch.int64)))
        cs = g.op("CumSum", r, axis0)
        starts = g.op("Sub", cs, r)
        total = g.op("ReduceSum", r, keepdims_i=1)
        zeros = g.op("ConstantOfShape", total, value_t=torch.tensor([0.0], dtype=torch.float32))
        ones = g.op("ConstantOfShape", g.op("Shape", starts), value_t=torch.tensor([1.0], dtype=torch.float32))
        mask = g.op("ScatterElements", zeros, starts, ones, axis_i=0)
        idx = g.op("CumSum", mask, axis0)
        idx = g.op("Sub", idx, g.op("Constant", value_t=torch.tensor([1.0], dtype=torch.float32)))
        idx = g.op("Clip", idx, g.op("Constant", value_t=torch.tensor(0.0, dtype=torch.float32)))
        idx = g.op("Cast", idx, to_i=TensorProto.INT64)
        d = sh._maybe_get_scalar(dim)
        if not isinstance(d, int):
            d = 1
        gath = g.op("Gather", self, idx, axis_i=d)
        # reshape to [..., total, ...] preserving rank
        shp = g.op("Shape", self)
        big = g.op("Constant", value_t=torch.tensor([2147483647], dtype=torch.int64))
        pre = g.op("Slice", shp, g.op("Constant", value_t=torch.tensor([0], dtype=torch.int64)),
                   g.op("Constant", value_t=torch.tensor([d], dtype=torch.int64)))
        post = g.op("Slice", shp, g.op("Constant", value_t=torch.tensor([d + 1], dtype=torch.int64)), big)
        shape = g.op("Concat", pre, total, post, axis_i=0)
        out = g.op("Reshape", gath, shape)
        try:
            rr = self.type().dim()
        except Exception:
            rr = None
        if rr is None:
            rr = 3
        out.setType(torch._C.TensorType.get().with_sizes([None] * rr))
        return out

    torch.onnx.register_custom_op_symbolic("aten::repeat_interleave", repeat_interleave_symbolic, OPSET)

    def nmha_symbolic(g, query, key, value, embed_dim, num_heads,
                      qkv_w, qkv_b, proj_w, proj_b, mask,
                      need_weights, average_attn_weights, is_causal=None):
        E = sh._maybe_get_scalar(embed_dim)
        H = sh._maybe_get_scalar(num_heads)
        if E is None or H is None:
            try:
                E = qkv_w.type().sizes()[1]
            except Exception:
                E = None
            if E is None:
                raise RuntimeError("nmha: cannot resolve embed_dim")
            H = 2 if H is None else H  # all heads = 2 in this model family
        E = int(E)
        H = int(H)
        D = E // H

        def ci(vals):
            return g.op("Constant", value_t=torch.tensor(vals, dtype=torch.int64))

        def cf(v):
            return g.op("Constant", value_t=torch.tensor(v, dtype=torch.float32))

        def sl(x, start, end):
            return g.op("Slice", x, ci([start]), ci([end]), ci([2]))

        qkv = g.op("MatMul", query, g.op("Transpose", qkv_w, perm_i=[1, 0]))
        qkv = g.op("Add", qkv, qkv_b)
        qs, ks, vs = sl(qkv, 0, E), sl(qkv, E, 2 * E), sl(qkv, 2 * E, 3 * E)
        shp = g.op("Shape", query)
        B = g.op("Gather", shp, ci([0]), axis_i=0)
        L = g.op("Gather", shp, ci([1]), axis_i=0)
        shape4 = g.op("Concat", B, L, ci([H]), ci([D]), axis_i=0)
        shape3 = g.op("Concat", B, L, ci([E]), axis_i=0)

        def split_heads(x):
            x = g.op("Reshape", x, shape4)
            return g.op("Transpose", x, perm_i=[0, 2, 1, 3])

        q, k, v = split_heads(qs), split_heads(ks), split_heads(vs)
        scores = g.op("MatMul", q, g.op("Transpose", k, perm_i=[0, 1, 3, 2]))
        scores = g.op("Mul", scores, cf(1.0 / math.sqrt(D)))
        if mask is not None and not (hasattr(mask, "node") and mask.node().kind() == "prim::Constant"):
            m = g.op("Cast", mask, to_i=TensorProto.FLOAT)  # bool key padding mask, True = pad
            m = g.op("Unsqueeze", m, ci([1, 2]))  # (B,1,1,L)
            scores = g.op("Add", scores, g.op("Mul", m, cf(-1e9)))
        attn = g.op("Softmax", scores, axis_i=-1)
        out = g.op("MatMul", attn, v)
        out = g.op("Transpose", out, perm_i=[0, 2, 1, 3])
        out = g.op("Reshape", out, shape3)
        out = g.op("MatMul", out, g.op("Transpose", proj_w, perm_i=[1, 0]))
        out = g.op("Add", out, proj_b)
        out.setType(torch._C.TensorType.get().with_sizes([None, None, None]))
        return out, out

    torch.onnx.register_custom_op_symbolic("aten::_native_multi_head_attention", nmha_symbolic, OPSET)


# ---------------------------------------------------------------------------
# Component exporters
# ---------------------------------------------------------------------------


def _ort_run(onnx_path: Path, inputs: dict):
    import onnxruntime as ort

    sess = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])
    return sess.run(None, inputs)


def export_tts_main(pack, out_dir: Path, speaker: str):
    print("[export] tts_main.onnx ...")
    t0 = time.time()
    jm = pack.models[0]
    wrapper = torch.jit.script(TtsMainWrapper(jm))
    sequence, sp_ids, durs_rate, pitch_coefs = build_model_input(
        pack, "Привет, это тестовый текст для проверки синтеза речи.", speaker
    )
    # capture the reference BEFORE the export: the export passes mutate the
    # loaded JIT model's graph state, afterwards torch inference drifts
    with torch.no_grad():
        ref_mag, ref_x, ref_y, ref_dur = wrapper(sequence, sp_ids, durs_rate, pitch_coefs)
    import torch.onnx._internal.torchscript_exporter.utils as onnx_utils

    saved = (
        _C._jit_pass_onnx_remove_inplace_ops_for_onnx,
        _C._jit_pass_onnx_node_shape_type_inference,
        onnx_utils._run_symbolic_function,
    )
    _install_graph_surgery()
    _install_type_preservation()
    _register_symbolics()
    try:
        torch.onnx.export(
            wrapper,
            (sequence, sp_ids, durs_rate, pitch_coefs),
            str(out_dir / "tts_main.onnx"),
            input_names=["sequence", "speaker_ids", "durs_rate", "pitch_coefs"],
            output_names=["mag", "x", "y", "dur_hat"],
            dynamic_axes={
                "sequence": {1: "len"},
                "durs_rate": {1: "len"},
                "pitch_coefs": {1: "len"},
                "mag": {2: "mel_len"},
                "x": {2: "mel_len"},
                "y": {2: "mel_len"},
                "dur_hat": {1: "len"},
            },
            opset_version=OPSET,
            dynamo=False,
        )
    finally:
        # the surgery is only valid for the main model graph; other exporters
        # must run under stock passes
        (_C._jit_pass_onnx_remove_inplace_ops_for_onnx,
         _C._jit_pass_onnx_node_shape_type_inference,
         onnx_utils._run_symbolic_function) = saved
    mag_o, x_o, y_o, dur_o = _ort_run(
        out_dir / "tts_main.onnx",
        {"sequence": sequence.numpy(), "speaker_ids": sp_ids.numpy(),
         "durs_rate": durs_rate.numpy(), "pitch_coefs": pitch_coefs.numpy()},
    )
    assert np.array_equal(ref_dur.numpy(), dur_o), "tts_main dur_hat diverged from torch"
    for name, r, o, tol in [("mag", ref_mag, mag_o, 5e-2), ("x", ref_x, x_o, 5e-2), ("y", ref_y, y_o, 5e-2)]:
        d = float(np.abs(r.numpy() - o).max())
        assert d < tol, f"tts_main {name} diverged from torch: {d}"
    print(f"[export] tts_main.onnx done ({time.time() - t0:.1f}s, "
          f"ORT parity: dur exact, mag {float(np.abs(ref_mag.numpy() - mag_o).max()):.2e})")


def export_istft(pack, out_dir: Path):
    print("[export] istft.onnx ...")
    t0 = time.time()
    istft = pack.wrapped_jit_v.istft
    wrapper = IstftExport(istft)
    freqs = istft.n_fft // 2 + 1
    B, T = 1, 100
    mag = torch.rand(B, freqs, T) * 2
    x = torch.randn(B, freqs, T) * 0.5
    y = torch.randn(B, freqs, T) * 0.5
    # sanity: parity against the package ISTFT before exporting
    with torch.no_grad():
        ref = istft(mag * (x + 1j * y))
        mine = wrapper(mag, x, y)
    diff = (ref - mine).abs().max().item()
    assert diff < 1e-5, f"ISTFT wrapper diverged from package: {diff}"
    torch.onnx.export(
        wrapper,
        (mag, x, y),
        str(out_dir / "istft.onnx"),
        input_names=["mag", "x", "y"],
        output_names=["audio"],
        dynamic_axes={"mag": {2: "T"}, "x": {2: "T"}, "y": {2: "T"}},
        opset_version=OPSET,
        dynamo=False,
    )
    (audio_o,) = _ort_run(out_dir / "istft.onnx", {"mag": mag.numpy(), "x": x.numpy(), "y": y.numpy()})
    ort_diff = float(np.abs(ref.numpy() - audio_o).max())
    assert ort_diff < 1e-4, f"istft.onnx diverged from torch: {ort_diff}"
    print(f"[export] istft.onnx done ({time.time() - t0:.1f}s, torch parity {diff:.2e}, ORT parity {ort_diff:.2e})")


def export_pqmf(pack, out_dir: Path):
    # (file name, target rate, N bands, taps, cutoff, beta) — parameters from
    # tts_package/package_utils.py WrappedJitV (pqmf_2 for 24k, pqmf_6 for 8k)
    for name, rate, n_bands, taps, cutoff, beta in [
        ("pqmf_24k.onnx", 24000, 2, 62, 0.25, 10.0),
        ("pqmf_8k.onnx", 8000, 6, 62, 0.12, 9.0),
    ]:
        print(f"[export] {name} ...")
        t0 = time.time()
        filters = pqmf_analysis_filters(n_bands, taps, cutoff, beta)
        # validate our recomputed filters against the package's own buffers
        pkg_pqmf = pack.wrapped_jit_v.pqmf_2 if n_bands == 2 else pack.wrapped_jit_v.pqmf_6
        fdiff = (filters - pkg_pqmf.H).abs().max().item()
        assert fdiff < 1e-6, f"PQMF filters diverged from package: {fdiff}"
        wrapper = torch.jit.script(PqmfExport(filters, n_bands, taps))
        audio = torch.randn(1, 1, 4800)
        with torch.no_grad():
            ref = pkg_pqmf(audio)[:, :1, :]
            mine = wrapper(audio)
        diff = (ref - mine).abs().max().item()
        assert diff < 1e-6, f"PQMF wrapper diverged from package: {diff}"
        torch.onnx.export(
            wrapper,
            (audio,),
            str(out_dir / name),
            input_names=["audio"],
            output_names=["band0"],
            dynamic_axes={"audio": {2: "T"}, "band0": {2: "T_out"}},
            opset_version=OPSET,
            dynamo=False,
        )
        (band_o,) = _ort_run(out_dir / name, {"audio": audio.numpy()})
        ort_diff = float(np.abs(ref.numpy() - band_o).max())
        assert ort_diff < 1e-5, f"{name} diverged from torch: {ort_diff}"
        print(f"[export] {name} done ({time.time() - t0:.1f}s, rate {rate}, "
              f"torch parity {diff:.2e}, ORT parity {ort_diff:.2e})")


def export_homosolver(pack, out_dir: Path):
    print("[export] homosolver.onnx ...")
    t0 = time.time()
    hs = pack.accentor.homosolver  # word embeddings already dequantized by unpack_q_model
    tok = hs.tokenizer
    a = tok("это [HOMO] тому [/HOMO] слово")
    b = tok("я [HOMO] замок [/HOMO] открыл дверь")
    L = max(len(a), len(b))
    pad = lambda v: v + [tok.pad_token_id] * (L - len(v))
    input_ids = torch.tensor([pad(a), pad(b)])
    starts = torch.tensor([a.index(tok.homo_start_id), b.index(tok.homo_start_id)])
    ends = torch.tensor([a.index(tok.homo_end_id), b.index(tok.homo_end_id)])
    with torch.no_grad():
        ref = hs.model(input_ids, starts, ends)
    torch.onnx.export(
        hs.model,
        (input_ids, starts, ends),
        str(out_dir / "homosolver.onnx"),
        input_names=["input_ids", "homo_start_ids", "homo_end_ids"],
        output_names=["logits"],
        dynamic_axes={
            "input_ids": {0: "batch", 1: "len"},
            "homo_start_ids": {0: "batch"},
            "homo_end_ids": {0: "batch"},
            "logits": {0: "batch"},
        },
        opset_version=OPSET,
        dynamo=False,
    )
    (logits_o,) = _ort_run(
        out_dir / "homosolver.onnx",
        {"input_ids": input_ids.numpy(), "homo_start_ids": starts.numpy(), "homo_end_ids": ends.numpy()},
    )
    ort_diff = float(np.abs(ref.numpy() - logits_o).max())
    assert ort_diff < 1e-4, f"homosolver.onnx diverged from torch: {ort_diff}"
    print(f"[export] homosolver.onnx done ({time.time() - t0:.1f}s, ORT parity {ort_diff:.2e})")


def export_accentor_tensor(pack, out_dir: Path):
    print("[export] accentor_tensor.onnx ...")
    t0 = time.time()
    acc_model = pack.accentor.accentor.model
    wrapper = torch.jit.script(AccentorTensorExport(acc_model))
    words = ["привет", "это", "тест"]
    with torch.no_grad():
        ref = acc_model(words)
    ind, offsets = accentor_inputs(acc_model.embedding.ngram_dict, words)
    with torch.no_grad():
        out = wrapper(ind, offsets)
    d0 = (ref[0] - out[0]).abs().max().item()
    d1 = (ref[1] - out[1]).abs().max().item()
    # Raw logits carry amplified float noise (the classifier weights reach
    # abs ~96, so a ~5e-7 embedding-level rounding difference shows up as
    # ~1e-2 on logits). What the consumer uses is argmax / thresholded
    # softmax — compare those instead.
    for i, name in enumerate(["stress", "yo"]):
        prob_diff = (torch.softmax(ref[i], 1) - torch.softmax(out[i], 1)).abs().max().item()
        argmax_equal = bool((ref[i].argmax(1) == out[i].argmax(1)).all())
        assert argmax_equal and prob_diff < 1e-6, (
            f"accentor {name} head diverged: prob_diff={prob_diff} argmax_equal={argmax_equal}"
        )
    torch.onnx.export(
        wrapper,
        (ind, offsets),
        str(out_dir / "accentor_tensor.onnx"),
        input_names=["ind", "offsets"],
        output_names=["stress_logits", "yo_logits"],
        dynamic_axes={
            "ind": {0: "n"},
            "offsets": {0: "w"},
            "stress_logits": {0: "w"},
            "yo_logits": {0: "w"},
        },
        opset_version=OPSET,
        dynamo=False,
    )
    s_o, y_o = _ort_run(out_dir / "accentor_tensor.onnx", {"ind": ind.numpy(), "offsets": offsets.numpy()})
    for r, o, name in [(ref[0], s_o, "stress"), (ref[1], y_o, "yo")]:
        prob_diff = float(np.abs(
            torch.softmax(r, 1).numpy() - torch.softmax(torch.from_numpy(o), 1).numpy()
        ).max())
        argmax_equal = bool((r.argmax(1).numpy() == o.argmax(1)).all())
        assert argmax_equal and prob_diff < 1e-4, (
            f"accentor_tensor.onnx {name} head diverged: prob_diff={prob_diff} argmax_equal={argmax_equal}"
        )
    print(f"[export] accentor_tensor.onnx done ({time.time() - t0:.1f}s, torch parity {d0:.2e}/{d1:.2e} logits, "
          f"ORT decision-level parity OK)")


# ---------------------------------------------------------------------------
# Dictionary / metadata extraction
# ---------------------------------------------------------------------------


def extract_dictionaries(pack, out_dir: Path):
    print("[export] dictionaries & frontend metadata ...")
    acc = pack.accentor.accentor  # AccentorNgram
    hs = pack.accentor.homosolver  # HomoSolver

    # ngrams.gz — space-separated grams, dict index = position (see _loadngrams)
    ngram_dict = acc.model.embedding.ngram_dict
    grams = [None] * len(ngram_dict)
    for gram, idx in ngram_dict.items():
        grams[idx] = gram
    assert all(g is not None and " " not in g for g in grams)
    with gzip.open(out_dir / "ngrams.gz", "wb") as f:
        f.write(" ".join(grams).encode("utf-8"))

    # exceptions.gz — "word stress_vowel_idx yo_char_idx" per line (see _getexceptions)
    lines = [f"{w} {s} {y}" for w, (s, y) in sorted(acc.exceptions.items())]
    with gzip.open(out_dir / "exceptions.gz", "wb") as f:
        f.write(("\n".join(lines) + "\n").encode("utf-8"))

    # homodict.json — word -> [accented variants]
    with open(out_dir / "homodict.json", "w", encoding="utf-8") as f:
        json.dump(dict(sorted(hs.homodict.items())), f, ensure_ascii=False, indent=0, sort_keys=True)

    # vocab.txt — BERT tokenizer vocab, token index = line number (added tokens included)
    tok = hs.tokenizer
    vocab_tokens = [None] * len(tok.vocab)
    for token, idx in tok.vocab.items():
        vocab_tokens[idx] = token
    assert all(t is not None for t in vocab_tokens)
    with open(out_dir / "vocab.txt", "w", encoding="utf-8") as f:
        f.write("\n".join(vocab_tokens) + "\n")

    # frontend.json — everything the Rust text frontend needs
    stress_logits, yo_logits = acc.model(["пример"])
    frontend = {
        "symbols": pack.symbols,
        "alphabet": pack.alphabet,
        "symbol_to_id": pack.symbol_to_id,
        "sos_token": pack.sos_token,
        "eos_token": pack.eos_token,
        "speakers": pack.speakers,
        "speaker_to_ids": pack.speaker_to_ids,
        "frame_window_sec": pack.window,
        "sample_rates": [8000, 24000, 48000],
        "native_sample_rate": 48000,
        "istft": {"n_fft": 2400, "hop_length": 600, "win_length": 2400, "freq_bins": 1201},
        "accentor": {
            "stress_token": acc.stress_token,
            "vowels": acc.vowels,
            "stop_words": acc.stop_words,
            "word_regex": acc.re_cond,
            "ngram_min_len": 1,
            "ngram_max_len": "len(word) + 3",
            "unk_token": "UNK",
            "stress_logits_dim": stress_logits.shape[1],
            "yo_logits_dim": yo_logits.shape[1],
            "stress_threshold": 0.5,
            "yo_threshold": 0.5,
        },
        "homosolver": {
            "pad_token_id": tok.pad_token_id,
            "cls_token_id": tok.cls_token_id,
            "sep_token_id": tok.sep_token_id,
            "unk_token_id": tok.unk_token_id,
            "homo_start_id": tok.homo_start_id,
            "homo_end_id": tok.homo_end_id,
            "never_split": sorted(tok.never_split),
            "word_pattern": hs.pattern.pattern,
        },
    }
    with open(out_dir / "frontend.json", "w", encoding="utf-8") as f:
        json.dump(frontend, f, ensure_ascii=False, indent=2)


# ---------------------------------------------------------------------------
# Manifest
# ---------------------------------------------------------------------------


def write_manifest(out_dir: Path, model_id: str, model_url: str | None, pt_path: Path, pt_sha256: str):
    import onnx
    import onnxruntime
    import scipy

    files = []
    for p in sorted(out_dir.iterdir()):
        if p.is_file():
            files.append({"path": p.name, "size": p.stat().st_size, "sha256": sha256_file(p)})
    manifest = {
        "model_id": model_id,
        "model_url": model_url,
        "source_pt": {"name": pt_path.name, "sha256": pt_sha256},
        "export_date_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "opset": OPSET,
        "tool_versions": {
            "torch": torch.__version__,
            "onnx": onnx.__version__,
            "onnxruntime": onnxruntime.__version__,
            "scipy": scipy.__version__,
            "python": sys.version.split()[0],
        },
        "files": files,
    }
    with open(out_dir / "manifest.json", "w", encoding="utf-8") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=2)
    print(f"[export] manifest.json ({len(files)} files)")


# ---------------------------------------------------------------------------
# Self-check: torch package vs ONNX bundle, end to end
# ---------------------------------------------------------------------------

SELF_CHECK_PHRASES = [
    # (text, sample_rate)
    ("Привет! Это тестовый текст для проверки синтеза речи.", 48000),
    ("Сервер обрабатывает запросы пользователей и сохраняет данные в базу.", 48000),
    ("В тысяча девятьсот восемьдесят четвёртом году вышло две тысячи двадцать четыре номера журнала.", 48000),
    ("Я уже стою у большого замка, но ключ от старого замка потерял.", 48000),
    ("Ёжик в тумане нашёл ёлку и съел всё.", 48000),
    ("Стоп! Кто идёт? Отвечай быстро: друг, враг; время — деньги...", 48000),
    ("Проверка фильтра для частоты двадцать четыре килогерца.", 24000),
    ("Проверка фильтра для частоты восемь килогерц.", 8000),
]

SELF_CHECK_TOL = 1e-3


def _save_wav(path: Path, audio: np.ndarray, sample_rate: int):
    pcm = np.clip(audio, -1.0, 1.0)
    pcm = (pcm * 32767).astype(np.int16)
    with wave.open(str(path), "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(sample_rate)
        wf.writeframes(pcm.tobytes())


def self_check(pack, out_dir: Path, speaker: str, tol: float = SELF_CHECK_TOL) -> bool:
    import onnxruntime as ort

    sc_dir = out_dir / "selfcheck"
    sc_dir.mkdir(exist_ok=True)
    providers = ["CPUExecutionProvider"]
    sess_main = ort.InferenceSession(str(out_dir / "tts_main.onnx"), providers=providers)
    sess_istft = ort.InferenceSession(str(out_dir / "istft.onnx"), providers=providers)
    sess_pqmf = {
        24000: ort.InferenceSession(str(out_dir / "pqmf_24k.onnx"), providers=providers),
        8000: ort.InferenceSession(str(out_dir / "pqmf_8k.onnx"), providers=providers),
    }

    def onnx_tts(text: str, sample_rate: int) -> np.ndarray:
        sequence, sp_ids, durs_rate, pitch_coefs = build_model_input(pack, text, speaker)
        mag, x, y, _dur_hat = sess_main.run(
            None,
            {
                "sequence": sequence.numpy(),
                "speaker_ids": sp_ids.numpy(),
                "durs_rate": durs_rate.numpy(),
                "pitch_coefs": pitch_coefs.numpy(),
            },
        )
        (audio,) = sess_istft.run(None, {"mag": mag, "x": x, "y": y})
        if sample_rate != 48000:
            (band,) = sess_pqmf[sample_rate].run(None, {"audio": audio.reshape(1, 1, -1)})
            audio = band.reshape(1, -1)
        return audio[0]

    report = []
    ok = True
    for i, (text, sr) in enumerate(SELF_CHECK_PHRASES):
        ref = pack.apply_tts(text=text, speaker=speaker, sample_rate=sr).numpy()
        got = onnx_tts(text, sr)
        len_diff = abs(len(ref) - len(got))
        n = min(len(ref), len(got))
        max_diff = float(np.abs(ref[:n] - got[:n]).max()) if n else float("inf")
        passed = max_diff <= tol and len_diff == 0
        ok = ok and passed
        report.append(
            {
                "phrase": text,
                "sample_rate": sr,
                "samples_torch": len(ref),
                "samples_onnx": len(got),
                "len_diff": len_diff,
                "max_abs_diff": max_diff,
                "tolerance": tol,
                "passed": passed,
            }
        )
        status = "OK  " if passed else "FAIL"
        print(f"[self-check] {status} sr={sr:<5} max_abs_diff={max_diff:.3e} len_diff={len_diff} :: {text[:60]}")
        _save_wav(sc_dir / f"{i:02d}_{sr}_torch.wav", ref, sr)
        _save_wav(sc_dir / f"{i:02d}_{sr}_onnx.wav", got, sr)

    with open(sc_dir / "report.json", "w", encoding="utf-8") as f:
        json.dump({"tolerance": tol, "speaker": speaker, "passed": ok, "phrases": report}, f, ensure_ascii=False, indent=2)
    print(f"[self-check] {'PASSED' if ok else 'FAILED'} — report: {sc_dir / 'report.json'}")
    return ok


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main():
    ap = argparse.ArgumentParser(description="Export Silero TTS torch.package to an ONNX bundle")
    ap.add_argument("--model", default="v5_ru", help="model id from models.yml (default: v5_ru)")
    ap.add_argument("--model-path", default=None, help="explicit path to the .pt package (skips cache/URL resolution)")
    ap.add_argument("--out", required=True, help="output bundle directory")
    ap.add_argument("--speaker", default="aidar", help="speaker used for export examples and self-check (default: aidar)")
    ap.add_argument("--self-check", dest="self_check", action="store_true", default=True,
                    help="run torch-vs-ONNX waveform parity check (default: on)")
    ap.add_argument("--no-self-check", dest="self_check", action="store_false")
    ap.add_argument("--self-check-only", action="store_true",
                    help="only run the self-check against an existing bundle in --out")
    args = ap.parse_args()

    pt_path, model_url = resolve_model_path(args.model, args.model_path)
    pt_sha256 = sha256_file(pt_path)
    print(f"[resolve] source: {pt_path} (sha256 {pt_sha256[:16]}...)")

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)

    _tts_model, pack = load_package(pt_path)
    if args.speaker not in pack.speakers:
        raise SystemExit(f"unknown speaker '{args.speaker}', available: {pack.speakers}")

    if args.self_check_only:
        if not self_check(pack, out_dir, args.speaker):
            raise SystemExit(1)
        return

    export_tts_main(pack, out_dir, args.speaker)
    export_istft(pack, out_dir)
    export_pqmf(pack, out_dir)
    export_homosolver(pack, out_dir)
    export_accentor_tensor(pack, out_dir)
    extract_dictionaries(pack, out_dir)
    write_manifest(out_dir, args.model, model_url, pt_path, pt_sha256)

    if args.self_check:
        # Run the self-check in a fresh process: the ONNX export passes mutate
        # the loaded torch model's graph state (post-export torch inference
        # drifts), which would corrupt the reference waveforms.
        import subprocess

        rc = subprocess.call(
            [sys.executable, str(Path(__file__).resolve()),
             "--self-check-only", "--model", args.model, "--model-path", str(pt_path),
             "--out", str(out_dir), "--speaker", args.speaker]
        )
        if rc != 0:
            raise SystemExit(rc)


if __name__ == "__main__":
    main()
