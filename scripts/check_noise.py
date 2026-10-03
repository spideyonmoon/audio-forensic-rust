"""Independent NumPy band-power/quiet-run controls; generated audio only.

All audio, expected numbers and CLI reports stay under ignored corpus/local.
The oracle is computed before invoking Rust; it never adopts Rust measurements.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import wave

import numpy as np
from numpy.lib.stride_tricks import sliding_window_view

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "corpus/local/results/noise-v6"
AUDIO = ROOT / "corpus/local/generated/noise-v6"
OUT.mkdir(parents=True, exist_ok=True)
AUDIO.mkdir(parents=True, exist_ok=True)
WINDOW, HOP = 4096, 2048
window = np.hanning(WINDOW).astype(np.float32)
normalization = 2 / (WINDOW * np.sum(window.astype(float)**2))


def expected_channel(x, rate):
    edges = np.diff(np.r_[False, np.abs(x) < .01, False].astype(np.int8))
    runs = [(int(a), int(b)) for a, b in zip(np.where(edges == 1)[0], np.where(edges == -1)[0])
            if b-a >= (rate+1)//2]
    starts = np.arange(0, len(x)-WINDOW, HOP)
    frames = sliding_window_view(x.astype(np.float32), WINDOW)[:len(x)-WINDOW:HOP]
    # f32 sample/window multiplication, followed by an independent f64 FFT.
    spectrum = np.fft.rfft((frames * window).astype(float), axis=1)
    power = np.abs(spectrum)**2 * normalization
    quiet = np.array([any(a <= s and s+WINDOW <= b for a, b in runs) for s in starts], dtype=bool)
    magnitude = np.abs(spectrum)
    peak = magnitude.max(axis=1)
    active = peak > (peak.max()+1e-12)*.001
    selected = magnitude[active]
    cutoff = None
    if len(selected):
        exceeds = 20*np.log10(selected/(selected.max(axis=1, keepdims=True)+1e-12)+1e-12) > -65
        indices = WINDOW//2 - np.argmax(exceeds[:, ::-1], axis=1)
        cutoff = float(np.percentile(indices, 95)*rate/WINDOW)

    def band(lo, hi):
        frequencies = np.fft.rfftfreq(WINDOW, 1/rate)
        mask = (frequencies >= lo) & (frequencies <= hi) if lo is not None else np.zeros(len(frequencies), bool)
        supported = np.count_nonzero(mask) >= 2
        def mean(rows):
            return float(power[rows][:, mask].sum(axis=1).mean()) if supported and np.count_nonzero(rows) >= 4 else None
        return {"requested_lower_hz": lo, "requested_upper_hz": hi,
                "bin_count": int(mask.sum()) if supported else 0,
                "lower_bin_hz": float(frequencies[mask][0]) if supported else None,
                "upper_bin_hz": float(frequencies[mask][-1]) if supported else None,
                "mean_square": mean(np.ones(len(frames), bool)), "quiet_mean_square": mean(quiet)}

    upper = rate/2 - 100
    return {"quiet_runs": len(runs), "quiet_samples": sum(b-a for a, b in runs),
            "longest_quiet_run_frames": max((b-a for a, b in runs), default=0),
            "quiet_intervals": [{"start_frame": a, "end_frame": b} for a, b in runs[:16]],
            "quiet_intervals_truncated": len(runs) > 16, "stft_frames": len(frames),
            "quiet_stft_frames": int(quiet.sum()), "cutoff_p95_hz": cutoff,
            "high_band": band(16000, min(22000, upper)),
            "above_cutoff_band": band(cutoff+1000 if cutoff and cutoff > 0 else None, upper)}


rng = np.random.default_rng(20261001)
cases = []
for rate in (44100, 48000, 96000, 384000):
    t = np.arange(rate*2)/rate
    hiss = rng.uniform(-.001, .001, len(t))
    left = .2*np.sin(2*np.pi*997*t) + hiss
    left[rate//2:rate*3//2] = hiss[rate//2:rate*3//2]
    cases.append((f"quiet_gap_{rate}", rate, np.c_[left, -left]))
rate = 48000
t = np.arange(rate*2)/rate
cases += [
    ("quiet_hiss", rate, rng.uniform(-.002, .002, (len(t), 2))),
    ("low_tone_faint_hiss", rate, np.c_[.2*np.sin(2*np.pi*997*t)+rng.uniform(-1e-5, 1e-5, len(t)), np.zeros(len(t))]),
    ("high_tone", rate, np.c_[.003*np.sin(2*np.pi*18000*t), .3*np.sin(2*np.pi*18000*t)]),
    ("zero", rate, np.zeros((len(t), 2))),
    ("unsupported16k", 16000, rng.uniform(-.001, .001, (32000, 2))),
]
records = []
for name, rate, audio in cases:
    words = np.round(audio * 2**23).astype(np.int32)
    audio = words.astype(float)/2**23
    packed = ((words[..., None] >> (8*np.arange(3))) & 255).astype(np.uint8).tobytes()
    path = AUDIO / f"{name}.wav"
    with wave.open(str(path), "wb") as handle:
        handle.setparams((2, 3, rate, 0, "NONE", "not compressed"))
        handle.writeframes(packed)
    expected = [expected_channel(audio[:, ch], rate) for ch in range(2)]
    (OUT/f"{name}.expected.json").write_text(json.dumps(expected, indent=2, allow_nan=False)+"\n")
    raw = subprocess.check_output(["ffmpeg", "-nostdin", "-v", "error", "-i", str(path),
        "-c:a", "pcm_s32le", "-f", "s32le", "pipe:1"])
    digest = hashlib.sha256(raw).hexdigest()
    assert raw == (words << 8).astype("<i4").tobytes()
    output = subprocess.check_output([str(ROOT/"target/release/audio-forensic.exe"), "--json", str(path)])
    (OUT/f"{name}.json").write_bytes(output)
    report = json.loads(output)[0]
    assert report["status"] == "analyzed"
    assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
    assert report["coverage"]["decoded_pcm_sha256"] == digest
    maximum_error = 0.0
    for ch, e in enumerate(expected):
        n = report["noise"][ch]
        for key, value in e.items():
            if key not in ("high_band", "above_cutoff_band", "cutoff_p95_hz"):
                assert n[key] == value, (name, ch, key, n[key], value)
        actual_cutoff = report["channels"][ch]["spectral"]["cutoff_p95_hz"]
        assert actual_cutoff == e["cutoff_p95_hz"], (name, ch, actual_cutoff, e["cutoff_p95_hz"])
        for key in ("high_band", "above_cutoff_band"):
            for field, value in e[key].items():
                actual = n[key][field]
                if field in ("mean_square", "quiet_mean_square") and value is not None:
                    error = abs(actual-value)
                    maximum_error = max(maximum_error, error)
                    assert error <= 1e-14 + 3e-5*value, (name, ch, key, field, actual, value)
                    db_key = "rms_dbfs" if field == "mean_square" else "quiet_rms_dbfs"
                    if value > 1e-12:
                        assert abs(n[key][db_key]-10*np.log10(value)) < .002
                    elif value == 0:
                        assert n[key][db_key] is None
                else:
                    assert actual == value, (name, ch, key, field, actual, value)
    records.append({"name": name, "sample_rate": rate, "pcm_sha256": digest,
                    "maximum_absolute_power_error": maximum_error, "checks_passed": True})
    print(name, "passed", "max absolute power error", maximum_error, flush=True)
(OUT/"summary.json").write_text(json.dumps({"numpy": np.__version__, "controls": records}, indent=2)+"\n")
