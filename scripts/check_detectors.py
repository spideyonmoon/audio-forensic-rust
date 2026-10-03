"""Exercise provisional detectors on known synthetic processing histories.

Requires the release executable and FFmpeg with libmp3lame, AAC and libopus.
Generated audio and results remain under ignored corpus/local. Encoder cases
are observations, not an accuracy benchmark or automatic threshold tuning.
"""
import json
import math
from pathlib import Path
import subprocess
import sys
import wave

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "corpus/local/generated/detectors-v2"
INPUT = OUT / "decoded"
RESULTS = ROOT / "corpus/local/results/synthetic-detectors-v2"
INPUT.mkdir(parents=True, exist_ok=True)
RESULTS.mkdir(parents=True, exist_ok=True)
manifest = {"ffmpeg": subprocess.check_output(["ffmpeg", "-version"], text=True).splitlines()[0],
            "scope": "Synthetic engineering controls, not independent source groups or classification accuracy",
            "commands": [], "cases": {}}


def ffmpeg(args):
    command = ["ffmpeg", "-nostdin", "-v", "error", "-y", *map(str, args)]
    manifest["commands"].append(command)
    subprocess.run(command, check=True, timeout=120)


def write_wave(path, block, rate=44100, channels=1):
    with wave.open(str(path), "wb") as handle:
        handle.setparams((channels, 3, rate, 0, "NONE", "not compressed"))
        for _ in range(9):
            handle.writeframesraw(block)


for name, fixture in [("clean_noise", "clip_noise.wav"), ("mastering_lowpass", "clip_wall.wav")]:
    with wave.open(str(ROOT / "tests/fixtures" / fixture), "rb") as handle:
        block = handle.readframes(handle.getnframes())
    write_wave(INPUT / f"{name}.wav", block)
    manifest["cases"][name] = {"history": f"Nine repeats of generated {fixture}; no lossy encoder"}
tone = b"".join(int(0.5 * 2**23 * math.sin(2 * math.pi * 12000 * i / 44100)).to_bytes(4, "little", signed=True)[:3] for i in range(88200))
write_wave(INPUT / "tone.wav", tone)
write_wave(INPUT / "silence.wav", bytes(88200 * 3))
manifest["cases"].update({"tone": {"history": "Generated 12 kHz sine; no lossy encoder"},
                         "silence": {"history": "Digital zero; no lossy encoder"}})
for name, codec, bitrate, extension in [("mp3_128", "libmp3lame", "128k", "mp3"),
        ("mp3_320", "libmp3lame", "320k", "mp3"), ("aac_128", "aac", "128k", "m4a"),
        ("opus_128", "libopus", "128k", "ogg")]:
    encoded = OUT / f"{name}.{extension}"
    ffmpeg(["-i", INPUT / "clean_noise.wav", "-c:a", codec, "-b:a", bitrate, encoded])
    ffmpeg(["-i", encoded, "-map_metadata", "-1", "-c:a", "pcm_s24le", INPUT / f"{name}.wav"])
    manifest["cases"][name] = {"history": f"Generated noise -> {codec} {bitrate} -> 24-bit PCM WAV"}

(RESULTS / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
subprocess.run([sys.executable, str(ROOT / "scripts/validate_local.py"), str(INPUT),
                "--output", str(RESULTS), "--source-history", "Generated processing histories; see manifest.json"], check=True)
observations = []
for path in sorted(RESULTS.glob("[0-9][0-9][0-9].json")):
    report = json.loads(path.read_text(encoding="utf-8"))[0]
    name = Path(report["source"]).stem
    segment = report["segments"][0]
    row = {"case": name, "pattern": segment["pattern"], "wall_probes": segment["wall_probes"],
           "eligible_probes": segment["eligible_probes"], "sampled_probes": len(segment["probes"]),
           "cutoffs_hz": [p["cutoff_hz"] for p in segment["probes"]],
           "codec_candidates": sorted({c["codec"] for p in segment["probes"] for c in p["codec_candidates"]}),
           "mqa_sync_matches": report["mqa"]["sync_matches"]}
    observations.append(row)
    assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
    if name in ("clean_noise", "tone", "silence"):
        assert row["wall_probes"] == 0, row
    if name in ("tone", "silence"):
        assert row["eligible_probes"] == 0, row
    if name == "mastering_lowpass":
        assert row["wall_probes"] >= 3, row  # A wall is not proof of lossy ancestry.
    print(f"{name}: {row['wall_probes']}/{row['eligible_probes']} wall/eligible probes; {row['codec_candidates']}")
(RESULTS / "observations.json").write_text(json.dumps(observations, indent=2) + "\n", encoding="utf-8")
