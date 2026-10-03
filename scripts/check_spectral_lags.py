"""Generated controls and independent dense NumPy spectral-lag arithmetic.

Expected JSON is immutable and saved before Rust runs; no private recordings.
Predeclared tolerances: amplitudes 1e-9 + 1e-4*expected, std .005 dB,
coefficients .0002, exact counts/status/coverage/nullability and PCM hashes.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import wave

import numpy as np
from numpy.lib.stride_tricks import sliding_window_view
from scipy import signal
from scipy.fft import rfft as rfft32

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=ROOT/'corpus/local/results/spectral-lags-v15')
parser.add_argument('--case-pattern', default='')
parser.add_argument('--schema-version', default='0.15.0')
args = parser.parse_args()
AUDIO = ROOT/'corpus/local/generated/spectral-lags-v15'
OUT = args.output
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)
N, HOP = 4096, 2048
WINDOW = np.hanning(N).astype(np.float32)
SCALE = 2/np.sum(WINDOW.astype(float))


def immutable(path, expected):
    text = json.dumps(expected, indent=2, allow_nan=False)+'\n'
    if path.exists():
        assert path.read_text() == text, ('changed oracle', path)
    else:
        path.write_text(text)


def save(name, samples, rate):
    if samples.ndim == 1:
        samples = samples[:, None]
    words = np.rint(samples*2**23).astype(np.int32)
    assert np.max(np.abs(words)) < 2**23
    raw = ((words[..., None] >> (8*np.arange(3))) & 255).astype(np.uint8).tobytes()
    path = AUDIO/f'{name}.wav'
    if path.exists():
        with wave.open(str(path), 'rb') as w:
            assert w.getframerate() == rate and w.getnchannels() == samples.shape[1]
            assert w.readframes(w.getnframes()) == raw, ('changed audio', path)
    else:
        with wave.open(str(path), 'wb') as w:
            w.setparams((samples.shape[1], 3, rate, 0, 'NONE', 'not compressed'))
            w.writeframes(raw)
    return path


def oracle(x, rate):
    starts = np.arange(0, len(x)-N, HOP)
    if len(starts):
        windows = (sliding_window_view(x.astype(np.float32), N)[:len(x)-N:HOP]*WINDOW).astype(float)
        magnitudes = np.abs(np.fft.rfft(windows, axis=1))
        peaks = magnitudes.max(axis=1)
        active = magnitudes[peaks > (peaks.max()+1e-12)*.001]
    else:
        active = np.empty((0, N//2+1))
    mean = active.mean(axis=0)*SCALE if len(active) else np.zeros(N//2+1)
    # Exact integer rational membership, evaluated as a dense mask.
    bins = np.arange(N//2+1)
    complete = rate > 40000
    mask = ((bins*rate >= 16000*N) & (bins*rate < 20000*N) & (bins < N//2)) if complete else np.zeros(len(bins), bool)
    selected = mean[mask]
    frequencies = bins[mask]*rate/N
    supported = complete and len(selected) >= 81
    reference = float(mean.max())
    eligible = selected > max(1e-6, reference*1e-4)
    energy_ok = supported and len(active) >= 4 and np.all(eligible)
    std = None
    centered = np.empty(0)
    if energy_ok:
        db = 20*np.log10(selected/reference)
        centered = db-db.mean()
        std = float(np.std(db))
    variable = std is not None and std > .1

    def lag_result(lag):
        pairs = max(0, len(selected)-lag)
        geometry = supported and pairs >= 16
        coefficient = float(np.dot(centered[:-lag], centered[lag:])/np.dot(centered, centered)) if geometry and variable else None
        return dict(lag_bins=lag, lag_hz=lag*rate/N, pair_count=pairs,
                    status='unsupported' if not geometry else ('measured' if variable else 'inconclusive'),
                    coefficient=coefficient)

    return dict(status='unsupported' if not supported else ('measured' if variable else 'inconclusive'),
                interval=dict(start_frame=0, end_frame=int(starts[-1]+N)) if len(starts) else None,
                stft_frames=len(starts), active_frames=len(active), requested_lower_hz=16000.,
                requested_upper_hz_exclusive=20000., lower_bin_hz=float(frequencies[0]) if complete else None,
                upper_bin_hz=float(frequencies[-1]) if complete else None, bin_count=len(selected),
                eligible_bins=int(np.sum(eligible)) if len(active) else 0,
                reference_peak_amplitude=reference if len(active) else None,
                minimum_bin_amplitude=float(selected.min()) if complete and len(active) else None,
                log_magnitude_std_db=std,
                lags=[dict(multiple=m, requested_lag_hz=m*rate/64,
                           target=lag_result(m*64), neighbours=[lag_result(m*64-3), lag_result(m*64+3)]) for m in (1, 2, 3)])


errors = dict(amplitude=0., std_db=0., coefficient=0.)


def fft_precision(x, rate):
    """Separate f32 diagnostic oracle; original f64 expected JSON stays intact.

    Deep leakage in rejected bands can differ between f32/f64 FFTs. Use the
    independent f32 FFT only for minima below the absolute applicability floor,
    retaining the original amplitude tolerance and recording the dense difference.
    """
    if len(x) <= N:
        return dict(minimum_bin_amplitude_f32=None, maximum_mean_amplitude_difference=0.)
    windows = sliding_window_view(x.astype(np.float32), N)[:len(x)-N:HOP]*WINDOW
    mags64 = np.abs(np.fft.rfft(windows.astype(float), axis=1))
    peaks = mags64.max(axis=1)
    active = peaks > (peaks.max()+1e-12)*.001
    if not np.any(active):
        return dict(minimum_bin_amplitude_f32=None, maximum_mean_amplitude_difference=0.)
    mean64 = mags64[active].mean(axis=0)*SCALE
    mean32 = np.abs(rfft32(windows, axis=1))[active].astype(float).mean(axis=0)*SCALE
    bins = np.arange(N//2+1)
    mask = (bins*rate >= 16000*N) & (bins*rate < 20000*N) & (bins < N//2)
    minimum = float(mean32[mask].min()) if rate > 40000 else None
    return dict(minimum_bin_amplitude_f32=minimum,
                maximum_mean_amplitude_difference=float(np.max(np.abs(mean32-mean64))))


def compare(actual, expected, path=''):
    if isinstance(expected, dict):
        for key, value in expected.items():
            compare(actual[key], value, path+'/'+key)
    elif isinstance(expected, list):
        assert len(actual) == len(expected), path
        for i, value in enumerate(expected):
            compare(actual[i], value, path+f'/{i}')
    elif isinstance(expected, float):
        error = abs(actual-expected) if actual is not None else float('inf')
        kind = 'amplitude' if path.endswith('amplitude') else ('std_db' if path.endswith('std_db') else ('coefficient' if path.endswith('coefficient') else None))
        tolerance = 1e-9+1e-4*abs(expected) if kind == 'amplitude' else (.005 if kind == 'std_db' else (.0002 if kind == 'coefficient' else 1e-9))
        assert error <= tolerance, (path, actual, expected, tolerance)
        if kind:
            errors[kind] = max(errors[kind], error)
    else:
        assert actual == expected, (path, actual, expected)


def shaped(period=64, slope=0., ripple=8.):
    k = np.arange(1, N//2)
    db = ripple*np.cos(2*np.pi*k/period)+slope*k/(N//2)
    spectrum = np.zeros(N//2+1, complex)
    spectrum[1:-1] = 10**(db/20)*np.exp(1j*k*7919)
    block = np.fft.irfft(spectrum, n=N)
    block *= .7/np.max(np.abs(block))
    return np.resize(block, 64000)


cases = []


def add(name, samples, rate, prefix=None):
    cases.append((save(name, samples, rate), prefix))


for rate in [8000, 32000, 40000, 40001, 44100, 48000, 64000, 96000, 192000, 202271, 202272, 384000]:
    x = shaped()
    add(f'ripple_{rate}', np.c_[x, -x*.5], rate)
x = shaped()
add('ripple_prefix', np.c_[x, -x], 64000, 10241)
add('ripple_short', np.c_[x, -x], 64000, 8193)
add('ripple_exact_window', np.c_[x, -x], 64000, 4096)
for period in [32, 93]:
    x = shaped(period=period)
    add(f'period_{period}', np.c_[x, -x*.5], 64000)
for slope in [-24., 24.]:
    x = shaped(slope=slope, ripple=0.)
    add(f'eq_{slope:+g}', np.c_[x, -x*.5], 64000)
rate = 48000
rng = np.random.default_rng(20261015)
x = rng.uniform(-.15, .15, 3*rate)
add('white_gain', np.c_[x, -x*.5], rate)
add('quiet_and_silent', np.c_[x*1e-6, x*0], rate)
low = signal.sosfilt(signal.butter(8, 8000, fs=rate, output='sos'), x)
add('lowpass_white', np.c_[low, x], rate)
freq = np.fft.rfftfreq(len(x), 1/rate)
notched = np.fft.irfft(np.fft.rfft(x)*((freq < 17500) | (freq > 18500)), n=len(x))
add('notch_white', np.c_[notched, x], rate)
t = np.arange(len(x))/rate
add('tones', np.c_[.5*np.sin(2*np.pi*1000*t), .5*np.sin(2*np.pi*18000*t)], rate)
gap = np.concatenate([x[:rate], np.zeros(rate), x[:rate]])
add('inactive_gap', np.c_[gap, -gap], rate)
impulses = np.zeros(rate)
impulses[1024::4096] = .5
add('flat_impulses', np.c_[impulses, -impulses], rate)
add('silence', np.zeros((rate, 2)), rate)
records = []
for path, prefix in cases:
    if args.case_pattern not in path.stem:
        continue
    with wave.open(str(path), 'rb') as w:
        rate, channels = w.getframerate(), w.getnchannels()
    raw = subprocess.check_output(['ffmpeg', '-nostdin', '-v', 'error', '-i', str(path), '-c:a', 'pcm_s32le', '-f', 's32le', 'pipe:1'])
    if prefix is not None:
        raw = raw[:prefix*channels*4]
    samples = np.frombuffer(raw, '<i4').reshape(-1, channels).astype(float)/2**31
    expected = [oracle(samples[:, ch], rate) for ch in range(channels)]
    immutable(OUT/f'{path.stem}.expected.json', expected)
    precision = [fft_precision(samples[:, ch], rate) for ch in range(channels)]
    immutable(OUT/f'{path.stem}.fft_precision.expected.json', precision)
    command = [str(ROOT/'target/release/audio-forensic.exe'), '--json']
    if prefix is not None:
        command += ['--max-seconds', str(prefix/rate)]
    output = subprocess.check_output(command+[str(path)])
    (OUT/f'{path.stem}.actual.json').write_bytes(output)
    r = json.loads(output)[0]
    assert r['status'] == 'analyzed' and r['schema_version'] == args.schema_version
    assert r['coverage']['decoded_pcm_sha256'] == hashlib.sha256(raw).hexdigest()
    assert r['coverage']['analyzed_frames'] == len(samples)
    assert r['ancestry_verdict'] == 'INCONCLUSIVE' and r['evidence_index'] is None
    for ch, e in enumerate(expected):
        comparison = e.copy()
        if e['minimum_bin_amplitude'] is not None and e['minimum_bin_amplitude'] < 1e-6:
            comparison['minimum_bin_amplitude'] = precision[ch]['minimum_bin_amplitude_f32']
        compare(r['spectral_lags'][ch], comparison, f'{path.stem}/ch{ch}')
    records.append(dict(case=path.stem, pcm_exact=True, statuses=[e['status'] for e in expected],
                        coefficients=[[p['target']['coefficient'] for p in e['lags']] for e in expected]))
    print(path.stem, records[-1]['statuses'], 'passed', flush=True)
assert records, 'No cases selected'
summary = OUT/('summary'+('-'+args.case_pattern if args.case_pattern else '')+'.json')
summary.write_text(json.dumps(dict(numpy=np.__version__, cases=records, maximum_errors=errors), indent=2)+'\n')
print('maximum errors', errors, flush=True)
