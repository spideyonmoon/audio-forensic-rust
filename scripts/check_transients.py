"""Local generated controls; independent SciPy filter/envelope/peak oracle.

Expected measurements are saved before invoking Rust. This verifies arithmetic,
coverage and bounded reporting, not source provenance or click detection accuracy.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import wave

import numpy as np
import scipy
from scipy import signal

ROOT = Path(__file__).resolve().parents[1]
AUDIO = ROOT / "corpus/local/generated/transients-v8"
OUT = ROOT / "corpus/local/results/transients-v8"
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)
ABS_TOL, REL_TOL = 1e-10, 1e-8


def save(name, samples, rate):
    words = np.round(samples * 2**23).astype(np.int32)
    assert np.max(np.abs(words)) < 2**23
    packed = ((words[..., None] >> (8*np.arange(3))) & 255).astype(np.uint8).tobytes()
    path = AUDIO / f"{name}.wav"
    with wave.open(str(path), "wb") as w:
        w.setparams((samples.shape[1], 3, rate, 0, "NONE", "not compressed"))
        w.writeframes(packed)
    return path


def median_bounds(envelope):
    if not len(envelope):
        return None, None, None
    ranks = [(len(envelope)-1)//2, len(envelope)//2]
    middle = np.partition(envelope, ranks)[ranks]
    bounds = []
    for value in middle:
        if value == 0:
            bounds.append((0., 0.))
        elif value <= 1e-9:
            bounds.append((0., 1e-9))
        else:
            db = np.floor(20*np.log10(value)*4)/4
            bounds.append((10**(db/20), 10**((db+.25)/20)))
    return float(middle.mean()), *np.mean(bounds, axis=0).tolist()


def oracle(samples, rate):
    samples = samples[:180*rate]
    width, warmup = (rate//2000)|1, (rate+49)//50
    half, distance = width//2, max(1, rate//100)
    # SciPy designs the SOS coefficients independently of the Rust biquads.
    filtered = signal.sosfilt(signal.butter(4, 1000, btype="highpass", fs=rate, output="sos"), samples)
    # Full convolution support only, with explicit center coordinates; no rolling sum.
    if len(samples) >= width:
        envelope = np.convolve(np.abs(filtered), np.ones(width)/width, mode="valid") * np.pi/2
        envelope = envelope[max(0, warmup-half):]
    else:
        envelope = np.empty(0)
    exact, lower, upper = median_bounds(envelope)
    start, end = warmup+1, len(samples)-half-1
    interval = {"start_frame": start, "end_frame": end} if end > start else None
    measured = interval is not None and end-start >= (rate+1)//2
    threshold = max(1e-4, 3*upper) if measured else None
    local, _ = signal.find_peaks(envelope)  # Includes SciPy plateau midpoint handling.
    accepted = []
    if measured:
        for point in local:
            frame = int(point+warmup)
            if envelope[point] > threshold and (not accepted or frame-accepted[-1]["frame"] >= distance):
                accepted.append({"frame": frame, "envelope_peak": float(envelope[point])})
    # Compare peak-selection policies on the same eligible envelope, separately
    # from the reference's different edges, threshold floor and normalizer.
    priority, _ = signal.find_peaks(envelope, height=threshold, distance=distance) if measured else ([], {})
    return {
        "status": "measured" if measured else "inconclusive",
        "interval": {"start_frame": 0, "end_frame": len(samples)},
        "eligible_peak_interval": interval, "envelope_samples": len(envelope),
        "highpass_cutoff_hz": 1000., "filter_order": 4, "smoothing_frames": width,
        "warmup_frames": warmup, "minimum_peak_distance_frames": distance,
        "baseline_median_lower": lower, "baseline_median_upper": upper,
        "envelope_threshold": threshold, "peak_count": len(accepted) if measured else None,
        "peaks_per_minute": len(accepted)*60*rate/(end-start) if measured else None,
        "events_truncated": len(accepted) > 128, "events": accepted[:128],
        "oracle_exact_median": exact,
        "oracle_height_priority_frames": [int(p+warmup) for p in priority[:128]],
        "oracle_height_priority_count": len(priority) if measured else None,
    }


def compare(actual, expected, path=""):
    if isinstance(expected, dict):
        for key, value in expected.items():
            if not key.startswith("oracle_"):
                compare(actual[key], value, path+"/"+key)
    elif isinstance(expected, list):
        assert len(actual) == len(expected), (path, len(actual), len(expected))
        for i, value in enumerate(expected):
            compare(actual[i], value, path+f"/{i}")
    elif isinstance(expected, float):
        # Exact-zero vs positive sub-1e-9 tails can differ because Rust flushes
        # filter state below 1e-30; compare the interval containment separately.
        if path.endswith(("/baseline_median_lower", "/baseline_median_upper")) and expected <= 1e-9:
            assert actual is not None and 0 <= actual <= 1e-9, (path, actual, expected)
        else:
            assert actual is not None and abs(actual-expected) <= ABS_TOL+REL_TOL*abs(expected), (path, actual, expected)
    else:
        assert actual == expected, (path, actual, expected)


rng = np.random.default_rng(20261008)
paths = []
for rate in [8000, 44100, 48000, 96000, 384000]:
    x = np.zeros((2*rate, 2))
    for second in [.1, .4, 1.2, 1.7]:
        x[int(second*rate)] = [.5, -.5]
    paths.append(save(f"impulses_{rate}", x, rate))
rate = 48000
t = np.arange(2*rate)/rate
paths.append(save("silence_low_level", np.c_[np.zeros(len(t)), rng.uniform(-2e-7, 2e-7, len(t))], rate))
x = rng.uniform(-.08, .08, len(t))
paths.append(save("stationary_noise", np.c_[x, -x], rate))
x = np.zeros((len(t), 2))
for start in [4800, 24000, 48000]:
    x[start] = [.2, .7]
    x[start+240] = [.7, .2]
paths.append(save("nearby_unequal", x, rate))
x = np.zeros((5*rate, 2))
for i in range(200):
    x[4800+i*960] = [.5, 0]
paths.append(save("dense_train", x, rate))
x = np.zeros(len(t))
for onset in [.25, 1., 1.5]:
    u = np.arange(rate//5)/rate
    x[int(onset*rate):int(onset*rate)+len(u)] = .5*(1-np.exp(-u/.001))*np.exp(-u/.025)*np.sin(2*np.pi*5000*u)
clipped = np.clip(.6*np.sin(2*np.pi*12000*t), -.2, .2)
paths.append(save("musical_attacks_clipping", np.c_[x, clipped], rate))
paths.append(save("steps_and_dc", np.c_[.4*((t >= .3) & (t < 1.3)), np.full(len(t), .4)], rate))
x = np.zeros((len(t), 2))
x[0] = [.5, -.5]
x[-1] = [.5, -.5]
paths.append(save("edge_impulses", x, rate))
paths.append(save("short", np.zeros((100, 2)), rate))
rate = 8000
x = np.zeros((181*rate, 2))
x[179*rate] = [.5, -.5]
x[180*rate+rate//2] = [.5, -.5]
paths.append(save("capped181s", x, rate))

records = []
for path in paths:
    with wave.open(str(path), "rb") as w:
        rate, channels = w.getframerate(), w.getnchannels()
    raw = subprocess.check_output(["ffmpeg", "-nostdin", "-v", "error", "-i", str(path), "-c:a", "pcm_s32le", "-f", "s32le", "pipe:1"])
    samples = np.frombuffer(raw, dtype="<i4").reshape(-1, channels).astype(float)/2**31
    expected = [oracle(samples[:, ch], rate) for ch in range(channels)]
    (OUT/f"{path.stem}.expected.json").write_text(json.dumps(expected, indent=2, allow_nan=False)+"\n")
    output = subprocess.check_output([str(ROOT/"target/release/audio-forensic.exe"), "--json", str(path)])
    (OUT/f"{path.stem}.json").write_bytes(output)
    report = json.loads(output)[0]
    assert report["status"] == "analyzed" and report["schema_version"] == "0.8.0"
    assert report["coverage"]["decoded_pcm_sha256"] == hashlib.sha256(raw).hexdigest()
    assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
    errors = []
    for ch, e in enumerate(expected):
        actual = report["transients"][ch]
        compare(actual, e, f"{path.stem}/ch{ch}")
        median = e["oracle_exact_median"]
        if median is not None:
            assert actual["baseline_median_lower"]-ABS_TOL <= median <= actual["baseline_median_upper"]+ABS_TOL
        errors += [abs(a["envelope_peak"]-b["envelope_peak"]) for a, b in zip(actual["events"], e["events"])]
    records.append({"name": path.stem, "sample_rate": rate, "channels": channels,
                    "pcm_hash_matches": True, "numerical_checks_passed": True,
                    "peak_counts": [e["peak_count"] for e in expected],
                    "max_peak_amplitude_error": max(errors, default=0.)})
    print(path.stem, records[-1]["peak_counts"], "passed", flush=True)
(OUT/"summary.json").write_text(json.dumps({"numpy": np.__version__, "scipy": scipy.__version__,
    "absolute_tolerance": ABS_TOL, "relative_tolerance": REL_TOL, "controls": records}, indent=2)+"\n")
