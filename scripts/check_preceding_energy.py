"""Independent SciPy full-array preceding-energy controls, generated audio only.

Immutable expectations before Rust, exact FFmpeg PCM hashes. Power tolerance
1e-12 + 1e-8*expected; exact counts, geometry, applicability and nulls.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import wave

import numpy as np
from scipy import signal

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=ROOT/'corpus/local/results/preceding-energy-v17')
parser.add_argument('--schema-version', default='0.17.0')
args = parser.parse_args()
AUDIO = ROOT/'corpus/local/generated/preceding-energy-v17'
OUT = args.output
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)


def immutable(path, value):
    text = json.dumps(value, indent=2, allow_nan=False)+'\n'
    if path.exists():
        assert path.read_text() == text, ('changed oracle', path)
    else:
        path.write_text(text)


def save(name, x, rate):
    if x.ndim == 1:
        x = x[:, None]
    words = np.rint(x*2**23).astype(np.int32)
    assert np.max(np.abs(words)) < 2**23
    raw = ((words[..., None] >> (8*np.arange(3))) & 255).astype(np.uint8).tobytes()
    path = AUDIO/f'{name}.wav'
    if path.exists():
        with wave.open(str(path), 'rb') as w:
            assert w.getframerate() == rate and w.getnchannels() == x.shape[1]
            assert w.readframes(w.getnframes()) == raw
    else:
        with wave.open(str(path), 'wb') as w:
            w.setparams((x.shape[1], 3, rate, 0, 'NONE', 'not compressed'))
            w.writeframes(raw)
    return path


def bounds(values, power=False):
    if not len(values):
        return None, None, None
    middle = np.sort(values)[[(len(values)-1)//2, len(values)//2]]
    pairs = []
    divisor = 10 if power else 20
    tiny = 1e-20 if power else 1e-9
    for v in middle:
        if v == 0:
            pairs.append((0., 0.))
        elif v <= tiny:
            pairs.append((0., tiny))
        else:
            db = np.floor(divisor*np.log10(v)*4)/4
            pairs.append((10**(db/divisor), 10**((db+.25)/divisor)))
    return float(middle.mean()), *np.mean(pairs, axis=0).tolist()


def peaks(x, rate):
    width, warmup = (rate//2000)|1, (rate+49)//50
    half, distance = width//2, max(1, rate//100)
    y = signal.sosfilt(signal.butter(4, 1000, btype='highpass', fs=rate, output='sos'), x)
    envelope = np.convolve(np.abs(y), np.ones(width)/width, mode='valid')*np.pi/2 if len(x) >= width else np.empty(0)
    envelope = envelope[max(0, warmup-half):]
    _, _, upper = bounds(envelope)
    start, end = warmup+1, len(x)-half-1
    available = end-start >= (rate+1)//2
    threshold = max(1e-4, 3*upper) if available else None
    accepted = []
    if available:
        local, _ = signal.find_peaks(envelope)
        for point in local:
            frame = int(point+warmup)
            if envelope[point] > threshold and (not accepted or frame-accepted[-1] >= distance):
                accepted.append(frame)
    return available, accepted


def oracle(samples, rate):
    x = samples[:180*rate]
    warmup = (rate+49)//50
    end_offset = (rate+99)//100
    selection, accepted = peaks(x, rate)
    supported = rate > 40000
    power = np.empty(0)
    if supported:
        sections = np.vstack([signal.butter(4, 10000, btype='highpass', fs=rate, output='sos'),
                              signal.butter(4, 20000, btype='lowpass', fs=rate, output='sos')])
        filtered = signal.sosfilt(sections, x)
        power = filtered*filtered
    exact, lower, upper = bounds(power[warmup:], True)
    threshold = max(1e-12, 3*upper) if upper is not None and selection else None
    contexts = []
    eligible = startup = above = 0
    for p in accepted:
        context = dict(frame=p, status='inconclusive', interval=None, mean_square=None,
                       above_baseline=None, unavailable_reason=None)
        if not supported:
            context.update(status='unsupported', unavailable_reason='band_unsupported')
        elif p < warmup*2:
            context['unavailable_reason'] = 'startup_or_short_context'
            startup += 1
        else:
            lo, hi = p-warmup, p-end_offset
            mean = float(np.mean(power[lo:hi]))
            flag = mean > threshold if threshold is not None else None
            context.update(status='measured', interval=dict(start_frame=lo, end_frame=hi),
                           mean_square=mean, above_baseline=flag)
            eligible += 1
            above += int(flag) if flag is not None else 0
        contexts.append(context)
    measured = supported and eligible > 0 and threshold is not None
    return dict(channel_index=0, status='unsupported' if not supported else ('measured' if measured else 'inconclusive'),
                interval=dict(start_frame=0, end_frame=len(x)), baseline_interval=dict(start_frame=warmup, end_frame=len(x)) if len(power)>warmup else None,
                baseline_samples=max(0, len(power)-warmup), baseline_median_lower_power=lower,
                baseline_median_upper_power=upper, threshold_power=threshold, requested_lower_hz=10000., requested_upper_hz=20000.,
                filter_order=8, warmup_frames=warmup, lookback_start_frames=warmup, lookback_end_frames=end_offset,
                history_frames=(rate+3)//4 if supported else 0,
                selected_peak_count=len(accepted) if selection else None, eligible_event_count=eligible,
                startup_ineligible_count=startup, history_expired_count=0, above_baseline_count=above if threshold is not None else None,
                above_baseline_fraction=above/eligible if measured else None, events_truncated=len(accepted)>128,
                events=contexts[:128], oracle_exact_median_power=exact)


maximum_power_error = 0.


def compare(a, e, path=''):
    global maximum_power_error
    if isinstance(e, dict):
        for key, value in e.items():
            if not key.startswith('oracle_'):
                compare(a[key], value, path+'/'+key)
    elif isinstance(e, list):
        assert len(a) == len(e), path
        for i, value in enumerate(e):
            compare(a[i], value, path+f'/{i}')
    elif isinstance(e, float):
        if path.endswith(('baseline_median_lower_power', 'baseline_median_upper_power')) and e <= 1e-20:
            assert a is not None and 0 <= a <= 1e-20, (path, a, e)
        else:
            assert a is not None and abs(a-e) <= 1e-12+1e-8*abs(e), (path, a, e)
            maximum_power_error = max(maximum_power_error, abs(a-e))
    else:
        assert a == e, (path, a, e)


cases = []
for rate in [8000, 40000, 40001, 44100, 48000, 96000, 384000]:
    x = np.zeros(2*rate)
    for sec in [.1, .4, 1.2, 1.7]:
        x[round(sec*rate)] = .5
    cases.append((save(f'impulses_{rate}', np.c_[x, -x*.5], rate), None))
rate = 48000
n = 2*rate
t = np.arange(n)/rate
rng = np.random.default_rng(20261017)
x = rng.uniform(-.08, .08, n)
for name, samples in [('stationary_noise', np.c_[x, -x]), ('silence', np.zeros((n,2))),
                      ('tiny_noise', np.c_[x*1e-6, -x*1e-6]), ('steps_dc', np.c_[.4*((t>.3)&(t<1.3)), np.full(n,.4)])]:
    cases.append((save(name, samples, rate), None))
x = np.zeros(n)
for onset in [.25, 1., 1.5]:
    u = np.arange(rate//5)/rate
    x[round(onset*rate):round(onset*rate)+len(u)] = .5*(1-np.exp(-u/.001))*np.exp(-u/.025)*np.sin(2*np.pi*12000*u)
cases.append((save('smooth_attacks_gain', np.c_[x, -x*.5], rate), None))
x = np.zeros(n); x[1200] = .5; x[12000] = .5; x[48000] = .5
cases.append((save('startup_prefix', np.c_[x,-x], rate), 38400))
cases.append((save('short', np.zeros((100,2)), rate), None))
x = np.zeros(5*rate); x[4800+np.arange(200)*960] = .5
cases.append((save('dense_train', np.c_[x,-x*.5], rate), None))
x = np.zeros(n)
for onset in [.5, 1.5]:
    start, end = round((onset-.025)*rate), round((onset-.005)*rate)
    x[start:end] = .15*np.sin(2*np.pi*15000*np.arange(end-start)/rate)
    x[round(onset*rate)] = .5
cases.append((save('prior_tone_and_impulses', np.c_[x,-x*.5], rate), None))
records = []
for path, prefix in cases:
    with wave.open(str(path), 'rb') as w:
        rate, channels = w.getframerate(), w.getnchannels()
    raw = subprocess.check_output(['ffmpeg','-nostdin','-v','error','-i',str(path),'-c:a','pcm_s32le','-f','s32le','pipe:1'])
    if prefix is not None:
        raw = raw[:prefix*channels*4]
    samples = np.frombuffer(raw,'<i4').reshape(-1,channels).astype(float)/2**31
    expected = [oracle(samples[:,ch],rate) for ch in range(channels)]
    for ch,e in enumerate(expected):
        e['channel_index'] = ch
    immutable(OUT/f'{path.stem}.expected.json', expected)
    command = [str(ROOT/'target/release/audio-forensic.exe'),'--json']
    if prefix is not None:
        command += ['--max-seconds', str(prefix/rate)]
    output = subprocess.check_output(command+[str(path)])
    (OUT/f'{path.stem}.actual.json').write_bytes(output)
    r = json.loads(output)[0]
    assert r['status']=='analyzed' and r['schema_version']==args.schema_version
    assert r['coverage']['decoded_pcm_sha256']==hashlib.sha256(raw).hexdigest()
    assert r['ancestry_verdict']=='INCONCLUSIVE' and r['evidence_index'] is None
    for ch,e in enumerate(expected):
        a = r['preceding_energy'][ch]
        compare(a,e,path.stem+f'/ch{ch}')
        median = e['oracle_exact_median_power']
        if median is not None:
            assert a['baseline_median_lower_power']-1e-12 <= median <= a['baseline_median_upper_power']+1e-12
        assert a['selected_peak_count']==r['transients'][ch]['peak_count']
    records.append(dict(case=path.stem,pcm_exact=True,statuses=[e['status'] for e in expected],
                        fractions=[e['above_baseline_fraction'] for e in expected],selected=[e['selected_peak_count'] for e in expected]))
    print(path.stem,records[-1]['statuses'],records[-1]['fractions'],'passed',flush=True)
(OUT/'summary.json').write_text(json.dumps(dict(numpy=np.__version__,cases=records,maximum_power_error=maximum_power_error),indent=2)+'\n')
print('maximum power error',maximum_power_error,flush=True)
