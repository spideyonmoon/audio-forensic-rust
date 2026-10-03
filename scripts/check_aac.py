"""Generated AAC/cross-codec controls. Requires NumPy, FFmpeg and release CLI.

Private recordings are never used. Output directories are configurable so that
historical evidence can be preserved. These checks are not accuracy estimates.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import wave

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--work", type=Path, default=ROOT / "corpus/local/generated/aac-v4")
parser.add_argument("--output", type=Path, default=ROOT / "corpus/local/results/aac-v4")
args = parser.parse_args()
INPUT = args.work / "decoded"
INPUT.mkdir(parents=True, exist_ok=True)
args.output.mkdir(parents=True, exist_ok=True)
manifest = {"numpy": np.__version__,
            "ffmpeg": subprocess.check_output(["ffmpeg", "-version"], text=True).splitlines()[0],
            "commands": [], "histories": {}, "expected_hits": [], "expected_no_hits": [],
            "scope": "Related generated controls only; no independent source groups or accuracy estimates"}


def ffmpeg(arguments):
    command = ["ffmpeg", "-nostdin", "-v", "error", "-y", *map(str, arguments)]
    manifest["commands"].append(command)
    subprocess.run(command, check=True, timeout=120)


def save(name, audio, rate, history, no_hit=False):
    words = np.rint(audio * 2**23).clip(-2**23, 2**23-1).astype("<i4")
    path = INPUT / f"{name}.wav"
    with wave.open(str(path), "wb") as handle:
        handle.setparams((audio.shape[1], 3, rate, 0, "NONE", "not compressed"))
        handle.writeframes(words.view(np.uint8).reshape(-1, 4)[:, :3].tobytes())
    manifest["histories"][name] = history
    if no_hit:
        manifest["expected_no_hits"].append(name)
    return path


def transcode(name, source, codec_args, history, expect_hit=False, suffix="m4a"):
    encoded = args.work / f"{name}.{suffix}"
    destination = INPUT / f"{name}.flac"
    ffmpeg(["-i", source, *codec_args, encoded])
    ffmpeg(["-i", encoded, "-map_metadata", "-1", "-c:a", "flac", "-sample_fmt", "s32", destination])
    manifest["histories"][name] = history
    if expect_hit:
        manifest["expected_hits"].append(name)
    return destination


for rate in (44100, 48000):
    n = rate * 4
    rng = np.random.default_rng(84)
    frequencies = np.fft.rfftfreq(n, 1/rate)
    audio = np.fft.irfft(np.fft.rfft(rng.standard_normal((n, 2)), axis=0) /
                        np.sqrt(np.maximum(frequencies[:, None], 40)), n=n, axis=0)
    audio *= 0.15 / np.std(audio, axis=0)
    source = save(f"clean_pink_{rate}", audio, rate, "Generated stereo pink noise, seed 84", True)
    for bitrate in (128, 256, 320):
        name = f"aac_{bitrate}_{rate}_no_tns"
        destination = transcode(name, source, ["-c:a", "aac", "-aac_tns", "0", "-b:a", f"{bitrate}k"],
            f"Generated stereo pink noise -> FFmpeg AAC {bitrate}k, TNS disabled -> 24-bit FLAC",
            expect_hit=True)
    transcode(f"aac_320_{rate}_tns", source, ["-c:a", "aac", "-aac_tns", "1", "-b:a", "320k"],
              "Same generated source -> AAC 320k with TNS enabled; exploratory, no expected hit")
    if rate == 44100:
        for label, expression in [("trim8", "atrim=start_sample=8"),
                                  ("trim137", "atrim=start_sample=137"),
                                  ("gain", "volume=0.37")]:
            name = f"aac_320_{label}"
            ffmpeg(["-i", destination, "-af", expression, "-map_metadata", "-1", "-c:a", "flac",
                    "-sample_fmt", "s32", INPUT / f"{name}.flac"])
            manifest["histories"][name] = f"AAC 320k no TNS -> {expression} -> 24-bit FLAC; exploratory"
        for label, transform in [("dual_mono", np.column_stack((audio[:, 0], audio[:, 0]))),
                                  ("quiet_side", np.column_stack((audio[:, 0], audio[:, 0] * (1-1e-7)))),
                                  ("anti_phase", np.column_stack((audio[:, 0], -audio[:, 0]))),
                                  ("silence", np.zeros_like(audio))]:
            save(f"clean_{label}", transform, rate, f"Generated {label} control", True)
        dual = INPUT / "clean_dual_mono.wav"
        transcode("aac_dual_mono", dual, ["-c:a", "aac", "-aac_tns", "0", "-b:a", "256k"],
                  "Generated dual mono -> AAC 256k TNS off -> 24-bit FLAC", True)
        transcode("aac_anti_phase", INPUT / "clean_anti_phase.wav",
                  ["-c:a", "aac", "-aac_tns", "0", "-b:a", "256k"],
                  "Generated anti-phase -> AAC 256k TNS off -> 24-bit FLAC", True)
        for name, codec, suffix in [("mp3_320", ["-c:a", "libmp3lame", "-b:a", "320k"], "mp3"),
                                     ("vorbis_q8", ["-c:a", "libvorbis", "-q:a", "8"], "ogg"),
                                     ("opus_128", ["-c:a", "libopus", "-b:a", "128k"], "ogg")]:
            transcode(name, source, codec, f"Generated source -> {name} -> 24-bit FLAC", suffix=suffix)
        transient = rng.standard_normal((n, 2)) * 0.003
        for seconds in (0.2, 0.7, 1.3, 1.8, 2.4, 3.2):
            start, length = int(seconds*rate), int(0.2*rate)
            transient[start:start+length] += rng.standard_normal((length, 2)) * 0.2 * np.exp(-np.arange(length)[:, None]/(rate*0.03))
        attacks = save("clean_transients", transient, rate, "Generated attack/decay noise", True)
        transcode("aac_transients", attacks, ["-c:a", "aac", "-aac_tns", "0", "-b:a", "256k"],
                  "Generated attacks -> AAC 256k TNS off -> 24-bit FLAC; exploratory block-switching case")

(args.output / "manifest.json").write_text(json.dumps(manifest, indent=2)+"\n", encoding="utf-8")
subprocess.run([sys.executable, str(ROOT / "scripts/validate_local.py"), str(INPUT), "--output", str(args.output),
                "--source-history", "Known generated processing; see manifest.json"], check=True)
observations, failures = [], []
for path in sorted(args.output.glob("[0-9][0-9][0-9].json")):
    report = json.loads(path.read_text(encoding="utf-8"))[0]
    name = Path(report["source"]).stem
    observations.append({"case": name, "aac": [{key: a[key] for key in (
        "basis", "status", "lattice_score", "eligible_probes", "phase_offset", "scalefactor_index")} for a in report["aac"]]})
    hit = any(a["status"] == "hit" for a in report["aac"])
    if name in manifest["expected_hits"] and not hit:
        failures.append(f"Expected lattice observation absent: {name}")
    if name in manifest["expected_no_hits"] and hit:
        failures.append(f"Unexpected lattice observation: {name}")
    assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
    print(json.dumps(observations[-1]), flush=True)
(args.output / "observations.json").write_text(json.dumps(observations, indent=2)+"\n", encoding="utf-8")
if failures:
    raise SystemExit("\n".join(failures))
