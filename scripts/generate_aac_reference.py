"""Generate AAC numerical oracles using the unchanged, pinned Python reference.

Only generated signals are used. Requires NumPy/SciPy and FFmpeg native AAC.
Writes new v0.4 fixtures; never rewrites the v0.1-v0.3 fixtures/oracles.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

import numpy as np
import scipy
from scipy.special import ndtr, ndtri

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / "reference/audio-forensic"
COMMIT = "c6ecce2296256b516709d87088896d1be913908c"
head = subprocess.check_output(["git", "-C", str(REFERENCE), "rev-parse", "HEAD"], text=True).strip()
dirty = subprocess.check_output(["git", "-C", str(REFERENCE), "status", "--porcelain", "--untracked-files=no"], text=True).strip()
if head != COMMIT or dirty:
    raise SystemExit("Reference must be clean and pinned")
sys.path.insert(0, str(REFERENCE))
from audio_forensic import SpectralEngine

OUT = ROOT / "tests/fixtures"
WORK = ROOT / "corpus/local/generated/reference-v4"
WORK.mkdir(parents=True, exist_ok=True)
oracle = {"reference_commit": COMMIT, "numpy": np.__version__, "scipy": scipy.__version__,
          "ffmpeg": subprocess.check_output(["ffmpeg", "-version"], text=True).splitlines()[0],
          "commands": [], "cases": []}


def ffmpeg(args):
    command = ["ffmpeg", "-nostdin", "-v", "error", "-y", *map(str, args)]
    oracle["commands"].append(command)
    return subprocess.check_output(command, timeout=120)


def anchors(audio):
    """Reference anchor policy, exposing its intermediate energy selection."""
    sig = audio.astype(np.float64) * 32768
    csq = np.concatenate(([0.0], np.cumsum(sig * sig)))
    starts = np.arange((len(sig) - 2048) // 1024) * 1024
    energies = csq[starts + 2048] - csq[starts]
    floor = max(2048 * 100, float(energies.max()) * 1e-6)
    selected = []
    for index in np.argsort(energies)[::-1]:
        if energies[index] < floor:
            break
        pos = int(starts[index])
        if pos < 512 or pos - 512 + 3071 > len(sig):
            continue
        if all(abs(pos - other["anchor_frame"]) > 2048 for other in selected):
            selected.append({"anchor_frame": pos, "anchor_rms": float(np.sqrt(energies[index]/2048)/32768)})
        if len(selected) >= 16:
            break
    return sorted(selected, key=lambda x: x["anchor_frame"])


for rate, bitrate in [(44100, 256), (48000, 320)]:
    source = OUT / f"grid_clean_{rate}.flac"
    for encoded in [False, True]:
        if encoded:
            path = WORK / f"aac_{bitrate}_{rate}.m4a"
            destination = OUT / f"aac_{bitrate}_{rate}.flac"
            ffmpeg(["-i", source, "-c:a", "aac", "-aac_tns", "0", "-b:a", f"{bitrate}k", path])
            ffmpeg(["-i", path, "-map_metadata", "-1", "-c:a", "flac", "-sample_fmt", "s32", destination])
        else:
            destination = source
        raw = ffmpeg(["-i", destination, "-c:a", "pcm_s32le", "-f", "s32le", "pipe:1"])
        audio = (np.frombuffer(raw, dtype="<i4").astype(np.float64) / 2**31).astype(np.float32)
        engine = SpectralEngine(destination, rate, channels=1)
        score = engine._mdct_quant_error(audio, None)
        case = {"file": destination.name, "encoded": encoded, "rate": rate,
                "pcm_sha256": hashlib.sha256(raw).hexdigest(), "score": score,
                "anchors": anchors(audio)}
        oracle["cases"].append(case)
        if "window" not in oracle:
            win = engine._kbd_window(2048)
            oracle["window"] = win.tolist()
            oracle["mdct_input"] = (audio[:2048].astype(float)*32768).tolist()
            oracle["mdct"] = engine._mdct_batch((audio[:2048].astype(float)*32768)[None, :], win)[0].tolist()
            counts = np.diff(engine._SWB_LONG_44_48).astype(float)
            mu, sigma = counts/12, np.sqrt(counts/180)
            oracle["gamma"] = (mu + sigma * ndtri(.01 + .99*ndtr(-mu/sigma))).tolist()
        print(f"{destination.name}: Python AAC score={score:.12f}, anchors={len(case['anchors'])}", flush=True)

# Portable command records: keep machine-specific root paths out of public fixtures.
oracle["commands"] = [[arg.replace(str(ROOT), "<PROJECT_ROOT>") for arg in cmd]
                      for cmd in oracle["commands"]]
(OUT / "aac_reference.json").write_text(json.dumps(oracle, indent=2, allow_nan=False)+"\n", encoding="utf-8")
