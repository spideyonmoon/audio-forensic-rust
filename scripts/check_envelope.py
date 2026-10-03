"""Local generated band-envelope controls; independent inverse-FFT/Pearson oracle."""
import hashlib
import json
from pathlib import Path
import subprocess
import wave
import numpy as np
from numpy.lib.stride_tricks import sliding_window_view

ROOT = Path(__file__).resolve().parents[1]
AUDIO = ROOT/"corpus/local/generated/envelope-v10"
OUT = ROOT/"corpus/local/results/envelope-v10"
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)
N, HOP = 4096, 2048
WINDOW = np.hanning(N).astype(np.float32)
ENERGY = np.sum(WINDOW.astype(float)**2)

def save(name, samples, rate):
    words = np.rint(samples*2**23).astype(np.int32)
    assert np.max(np.abs(words)) < 2**23
    packed = ((words[..., None] >> (8*np.arange(3))) & 255).astype(np.uint8).tobytes()
    path = AUDIO/f"{name}.wav"
    with wave.open(str(path), "wb") as w:
        w.setparams((samples.shape[1], 3, rate, 0, "NONE", "not compressed"))
        w.writeframes(packed)
    return path

def oracle(x, rate):
    starts = np.arange(0, len(x)-N, HOP)
    if len(starts):
        windows = (sliding_window_view(x.astype(np.float32), N)[:len(x)-N:HOP]*WINDOW).astype(float)
        spectrum = np.fft.rfft(windows, axis=1)
        peaks = np.abs(spectrum).max(axis=1)
        active = peaks > (peaks.max()+1e-12)*.001
        windows, spectrum = windows[active], spectrum[active]
    else:
        windows, spectrum = np.empty((0, N)), np.empty((0, N//2+1), complex)
    count = len(windows)
    broadband = np.mean(np.sum(windows**2, axis=1)/ENERGY) if count else 0.
    frequencies = np.fft.rfftfreq(N, 1/rate)
    bands, envelopes = [], []
    for low, high in [(1000, 8000), (16000, 22000)]:
        mask = (frequencies >= low) & (frequencies <= high)
        supported = high < rate/2 and mask.sum() >= 2
        freq = frequencies[mask] if supported else np.empty(0)
        if supported and count:
            # Actual time-domain band-masked periodic window signals.
            filtered = np.fft.irfft(spectrum*mask, n=N, axis=1)
            powers = np.sum(filtered**2, axis=1)/ENERGY
            rms = np.sqrt(powers)
            power, mean, std = float(powers.mean()), float(rms.mean()), float(rms.std())
        else:
            rms = np.empty(0)
            power, mean, std = None, None, None
        energy = power is not None and power > max(1e-12, broadband*1e-8)
        varying = std is not None and std > 1e-8 and std > mean*1e-3
        measured = supported and count >= 10 and energy and varying
        bands.append({"status": "unsupported" if not supported else ("measured" if measured else "inconclusive"),
            "requested_lower_hz": float(low), "requested_upper_hz": float(high),
            "lower_bin_hz": float(freq[0]) if supported else None, "upper_bin_hz": float(freq[-1]) if supported else None,
            "bin_count": len(freq), "mean_square": power, "mean_rms": mean, "std_rms": std,
            "energy_eligible": bool(energy), "variation_eligible": bool(varying)})
        envelopes.append(rms)
    measured = all(b["status"] == "measured" for b in bands)
    coefficient = float(np.corrcoef(envelopes)[0, 1]) if measured else None
    return {"status": "unsupported" if any(b["status"] == "unsupported" for b in bands) else ("measured" if measured else "inconclusive"),
        "interval": {"start_frame": 0, "end_frame": int(starts[-1]+N)} if len(starts) else None,
        "stft_frames": len(starts), "active_frames": count,
        "mid_band": bands[0], "high_band": bands[1], "coefficient": coefficient}

def compare(actual, expected, path=""):
    if isinstance(expected, dict):
        for key, value in expected.items():
            compare(actual[key], value, path+"/"+key)
    elif isinstance(expected, float):
        if path.endswith("/coefficient"):
            tolerance = .001
        elif path.endswith("/mean_square"):
            tolerance = 1e-14+1e-5*abs(expected)
        elif path.endswith(("/mean_rms", "/std_rms")):
            tolerance = 1e-8+1e-5*abs(expected)
        else:
            tolerance = 1e-9
        assert actual is not None and abs(actual-expected) <= tolerance, (path, actual, expected, tolerance)
    else:
        assert actual == expected, (path, actual, expected)

def tones(rate, mode):
    t = np.arange(3*rate)/rate
    mid = np.full(len(t), .2) if mode == "constant" else .2+.12*np.sin(2*np.pi*t)
    high = .2-.12*np.sin(2*np.pi*t) if mode == "opposite" else mid.copy()
    if mode == "independent": high = .2+.12*np.sin(2*np.pi*3*t)
    if mode == "mid_only": high.fill(0)
    if mode == "high_only": mid.fill(0)
    return mid*np.sin(2*np.pi*3000*t)+high*np.sin(2*np.pi*18000*t)

paths = []
for rate in [8000, 32000, 44000, 44001, 44100, 48000, 96000, 384000]:
    x = tones(rate, "shared")
    paths.append(save(f"shared_{rate}", np.c_[x, -x*.5], rate))
rate = 48000
for mode in ["opposite", "independent", "constant", "mid_only", "high_only"]:
    x = tones(rate, mode)
    paths.append(save(mode, np.c_[x, -x], rate))
x = tones(rate, "shared")
paths.append(save("tiny_and_silence", np.c_[x*1e-6, np.zeros(len(x))], rate))
paths.append(save("silent_channel", np.c_[x, np.zeros(len(x))], rate))
x[rate:2*rate] = 0
paths.append(save("inactive_gap", np.c_[x, -x], rate))
rng = np.random.default_rng(20261010)
white = rng.uniform(-.3, .3, (3*rate, 2))
paths.append(save("independent_noise", white, rate))
paths.append(save("short", np.zeros((4096, 2)), rate))

records = []
for path in paths:
    with wave.open(str(path), "rb") as w:
        rate, channels = w.getframerate(), w.getnchannels()
    raw = subprocess.check_output(["ffmpeg", "-nostdin", "-v", "error", "-i", str(path), "-c:a", "pcm_s32le", "-f", "s32le", "pipe:1"])
    audio = np.frombuffer(raw, dtype="<i4").reshape(-1, channels).astype(float)/2**31
    expected = [oracle(audio[:, ch], rate) for ch in range(channels)]
    (OUT/f"{path.stem}.expected.json").write_text(json.dumps(expected, indent=2, allow_nan=False)+"\n")
    output = subprocess.check_output([str(ROOT/"target/release/audio-forensic.exe"), "--json", str(path)])
    (OUT/f"{path.stem}.json").write_bytes(output)
    r = json.loads(output)[0]
    assert r["status"] == "analyzed" and r["schema_version"] == "0.10.0"
    assert r["coverage"]["decoded_pcm_sha256"] == hashlib.sha256(raw).hexdigest()
    assert r["ancestry_verdict"] == "INCONCLUSIVE" and r["evidence_index"] is None
    errors = []
    for ch, e in enumerate(expected):
        compare(r["envelope"][ch], e, f"{path.stem}/ch{ch}")
        if e["coefficient"] is not None: errors.append(abs(r["envelope"][ch]["coefficient"]-e["coefficient"]))
    records.append({"name": path.stem, "sample_rate": rate, "channels": channels,
        "pcm_hash_matches": True, "numerical_checks_passed": True,
        "statuses": [e["status"] for e in expected], "coefficients": [e["coefficient"] for e in expected],
        "max_coefficient_error": max(errors, default=0.)})
    print(path.stem, records[-1]["statuses"], records[-1]["coefficients"], "passed", flush=True)
(OUT/"summary.json").write_text(json.dumps({"numpy": np.__version__, "controls": records}, indent=2)+"\n")
