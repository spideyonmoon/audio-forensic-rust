"""Synthetic two-second clip oracles from the clean, pinned Python reference.

No private recordings are used. Cargo tests consume the saved files without Python.
"""
import json
import subprocess
import sys
import wave
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / "reference/audio-forensic"
COMMIT = "c6ecce2296256b516709d87088896d1be913908c"
head = subprocess.check_output(["git", "-C", str(REFERENCE), "rev-parse", "HEAD"], text=True).strip()
dirty = subprocess.check_output(["git", "-C", str(REFERENCE), "status", "--porcelain", "--untracked-files=no"], text=True).strip()
if head != COMMIT or dirty:
    raise SystemExit("The reference must be clean and at the pinned commit")
sys.path.insert(0, str(REFERENCE))
from audio_forensic import SpectralEngine

OUT = ROOT / "tests/fixtures"
expected = {"reference_commit": COMMIT, "numpy": np.__version__, "cases": []}
for name, rate, wall in [("clip_noise", 44100, None), ("clip_wall", 44100, 16750),
                         ("clip_high_wall", 44100, 20200), ("clip_overlap", 48000, 19550)]:
    rng = np.random.default_rng(20260930)
    signal = rng.normal(size=rate * 2)
    if wall is not None:
        spectrum = np.fft.rfft(signal)
        spectrum[np.fft.rfftfreq(len(signal), 1 / rate) > wall] = 0
        signal = np.fft.irfft(spectrum, n=len(signal))
    native = np.rint(signal / np.max(np.abs(signal)) * 0.7 * 2**23).astype("<i4")
    audio = native.astype(np.float64) / 2**23
    with wave.open(str(OUT / f"{name}.wav"), "wb") as handle:
        handle.setparams((1, 3, rate, 0, "NONE", "not compressed"))
        handle.writeframes(native.view(np.uint8).reshape(-1, 4)[:, :3].tobytes())
    engine = SpectralEngine(Path("fixture.wav"), rate, channels=1)
    _, _, _, probes = engine._segment_voting(np.tile(audio, 9))
    offset, cutoff, cliff, high_band, peak_db = probes[0]
    assert offset == 0
    # Original reports zero for an unavailable cliff; Rust uses null explicitly.
    cliff_available = cutoff > 450 and cutoff + 450 < rate / 2
    mag = np.abs(np.fft.rfft(audio * np.hanning(len(audio))))
    ref = mag.max() + 1e-12
    freqs = np.fft.rfftfreq(len(audio), 1 / rate)
    above = (freqs >= cutoff + 800) & (freqs <= rate / 2 - 100)
    above_db = float(20 * np.log10(np.sqrt(np.mean((mag[above] / ref)**2)) + 1e-15)) if above.any() else None
    db = 20 * np.log10(mag / ref + 1e-12)
    occupied = db[(freqs >= 1000) & (freqs < cutoff - 1000)]
    expected["cases"].append({"file": f"{name}.wav", "sample_rate": rate,
        "cutoff_hz": cutoff, "cliff_db": cliff if cliff_available else None,
        "high_band_relative_db": high_band, "peak_dbfs": peak_db,
        "above_cutoff_relative_db": above_db,
        "occupied_band_fraction": float(np.mean(occupied > -45)),
        "wall_observed": wall is not None})
(OUT / "detector_reference.json").write_text(json.dumps(expected, indent=2, allow_nan=False) + "\n", encoding="utf-8")
print(f"Generated {len(expected['cases'])} synthetic clip oracles")
