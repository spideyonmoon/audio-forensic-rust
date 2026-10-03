"""Local generated controls: independent NumPy FFT and dense frame/bin oracle."""
import hashlib
import json
from pathlib import Path
import subprocess
import wave
import numpy as np
from numpy.lib.stride_tricks import sliding_window_view

ROOT = Path(__file__).resolve().parents[1]
AUDIO = ROOT/"corpus/local/generated/sparsity-v11"
OUT = ROOT/"corpus/local/results/sparsity-v11"
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)
N, HOP = 4096, 2048
WINDOW = np.hanning(N).astype(np.float32)
RELATIVE = 10**(-95/20)
# Predeclared f32 FFT comparison allowance, in magnitude/frame-peak units.
# Counts may differ only by the number of independently identified near-threshold
# observations, and must agree exactly when that number is zero.
AMBIGUITY = 2e-7

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
        magnitudes = np.abs(np.fft.rfft(windows, axis=1))
        peaks = magnitudes.max(axis=1)
        active = peaks > (peaks.max()+1e-12)*.001
        magnitudes, peaks = magnitudes[active], peaks[active]
    else:
        magnitudes, peaks = np.empty((0, N//2+1)), np.empty(0)
    count = len(peaks)
    frequencies = np.fft.rfftfreq(N, 1/rate)
    cutoff = None
    if count:
        above = 20*np.log10(magnitudes/(peaks[:, None]+1e-12)+1e-12) > -65
        last = N//2-np.argmax(above[:, ::-1], axis=1)
        cutoff = float(np.percentile(last, 95, method="linear"))*rate/N
    eligible = peaks*2/WINDOW.astype(float).sum() > 1e-6
    eligible_count = int(eligible.sum())
    mask = (frequencies > 0) & (frequencies < rate/2) & (frequencies < (cutoff or 0))
    bins = int(mask.sum())
    measured = eligible_count >= 4 and bins >= 10
    ratios = magnitudes[eligible][:, mask]/peaks[eligible, None]
    sparse = int((ratios < RELATIVE).sum()) if measured else None
    total = eligible_count*bins if measured else None
    ambiguous = int((np.abs(ratios-RELATIVE) <= AMBIGUITY).sum()) if measured else 0
    return {"status": "measured" if measured else "inconclusive",
        "interval": {"start_frame": 0, "end_frame": int(starts[-1]+N)} if len(starts) else None,
        "stft_frames": len(starts), "active_frames": count, "eligible_frames": eligible_count,
        "below_peak_floor_frames": count-eligible_count, "cutoff_p95_hz": cutoff,
        "lower_bin_hz": float(frequencies[mask][0]) if bins else None,
        "upper_bin_hz": float(frequencies[mask][-1]) if bins else None, "bin_count": bins,
        "sparse_bin_observations": sparse, "total_bin_observations": total,
        "fraction": sparse/total if measured else None}, ambiguous

def compare(actual, expected, ambiguous, path):
    for key, value in expected.items():
        a = actual[key]
        if value is None:
            assert a is None, (path, key, a, value)
        elif key == "sparse_bin_observations":
            assert abs(a-value) <= ambiguous, (path, key, a, value, ambiguous)
        elif key == "fraction":
            assert abs(a-value) <= ambiguous/expected["total_bin_observations"]+1e-14, (path, key, a, value)
            assert abs(a-actual["sparse_bin_observations"]/actual["total_bin_observations"]) < 1e-14
        elif isinstance(value, float):
            assert abs(a-value) <= 1e-9, (path, key, a, value)
        else:
            assert a == value, (path, key, a, value)

paths = []
for rate in [8000, 32000, 44100, 48000, 96000, 384000]:
    x = np.zeros((max(rate, 16385), 2))
    x[1024::4096] = [.5, -.25]
    paths.append(save(f"flat_impulses_{rate}", x, rate))
rate = 48000
rng = np.random.default_rng(20261011)
x = rng.uniform(-.2, .2, 3*rate)
paths.append(save("white_gain_antiphase", np.c_[x, -x*.5], rate))
spectrum = np.fft.rfft(x)
freq = np.fft.rfftfreq(len(x), 1/rate)
notched = np.fft.irfft(spectrum*((freq < 6000) | (freq > 10000)), n=len(x))
lowpass = np.fft.irfft(spectrum*(freq < 8000), n=len(x))
paths.append(save("notch_and_lowpass", np.c_[notched, lowpass], rate))
t = np.arange(len(x))/rate
tone = .4*np.sin(2*np.pi*6000*t)
paths.append(save("tone_gain_antiphase", np.c_[tone, -tone*.5], rate))
paths.append(save("silence_tiny", np.c_[np.zeros(len(x)), tone*1e-6], rate))
paths.append(save("dc_low_tone", np.c_[np.full(len(x), .25), .4*np.sin(2*np.pi*10*t)], rate))
gap = tone.copy()
gap[rate:2*rate] = 0
paths.append(save("gap_silent_channel", np.c_[gap, np.zeros(len(x))], rate))
mixed = np.r_[lowpass[:rate], x[rate:]]
paths.append(save("changing_bandwidth", np.c_[mixed, -mixed], rate))
for frames in [0, 3, 4]:
    length = N if frames == 0 else N+(frames-1)*HOP+1
    paths.append(save(f"frames_{frames}", np.c_[tone[:length], -tone[:length]], rate))
for bin_index in [9, 10]:
    edge_tone = .4*np.sin(2*np.pi*(rate/N*bin_index)*t)
    paths.append(save(f"selected_bins_{bin_index}", np.c_[edge_tone, -edge_tone], rate))
# The global cutoff includes active frames rejected by the absolute peak floor.
# Strong tone and quiet broadband both pass global activity; only the tone has
# a coherent spectral peak above the floor. Float WAV controls in Rust test the
# floor further below the 24-bit quantization step.
small = .0001*np.sin(2*np.pi*1000*t)
small[rate:] = x[rate:]*1e-5
paths.append(save("global_cutoff_quiet_frames", np.c_[small, -small], rate))

records = []
for path in paths:
    with wave.open(str(path), "rb") as w:
        rate, channels = w.getframerate(), w.getnchannels()
    raw = subprocess.check_output(["ffmpeg", "-nostdin", "-v", "error", "-i", str(path), "-c:a", "pcm_s32le", "-f", "s32le", "pipe:1"])
    samples = np.frombuffer(raw, dtype="<i4").reshape(-1, channels).astype(float)/2**31
    expected = [oracle(samples[:, ch], rate) for ch in range(channels)]
    saved = json.dumps(expected, indent=2, allow_nan=False)+"\n"
    reference = OUT/f"{path.stem}.expected.json"
    if reference.exists():
        assert reference.read_text() == saved, "Refusing to change saved oracle: "+str(reference)
    else:
        reference.write_text(saved)
    output = subprocess.check_output([str(ROOT/"target/release/audio-forensic.exe"), "--json", str(path)])
    (OUT/f"{path.stem}.json").write_bytes(output)
    report = json.loads(output)[0]
    assert report["status"] == "analyzed" and report["schema_version"] == "0.11.0"
    assert report["coverage"]["decoded_pcm_sha256"] == hashlib.sha256(raw).hexdigest()
    assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
    errors = []
    for ch, (e, ambiguous) in enumerate(expected):
        compare(report["sparsity"][ch], e, ambiguous, f"{path.stem}/ch{ch}")
        if e["sparse_bin_observations"] is not None:
            errors.append(abs(report["sparsity"][ch]["sparse_bin_observations"]-e["sparse_bin_observations"]))
    records.append({"name": path.stem, "pcm_hash_matches": True, "numerical_checks_passed": True,
        "statuses": [e["status"] for e, _ in expected], "fractions": [report["sparsity"][ch]["fraction"] for ch in range(channels)],
        "ambiguous_observations": [a for _, a in expected], "max_sparse_count_error": max(errors, default=0)})
    print(path.stem, records[-1], flush=True)
(OUT/"summary.json").write_text(json.dumps({"numpy": np.__version__, "relative_ratio_ambiguity": AMBIGUITY, "controls": records}, indent=2)+"\n")
