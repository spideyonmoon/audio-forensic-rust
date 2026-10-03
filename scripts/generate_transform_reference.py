"""Generate synthetic resampling/Vorbis fixtures and pinned Python oracles.

Requires NumPy, SciPy, FFmpeg/libvorbis and the clean reference checkout. Ordinary
Cargo tests need none of these. Private recordings are never used or modified.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import wave

import numpy as np
import scipy

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
WORK = ROOT / "corpus/local/generated/reference-v3"
WORK.mkdir(parents=True, exist_ok=True)
oracle = {"reference_commit": COMMIT, "numpy": np.__version__, "scipy": scipy.__version__,
          "ffmpeg": subprocess.check_output(["ffmpeg", "-version"], text=True).splitlines()[0],
          "resampling": [], "vorbis": []}


def ffmpeg(args):
    subprocess.run(["ffmpeg", "-nostdin", "-v", "error", "-y", *map(str, args)], check=True, timeout=120)


def save_native(name, audio, rate):
    words = np.rint(audio * 2**23).clip(-2**23, 2**23-1).astype("<i4")
    path = WORK / f"{name}.wav"
    with wave.open(str(path), "wb") as handle:
        handle.setparams((1, 3, rate, 0, "NONE", "not compressed"))
        handle.writeframes(words.view(np.uint8).reshape(-1, 4)[:, :3].tobytes())
    return path, words


def pcm_hash(words):
    return hashlib.sha256((words * 256).astype("<i4").tobytes()).hexdigest()


for name, rate in [("full", 48000), ("notch", 48000), ("wall", 48000),
                   ("mirror", 48000), ("codec_wall", 48000), ("full96", 96000)]:
    n = rate
    rng = np.random.default_rng(337)
    spectrum = np.fft.rfft(rng.standard_normal(n))
    freqs = np.fft.rfftfreq(n, 1/rate)
    if name == "notch":
        spectrum[(freqs > 21850) & (freqs < 22250)] = 0
        spectrum[freqs >= 22250] *= 10**(-30/20)
    elif name in ("wall", "codec_wall"):
        spectrum[freqs > (22050 if name == "wall" else 20460)] = 0
    elif name == "mirror":
        fold = np.searchsorted(freqs, 22050)
        indices = np.arange(1, len(spectrum)-fold)
        spectrum[fold+indices] = np.conj(spectrum[fold-indices]) * 10**(-6/20)
    audio = np.fft.irfft(spectrum, n=n)
    path, words = save_native(f"rate_{name}", audio * 0.7 / np.max(np.abs(audio)), rate)
    destination = OUT / f"rate_{name}.flac"
    ffmpeg(["-i", path, "-map_metadata", "-1", "-c:a", "flac", destination])
    engine = SpectralEngine(destination, rate, channels=1)
    frames, _, _ = engine._compute_stft((words.astype(float)/2**23).astype(np.float32))
    frames = frames[engine._active_frame_mask(frames)]
    bins = engine._freq_bins()
    original_hit = engine._resample_check(frames, bins)
    avg = frames.mean(axis=0).astype(float)
    db = 20*np.log10(avg/(avg.max()+1e-12)+1e-12)
    bin_hz = bins[1]-bins[0]
    def band(lo, hi):
        return float(db[max(0, int(lo/bin_hz)):min(len(db), int(hi/bin_hz)+1)].mean())
    candidates = []
    for source_rate in engine.RESAMPLE_SOURCE_RATES:
        fn = source_rate/2
        if fn+1200 > rate/2-200:
            continue
        c, g = int(round(fn/bin_hz)), int(300/bin_hz)
        half = min(int(1800/bin_hz), len(db)-4-c-g, c-g)
        corr = None
        if (half*bin_hz >= 700 and band(fn-900, fn-350) >= -80
                and band(fn+450, min(fn+2000, rate/2-200)) > -85):
            lo = np.arange(c-g-half, c-g)
            target = 2*fn/bin_hz-lo
            high = target.astype(int)
            fraction = target-high
            a = np.log(frames[:, lo].astype(float)+1e-12)
            b = np.log(frames[:, high]*(1-fraction)+frames[:, high+1]*fraction+1e-12)
            a -= a.mean(axis=1, keepdims=True)
            b -= b.mean(axis=1, keepdims=True)
            corr = float(np.mean((a*b).sum(axis=1)/(np.sqrt((a*a).sum(axis=1)*(b*b).sum(axis=1))+1e-12)))
        candidates.append({"source_rate": source_rate, "edge_relative_db": band(fn-900, fn-350),
            "notch_relative_db": band(fn-150, fn+250), "above_relative_db": band(fn+450, min(fn+2000, rate/2-200)),
            "ceiling_relative_db": band(fn+450, rate/2-200), "mirror_correlation": corr})
    oracle["resampling"].append({"file": destination.name, "pcm_sha256": pcm_hash(words),
        "original_hit": original_hit, "active_frames": len(frames), "candidates": candidates})
    print(f"{name}: Python {original_hit}", flush=True)

for rate, quality in [(44100, 6), (48000, 10)]:
    rng = np.random.default_rng(17)
    n = rate*2
    freq = np.fft.rfftfreq(n, 1/rate)
    audio = np.fft.irfft(np.fft.rfft(rng.standard_normal(n))/np.sqrt(np.maximum(freq, 40)), n=n)
    audio *= 0.15/np.std(audio)
    source, words = save_native(f"grid_source_{rate}", audio, rate)
    for encoded in [False, True]:
        name = f"grid_{'q'+str(quality) if encoded else 'clean'}_{rate}"
        destination = OUT / f"{name}.flac"
        if encoded:
            ogg = WORK / f"{name}.ogg"
            ffmpeg(["-i", source, "-c:a", "libvorbis", "-q:a", quality, ogg])
            ffmpeg(["-i", ogg, "-map_metadata", "-1", "-c:a", "flac", "-sample_fmt", "s32", destination])
        else:
            ffmpeg(["-i", source, "-map_metadata", "-1", "-c:a", "flac", destination])
        raw = subprocess.check_output(["ffmpeg", "-nostdin", "-v", "error", "-i", str(destination), "-c:a", "pcm_s32le", "-f", "s32le", "pipe:1"])
        decoded = (np.frombuffer(raw, dtype="<i4").astype(np.float64)/2**31).astype(np.float32)
        engine = SpectralEngine(destination, rate, channels=1)
        score, support, active, _ = engine._vorbis_grid(decoded, None)
        oracle["vorbis"].append({"file": destination.name, "pcm_sha256": hashlib.sha256(raw).hexdigest(),
            "score": score if score >= 0 else None, "support": support, "active": active,
            "encoded": encoded})
        print(f"{name}: Python score={score}, support={support}/{active}", flush=True)
(OUT / "transform_reference.json").write_text(json.dumps(oracle, indent=2, allow_nan=False)+"\n", encoding="utf-8")
