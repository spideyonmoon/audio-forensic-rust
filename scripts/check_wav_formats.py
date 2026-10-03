"""Generated WAV support matrix with independent sample packing and FFmpeg PCM.

Writes immutable local receipts. Malformed and deliberately unsupported cases
must retain structured outcomes in full and prefix analysis. No private music.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess

from validate_report_schema import DEFAULT_SCHEMA, ROOT, offline_validator, strict_json, validate_document


def digest(data):
    return hashlib.sha256(data).hexdigest()


def chunk(tag, data):
    return tag + struct.pack('<I', len(data)) + data + bytes(len(data) % 2)


def wave(bits=16, channels=1, floating=False, valid=None, mask=None, frames=1025):
    """Expectations derive from values before packing, never a Rust report."""
    extensible = valid is not None
    valid = bits if valid is None else valid
    samples, expected = bytearray(), bytearray()
    for i in range(frames * channels):
        if floating:
            value = (0.0, 0.125, -0.25, 0.5, -0.875, 1.0, -1.0)[i % 7]
            samples.extend(struct.pack('<f' if bits == 32 else '<d', value))
            expected.extend(struct.pack('<d', value))
        else:
            # Include extrema, unit LSBs and independently varying channels.
            peak = 1 << (valid - 1)
            value = (-peak, peak - 1, -1, 0, 1, peak // 2, -peak // 2)[i % 7]
            stored = value << (bits - valid)
            if bits == 8:
                samples.append(stored + 128)
            else:
                samples.extend(stored.to_bytes(bits // 8, 'little', signed=True))
            expected.extend(struct.pack('<i', value << (32 - valid)))
    tag = 65534 if extensible else 3 if floating else 1
    fmt = struct.pack('<HHIIHH', tag, channels, 8000, 8000 * channels * bits // 8,
                      channels * bits // 8, bits)
    if extensible:
        fmt += struct.pack('<HHI', 22, valid, (4 if channels == 1 else 3) if mask is None else mask)
        fmt += struct.pack('<I', 3 if floating else 1) + bytes.fromhex('00001000800000aa00389b71')
    body = b'WAVE' + chunk(b'fmt ', fmt) + chunk(b'data', samples)
    return b'RIFF' + struct.pack('<I', len(body)) + body, bytes(expected)


def cases():
    result = []

    def add(name, content, pcm=None, channels=1, floating=False, status='analyzed', diagnostic=None):
        result.append(dict(name=name, content=content, pcm=pcm, channels=channels,
                           floating=floating, status=status, diagnostic=diagnostic))

    for channels in (1, 2):
        for bits in (8, 16, 24, 32):
            for extensible in (False, True):
                data, pcm = wave(bits, channels, valid=bits if extensible else None)
                add(f'int-{bits}-{channels}-ext{int(extensible)}', data, pcm, channels)
        for bits in (32, 64):
            for extensible in (False, True):
                data, pcm = wave(bits, channels, True, bits if extensible else None)
                add(f'float-{bits}-{channels}-ext{int(extensible)}', data, pcm, channels, True)
        for bits, valid in ((8, 7), (16, 12), (24, 20), (32, 24)):
            data, pcm = wave(bits, channels, valid=valid)
            add(f'valid-{valid}-in-{bits}-{channels}', data, pcm, channels)
        for mask in (0, 1 if channels == 1 else 3):
            data, pcm = wave(16, channels, valid=16, mask=mask)
            add(f'ordinary-mask-{mask}-{channels}', data, pcm, channels)
    base, pcm = wave()
    ext, ext_pcm = wave(24, valid=20)
    data_at = base.index(b'data')
    ext_data_at = ext.index(b'data')

    def patch(data, offset, fmt, value):
        out = bytearray(data)
        struct.pack_into(fmt, out, offset, value)
        return bytes(out)

    add('riff-sentinel-known-data', patch(base, 4, '<I', 0xffffffff), pcm)
    unknown = patch(base, data_at + 4, '<I', 0xffffffff)
    add('unknown-data-known-riff', unknown, pcm, status='unsupported', diagnostic='unknown-length WAV data')
    add('unknown-riff-and-data', patch(unknown, 4, '<I', 0xffffffff), pcm,
        status='unsupported', diagnostic='unknown-length WAV data')
    # RF64 size block is generated without allocating a giant file.
    ds64 = struct.pack('<QQQI', len(base) + 36 - 8, len(pcm) // 2, 1025, 0)
    rf64 = b'RF64' + b'\xff' * 4 + base[8:12] + chunk(b'ds64', ds64) + unknown[12:]
    add('rf64', rf64, pcm, status='unsupported', diagnostic='RF64')
    add('rifx', b'RIFX' + base[4:], status='unsupported', diagnostic='RIFX')
    add('bad-byte-rate', patch(base, 28, '<I', 1), status='failed', diagnostic='byte rate')
    add('zero-byte-rate', patch(base, 28, '<I', 0), status='failed', diagnostic='byte rate')
    add('zero-valid-bits', patch(ext, 38, '<H', 0), status='failed', diagnostic='valid bits')
    add('excess-valid-bits', patch(ext, 38, '<H', 25), status='failed', diagnostic='valid bits')
    add('short-ext-size', patch(ext, 36, '<H', 21), status='failed', diagnostic='extension size')
    add('long-ext-size', patch(ext, 36, '<H', 65535), status='failed', diagnostic='extension size')
    bad_pad = bytearray(ext); bad_pad[ext_data_at + 8] |= 1
    add('nonzero-unused-bits', bytes(bad_pad), status='failed', diagnostic='declared precision')
    for channels, mask in ((1, 8), (2, 12), (2, 1), (1, 3), (2, 0x80000001)):
        data, _ = wave(16, channels, valid=16, mask=mask)
        add(f'unsupported-mask-{channels}-{mask}', data, status='unsupported', diagnostic='channel mask')
    for floating, bits in ((False, 64), (True, 16)):
        data = patch(ext, 44, '<I', 3 if floating else 1)
        data = patch(patch(patch(data, 34, '<H', bits), 32, '<H', bits // 8), 28, '<I', 8000 * bits // 8)
        data = patch(data, 38, '<H', bits)
        add(f'unsupported-subtype-width-{floating}', data, status='unsupported', diagnostic='sample width')
    guid = bytearray(ext); guid[59] ^= 1
    add('unknown-guid', bytes(guid), status='unsupported', diagnostic='subtype')
    ambi = bytearray(ext); ambi[48:60] = bytes.fromhex('2107d3118644c8c1ca000000')
    add('ambisonic-guid', bytes(ambi), ext_pcm, status='unsupported', diagnostic='subtype')
    # Byte-complete subchunks, including an odd JUNK pad and trailing metadata.
    body = base[8:12] + chunk(b'JUNK', b'x') + base[12:]
    add('odd-leading-junk', b'RIFF' + struct.pack('<I', len(body)) + body, pcm)
    body = base[8:] + chunk(b'LIST', b'INFO' + chunk(b'INAM', b'generated\0'))
    add('trailing-info', b'RIFF' + struct.pack('<I', len(body)) + body, pcm)
    for name, size in (('fact-zero', 0), ('fact-wrong', 23), ('fact-correct', 1025)):
        body = base[8:data_at] + chunk(b'fact', struct.pack('<I', size)) + base[data_at:]
        add(name, b'RIFF' + struct.pack('<I', len(body)) + body, pcm)
    # Actual truncation must not become a successful short full analysis.
    add('truncated-tail-byte', base[:-1], status='failed')
    add('partial-frame', patch(base, data_at + 4, '<I', 2049), status='failed', diagnostic='partial PCM frame')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--ffmpeg', default=shutil.which('ffmpeg'))
    args = parser.parse_args()
    work = args.output.resolve()
    if not work.is_relative_to((ROOT / 'corpus/local').resolve()) or work.exists():
        raise ValueError('Use a new output directory under corpus/local')
    if not args.ffmpeg:
        raise ValueError('Independent PCM validation requires local FFmpeg')
    work.mkdir(parents=True)
    inputs = work / 'inputs'; inputs.mkdir()
    binary = work / 'validated-cli.exe'
    original_hash = digest(args.binary.read_bytes())
    shutil.copy2(args.binary, binary)
    matrix = cases()
    manifest = []
    for case in matrix:
        path = inputs / (case['name'] + '.wav')
        path.write_bytes(case['content'])
        item = {key: value for key, value in case.items() if key not in ('content', 'pcm')}
        item['sha256'] = digest(case['content'])
        if case['pcm'] is not None:
            item['expected_pcm_sha256'] = digest(case['pcm'])
            fmt = 'f64le' if case['floating'] else 's32le'
            command = [args.ffmpeg, '-nostdin', '-v', 'error', '-i', str(path), '-map', '0:a:0',
                       '-c:a', 'pcm_' + fmt, '-f', fmt, '-']
            process = subprocess.run(command, capture_output=True, timeout=30)
            (work / (case['name'] + '-ffmpeg.stderr')).write_bytes(process.stderr)
            item['ffmpeg_command'] = command
            item['ffmpeg_exit'] = process.returncode
            item['ffmpeg_pcm_sha256'] = digest(process.stdout)
            item['ffmpeg_exact'] = process.returncode == 0 and process.stdout == case['pcm']
            if case['name'].startswith('valid-24-in-32-'):
                # This host's FFmpeg auto-detects pcm_f24le for the standard
                # integer GUID with 24 valid bits in a 32-bit container. Retain
                # that disagreement; separately select the declared PCM codec.
                explicit = command[:4] + ['-c:a', 'pcm_s32le'] + command[4:]
                forced = subprocess.run(explicit, capture_output=True, timeout=30)
                item['explicit_decoder_command'] = explicit
                item['explicit_decoder_exit'] = forced.returncode
                item['explicit_decoder_pcm_sha256'] = digest(forced.stdout)
                item['explicit_decoder_exact'] = forced.returncode == 0 and forced.stdout == case['pcm']
                (work / (case['name'] + '-explicit.stderr')).write_bytes(forced.stderr)
        manifest.append(item)
    (work / 'manifest.json').write_text(json.dumps({'generated_only': True, 'binary_sha256': original_hash,
        'version': subprocess.check_output([str(binary), '--version']).decode().strip(),
        'ffmpeg_version': subprocess.check_output([args.ffmpeg, '-version']).decode().splitlines()[0],
        'cases': manifest}, indent=2) + '\n')
    validator = offline_validator(strict_json(DEFAULT_SCHEMA.read_bytes()))
    errors, statuses = [], {}
    for mode, flags in (('full', []), ('prefix', ['--max-seconds', '0.05'])):
        command = [str(binary), '--json', '--deadline-seconds', '2', *flags, str(inputs)]
        process = subprocess.run(command, capture_output=True, timeout=120)
        (work / (mode + '-reports.json')).write_bytes(process.stdout)
        (work / (mode + '-stderr.txt')).write_bytes(process.stderr)
        if process.returncode != 1:
            errors.append(f'{mode}: batch exit {process.returncode}, expected 1')
        reports = validate_document(strict_json(process.stdout), validator)
        by_name = {Path(report['source']).stem: report for report in reports}
        assert len(reports) == len(matrix) and set(by_name) == {case['name'] for case in matrix}
        statuses[mode] = dict(Counter(report['status'] for report in reports))
        for case in matrix:
            report = by_name[case['name']]
            # A valid measured prefix can precede tail damage outside its scope.
            expected = 'analyzed' if mode == 'prefix' and case['name'] == 'truncated-tail-byte' else case['status']
            problem = []
            if report['status'] != expected:
                problem.append(f"status {report['status']} != {expected}")
            elif expected != 'analyzed' and case['diagnostic'] and not any(case['diagnostic'] in d['message'] for d in report['diagnostics']):
                problem.append('missing diagnostic: ' + case['diagnostic'])
            if expected == report['status'] == 'analyzed':
                expected_pcm = wave()[1] if case['name'] == 'truncated-tail-byte' else case['pcm']
                frames = 1025 if mode == 'full' else 400
                expected_pcm = expected_pcm[:frames * case['channels'] * (8 if case['floating'] else 4)]
                if report['coverage']['decoded_pcm_sha256'] != digest(expected_pcm):
                    problem.append('PCM mismatch')
                if report['coverage']['analyzed_frames'] != frames:
                    problem.append('frame count mismatch')
                if report['stream']['channels'] != case['channels'] or report['stream']['integer_pcm'] == case['floating']:
                    problem.append('channel/precision mismatch')
            if problem:
                errors.append({'mode': mode, 'case': case['name'], 'problems': problem, 'diagnostics': report['diagnostics']})
    errors += [case['name'] + ': independent FFmpeg mismatch' for case in manifest
               if case.get('explicit_decoder_exact', case.get('ffmpeg_exact')) is False]
    assert all(digest((inputs / (case['name'] + '.wav')).read_bytes()) == item['sha256'] for case, item in zip(matrix, manifest))
    assert digest(args.binary.read_bytes()) == digest(binary.read_bytes()) == original_hash
    summary = {'passed': not errors, 'cases': len(matrix), 'reports': 2 * len(matrix), 'statuses': statuses,
               'independent_pcm_cases': sum('ffmpeg_exact' in case for case in manifest),
               'ffmpeg_autodetect_disagreements': [case['name'] for case in manifest if case.get('ffmpeg_exact') is False],
               'errors': errors, 'inputs_and_binary_unchanged': True}
    (work / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary))
    raise SystemExit(bool(errors))


if __name__ == '__main__':
    main()
