"""Generated FLAC compression controls with independent exact integer PCM.

Development only: uses local FFmpeg for encoding and an independent decode.
No recordings, network access or production dependency is involved.
"""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import shutil
import struct
import subprocess

from validate_report_schema import DEFAULT_SCHEMA, ROOT, offline_validator, strict_json, validate_document


def digest(data):
    return hashlib.sha256(data).hexdigest()


def generated(bits, channels, signal):
    words = []
    seed = 42
    scale = 1 << (bits - 1)
    for i in range(1097):
        for channel in range(channels):
            seed = (1664525 * seed + 1013904223) & 0xffffffff
            if signal == 'silence':
                word = 0
            elif signal == 'ramp':
                word = ((i * (channel + 1) * 7) % 127 - 63) * (scale // 128)
            elif signal == 'noise':
                word = (seed >> (32 - bits)) - scale
            else:
                word = round((math.sin(i * .047 * (channel + 1)) + .2 * math.sin(i * .11)) * scale * .6)
            words.append(word)
    pcm = b''.join(struct.pack('<i', word << (32 - bits)) for word in words)
    # FFmpeg's encoder accepts s16/s32; promote generated 8-bit PCM exactly.
    raw_bits = 16 if bits <= 16 else 32
    raw = b''.join(struct.pack('<h' if raw_bits == 16 else '<i', word << (raw_bits - bits)) for word in words)
    return raw, pcm, raw_bits


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--ffmpeg', default=shutil.which('ffmpeg'))
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to((ROOT / 'corpus/local').resolve()) or output.exists() or not args.ffmpeg:
        raise ValueError('Use local FFmpeg and a new output directory under corpus/local')
    output.mkdir(parents=True)
    inputs = output / 'inputs'; inputs.mkdir()
    binary = output / 'validated-cli.exe'
    shutil.copyfile(args.binary, binary)
    shutil.copyfile(__file__, output / Path(__file__).name)
    cases, errors = {}, []
    for bits in (8, 16, 24):
        for channels in (1, 2):
            for rate in (8000, 48000):
                for level in (0, 8):
                    for signal in ('silence', 'ramp', 'noise', 'tones'):
                        name = f'b{bits}-c{channels}-r{rate}-l{level}-{signal}'
                        raw, pcm, raw_bits = generated(bits, channels, signal)
                        path = inputs / (name + '.flac')
                        command = [args.ffmpeg, '-nostdin', '-v', 'error', '-xerror', '-f', f's{raw_bits}le',
                                   '-ar', str(rate), '-ac', str(channels), '-i', '-', '-c:a', 'flac',
                                   '-compression_level', str(level), str(path)]
                        encoded = subprocess.run(command, input=raw, capture_output=True, timeout=30)
                        assert encoded.returncode == 0, encoded.stderr
                        decode = [args.ffmpeg, '-nostdin', '-v', 'error', '-xerror', '-err_detect', 'crccheck+explode',
                                  '-i', str(path), '-c:a', 'pcm_s32le', '-f', 's32le', '-']
                        decoded = subprocess.run(decode, capture_output=True, timeout=30)
                        assert decoded.returncode == 0 and decoded.stdout == pcm, name
                        (output / (name + '-ffmpeg.stderr')).write_bytes(encoded.stderr + decoded.stderr)
                        data = path.read_bytes()
                        stream_bits = ((int.from_bytes(data[18:26], 'big') >> 36) & 31) + 1
                        prefix_frames = min(1097, round(rate * .05))
                        cases[name] = {'input_sha256': digest(data), 'source_bits': bits, 'stream_bits': stream_bits,
                            'channels': channels, 'rate': rate, 'compression_level': level, 'signal': signal,
                            'full_pcm_sha256': digest(pcm), 'prefix_pcm_sha256': digest(pcm[:prefix_frames * channels * 4]),
                            'prefix_frames': prefix_frames, 'encode_command': command, 'decode_command': decode,
                            'independent_exact': True}
    validator = offline_validator(strict_json(DEFAULT_SCHEMA.read_bytes()))
    statuses = {}
    for mode, flags in [('full', []), ('prefix', ['--max-seconds', '.05'])]:
        with (output / (mode + '-reports.json')).open('xb') as out, (output / (mode + '-stderr.txt')).open('xb') as err:
            process = subprocess.run([str(binary), '--json', '--deadline-seconds', '5', *flags, str(inputs)], stdout=out, stderr=err, timeout=180)
        reports = validate_document(strict_json((output / (mode + '-reports.json')).read_bytes()), validator)
        assert len(reports) == len(cases)
        statuses[mode] = dict(Counter(r['status'] for r in reports))
        for report in reports:
            name = Path(report['source']).stem
            case = cases[name]
            if report['status'] != 'analyzed':
                errors.append({'mode': mode, 'case': name, 'diagnostics': report['diagnostics']})
                continue
            assert report['coverage']['decoded_pcm_sha256'] == case[mode + '_pcm_sha256'], name
            assert report['coverage']['analyzed_frames'] == (1097 if mode == 'full' else case['prefix_frames']), name
            assert report['stream']['bits_per_sample'] == case['stream_bits'], name
            assert report['stream']['channels'] == case['channels'], name
        assert process.returncode == bool(errors), process.returncode
    assert all(digest((inputs / (name + '.flac')).read_bytes()) == case['input_sha256'] for name, case in cases.items())
    assert digest(binary.read_bytes()) == digest(args.binary.read_bytes())
    manifest = {'generated_only': True, 'cases': cases, 'binary_sha256': digest(binary.read_bytes()),
                'version': subprocess.check_output([str(binary), '--version']).decode().strip(),
                'ffmpeg_version': subprocess.check_output([args.ffmpeg, '-version']).decode().splitlines()[0]}
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    summary = {'passed': not errors, 'cases': len(cases), 'reports': len(cases) * 2,
               'independent_exact_pcm': len(cases), 'statuses': statuses, 'errors': errors}
    (output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary))
    raise SystemExit(bool(errors))


if __name__ == '__main__':
    main()
