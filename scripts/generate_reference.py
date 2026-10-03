"""Regenerate small deterministic PCM fixtures and Python-derived numerical oracles.

Requires numpy/scipy and the pinned reference checkout. Normal cargo tests do not.
Only generated signals are written here; supplied music is never modified.
"""
import json
import hashlib
import shutil
import subprocess
import sys
import wave
from pathlib import Path

import numpy as np
import scipy

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / "reference" / "audio-forensic"
COMMIT = "c6ecce2296256b516709d87088896d1be913908c"
head = subprocess.check_output(["git", "-C", str(REFERENCE), "rev-parse", "HEAD"], text=True).strip()
dirty = subprocess.check_output(["git", "-C", str(REFERENCE), "status", "--porcelain", "--untracked-files=no"], text=True).strip()
if head != COMMIT or dirty:
    raise SystemExit("The reference must be clean and at the pinned commit; review changes before regenerating oracles")
sys.path.insert(0, str(ROOT / "reference" / "audio-forensic"))
from audio_forensic import SpectralEngine, _effective_bits

OUT = ROOT / "tests" / "fixtures"
OUT.mkdir(parents=True, exist_ok=True)
rng = np.random.default_rng(20260930)
n = 24577
noise = rng.integers(-24000, 24000, (n, 2), dtype=np.int32)
dark = np.fft.rfft(noise[:, 0].astype(float))
dark[np.fft.rfftfreq(n, 1 / 44100) > 15000] = 0
dark = np.clip(np.fft.irfft(dark, n=n), -32000, 32000).astype(np.int32)
dark = np.column_stack([dark, -dark])
activity = noise.copy()
activity[:12000] = 0
activity[12000:15000] //= 2000
cases = [("noise16", noise, 16), ("wall24", dark * 256, 24), ("activity16", activity, 16)]
expected = {"reference_commit": COMMIT, "numpy": np.__version__, "scipy": scipy.__version__, "cases": []}
for name, native, bits in cases:
    with wave.open(str(OUT / f"{name}.wav"), "wb") as handle:
        handle.setparams((2, bits // 8, 44100, 0, "NONE", "not compressed"))
        if bits == 16:
            handle.writeframes(native.astype("<i2").tobytes())
        else:
            handle.writeframes(native.astype("<i4").view(np.uint8).reshape(-1, 4)[:, :3].tobytes())
    channels = []
    for channel in range(2):
        audio = (native[:, channel].astype(np.float64) / 2 ** (bits - 1)).astype(np.float32)
        engine = SpectralEngine(Path("fixture.wav"), 44100, channels=1)
        mags, _, _ = engine._compute_stft(audio)
        frames = len(mags)
        mags = mags[engine._active_frame_mask(mags)]
        bins = engine._freq_bins()
        cutoffs = engine._cutoff_per_frame(mags, bins)
        cutoff = float(np.percentile(cutoffs, 95))
        words = native[:, channel] * (2 ** (32 - bits))
        channels.append({
            "samples": len(audio), "peak": float(np.max(np.abs(audio.astype(float)))),
            "rms": float(np.sqrt(np.mean(audio.astype(float) ** 2))),
            "dc_offset": float(audio.astype(float).mean()),
            "effective_bits": _effective_bits(words, 1),
            "spectral": {"frames": frames, "active_frames": len(mags),
                "cutoff_p95_hz": cutoff, "cutoff_variance_hz2": float(np.var(cutoffs)),
                "cliff_depth_db": engine._cliff_depth(mags, bins, cutoff),
                "sharpness_db_per_bin": engine._sharpness(mags, bins, cutoff),
                "hf_magnitude_ratio": engine._hf_energy_ratio(mags, bins),
                "entropy_bits": engine._spectral_entropy(mags),
                "noise_above_cutoff_db": engine._noise_floor_above_cutoff(mags, bins, cutoff)}})
    pcm_hash = hashlib.sha256((native * (2 ** (32-bits))).astype("<i4").tobytes()).hexdigest()
    expected["cases"].append({"file": f"{name}.wav", "channels": channels, "pcm_sha256": pcm_hash})
    if not shutil.which("ffmpeg"):
        raise SystemExit("ffmpeg is required to generate the checked-in FLAC fixtures")
    subprocess.run(["ffmpeg", "-nostdin", "-v", "error", "-y", "-i", str(OUT / f"{name}.wav"),
                    "-map_metadata", "-1", "-c:a", "flac", str(OUT / f"{name}.flac")], check=True)
    expected["cases"].append({"file": f"{name}.flac", "channels": channels, "pcm_sha256": pcm_hash})
(OUT / "reference.json").write_text(json.dumps(expected, indent=2, allow_nan=False) + "\n", encoding="utf-8")
print(f"Generated {len(cases)} fixtures in {OUT}")
