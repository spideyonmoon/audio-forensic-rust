"""Freeze independent direct-DFT controls (no Rust FFT or private audio)."""
import argparse
import json
import math
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]


def f32(x):
    return struct.unpack('<f', struct.pack('<f', x))[0]


def oracle():
    n = 1024
    w = [f32(.5 - .5 * math.cos(math.tau * i / (n - 1))) for i in range(n)]
    energy = sum(x*x for x in w)
    cases = []
    for name in ['tone64', 'dc', 'impulse', 'nyquist']:
        samples = [f32(.5 * math.sin(math.tau * 64*i/n)) if name == 'tone64'
                   else .25 if name == 'dc'
                   else (.75 if i == 512 else 0.) if name == 'impulse'
                   else (.25 if i % 2 == 0 else -.25) for i in range(n)]
        # Multiplication also rounds to f32 in the production input window.
        x = [f32(s*t) for s,t in zip(samples, w)]
        power = []
        for k in range(n//2+1):
            re = sum(v * math.cos(math.tau*k*i/n) for i,v in enumerate(x))
            im = -sum(v * math.sin(math.tau*k*i/n) for i,v in enumerate(x))
            power.append((re*re+im*im) * (1 if k in (0,n//2) else 2) / (n*energy))
        cases.append({'name':name, 'power_per_bin':power, 'total_power':sum(power)})
    return {'method':'independent-f64-direct-dft-of-f32-windowed-input-v1',
            'fft_size':n, 'window_energy':energy, 'absolute_power_tolerance':2e-7,
            'cases':cases}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = json.dumps(oracle(), indent=2, allow_nan=False)+'\n'
    path = ROOT/'tests/fixtures/spectrogram_reference.json'
    if args.check:
        assert path.read_text() == data, 'frozen spectrogram oracle drift'
    else:
        with path.open('x', encoding='utf-8') as out:
            out.write(data)
    print('PASS: four independent 513-bin direct-DFT vectors')
