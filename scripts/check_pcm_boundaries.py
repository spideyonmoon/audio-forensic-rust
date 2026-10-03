"""Generated WAV precision/amplitude boundaries with independent exact PCM.

Development-only: the Rust core does not use Python or FFmpeg. Each run retains
new private inputs, precomputed expectations, commands, reports and fingerprints.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess

from check_wav_formats import chunk
from validate_report_schema import DEFAULT_SCHEMA, ROOT, offline_validator, strict_json, validate_document


def digest(data):
    return hashlib.sha256(data).hexdigest()


def wave(bits, precision, channels, floating, payload):
    alignment = channels * bits // 8
    fmt = struct.pack('<HHIIHHHHI', 65534, channels, 8000, 8000 * alignment,
                      alignment, bits, 22, precision, 4 if channels == 1 else 3)
    fmt += struct.pack('<I', 3 if floating else 1) + bytes.fromhex('00001000800000aa00389b71')
    body = b'WAVE' + chunk(b'fmt ', fmt) + chunk(b'data', payload)
    return b'RIFF' + struct.pack('<I', len(body)) + body


def cases():
    result = []
    for channels in (1, 2):
        for bits in (8, 16, 24, 32):
            for precision in range(1, bits + 1):
                peak = 1 << (precision - 1)
                # Every value fits even at one-bit precision. Channels differ.
                values = [-peak, peak - 1, 0, -1, peak // 2, -(peak // 2)]
                words = [values[(i // channels + 3 * (i % channels)) % len(values)]
                         for i in range(1025 * channels)]
                stored = [x << (bits - precision) for x in words]
                payload = (bytes(x + 128 for x in stored) if bits == 8 else
                           b''.join(x.to_bytes(bits // 8, 'little', signed=True) for x in stored))
                pcm = b''.join(struct.pack('<i', x << (32 - precision)) for x in words)
                result.append({'name': f'integer-{channels}-{bits}-{precision}', 'channels': channels,
                    'bits': bits, 'precision': precision, 'floating': False, 'invalid_at': None,
                    'content': wave(bits, precision, channels, False, payload), 'pcm': pcm,
                    'decoder': 'pcm_u8' if bits == 8 else f'pcm_s{bits}le'})
        for bits in (32, 64):
            fmt = '<f' if bits == 32 else '<d'
            uint = '<I' if bits == 32 else '<Q'
            minimum_normal = 0x00800000 if bits == 32 else 0x0010000000000000
            limit_word = 0x41800000 if bits == 32 else 0x4030000000000000
            from_word = lambda word: struct.unpack(fmt, struct.pack(uint, word))[0]
            legal = {'signed-zero': [0.0, -0.0], 'subnormal': [from_word(1), -from_word(1)],
                     'minimum-normal': [from_word(minimum_normal), -from_word(minimum_normal)],
                     'over-unity': [1.25, -2.0], 'amplitude-limit': [16.0, -16.0]}
            for label, values in legal.items():
                words = [values[i % 2] for i in range(1025 * channels)]
                payload = b''.join(struct.pack(fmt, x) for x in words)
                pcm = b''.join(struct.pack('<d', x) for x in words)
                result.append({'name': f'float-{channels}-{bits}-{label}', 'channels': channels,
                    'bits': bits, 'precision': bits, 'floating': True, 'invalid_at': None,
                    'content': wave(bits, bits, channels, True, payload), 'pcm': pcm,
                    'decoder': f'pcm_f{bits}le'})
            for label, value in [('positive-inf', float('inf')), ('negative-inf', -float('inf')),
                                 ('nan', float('nan')), ('over-limit', from_word(limit_word + 1)),
                                 ('under-limit', -from_word(limit_word + 1))]:
                for frame in (0, 900):
                    words = [0.125] * (1025 * channels)
                    words[frame * channels + channels - 1] = value
                    payload = b''.join(struct.pack(fmt, x) for x in words)
                    pcm = b''.join(struct.pack('<d', x) for x in words)
                    result.append({'name': f'float-{channels}-{bits}-{label}-at-{frame}',
                        'channels': channels, 'bits': bits, 'precision': bits, 'floating': True,
                        'invalid_at': frame, 'content': wave(bits, bits, channels, True, payload),
                        'pcm': pcm, 'decoder': f'pcm_f{bits}le'})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--ffmpeg', default=shutil.which('ffmpeg'))
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to((ROOT / 'corpus/local').resolve()) or output.exists():
        raise ValueError('Use a new output directory under corpus/local')
    if not args.ffmpeg:
        raise ValueError('Independent PCM comparison needs local FFmpeg')
    output.mkdir(parents=True)
    shutil.copyfile(__file__, output / 'check_pcm_boundaries.py')
    inputs = output / 'inputs'
    inputs.mkdir()
    binary = output / 'validated-cli.exe'
    shutil.copyfile(args.binary, binary)
    binary_hash = digest(args.binary.read_bytes())
    matrix = cases()
    manifest, errors = [], []
    for case in matrix:
        path = inputs / (case['name'] + '.wav')
        path.write_bytes(case['content'])
        entry = {key: value for key, value in case.items() if key not in ('content', 'pcm')}
        entry.update(sha256=digest(case['content']), expected_pcm_sha256=digest(case['pcm']))
        if case['invalid_at'] is None:
            fmt = 'f64le' if case['floating'] else 's32le'
            command = [args.ffmpeg, '-nostdin', '-v', 'error', '-c:a', case['decoder'], '-i', str(path),
                       '-map', '0:a:0', '-c:a', 'pcm_' + fmt, '-f', fmt, '-']
            process = subprocess.run(command, capture_output=True, timeout=30)
            (output / (case['name'] + '-ffmpeg.stderr')).write_bytes(process.stderr)
            entry.update(ffmpeg_command=command, ffmpeg_exit=process.returncode,
                         ffmpeg_pcm_sha256=digest(process.stdout),
                         ffmpeg_exact=process.returncode == 0 and process.stdout == case['pcm'])
            if not entry['ffmpeg_exact']:
                errors.append({'case': case['name'], 'error': 'independent PCM mismatch'})
        manifest.append(entry)
    metadata = {'generated_only': True, 'binary_sha256': binary_hash,
        'generator_sha256': digest(Path(__file__).read_bytes()),
        'schema_sha256': digest(DEFAULT_SCHEMA.read_bytes()),
        'version': subprocess.check_output([str(binary), '--version']).decode().strip(),
        'ffmpeg_version': subprocess.check_output([args.ffmpeg, '-version']).decode().splitlines()[0],
        'independent_decoder_selection': 'Explicit header-declared codec; automatic detection is not tested here',
        'cases': manifest}
    (output / 'manifest.json').write_text(json.dumps(metadata, indent=2) + '\n')
    validator = offline_validator(strict_json(DEFAULT_SCHEMA.read_bytes()))
    statuses = {}
    for mode, flags, frames in [('full', [], 1025), ('prefix', ['--max-seconds', '0.05'], 400)]:
        command = [str(binary), '--json', '--deadline-seconds', '3', *flags, str(inputs)]
        with (output / (mode + '-reports.json')).open('xb') as out, (output / (mode + '-stderr.txt')).open('xb') as err:
            process = subprocess.run(command, stdout=out, stderr=err, timeout=180)
        if process.returncode != 1:
            errors.append({'mode': mode, 'error': f'exit {process.returncode}, expected 1'})
        reports = validate_document(strict_json((output / (mode + '-reports.json')).read_bytes()), validator)
        by_name = {Path(report['source']).stem: report for report in reports}
        assert len(reports) == len(matrix) and set(by_name) == {case['name'] for case in matrix}
        statuses[mode] = dict(Counter(report['status'] for report in reports))
        for case in matrix:
            report = by_name[case['name']]
            invalid = case['invalid_at'] is not None and case['invalid_at'] < frames
            expected = 'failed' if invalid else 'analyzed'
            if report['status'] != expected:
                errors.append({'mode': mode, 'case': case['name'], 'expected': expected, 'actual': report['status'],
                               'diagnostics': report['diagnostics']})
            elif invalid:
                assert report['coverage'] is None and not report['channels'] and not report['detectors']
                assert any('float PCM amplitude' in d['message'] for d in report['diagnostics'])
            else:
                length = frames * case['channels'] * (8 if case['floating'] else 4)
                if report['coverage']['decoded_pcm_sha256'] != digest(case['pcm'][:length]):
                    errors.append({'mode': mode, 'case': case['name'], 'error': 'Rust PCM mismatch'})
                assert report['coverage']['analyzed_frames'] == frames
                assert report['stream']['channels'] == case['channels']
                assert report['stream']['integer_pcm'] != case['floating']
                assert report['stream']['bits_per_sample'] == case['precision']
    assert digest(args.binary.read_bytes()) == digest(binary.read_bytes()) == binary_hash
    assert all(digest((inputs / (item['name'] + '.wav')).read_bytes()) == item['sha256'] for item in manifest)
    summary = {'passed': not errors, 'cases': len(matrix), 'reports': 2 * len(matrix),
               'independent_pcm_cases': sum('ffmpeg_exact' in item for item in manifest),
               'statuses': statuses, 'errors': errors, 'inputs_and_binary_unchanged': True}
    (output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary))
    raise SystemExit(bool(errors))


if __name__ == '__main__':
    main()
