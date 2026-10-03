"""Frozen spectral-structure oracles from generated public audio only.

Reads the unchanged Python baseline for the original values; independent NumPy
sliding windows and phase histograms describe the explicitly changed Rust policy.
Requires NumPy/SciPy and FFmpeg only when generating, not for ordinary Rust tests.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

import numpy as np
import scipy
from numpy.lib.stride_tricks import sliding_window_view

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / "reference/audio-forensic"
COMMIT = "c6ecce2296256b516709d87088896d1be913908c"
if subprocess.check_output(["git", "-C", str(REFERENCE), "rev-parse", "HEAD"], text=True).strip() != COMMIT:
    raise SystemExit("Reference must be pinned")
if subprocess.check_output(["git", "-C", str(REFERENCE), "status", "--porcelain", "--untracked-files=no"], text=True).strip():
    raise SystemExit("Reference must be clean")
sys.path.insert(0, str(REFERENCE))
from audio_forensic import SpectralEngine

OUT = ROOT / "tests/fixtures"
oracle = {"reference_commit": COMMIT, "numpy": np.__version__, "scipy": scipy.__version__,
          "ffmpeg": subprocess.check_output(["ffmpeg", "-version"], text=True).splitlines()[0],
          "policy": "All active native-channel frames; centered f64 scatter; exact-bin mode; energy-gated adjacent phase pairs >=10 kHz, excluding Nyquist",
          "cases": []}


def changed_policy(mags, spectrum, active, rate):
    selected = mags[active].astype(float)
    log_power = np.maximum(20*np.log10(selected/(selected.max(axis=1, keepdims=True)+1e-12)+1e-12), -110) / 10 * np.log(10)
    windows = sliding_window_view(np.pad(log_power, ((0, 0), (2, 2)), mode="edge"), 5, axis=1)
    scatter = windows.std(axis=2)
    smoothed = sliding_window_view(np.pad(scatter, ((0, 0), (2, 2)), mode="edge"), 5, axis=1).mean(axis=2)
    maximum = smoothed.max(axis=1)
    keep = maximum > 1e-6
    exceeds = smoothed[keep] >= np.minimum(.6, maximum[keep, None]*.25)
    bounds = 2048 - np.argmax(exceeds[:, ::-1], axis=1)
    counts = np.bincount(bounds, minlength=2049)
    start = min(2048, int(np.ceil(10000*4096/rate)))
    supported = 2048-start >= 2
    hist = np.zeros(36, dtype=np.int64)
    frame_pairs = 0
    # Complex64 -> complex128 angles, matching the declared Rust f64 phase basis.
    phase = np.angle(spectrum.astype(np.complex128))
    for i in range(1, len(mags)):
        if not active[i-1] or not active[i]:
            continue
        a, b = mags[i-1, start:2048], mags[i, start:2048]
        eligible = ((a > max(1e-8, float(mags[i-1].max())*1e-4)) &
                    (b > max(1e-8, float(mags[i].max())*1e-4)))
        if np.count_nonzero(eligible) < 2:
            continue
        delta = phase[i, start:2048][eligible] - phase[i-1, start:2048][eligible]
        wrapped = np.arctan2(np.sin(delta), np.cos(delta))
        hist += np.histogram(wrapped, bins=36, range=(-np.pi, np.pi))[0]
        frame_pairs += 1
    probabilities = hist[hist > 0] / max(1, hist.sum())
    return {"stft_frames": len(mags), "active_frames": int(active.sum()),
            "bound_frames": len(bounds), "flat_frames": int((~keep).sum()),
            "average_scatter_bound_hz": float(bounds.mean()*rate/4096) if len(bounds) >= 4 else None,
            "modal_scatter_bound_hz": float(counts.argmax()*rate/4096) if len(bounds) >= 4 else None,
            "phase_band_start_hz": start*rate/4096 if supported else None,
            "phase_frame_pairs": frame_pairs, "phase_bin_pairs": int(hist.sum()),
            "high_band_phase_entropy_bits": float(-np.sum(probabilities*np.log2(probabilities))) if frame_pairs >= 3 else None}


for name in ("noise16.wav", "wall24.wav", "activity16.wav", "rate_full96.flac", "clip_wall.wav", "aac_256_44100.flac"):
    path = OUT / name
    metadata = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-select_streams", "a:0",
            "-show_entries", "stream=sample_rate,channels", "-of", "json", str(path)]))["streams"][0]
    rate, channels = int(metadata["sample_rate"]), metadata["channels"]
    raw = subprocess.check_output(["ffmpeg", "-nostdin", "-v", "error", "-i", str(path), "-c:a", "pcm_s32le", "-f", "s32le", "pipe:1"])
    audio = (np.frombuffer(raw, dtype="<i4").reshape(-1, channels).astype(float)/2**31).astype(np.float32)
    case = {"file": name, "sample_rate": rate, "pcm_sha256": hashlib.sha256(raw).hexdigest(), "channels": []}
    for channel in range(channels):
        engine = SpectralEngine(path, rate, channels=1)
        mags, phase, _ = engine._compute_stft(audio[:, channel])
        active = engine._active_frame_mask(mags)
        original = engine._aucdtect_features(mags[active], phase[active], engine._freq_bins())
        # Separate SciPy complex transform, same Hann/f32 conventions as reference.
        windows = sliding_window_view(audio[:, channel], 4096)[0:len(audio)-4096:2048]
        spectrum = scipy.fft.rfft(windows*np.hanning(4096).astype(np.float32), axis=1)
        expected = changed_policy(mags, spectrum, active, rate)
        case["channels"].append({"reference": {"average_bound_hz": original[0],
            "histogram_mode_hz": original[1], "ungated_phase_entropy_bits": original[2]}, "expected": expected})
    oracle["cases"].append(case)
    print(name, json.dumps([c["expected"] for c in case["channels"]]), flush=True)

(OUT / "structure_reference.json").write_text(json.dumps(oracle, indent=2, allow_nan=False)+"\n", encoding="utf-8")
