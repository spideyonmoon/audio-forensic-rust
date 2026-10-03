"""Generated spectral-structure applicability controls; no ancestry ground truth.

Requires NumPy, FFmpeg and the release CLI. New output locations preserve prior
milestones. Private recordings are never read or changed.
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
parser.add_argument("--work", type=Path, default=ROOT / "corpus/local/generated/structure-v5")
parser.add_argument("--output", type=Path, default=ROOT / "corpus/local/results/structure-v5")
args = parser.parse_args()
INPUT = args.work / "decoded"
INPUT.mkdir(parents=True, exist_ok=True)
args.output.mkdir(parents=True, exist_ok=True)
manifest = {"numpy": np.__version__, "seed": 508,
            "ffmpeg": subprocess.check_output(["ffmpeg", "-version"], text=True).splitlines()[0],
            "histories": {}, "scope": "Related generated signals and filters only; no codec or authenticity labels"}


def save(name, audio, rate, history):
    words = np.rint(audio * 2**23).clip(-2**23, 2**23-1).astype("<i4")
    with wave.open(str(INPUT / f"{name}.wav"), "wb") as handle:
        handle.setparams((audio.shape[1], 3, rate, 0, "NONE", "not compressed"))
        handle.writeframes(words.view(np.uint8).reshape(-1, 4)[:, :3].tobytes())
    manifest["histories"][name] = history


rate = 48000
rng = np.random.default_rng(508)
noise = rng.standard_normal((rate*2, 2))*0.1
save("white_noise", noise, rate, "Generated independent stereo white noise, seed 508; no codec")
freq = np.fft.rfftfreq(len(noise), 1/rate)
for cutoff in (9000, 15000):
    transformed = np.fft.rfft(noise, axis=0)
    transformed[freq > cutoff] = 0
    filtered = np.fft.irfft(transformed, n=len(noise), axis=0)
    save(f"lowpass_{cutoff}", filtered, rate, f"Same generated noise, FFT lowpass at {cutoff} Hz; no codec")
save("silent_channel", np.column_stack((noise[:, 0], np.zeros(len(noise)))), rate,
     "Generated noise left, exact silence right")
save("anti_phase", np.column_stack((noise[:, 0], -noise[:, 0])), rate,
     "Generated noise, opposite-sign native channels")
save("silence", np.zeros_like(noise), rate, "Exact digital silence")
gapped = noise.copy()
gapped[rate//2:rate] = 0
save("inactive_gap", gapped, rate, "Generated noise with 0.5-second silent gap")
tone = 0.2*np.sin(2*np.pi*12000*np.arange(rate*2)/rate)
save("hf_tone", np.column_stack((tone, tone)), rate, "Generated 12 kHz tone; no codec")
save("low_rate", rng.standard_normal((16000*2, 2))*0.1, 16000,
     "Generated stereo noise at 16 kHz; 10 kHz+ phase band unavailable")
(args.output / "manifest.json").write_text(json.dumps(manifest, indent=2)+"\n", encoding="utf-8")
subprocess.run([sys.executable, str(ROOT / "scripts/validate_local.py"), str(INPUT), "--output", str(args.output),
    "--source-history", "Generated controls; see manifest.json"], check=True)
observations = []
for path in sorted(args.output.glob("[0-9][0-9][0-9].json")):
    report = json.loads(path.read_text(encoding="utf-8"))[0]
    name = Path(report["source"]).stem
    rows = report["spectral_structure"]
    assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
    assert all(s["scatter_status"] in ("measured", "inconclusive") and
               s["phase_status"] in ("measured", "inconclusive", "unsupported") for s in rows)
    if name == "white_noise":
        assert all(s["high_band_phase_entropy_bits"] > 5.1 for s in rows)
    elif name == "lowpass_9000":
        assert all(s["high_band_phase_entropy_bits"] is None for s in rows)
    elif name == "hf_tone":
        assert all(s["high_band_phase_entropy_bits"] == 0 for s in rows)
    elif name == "low_rate":
        assert all(s["phase_status"] == "unsupported" for s in rows)
    elif name == "inactive_gap":
        assert all(s["phase_frame_pairs"] < s["active_frames"] - 1 for s in rows)
    elif name == "silent_channel":
        assert rows[0]["phase_status"] == "measured" and rows[1]["phase_status"] == "inconclusive"
        assert rows[1]["average_scatter_bound_hz"] is None
    elif name == "anti_phase":
        assert abs(rows[0]["high_band_phase_entropy_bits"]-rows[1]["high_band_phase_entropy_bits"]) < 1e-12
    elif name == "silence":
        assert all(s["high_band_phase_entropy_bits"] is None and s["average_scatter_bound_hz"] is None for s in rows)
    observations.append({"case": name, "channels": rows})
    print(name, json.dumps(rows), flush=True)
(args.output / "observations.json").write_text(json.dumps(observations, indent=2)+"\n", encoding="utf-8")
