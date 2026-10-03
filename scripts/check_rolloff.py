"""Local generated EQ/filter controls with an independent NumPy STFT oracle.

Never uses user recordings. Expected results are saved before running Rust.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import wave

import numpy as np
from numpy.lib.stride_tricks import sliding_window_view
from scipy import signal

ROOT = Path(__file__).resolve().parents[1]
AUDIO = ROOT/"corpus/local/generated/rolloff-v9"
OUT = ROOT/"corpus/local/results/rolloff-v9"
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)
N, HOP = 4096, 2048
WINDOW = np.hanning(N).astype(np.float32)
SCALE = 2/np.sum(WINDOW.astype(float))


def save(name, samples, rate):
    words = np.rint(samples * 2**23).astype(np.int32)
    assert np.max(np.abs(words)) < 2**23
    packed = ((words[..., None] >> (8*np.arange(3))) & 255).astype(np.uint8).tobytes()
    path = AUDIO/f"{name}.wav"
    with wave.open(str(path), "wb") as w:
        w.setparams((samples.shape[1], 3, rate, 0, "NONE", "not compressed"))
        w.writeframes(packed)
    return path


def oracle(samples, rate):
    starts = np.arange(0, len(samples)-N, HOP)
    if len(starts):
        windows = (sliding_window_view(samples.astype(np.float32), N)[:len(samples)-N:HOP]*WINDOW).astype(float)
        magnitudes = np.abs(np.fft.rfft(windows, axis=1))
        peaks = magnitudes.max(axis=1)
        selected = magnitudes[peaks > (peaks.max()+1e-12)*.001]
    else:
        selected = np.empty((0, N//2+1))
    mean = selected.mean(axis=0)*SCALE if len(selected) else np.zeros(N//2+1)
    reference = float(mean.max())
    frequencies = np.fft.rfftfreq(N, 1/rate)
    bands = []
    for center in [12000, 18000]:
        mask = (frequencies >= center-250) & (frequencies <= center+250)
        supported = center+250 < rate/2 and np.sum(mask) >= 2
        freq = frequencies[mask] if supported else np.empty(0)
        amplitude = mean[mask] if supported else np.empty(0)
        eligible = amplitude > max(1e-6, reference*1e-4)
        measured = supported and len(selected) >= 4 and np.all(eligible)
        bands.append({"status": "unsupported" if not supported else ("measured" if measured else "inconclusive"),
            "requested_center_hz": float(center), "requested_lower_hz": float(center-250), "requested_upper_hz": float(center+250),
            "lower_bin_hz": float(freq[0]) if supported else None, "upper_bin_hz": float(freq[-1]) if supported else None,
            "mean_bin_hz": float(freq.mean()) if supported else None,
            "bin_count": len(freq), "eligible_bins": int(np.sum(eligible)),
            "minimum_bin_amplitude": float(amplitude.min()) if supported and len(selected) else None,
            "mean_relative_level_db": float(np.mean(20*np.log10(amplitude/reference))) if measured else None})
    lo, hi = bands
    measured = lo["status"] == hi["status"] == "measured"
    slope = (hi["mean_relative_level_db"]-lo["mean_relative_level_db"])*1000/(hi["mean_bin_hz"]-lo["mean_bin_hz"]) if measured else None
    return {"status": "unsupported" if any(b["status"] == "unsupported" for b in bands) else ("measured" if measured else "inconclusive"),
            "interval": {"start_frame": 0, "end_frame": int(starts[-1]+N)} if len(starts) else None,
            "stft_frames": len(starts), "active_frames": len(selected),
            "reference_peak_amplitude": reference if len(selected) else None,
            "lower_band": lo, "upper_band": hi, "slope_db_per_khz": slope}


def compare(actual, expected, path=""):
    if isinstance(expected, dict):
        for key, value in expected.items():
            compare(actual[key], value, path+"/"+key)
    elif isinstance(expected, float):
        if path.endswith("amplitude"):
            tolerance = 1e-9 + abs(expected)*1e-4
        elif path.endswith("_db"):
            tolerance = .005
        elif path.endswith("slope_db_per_khz"):
            tolerance = .002
        else:
            tolerance = 1e-9
        assert actual is not None and abs(actual-expected) <= tolerance, (path, actual, expected, tolerance)
    else:
        assert actual == expected, (path, actual, expected)


paths = []
for rate in [8000, 32000, 36500, 36501, 44100, 48000, 96000, 384000]:
    x = np.zeros((max(rate, 16385), 2))
    for i in range(1024, len(x), 4096):
        x[i] = [.5, -.5]
    paths.append(save(f"flat_impulses_{rate}", x, rate))
rng = np.random.default_rng(20261009)
rate = 48000
comb = np.zeros((rate, 2))
comb[1024::2048] = [.5, -.5]
paths.append(save("gapped_spectrum_comb", comb, rate))
x = rng.uniform(-.15, .15, 3*rate)
paths.append(save("white_gain_antiphase", np.c_[x, -x*.5], rate))
freq = np.fft.rfftfreq(len(x), 1/rate)
spectrum = np.fft.rfft(x)
for slope in [-4, 4]:
    shaped = np.fft.irfft(spectrum*10**(slope*np.clip((freq-12000)/1000, -2, 8)/20), n=len(x))
    shaped *= .7/np.max(np.abs(shaped))
    paths.append(save(f"eq_{slope:+d}", np.c_[shaped, -shaped*.5], rate))
lowpass = signal.sosfilt(signal.butter(6, 8000, fs=rate, output="sos"), x)
brickwall = np.fft.irfft(spectrum*(freq < 16000), n=len(x))
paths.append(save("lowpass_brickwall", np.c_[lowpass, brickwall], rate))
notched = np.fft.irfft(spectrum*((freq < 17400) | (freq > 18600)), n=len(x))
paths.append(save("notch_and_white", np.c_[notched, x], rate))
paths.append(save("silence_tiny", np.c_[np.zeros(len(x)), x*1e-6], rate))
t = np.arange(len(x))/rate
paths.append(save("tones", np.c_[.5*np.sin(2*np.pi*1000*t), .5*np.sin(2*np.pi*12000*t)], rate))
gap = np.concatenate([x[:rate], np.zeros(rate), x[:rate]])
paths.append(save("inactive_gap", np.c_[gap, -gap], rate))
paths.append(save("short", np.zeros((4096, 2)), rate))

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
    r = json.loads(output)[0]
    assert r["status"] == "analyzed" and r["schema_version"] == "0.9.0"
    assert r["coverage"]["decoded_pcm_sha256"] == hashlib.sha256(raw).hexdigest()
    assert r["ancestry_verdict"] == "INCONCLUSIVE" and r["evidence_index"] is None
    errors = []
    for ch, e in enumerate(expected):
        compare(r["rolloff"][ch], e, f"{path.stem}/ch{ch}")
        if e["slope_db_per_khz"] is not None:
            errors.append(abs(r["rolloff"][ch]["slope_db_per_khz"]-e["slope_db_per_khz"]))
    records.append({"name": path.stem, "sample_rate": rate, "channels": channels,
        "pcm_hash_matches": True, "numerical_checks_passed": True,
        "statuses": [e["status"] for e in expected], "slopes_db_per_khz": [e["slope_db_per_khz"] for e in expected],
        "max_slope_error": max(errors, default=0.)})
    print(path.stem, records[-1]["statuses"], records[-1]["slopes_db_per_khz"], "passed", flush=True)
(OUT/"summary.json").write_text(json.dumps({"numpy": np.__version__, "controls": records}, indent=2)+"\n")
