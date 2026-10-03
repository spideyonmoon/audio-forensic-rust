"""Generated native FLAC continuity and integrity characterization.

Verbatim FLAC frames, CRCs, MD5 and exact PCM are built before Rust runs.
FFmpeg is an independent PCM comparator; ordinary Rust tests need no process.
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


def crc(data, width, polynomial):
    value = 0
    for byte in data:
        value ^= byte << (width - 8)
        for _ in range(8):
            value = (value << 1) ^ (polynomial if value & (1 << (width - 1)) else 0)
            value &= (1 << width) - 1
    return value


def generated(channels=1, unknown=False, md5='correct'):
    lengths = [256, 256, 256, 256, 73]
    words = [((i * 71 + (i % channels) * 919) % 32000) - 16000
             for i in range(sum(lengths) * channels)]
    pcm = b''.join(struct.pack('<i', x << 16) for x in words)
    raw = b''.join(struct.pack('<h', x) for x in words)
    frames, start = [], 0
    for number, length in enumerate(lengths):
        header = bytes([255, 248, 0x64, ((channels - 1) << 4) | 8, number, length - 1])
        frame = header + bytes([crc(header, 8, 7)])
        for channel in range(channels):
            frame += b'\x02' + b''.join(struct.pack('>h', words[(start + i) * channels + channel])
                                       for i in range(length))
        frame += crc(frame, 16, 0x8005).to_bytes(2, 'big')
        frames.append(frame)
        start += length
    info = struct.pack('>HH', 256, 256)
    info += min(map(len, frames)).to_bytes(3, 'big') + max(map(len, frames)).to_bytes(3, 'big')
    fields = (8000 << 44) | ((channels - 1) << 41) | (15 << 36) | (0 if unknown else len(words) // channels)
    info += fields.to_bytes(8, 'big')
    checksum = hashlib.md5(raw).digest()
    if md5 == 'absent':
        checksum = bytes(16)
    elif md5 == 'wrong':
        checksum = bytes([checksum[0] ^ 1]) + checksum[1:]
    info += checksum
    return b'fLaC\x80\x00\x00\x22' + info, frames, pcm


def cases():
    matrix = []
    for channels in (1, 2):
        for unknown in (False, True):
            for md5 in ('correct', 'absent', 'wrong'):
                prefix, frames, pcm = generated(channels, unknown, md5)
                variants = {'clean': b''.join(frames)}
                variable = []
                for index, frame in enumerate(frames):
                    head = bytes([255, 249, frame[2], frame[3]]) + chr(index * 256).encode('utf-8') + frame[5:6]
                    changed = head + bytes([crc(head, 8, 7)]) + frame[7:-2]
                    variable.append(changed + crc(changed, 16, 0x8005).to_bytes(2, 'big'))
                variable_prefix = bytearray(prefix)
                variable_prefix[8:10] = (73).to_bytes(2, 'big')
                variable_prefix[12:15] = min(map(len, variable)).to_bytes(3, 'big')
                variable_prefix[15:18] = max(map(len, variable)).to_bytes(3, 'big')
                variants['variable-blocks'] = b''.join(variable)
                for name, index in [('first', 0), ('inside-prefix', 1), ('outside-prefix', 3)]:
                    variants['drop-' + name] = b''.join(frame for i, frame in enumerate(frames) if i != index)
                    mutated = list(frames)
                    damaged = bytearray(mutated[index]); damaged[-1] ^= 1
                    mutated[index] = damaged
                    variants['bad-crc-' + name] = b''.join(mutated)
                    mutated = list(frames)
                    damaged = bytearray(mutated[index]); damaged[6] ^= 1
                    mutated[index] = damaged
                    variants['bad-header-' + name] = b''.join(mutated)
                variants.update({'leading-junk': b'generated-junk' + b''.join(frames),
                    'between-junk': frames[0] + b'generated-junk' + b''.join(frames[1:]),
                    'trailing-junk': b''.join(frames) + b'generated-junk',
                    'partial-last': b''.join(frames)[:-1],
                    'whole-last-removed': b''.join(frames[:-1]),
                    'duplicated-late-frame': b''.join(frames[:4] + frames[3:]),
                    'concatenated-stream': b''.join(frames) + prefix + b''.join(frames)})
                hidden = bytearray(frames[0]); hidden[2] = 0x65
                hidden[6] = crc(hidden[:6], 8, 7)
                hidden[-2:] = crc(hidden[:-2], 16, 0x8005).to_bytes(2, 'big')
                variants['hidden-extra-frame'] = frames[0] + hidden + b''.join(frames[1:])
                variants['zero-between'] = frames[0] + bytes(16) + b''.join(frames[1:])
                variants['zero-trailing'] = b''.join(frames) + bytes(16)
                for name, body in variants.items():
                    prefix_ok = name not in ('drop-first', 'drop-inside-prefix', 'bad-crc-first',
                        'bad-crc-inside-prefix', 'bad-header-first', 'bad-header-inside-prefix',
                        'leading-junk', 'between-junk', 'hidden-extra-frame', 'zero-between')
                    complete = name in ('clean', 'variable-blocks') or (name == 'whole-last-removed' and unknown and md5 == 'absent')
                    full_ok = complete and md5 != 'wrong'
                    full_frames = 1024 if name == 'whole-last-removed' else 1097
                    matrix.append({'name': f'{channels}-unknown{int(unknown)}-md5-{md5}-{name}',
                        'variant': name, 'channels': channels, 'unknown_total': unknown, 'md5': md5,
                        'expected_full': 'analyzed' if full_ok else 'failed',
                        'expected_prefix': 'analyzed' if prefix_ok else 'failed',
                        'full_frames': full_frames, 'content': (bytes(variable_prefix) if name == 'variable-blocks' else prefix) + body, 'pcm': pcm})
    matrix.extend(layout_cases())
    return matrix


def layout_cases():
    """Hand-packed zero PCM exercises each compressed subframe layout branch."""
    controls = []
    for channels in (1, 2):
        for kind in ('constant', 'wasted-verbatim', 'fixed-rice4', 'fixed-rice5',
                     'fixed-escape4', 'fixed-escape5', 'lpc-rice4', 'lpc-escape5'):
            prefix, _, _ = generated(channels, True, 'absent')
            frames = []
            for number, length in enumerate((256, 256, 256, 256, 73)):
                bits = ''
                for _ in range(channels):
                    if kind == 'constant':
                        bits += '00000000' + '0' * 16
                    elif kind == 'wasted-verbatim':
                        bits += '00000011' + '0' * 14 + '1' + '0' * length
                    else:
                        order = int(kind.startswith('lpc'))
                        bits += '01000000' + '0' * 16 + '0000' + '00000' + '0' if order else '00010000'
                        five = kind.endswith('5')
                        width = 5 if five else 4
                        bits += ('01' if five else '00') + '0000'
                        bits += '1' * width + '00000' if 'escape' in kind else '0' * width + '1' * (length - order)
                padding = (-len(bits)) % 8
                bits += '0' * padding
                subframes = int(bits, 2).to_bytes(len(bits) // 8, 'big')
                head = bytes([255, 248, 0x64, ((channels - 1) << 4) | 8, number, length - 1])
                frame = head + bytes([crc(head, 8, 7)]) + subframes
                frames.append(frame + crc(frame, 16, 0x8005).to_bytes(2, 'big'))
            prefix = bytearray(prefix)
            prefix[12:15] = min(map(len, frames)).to_bytes(3, 'big')
            prefix[15:18] = max(map(len, frames)).to_bytes(3, 'big')
            controls.append({'name': f'{channels}-layout-{kind}', 'variant': kind, 'channels': channels,
                'unknown_total': True, 'md5': 'absent', 'expected_full': 'analyzed', 'expected_prefix': 'analyzed',
                'full_frames': 1097, 'content': bytes(prefix) + b''.join(frames), 'pcm': bytes(1097 * channels * 4)})
            if kind == 'wasted-verbatim':
                changed = bytearray(frames[0]); changed[-3] |= 1
                changed[-2:] = crc(changed[:-2], 16, 0x8005).to_bytes(2, 'big')
                controls.append({'name': f'{channels}-nonzero-frame-padding', 'variant': 'nonzero-frame-padding',
                    'channels': channels, 'unknown_total': True, 'md5': 'absent', 'expected_full': 'failed',
                    'expected_prefix': 'failed', 'full_frames': 1097,
                    'content': bytes(prefix) + changed + b''.join(frames[1:]), 'pcm': bytes(1097 * channels * 4)})
    return controls


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--ffmpeg', default=shutil.which('ffmpeg'))
    parser.add_argument('--characterize', action='store_true', help='Retain proposed-contract mismatches without failing the process')
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to((ROOT / 'corpus/local').resolve()) or output.exists():
        raise ValueError('Use a new output directory under corpus/local')
    if not args.ffmpeg:
        raise ValueError('Independent PCM comparison requires local FFmpeg')
    output.mkdir(parents=True)
    inputs = output / 'inputs'; inputs.mkdir()
    shutil.copyfile(__file__, output / 'check_flac_integrity.py')
    binary = output / 'validated-cli.exe'
    shutil.copyfile(args.binary, binary)
    binary_hash = digest(args.binary.read_bytes())
    matrix = cases()
    manifest, independent_errors = [], []
    for case in matrix:
        path = inputs / (case['name'] + '.flac')
        path.write_bytes(case['content'])
        item = {key: value for key, value in case.items() if key not in ('content', 'pcm')}
        item.update(sha256=digest(case['content']), expected_pcm_sha256=digest(case['pcm'][:case['full_frames'] * case['channels'] * 4]))
        if case['expected_full'] == 'analyzed':
            command = [args.ffmpeg, '-nostdin', '-v', 'error', '-xerror', '-i', str(path),
                       '-map', '0:a:0', '-c:a', 'pcm_s32le', '-f', 's32le', '-']
            process = subprocess.run(command, capture_output=True, timeout=30)
            (output / (case['name'] + '-ffmpeg.stderr')).write_bytes(process.stderr)
            expected = case['pcm'][:case['full_frames'] * case['channels'] * 4]
            item.update(ffmpeg_command=command, ffmpeg_exit=process.returncode,
                        ffmpeg_pcm_sha256=digest(process.stdout),
                        ffmpeg_exact=process.returncode == 0 and process.stdout == expected)
            if not item['ffmpeg_exact']:
                independent_errors.append(case['name'])
        manifest.append(item)
    (output / 'manifest.json').write_text(json.dumps({'generated_only': True, 'binary_sha256': binary_hash,
        'version': subprocess.check_output([str(binary), '--version']).decode().strip(),
        'generator_sha256': digest(Path(__file__).read_bytes()), 'schema_sha256': digest(DEFAULT_SCHEMA.read_bytes()),
        'ffmpeg_version': subprocess.check_output([args.ffmpeg, '-version']).decode().splitlines()[0],
        'cases': manifest}, indent=2) + '\n')
    validator = offline_validator(strict_json(DEFAULT_SCHEMA.read_bytes()))
    errors, statuses = [], {}
    for mode, flags in [('full', []), ('prefix', ['--max-seconds', '0.05'])]:
        command = [str(binary), '--json', '--deadline-seconds', '3', *flags, str(inputs)]
        with (output / (mode + '-reports.json')).open('xb') as out, (output / (mode + '-stderr.txt')).open('xb') as err:
            process = subprocess.run(command, stdout=out, stderr=err, timeout=180)
        assert process.returncode == 1, process.returncode
        reports = validate_document(strict_json((output / (mode + '-reports.json')).read_bytes()), validator)
        by_name = {Path(report['source']).stem: report for report in reports}
        assert len(reports) == len(matrix) and set(by_name) == {case['name'] for case in matrix}
        statuses[mode] = dict(Counter(report['status'] for report in reports))
        for case in matrix:
            report, problems = by_name[case['name']], []
            expected = case['expected_' + mode]
            if report['status'] != expected:
                problems.append(f"status {report['status']} != {expected}")
            elif expected == 'analyzed':
                count = case['full_frames'] if mode == 'full' else 400
                coverage = report['coverage']
                if coverage['decoded_pcm_sha256'] != digest(case['pcm'][:count * case['channels'] * 4]):
                    problems.append('exact PCM mismatch')
                if coverage['analyzed_frames'] != count:
                    problems.append('frame count mismatch')
                if report['stream']['declared_frames'] != (None if case['unknown_total'] else 1097):
                    problems.append('declared total mismatch')
                if coverage['length_matches_header'] != (None if mode == 'prefix' or case['unknown_total'] else True):
                    problems.append('header length availability mismatch')
                if coverage['decoder_verification'] != (True if mode == 'full' and case['md5'] == 'correct' else None):
                    problems.append('checksum availability mismatch')
                if coverage['reached_end'] != (mode == 'full'):
                    problems.append('EOF scope mismatch')
            else:
                assert not report['channels'] and not report['detectors'] and report['coverage'] is None
            if problems:
                errors.append({'mode': mode, 'case': case['name'], 'problems': problems, 'diagnostics': report['diagnostics']})
    assert all(digest((inputs / (case['name'] + '.flac')).read_bytes()) == item['sha256'] for case, item in zip(matrix, manifest))
    assert digest(args.binary.read_bytes()) == digest(binary.read_bytes()) == binary_hash
    summary = {'passed': not errors and not independent_errors, 'cases': len(matrix), 'reports': len(matrix) * 2,
        'statuses': statuses, 'independent_pcm_cases': sum('ffmpeg_exact' in item for item in manifest),
        'independent_errors': independent_errors, 'errors': errors, 'inputs_and_binary_unchanged': True}
    (output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps({key: value for key, value in summary.items() if key != 'errors'}), f'contract_mismatches={len(errors)}')
    raise SystemExit(bool(independent_errors or (errors and not args.characterize)))


if __name__ == '__main__':
    main()
