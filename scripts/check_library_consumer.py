"""Exercise the CLI-free public-API example using generated/public inputs only."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from validate_report_schema import offline_validator, strict_json, validate_report, DEFAULT_SCHEMA

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to((ROOT / 'corpus/local').resolve()) or output.exists():
        raise ValueError('Output must be a new directory beneath corpus/local')
    output.mkdir(parents=True)
    binary = args.binary.resolve()
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
    validator = offline_validator(strict_json(DEFAULT_SCHEMA.read_bytes()))
    fixtures = ROOT / 'tests/fixtures'
    oracle = json.loads((fixtures / 'reference.json').read_text())
    hashes = {c['file']: c['pcm_sha256'] for c in oracle['cases']}
    unsupported = output / 'unsupported.bin'
    unsupported.write_bytes(b'generated non-audio control')
    cases = [
        ('wav', fixtures / 'noise16.wav', [], 'analyzed'),
        ('flac', fixtures / 'noise16.flac', [], 'analyzed'),
        ('missing', output / 'absent.wav', [], 'failed'),
        ('unsupported', unsupported, [], 'unsupported'),
        ('cancel', fixtures / 'noise16.flac', ['0'], 'cancelled'),
    ]
    records = []
    for name, path, extra, expected in cases:
        before = hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None
        process = subprocess.run([str(binary), str(path), *extra], capture_output=True, timeout=120)
        (output / f'{name}.json').write_bytes(process.stdout)
        (output / f'{name}.stderr').write_bytes(process.stderr)
        report = strict_json(process.stdout)
        validate_report(report, validator)
        assert report['status'] == expected, (name, report['status'], report['diagnostics'])
        assert process.returncode == (0 if expected == 'analyzed' else 1), name
        if expected == 'analyzed':
            assert report['coverage']['decoded_pcm_sha256'] == hashes[path.name], name
        if before is not None:
            assert hashlib.sha256(path.read_bytes()).hexdigest() == before, name
        records.append({'case': name, 'status': report['status'], 'exit': process.returncode,
                        'input_sha256': before, 'schema_valid': True,
                        'exact_pcm': expected == 'analyzed'})
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == binary_hash
    summary = {'passed': True, 'binary_sha256': binary_hash, 'cases': records,
               'note': 'Cancellation smoke uses immediate cancellation on a nontrivial generated fixture; scheduling can affect timing.'}
    (output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary))


if __name__ == '__main__':
    main()
