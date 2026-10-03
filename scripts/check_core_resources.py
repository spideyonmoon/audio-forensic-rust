"""Generated duration/rate resource controls; no private audio or accuracy claims.

Uses one second of PCM at a time. Existing controls must match the generator;
reports use a separate directory so earlier measurements remain preserved.
"""
import argparse
from array import array
import math
from pathlib import Path
import subprocess
import sys
import wave

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--work', type=Path, default=ROOT/'corpus/local/generated/core-resources-v18')
parser.add_argument('--output', type=Path, default=ROOT/'corpus/local/results/core-resources-v18')
parser.add_argument('--generate-only', action='store_true')
args = parser.parse_args()
args.work.mkdir(parents=True, exist_ok=True)
cases = [(48000, seconds) for seconds in (10, 120, 600)]
cases += [(rate, 10) for rate in (8000, 44100, 96000, 192000, 384000)]
cases += [(384000, 120)]
for rate, seconds in cases:
    samples = array('h', (value for i in range(rate) for value in (
        int(16000*math.sin(2*math.pi*997*i/rate)),
        int(12000*math.sin(2*math.pi*1379*i/rate)))))
    if sys.byteorder != 'little':
        samples.byteswap()
    block = samples.tobytes()
    path = args.work/f'sine-{rate:06d}hz-{seconds:04d}s.wav'
    if path.exists():
        with wave.open(str(path), 'rb') as handle:
            assert handle.getparams()[:4] == (2, 2, rate, rate*seconds)
            for _ in range(seconds):
                assert handle.readframes(rate) == block
    else:
        with wave.open(str(path), 'wb') as handle:
            handle.setparams((2, 2, rate, 0, 'NONE', 'not compressed'))
            for _ in range(seconds):
                handle.writeframesraw(block)
    print(f'Generated/verified {path.name}', flush=True)
if not args.generate_only:
    subprocess.run([sys.executable, str(ROOT/'scripts/validate_local.py'), str(args.work),
                    '--output', str(args.output), '--source-history',
                    'generated integer sine controls; scripts/check_core_resources.py'], check=True)
