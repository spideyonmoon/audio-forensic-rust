"""Independent stored-window SciPy oracle and FFmpeg checks; generated audio only.

Tolerances fixed before execution: 1e-6 LU for dense oracle; 0.11 LU for
FFmpeg's rounded/histogram integrated result; 0.002 LU for its M/S metadata.
Counts, applicability and coverage must agree exactly. Existing oracles are immutable.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import wave
import numpy as np
import scipy
from scipy.signal import lfilter

ROOT = Path(__file__).resolve().parents[1]
AUDIO = ROOT / 'corpus/local/generated/loudness-v13'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=ROOT/'corpus/local/results/loudness-v13')
OUT = parser.parse_args().output
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)


def immutable(path, value):
    text = json.dumps(value, indent=2, allow_nan=False) + '\n'
    if path.exists():
        assert path.read_text() == text, ('oracle changed', path)
    else:
        path.write_text(text)


def coefficients(rate):
    if rate == 48000:
        # Published BS.1770 coefficients, independent of Rust's parameterization.
        return ([1.53512485958697, -2.69169618940638, 1.19839281085285],
                [1., -1.69065929318241, .73248077421585],
                [1., -2., 1.], [1., -1.99004745483398, .99007225036621])
    # Same frequency-response design as libebur128; vector filtering and dense
    # window means independently check the streaming aggregation and both gates.
    k = np.tan(np.pi * 1681.974450955533 / rate)
    q = .7071752369554196
    vh = 10 ** (3.999843853973347 / 20)
    vb = vh ** .4996667741545416
    d = 1 + k / q + k*k
    b = [(vh + vb*k/q + k*k)/d, 2*(k*k-vh)/d, (vh-vb*k/q+k*k)/d]
    a = [1., 2*(k*k-1)/d, (1-k/q+k*k)/d]
    k = np.tan(np.pi * 38.13547087602444 / rate)
    q = .5003270373238773
    d = 1 + k/q + k*k
    return b, a, [1., -2., 1.], [1., 2*(k*k-1)/d, (1-k/q+k*k)/d]


def level(power):
    return float(-.691 + 10*np.log10(power)) if power > 0 else None


def oracle(x, rate):
    hop = rate // 10
    supported = rate % 10 == 0
    chunks = len(x) // hop if supported else 0
    n = max(0, chunks - 3)
    end = chunks*hop if n else 0
    momentary = short = integrated = gate = None
    absolute_count = gated_count = 0
    if supported:
        b, a, hb, ha = coefficients(rate)
        filtered = lfilter(hb, ha, lfilter(b, a, x, axis=0), axis=0)
        power = np.sum(filtered**2, axis=1)
        # Dense direct slice means, without Rust's 100 ms chunk ring or online means.
        m = np.array([power[i:i+4*hop].mean() for i in range(0, len(x)-4*hop+1, hop)])
        s = np.array([power[i:i+30*hop].mean() for i in range(0, len(x)-30*hop+1, hop)])
        absolute = m[m > 10**((-70+.691)/10)]
        absolute_count = len(absolute)
        if len(absolute):
            threshold = absolute.mean()*.1
            gate = level(threshold)
            gated = absolute[absolute > threshold]
            gated_count = len(gated)
            integrated = level(gated.mean()) if len(gated) else None
        momentary = level(m.max()) if len(m) else None
        short = level(s.max()) if len(s) else None
    return dict(status='unsupported' if not supported else 'measured' if integrated is not None else 'inconclusive',
                channel_weights=[1.]*x.shape[1], analyzed_frames=len(x),
                interval=dict(start_frame=0, end_frame=end) if end else None,
                hop_frames=hop if supported else None, block_frames=4*hop if supported else None,
                trailing_frames=len(x)-end, complete_blocks=n,
                absolute_gated_blocks=absolute_count, relative_gated_blocks=gated_count,
                relative_gate_lufs=gate, integrated_lufs=integrated,
                momentary_max_lufs=momentary, short_term_max_lufs=short,
                short_term_windows=max(0, chunks-29))


def save(name, x, rate):
    if x.ndim == 1:
        x = x[:, None]
    words = np.rint(x*2**23).astype(np.int32)
    assert np.max(np.abs(words)) < 2**23
    raw = ((words[..., None] >> (8*np.arange(3))) & 255).astype(np.uint8).tobytes()
    path = AUDIO/(name+'.wav')
    if path.exists():
        with wave.open(str(path), 'rb') as w:
            assert w.getframerate() == rate and w.getnchannels() == x.shape[1]
            assert w.readframes(w.getnframes()) == raw
    else:
        with wave.open(str(path), 'wb') as w:
            w.setparams((x.shape[1], 3, rate, 0, 'NONE', 'not compressed'))
            w.writeframes(raw)
    return path, words.astype(np.float64)/2**23, rate


def tone(rate, seconds, gain=.1, freq=997):
    return gain*np.sin(2*np.pi*freq*np.arange(round(rate*seconds))/rate)


cases = []
for rate in [8000, 11020, 22050, 44100, 48000, 96000, 192000, 384000, 11025, 44101]:
    x = tone(rate, .7) + tone(rate, .7, .02, 79)
    cases.append((*save('rate_'+str(rate), np.c_[x, -.5*x], rate), None))
rate = 48000
for name, x in [('mono', tone(rate, 4)), ('stereo', np.tile(tone(rate,4)[:,None], (1,2))),
                ('antiphase', np.c_[tone(rate,4), -tone(rate,4)]),
                ('silence', np.zeros((rate*4, 2))), ('below_gate', tone(rate, 4, 1e-5)),
                ('above_gate', tone(rate, 4, .0006)),
                ('relative_gate', np.concatenate([tone(rate,4),tone(rate,4,.001)])),
                ('loud_tail', np.concatenate([tone(rate,3,.01),tone(rate,1,.8)])),
                ('dc', np.full(rate*4, .1))]:
    cases.append((*save(name, x, rate), None))
base = save('prefix', tone(8000, 4), 8000)
for frames in [1, 3199, 3200, 3201, 23999, 24000]:
    cases.append((*base, (frames+.1)/8000))
base = save('loud_tail', np.concatenate([tone(rate,3,.01),tone(rate,1,.8)]), rate)
cases.append((*base, 3.))
rng = np.random.default_rng(20261002)
x = rng.uniform(-.08,.08, (rate*6,2))
x[rate:rate*2] *= .0001
x[rate*4:] *= .02
cases.append((*save('noise_steps',x,rate), None))

results = []
max_error = 0.
for path, full, rate, limit in cases:
    name = path.stem + (f'_prefix_{limit:.9f}' if limit is not None else '')
    x = full[:int(np.floor(limit*rate))] if limit is not None else full
    expected = oracle(x, rate)
    immutable(OUT/(name+'.expected.json'), expected)
    command = [str(ROOT/'target/release/audio-forensic.exe'), '--json']
    if limit is not None:
        command += ['--max-seconds', str(limit)]
    report = json.loads(subprocess.check_output(command+[str(path)]))[0]
    (OUT/(name+'.actual.json')).write_text(json.dumps(report, indent=2))
    assert report['status'] == 'analyzed', report['diagnostics']
    assert report['ancestry_verdict'] == 'INCONCLUSIVE' and report['evidence_index'] is None
    for key, value in expected.items():
        actual = report['loudness'][key]
        if isinstance(value, float):
            error = abs(actual-value)
            max_error = max(max_error, error)
            assert error < 1e-6, (name, key, actual, value)
        else:
            assert actual == value, (name, key, actual, value)
    raw = subprocess.check_output(['ffmpeg','-nostdin','-v','error','-i',str(path),
                                  '-c:a','pcm_s32le','-f','s32le','pipe:1'])
    raw = raw[:len(x)*x.shape[1]*4]
    assert hashlib.sha256(raw).hexdigest() == report['coverage']['decoded_pcm_sha256']
    ff = None
    # FFmpeg itself is a distinct implementation. Compare native-rate processing;
    # omit unavailable readings and pre-full-window zero-padded displays.
    if rate % 10 == 0 and len(x) >= rate*.4:
        graph = f'atrim=end_sample={len(x)},ebur128=metadata=1,ametadata=print'
        process = subprocess.run(['ffmpeg','-nostdin','-v','info','-i',str(path),'-af',graph,
                                  '-f','null','-'], capture_output=True, text=True, check=True)
        (OUT/(name+'.ffmpeg.log')).write_text(process.stderr)
        values = {k: [float(v) for v in re.findall(r'lavfi\.r128\.'+k+r'=([-\d.]+)',process.stderr)]
                  for k in ['I','M','S']}
        ff = {}
        for key, k, skip, tolerance in [('integrated_lufs','I',0,.11),
                ('momentary_max_lufs','M',3,.002), ('short_term_max_lufs','S',29,.002)]:
            if expected[key] is not None:
                observed = values[k][-1] if k == 'I' else max(values[k][skip:])
                ff[key] = observed
                assert abs(observed-expected[key]) <= tolerance, (name,key,observed,expected[key])
    results.append(dict(case=name,ffmpeg=ff,pcm_exact=True))
    print(name, 'PASS', flush=True)
(OUT/'summary.json').write_text(json.dumps(dict(cases=results, maximum_oracle_error_lu=max_error,
    numpy=np.__version__, scipy=scipy.__version__), indent=2))
print(f'{len(results)} cases passed; maximum dense-oracle error {max_error:.3e} LU')
