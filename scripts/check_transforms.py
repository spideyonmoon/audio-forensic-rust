"""Known processing controls for the Rust resampling/Vorbis milestone.

Requires NumPy and FFmpeg/libvorbis/SoXR. Uses only generated signals. The release
binary consumes WAV/FLAC; FFmpeg is a fixture generator and independent comparator.
"""
import json
from pathlib import Path
import subprocess
import sys
import wave

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
WORK = ROOT / "corpus/local/generated/transforms-v3"
INPUT = WORK / "decoded"
RESULT = ROOT / "corpus/local/results/synthetic-transforms-v3"
INPUT.mkdir(parents=True, exist_ok=True)
RESULT.mkdir(parents=True, exist_ok=True)
manifest = {"ffmpeg": subprocess.check_output(["ffmpeg", "-version"], text=True).splitlines()[0],
    "numpy": np.__version__, "commands": [], "histories": {},
    "scope": "Known synthetic processing, not independent source groups or accuracy estimates"}


def ffmpeg(args):
    command = ["ffmpeg", "-nostdin", "-v", "error", "-y", *map(str, args)]
    manifest["commands"].append(command)
    subprocess.run(command, check=True, timeout=120)


def save(name, audio, rate=44100):
    words = np.rint(audio * 2**23).clip(-2**23, 2**23-1).astype("<i4")
    path = INPUT / f"{name}.wav"
    with wave.open(str(path), "wb") as handle:
        handle.setparams((audio.shape[1], 3, rate, 0, "NONE", "not compressed"))
        handle.writeframes(words.view(np.uint8).reshape(-1, 4)[:, :3].tobytes())
    return path


rate = 44100
rng = np.random.default_rng(17)
n = rate * 4
frequency = np.fft.rfftfreq(n, 1/rate)
audio = np.fft.irfft(np.fft.rfft(rng.standard_normal((n, 2)), axis=0) /
                     np.sqrt(np.maximum(frequency[:, None], 40)), n=n, axis=0)
audio *= 0.15 / np.std(audio, axis=0)
source = save("clean_pink", audio)
manifest["histories"][source.stem] = "Synthetic stereo pink noise, seed 17, no encoder"
for quality in (4, 6, 8, 10):
    name = f"vorbis_q{quality}"
    encoded = WORK / f"{name}.ogg"
    ffmpeg(["-i", source, "-c:a", "libvorbis", "-q:a", quality, encoded])
    ffmpeg(["-i", encoded, "-map_metadata", "-1", "-c:a", "flac", "-sample_fmt", "s32", INPUT / f"{name}.flac"])
    manifest["histories"][name] = f"Generated pink noise -> libvorbis q{quality} -> 24-bit FLAC"
ffmpeg(["-i", INPUT / "vorbis_q10.flac", "-af", "atrim=start_sample=137,volume=0.37", "-map_metadata", "-1",
        "-c:a", "flac", "-sample_fmt", "s32", INPUT / "vorbis_q10_trim_gain.flac"])
manifest["histories"]["vorbis_q10_trim_gain"] = "q10 decode -> trim 137 samples -> gain 0.37 -> quantize 24-bit FLAC"
for name, filter_value in [("resample_swr", "aresample=48000"),
                            ("resample_soxr", "aresample=48000:resampler=soxr:precision=28")]:
    ffmpeg(["-i", source, "-af", filter_value, "-map_metadata", "-1", "-c:a", "flac", "-sample_fmt", "s32", INPUT / f"{name}.flac"])
    manifest["histories"][name] = f"Generated 44.1 kHz pink noise -> {filter_value} -> 24-bit FLAC; no lossy codec"

rng = np.random.default_rng(374)
transients = rng.standard_normal((rate*12, 2)) * 0.012
for seconds in (0.15, 0.85, 1.7, 2.4, 3.15, 3.8, 4.65, 5.25, 6.2, 7.1, 7.8, 8.9, 9.7, 10.3, 11.1):
    start, length = int(seconds*rate), int(0.35*rate)
    transients[start:start+length] += rng.standard_normal((length, 2))*0.17*np.exp(-np.arange(length)[:, None]/(rate*0.09))
source = save("clean_transients", transients)
manifest["histories"][source.stem] = "Generated attack/decay noise, seed 374; pinned Python regression design"
ffmpeg(["-i", source, "-c:a", "libvorbis", "-q:a", "8", WORK / "transients.ogg"])
ffmpeg(["-i", WORK / "transients.ogg", "-map_metadata", "-1", "-c:a", "flac", "-sample_fmt", "s32", INPUT / "vorbis_transients_q8.flac"])
manifest["histories"]["vorbis_transients_q8"] = "Generated attacks -> libvorbis q8 -> 24-bit FLAC"
(RESULT / "manifest.json").write_text(json.dumps(manifest, indent=2)+"\n", encoding="utf-8")
subprocess.run([sys.executable, str(ROOT / "scripts/validate_local.py"), str(INPUT), "--output", str(RESULT),
                "--source-history", "Known generated processing; see manifest.json"], check=True)
observations = []
for path in sorted(RESULT.glob("[0-9][0-9][0-9].json")):
    report = json.loads(path.read_text(encoding="utf-8"))[0]
    name = Path(report["source"]).stem
    row = {"case": name, "vorbis": [{k: v[k] for k in ("channel_index", "status", "zero_excess_score", "supporting_probes", "active_probes")} for v in report["vorbis"]],
           "resampling": [{"channel_index": r["channel_index"], "status": r["status"],
                           "matches": [{"source_rate": c["source_rate"], "modes": c["modes"]} for c in r["candidates"] if c["modes"]]} for r in report["resampling"]]}
    observations.append(row)
    assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
    if name.startswith("clean_"):
        assert all(v["status"] != "hit" for v in report["vorbis"]), row
    if name in {"vorbis_q4", "vorbis_q6", "vorbis_q8", "vorbis_q10", "vorbis_transients_q8"}:
        assert any(v["status"] == "hit" for v in report["vorbis"]), row
    print(json.dumps(row), flush=True)
(RESULT / "observations.json").write_text(json.dumps(observations, indent=2)+"\n", encoding="utf-8")
