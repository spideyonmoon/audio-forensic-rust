"""Generated controls with an independent inverse-FFT/time-domain oracle.

Only local generated audio is used. Expected values are saved before invoking
Rust; correlation is computed from filtered samples, not the Rust cosine sum.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import wave

import numpy as np
from numpy.lib.stride_tricks import sliding_window_view

ROOT = Path(__file__).resolve().parents[1]
AUDIO = ROOT / "corpus/local/generated/noise-dynamics-v7"
OUT = ROOT / "corpus/local/results/noise-dynamics-v7"
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)
N, HOP = 4096, 2048
WINDOW = np.hanning(N).astype(np.float32)
WINDOW_ENERGY = np.sum(WINDOW.astype(float)**2)


def save(name, x, rate):
    words = np.round(x * 2**23).astype(np.int32)
    packed = ((words[..., None] >> (8*np.arange(3))) & 255).astype(np.uint8).tobytes()
    path = AUDIO / f"{name}.wav"
    with wave.open(str(path), "wb") as w:
        w.setparams((x.shape[1], 3, rate, 0, "NONE", "not compressed"))
        w.writeframes(packed)
    return path


def oracle(x, rate):
    starts = np.arange(0, len(x)-N, HOP)
    windowed = (sliding_window_view(x.astype(np.float32), N)[:len(x)-N:HOP] * WINDOW).astype(float)
    spectrum = np.fft.rfft(windowed, axis=1)
    mags = np.abs(spectrum)
    peaks = mags.max(axis=1)
    active = peaks > (peaks.max()+1e-12)*.001
    selected = mags[active]
    cutoff = None
    if len(selected):
        exceeds = 20*np.log10(selected/(selected.max(axis=1, keepdims=True)+1e-12)+1e-12) > -65
        cutoff = float(np.percentile(N//2-np.argmax(exceeds[:, ::-1], axis=1), 95)*rate/N)
    broadband = np.sum(windowed**2, axis=1)/WINDOW_ENERGY
    frequencies = np.fft.rfftfreq(N, 1/rate)
    result = {"cutoff_p95_hz": cutoff}
    for name, low, high in [("high_band", 16000, min(22000, rate/2-100)),
                            ("above_cutoff_band", cutoff+1000 if cutoff and cutoff > 0 else None, rate/2-100)]:
        mask = ((frequencies >= low) & (frequencies <= high)) if low else np.zeros(len(frequencies), bool)
        supported = mask.sum() >= 2
        # Actual time-domain band-masked periodic signals; no Rust reduction formula.
        filtered = np.fft.irfft(spectrum * mask, n=N, axis=1)
        power = np.sum(filtered**2, axis=1)/WINDOW_ENERGY
        usable = supported and len(starts) >= 4 and power.mean() > max(1e-12, broadband.mean()*1e-8)
        correlations = []
        for minimum, base in [(25, 50), (50, 100)]:
            lag = max(minimum, int(np.floor(base*rate/44100+.5)))
            value = float(np.sum(filtered*np.roll(filtered, -lag, axis=1))/np.sum(filtered**2)) if usable else None
            correlations.append({"lag_frames": lag, "coefficient": value,
                "status": "measured" if usable else ("inconclusive" if supported else "unsupported")})
        blocks = []
        for second in range(min(len(x)//rate, 180)):
            selected = (starts >= second*rate) & (starts+N <= (second+1)*rate)
            if not selected.any():
                continue
            mean = float(power[selected].mean()) if supported else None
            eligible = supported and mean > max(1e-12, float(broadband[selected].mean())*1e-8)
            blocks.append({"block_index": second,
                "interval": {"start_frame": int(starts[selected][0]), "end_frame": int(starts[selected][-1]+N)},
                "stft_frames": int(selected.sum()), "mean_square": mean,
                "rms_dbfs": float(10*np.log10(mean)) if mean and mean > 0 else None, "eligible": bool(eligible)})
        count = sum(b["eligible"] for b in blocks)
        sufficient = len(blocks) >= 2 and count == len(blocks)
        result[name] = {"correlations": correlations, "temporal_variation": {
            "status": "unsupported" if not supported else ("measured" if sufficient else "inconclusive"),
            "eligible_blocks": count, "search_limit_frames": 180*rate,
            "level_std_db": float(np.std([b["rms_dbfs"] for b in blocks])) if sufficient else None,
            "blocks": blocks}}
    return result


rng = np.random.default_rng(20261002)
rate = 48000
t = np.arange(rate*3)/rate
base = rng.uniform(-.002, .002, (rate, 2))
steps = np.concatenate([base, base*.5, base*.25])
paths = [save("level_steps", steps, rate),
         save("stationary_hiss", np.tile(base, (3, 1)), rate),
         save("quiet_middle", np.concatenate([base, np.zeros_like(base), base]), rate),
         save("tone_leakage", np.c_[.3*np.sin(2*np.pi*997*t), .2*np.sin(2*np.pi*17891*t)], rate),
         save("below_floor", np.c_[rng.uniform(-1e-7, 1e-7, len(t)), np.zeros(len(t))], rate)]
impulses = np.zeros((len(t), 2))
for second in (.2, .6, 1.2, 2.2, 2.6, 2.8):
    impulses[int(second*rate)] = [.5, -.5]
paths.append(save("impulses", impulses, rate))
rate = 8000
t = np.arange(rate)/rate
low_rate = np.c_[.2*np.sin(2*np.pi*997*t)+rng.uniform(-2e-5, 2e-5, len(t)), np.zeros(len(t))]
long = np.tile(low_rate, (182, 1))
long[180*rate:] = 0
paths.append(save("capped182s", long, rate))
paths += sorted((ROOT/"corpus/local/generated/noise-v6").glob("*.wav"))
assert len(paths) == 16, "Run the existing generated v0.6 controls first if absent"


def compare(actual, expected, path=""):
    if isinstance(expected, dict):
        for key, value in expected.items():
            compare(actual[key], value, path+"/"+key)
    elif isinstance(expected, list):
        assert len(actual) == len(expected), (path, len(actual), len(expected))
        for i, value in enumerate(expected):
            compare(actual[i], value, path+f"/{i}")
    elif isinstance(expected, float):
        if path.endswith("/mean_square"):
            tolerance = 1e-14 + 3e-5*expected
        elif path.endswith("/coefficient"):
            tolerance = .001
        elif path.endswith("/rms_dbfs") and expected < -110:
            # Below -110 dBFS compare in linear power, where the f32 floor matters.
            return
        elif path.endswith(("/rms_dbfs", "/level_std_db")):
            tolerance = .005
        else:
            tolerance = 1e-9
        assert actual is not None and abs(actual-expected) <= tolerance, (path, actual, expected, tolerance)
    else:
        assert actual == expected, (path, actual, expected)


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
    report = json.loads(output)[0]
    assert report["status"] == "analyzed"
    assert report["coverage"]["decoded_pcm_sha256"] == hashlib.sha256(raw).hexdigest()
    assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
    for ch in range(channels):
        cutoff = expected[ch].pop("cutoff_p95_hz")
        compare(report["channels"][ch]["spectral"]["cutoff_p95_hz"], cutoff, "cutoff")
        compare(report["noise"][ch], expected[ch], f"{path.stem}/ch{ch}")
    records.append({"name": path.stem, "sample_rate": rate, "channels": channels,
                    "pcm_hash_matches": True, "numerical_checks_passed": True})
    print(path.stem, "passed", flush=True)
(OUT/"summary.json").write_text(json.dumps({"numpy": np.__version__, "controls": records}, indent=2)+"\n")
