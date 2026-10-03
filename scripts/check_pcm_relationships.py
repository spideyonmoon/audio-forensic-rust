"""Independent dense PCM RMS/crest and centered stereo oracle, generated only.

Immutable expected JSON before Rust; exact integer s32le / float f64le hashes.
Tolerances: amplitude 1e-14 + 1e-11*expected (tiny amplitudes relative 1e-11),
crest relative 1e-11, crest dB 1e-9, correlation 1e-10. Exact discrete fields.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import wave

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=ROOT/'corpus/local/results/pcm-relationships-v16')
parser.add_argument('--schema-version', default='0.16.0')
args = parser.parse_args()
AUDIO = ROOT/'corpus/local/generated/pcm-relationships-v16'
OUT = args.output
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)


def immutable(path, value):
    text = json.dumps(value, indent=2, allow_nan=False)+'\n'
    if path.exists():
        assert path.read_text() == text, ('changed oracle', path)
    else:
        path.write_text(text)


def save(name, x, rate, floating=False):
    if x.ndim == 1:
        x = x[:, None]
    channels = x.shape[1]
    path = AUDIO/f'{name}.wav'
    if floating:
        raw = x.astype('<f8').tobytes()
        header = b'RIFF'+struct.pack('<I', 36+len(raw))+b'WAVEfmt '+struct.pack('<IHHIIHH', 16, 3, channels, rate, rate*channels*8, channels*8, 64)+b'data'+struct.pack('<I', len(raw))
        content = header+raw
        if path.exists():
            assert path.read_bytes() == content
        else:
            path.write_bytes(content)
    else:
        words = np.rint(x*2**23).astype(np.int32)
        assert np.max(np.abs(words)) < 2**23
        raw = ((words[..., None] >> (8*np.arange(3))) & 255).astype(np.uint8).tobytes()
        if path.exists():
            with wave.open(str(path), 'rb') as w:
                assert w.getframerate() == rate and w.getnchannels() == channels
                assert w.readframes(w.getnframes()) == raw
        else:
            with wave.open(str(path), 'wb') as w:
                w.setparams((channels, 3, rate, 0, 'NONE', 'not compressed'))
                w.writeframes(raw)
    return path, rate, channels, floating


def oracle(x):
    peak = np.max(np.abs(x), axis=0)
    normalized = np.divide(x, peak, out=np.zeros_like(x), where=peak>0)
    rms_norm = np.sqrt(np.mean(normalized*normalized, axis=0))
    channel = []
    for ch in range(x.shape[1]):
        crest = float(1/rms_norm[ch]) if peak[ch] else None
        channel.append(dict(samples=len(x), peak=float(peak[ch]), rms=float(peak[ch]*rms_norm[ch]),
                            dc_offset=float(np.mean(x[:, ch])),
                            zero_samples=int(np.sum(x[:, ch] == 0)),
                            crest_factor_status='measured' if crest is not None else 'inconclusive',
                            crest_factor_linear=crest,
                            crest_factor_db=float(20*np.log10(crest)) if crest is not None else None))
    if x.shape[1] != 2:
        stereo = dict(status='unsupported', pair_count=0, channel_indices=[], interval=None,
                      mean=[None, None], std=[None, None], variation_eligible=[False, False], coefficient=None)
    else:
        centered = normalized-normalized.mean(axis=0)
        variance = np.mean(centered*centered, axis=0)
        std_norm = np.sqrt(variance)
        std = peak*std_norm
        eligible = (std>1e-8) & (std_norm>1e-6)
        measured = len(x) >= 2 and np.all(eligible)
        rho = float(np.mean(centered[:, 0]*centered[:, 1])/np.sqrt(variance[0])/np.sqrt(variance[1])) if measured else None
        stereo = dict(status='measured' if measured else 'inconclusive', pair_count=len(x),
                      channel_indices=[0, 1], interval=dict(start_frame=0, end_frame=len(x)),
                      mean=(peak*normalized.mean(axis=0)).tolist(), std=std.tolist(),
                      variation_eligible=eligible.tolist(), coefficient=float(np.clip(rho, -1, 1)) if measured else None)
    return dict(channels=channel, stereo_correlation=stereo)


errors = dict(amplitude_absolute=0., tiny_relative=0., crest_relative=0., crest_db=0., correlation=0.)


def compare(a, e, path=''):
    if isinstance(e, dict):
        for key, value in e.items():
            compare(a[key], value, path+'/'+key)
    elif isinstance(e, list):
        assert len(a) == len(e), path
        for i, value in enumerate(e):
            compare(a[i], value, path+f'/{i}')
    elif isinstance(e, float):
        assert a is not None, path
        error = abs(a-e)
        if path.endswith('coefficient'):
            limit, kind = 1e-10, 'correlation'
        elif path.endswith('crest_factor_db'):
            limit, kind = 1e-9, 'crest_db'
        elif path.endswith('crest_factor_linear'):
            error, limit, kind = abs(a/e-1), 1e-11, 'crest_relative'
        elif 0 < abs(e) < 1e-100:
            error, limit, kind = abs(a/e-1), 1e-11, 'tiny_relative'
        else:
            limit, kind = 1e-14+1e-11*abs(e), 'amplitude_absolute'
        assert error <= limit, (path, a, e, error, limit)
        errors[kind] = max(errors[kind], error)
    else:
        assert a == e, (path, a, e)


cases = []
n = 16384
t = np.arange(n)
a = .5*np.sin(2*np.pi*t/64)
for rate in [8000, 8001, 44100, 48000, 384000]:
    cases.append((*save(f'gain_antiphase_{rate}', np.c_[a, -a*.5], rate), None))
for phase in [0., np.pi/3, np.pi/2, np.pi]:
    b = .25*np.sin(2*np.pi*t/64+phase)-.25
    cases.append((*save(f'phase_{phase:.6f}', np.c_[a+.125, b], 48000), None))
rng = np.random.default_rng(20261016)
x = rng.uniform(-.5, .5, (n, 2))
for name, samples in [('independent_noise', x), ('silence', x*0), ('silent_right', np.c_[a, a*0]),
                      ('dc_right', np.c_[a, np.full(n, .25)]), ('quiet_right', np.c_[a, a*1e-7]),
                      ('square', np.c_[.25*(-1.)**t, .5*(-1.)**t]), ('dc', np.full((n, 2), .25))]:
    cases.append((*save(name, samples, 48000), None))
impulse = np.zeros(n); impulse[0] = .5
cases.append((*save('impulse', np.c_[impulse, -impulse*.5], 48000), None))
gap = x.copy(); gap[4096:12288] = 0
cases.append((*save('inactive_gap', gap, 48000), None))
cases.append((*save('mono', a, 48000), None))
for frames in [1, 2, 4096, 8193]:
    cases.append((*save(f'prefix_{frames}', np.c_[a+.125, -a*.5-.25], 48000), frames))
for level in [1e-200, 1e-300]:
    small = level*(-1.)**t
    cases.append((*save(f'tiny_{level:.0e}', np.c_[small, -small*.5], 48000, True), None))
cases.append((*save('dc_dominated', np.c_[a, .5+a*1e-7], 48000, True), None))
records = []
for path, rate, channels, floating, prefix in cases:
    encoding = 'f64le' if floating else 's32le'
    raw = subprocess.check_output(['ffmpeg', '-nostdin', '-v', 'error', '-i', str(path), '-c:a', 'pcm_'+encoding, '-f', encoding, 'pipe:1'])
    size = 8 if floating else 4
    if prefix is not None:
        raw = raw[:prefix*channels*size]
    samples = np.frombuffer(raw, '<f8' if floating else '<i4').reshape(-1, channels).astype(float)
    if not floating:
        samples /= 2**31
    expected = oracle(samples)
    immutable(OUT/f'{path.stem}.expected.json', expected)
    command = [str(ROOT/'target/release/audio-forensic.exe'), '--json']
    if prefix is not None:
        command += ['--max-seconds', str(prefix/rate)]
    output = subprocess.check_output(command+[str(path)])
    (OUT/f'{path.stem}.actual.json').write_bytes(output)
    r = json.loads(output)[0]
    assert r['status'] == 'analyzed' and r['schema_version'] == args.schema_version
    assert r['coverage']['decoded_pcm_sha256'] == hashlib.sha256(raw).hexdigest()
    assert r['ancestry_verdict'] == 'INCONCLUSIVE' and r['evidence_index'] is None
    compare(r, expected, path.stem)
    records.append(dict(case=path.stem, pcm_exact=True, stereo_status=r['stereo_correlation']['status'],
                        correlation=r['stereo_correlation']['coefficient']))
    print(path.stem, records[-1]['stereo_status'], 'passed', flush=True)
(OUT/'summary.json').write_text(json.dumps(dict(numpy=np.__version__, cases=records, maximum_errors=errors), indent=2)+'\n')
print('maximum errors', errors, flush=True)
