"""Generated-only WAV/FLAC header mutation and truncation campaign.

Uses a copied CLI, bounded per-input deadlines and an isolated batch process.
Receipts and generated files must be new and private. This is a deterministic
engineering sweep, not fuzzing completeness, allocation sandboxing or accuracy.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import time

from validate_report_schema import DEFAULT_SCHEMA, ROOT, offline_validator, strict_json, validate_document


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--ffmpeg', default=shutil.which('ffmpeg'))
    args = parser.parse_args()
    work = args.output.resolve()
    if not work.is_relative_to((ROOT / 'corpus/local').resolve()) or work.exists():
        raise ValueError('Campaign must use a new directory under corpus/local')
    if not args.ffmpeg:
        raise ValueError('FFmpeg is required only to generate the private FLAC control')
    work.mkdir(parents=True)
    inputs = work / 'inputs'
    inputs.mkdir()
    binary = work / 'validated-cli.exe'
    original_binary_sha = sha(args.binary)
    shutil.copy2(args.binary, binary)
    assert sha(binary) == original_binary_sha
    version = subprocess.check_output([str(binary), '--version']).decode().strip()
    words = [(i * 71) % 32_000 - 16_000 for i in range(64)]
    pcm = b''.join(struct.pack('<h', word) for word in words)
    wav = (b'RIFF' + struct.pack('<I', 36 + len(pcm)) + b'WAVEfmt ' +
           struct.pack('<IHHIIHH', 16, 1, 1, 8000, 16000, 2, 16) +
           b'data' + struct.pack('<I', len(pcm)) + pcm)
    source = work / 'source.wav'
    source.open('xb').write(wav)
    flac_path = work / 'source.flac'
    command = [args.ffmpeg, '-nostdin', '-v', 'error', '-i', str(source),
               '-map_metadata', '-1', '-fflags', '+bitexact', '-flags:a', '+bitexact',
               '-c:a', 'flac', '-sample_fmt', 's16', str(flac_path)]
    subprocess.run(command, check=True, timeout=30, capture_output=True)
    flac = flac_path.read_bytes()
    assert flac[:4] == b'fLaC' and len(flac) > 42
    cases = []

    def save(kind, extension, content):
        name = f'{len(cases):04d}-{kind}.{extension}'
        path = inputs / name
        path.open('xb').write(content)
        cases.append({'file': name, 'kind': kind, 'sha256': sha(path), 'bytes': len(content)})

    for extension, content, header in [('wav', wav, 44), ('flac', flac, 42)]:
        save('baseline', extension, content)
        for offset in range(header):
            for bit in range(8):
                mutated = bytearray(content)
                mutated[offset] ^= 1 << bit
                save(f'header-{offset:02d}-bit-{bit}', extension, mutated)
        for length in sorted(set(range(header + 1)) | {header + 1, header + 2, len(content) // 2, len(content) - 2, len(content) - 1}):
            save(f'truncate-{length:05d}', extension, content[:length])
    manifest = {'version': version, 'binary_sha256': original_binary_sha,
                'ffmpeg_version': subprocess.check_output([args.ffmpeg, '-version']).decode().splitlines()[0],
                'flac_command': command, 'generated_only': True, 'cases': cases}
    with (work / 'manifest.json').open('x', encoding='utf-8') as handle:
        json.dump(manifest, handle, indent=2); handle.write('\n')
    start = time.monotonic()
    try:
        with (work / 'reports.json').open('xb') as out, (work / 'stderr.txt').open('xb') as err:
            process = subprocess.run([str(binary), '--json', '--deadline-seconds', '1', str(inputs)],
                                     stdout=out, stderr=err, timeout=120)
        assert process.returncode in (0, 1), f'Unexpected process exit: {process.returncode}'
        assert (work / 'reports.json').stat().st_size < 80 * 1024 * 1024
        validator = offline_validator(strict_json(DEFAULT_SCHEMA.read_bytes()))
        reports = validate_document(strict_json((work / 'reports.json').read_bytes()), validator)
        assert len(reports) == len(cases)
        expected_pcm = hashlib.sha256(b''.join(struct.pack('<i', word << 16) for word in words)).hexdigest()
        by_name = {Path(report['source']).name: report for report in reports}
        assert set(by_name) == {case['file'] for case in cases}
        for case in cases:
            assert sha(inputs / case['file']) == case['sha256'], 'Generated mutation changed'
            report = by_name[case['file']]
            if case['kind'] == 'baseline':
                assert report['status'] == 'analyzed', report['diagnostics']
                assert report['coverage']['decoded_pcm_sha256'] == expected_pcm
                assert report['coverage']['analyzed_frames'] == 64
            if report['status'] != 'analyzed':
                assert report['channels'] == [] and report['coverage'] is None and report['detectors'] == []
        assert sha(binary) == original_binary_sha and sha(args.binary) == original_binary_sha
        assert source.read_bytes() == wav and flac_path.read_bytes() == flac
        summary = {'passed': True, 'cases': len(cases), 'statuses': dict(Counter(r['status'] for r in reports)),
                   'exit_code': process.returncode, 'baseline_exact_pcm': True,
                   'schema_valid': True, 'generated_inputs_unchanged': True,
                   'binary_sha256': original_binary_sha, 'elapsed_seconds': time.monotonic() - start,
                   'scope': 'Deterministic header/truncation sweep; no complete fuzzing or accuracy claim'}
    except Exception as error:
        summary = {'passed': False, 'cases': len(cases), 'error_type': type(error).__name__, 'error': str(error),
                   'elapsed_seconds': time.monotonic() - start}
        with (work / 'summary.json').open('x', encoding='utf-8') as handle:
            json.dump(summary, handle, indent=2); handle.write('\n')
        raise
    with (work / 'summary.json').open('x', encoding='utf-8') as handle:
        json.dump(summary, handle, indent=2); handle.write('\n')
    print(json.dumps(summary))


if __name__ == '__main__':
    main()
