"""P08 frozen-tree receipts and finite acceptance; generated inputs only.

Use --freeze before building, then --binary/--rust-log after regression.
Existing numerical expectations are read, never regenerated.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[1]
GROUPS = {
    'G01': ['metadata', 'alac'], 'G02': ['metadata'],
    'G03': ['core', 'byproducts', 'tool_statistics'], 'G04': ['tool_statistics'],
    'G05': ['loudness', 'listening_levels', 'byproducts'], 'G06': ['tool_statistics'],
    'G07': ['byproducts'], 'G08': ['parity', 'reference_inputs'],
    'G09': ['reference_inputs', 'lib'], 'G10': ['flac_integrity', 'wav_formats', 'lib'],
    'G11': ['detectors', 'reference_inputs', 'lib'],
    'G12': ['transforms', 'reference_inputs', 'lib'], 'G13': ['structure', 'reference_inputs'],
    'G14': ['aac', 'reference_inputs'], 'G15': ['transforms', 'reference_inputs'],
    'G16': ['noise', 'reference_inputs', 'lib'],
    'G17': ['transients', 'preceding_energy', 'reference_inputs', 'lib'],
    'G18': ['rolloff', 'reference_inputs', 'lib'],
    'G19': ['sparsity', 'envelope', 'reference_inputs', 'lib'],
    'G20': ['spectral_lags', 'reference_inputs', 'lib'],
    'G21': ['core', 'noise_floor', 'reference_inputs', 'lib'],
    'G22': ['reference_assessment', 'lib'],
    'G23': ['detectors', 'metadata', 'reference_assessment', 'lib'],
    'G24': ['reference_assessment', 'lib'], 'G25': ['spectrogram', 'spectrogram_png'],
    'G26': ['product', 'product_cli', 'report_contract'], 'G27': ['product', 'product_cli'],
    'G28': ['product_cli', 'cli_contract', 'jobs', 'progress', 'worker_budget'],
    'G29': ['product_cli', 'source_contract', 'progress'],
    'G30': ['alac', 'wav_formats', 'flac_integrity'],
}


def snapshot():
    files = [ROOT / name for name in ('Cargo.toml', 'Cargo.lock')]
    for folder in ('src', 'tests', 'schemas', 'scripts', 'assets'):
        files += [p for p in (ROOT / folder).rglob('*')
                  if p.is_file() and '__pycache__' not in p.parts]
    return {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(files)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--freeze', action='store_true')
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--rust-log', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    frozen = args.output / 'frozen-tree.json'
    if args.freeze:
        with frozen.open('x', encoding='utf-8') as out:
            json.dump(snapshot(), out, indent=2)
        print(f'PASS: frozen {len(snapshot())} source/fixture/schema/helper files')
        return
    assert args.binary and args.rust_log
    assert snapshot() == json.loads(frozen.read_text()), 'Tree changed after freeze'
    log = args.rust_log.read_text(encoding='utf-8-sig').replace('\\', '/')
    totals = [tuple(map(int, x)) for x in re.findall(
        r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', log)]
    assert totals and all(f == 0 and i == 0 for _, f, i in totals)
    assert 'test result: FAILED' not in log
    for suites in GROUPS.values():
        for suite in suites:
            needle = 'unittests src/lib.rs' if suite == 'lib' else f'tests/{suite}.rs'
            assert needle in log, f'Missing regression executable: {suite}'
    from validate_report_schema import strict_json, offline_validator, validate_report
    native = offline_validator(strict_json((ROOT / 'schemas/analysis-report-0.18.0.schema.json').read_bytes()))
    product = offline_validator(strict_json((ROOT / 'schemas/product-1.schema.json').read_bytes()))
    # Container-signature controls only: no native DSD decoding or conversion oracle.
    dsf = b'DSD ' + struct.pack('<QQQ', 28, 28, 0)
    dff = b'FRM8' + struct.pack('>Q', 4) + b'DSD '
    outcomes = []
    for name, data in [('dsf', dsf), ('dff', dff)]:
        for extension in (name, 'wav'):
            path = args.output / f'generated-{name}.{extension}'
            path.write_bytes(data)
            for mode in ('native', 'product', 'fast'):
                flags = ['--json'] if mode == 'native' else ['--product-json']
                if mode == 'fast':
                    flags.append('--fast')
                result = subprocess.run([str(args.binary.resolve()), *flags, str(path.resolve())],
                                        capture_output=True, timeout=60)
                assert result.returncode == 1
                value = strict_json(result.stdout)
                assert len(value) == 1
                if mode == 'native':
                    report = value[0]
                else:
                    product.validate(value)
                    report = value[0]['measurement_report']
                    assessment = value[0]['reference_assessment']
                    assert assessment['status'] == 'unsupported'
                    assert assessment['scores'] is None and assessment['reference_label'] is None
                    assert value[0]['reference_inputs'] is None
                validate_report(report, native)
                assert report['status'] == 'unsupported' and report['coverage'] is None
                assert report['ancestry_verdict'] == 'INCONCLUSIVE' and report['evidence_index'] is None
                assert any('signature required' in d['message'] for d in report['diagnostics'])
                stem = f'{name}-{extension}-{mode}'
                (args.output / f'{stem}.json').write_bytes(result.stdout)
                (args.output / f'{stem}.stderr').write_bytes(result.stderr)
                outcomes.append({'case': stem, 'status': report['status'], 'exit': result.returncode})
    assert snapshot() == json.loads(frozen.read_text())
    summary = {'engine': '0.32.0', 'rust_totals': {
        'passed': sum(p for p, _, _ in totals), 'failed': 0, 'ignored': 0,
        'executables_and_doctests': len(totals)}, 'groups': GROUPS,
        'rule_ids': [f'R{i:02}' for i in range(1, 35)],
        'dsd': {'disposition': 'F02 deferred beyond initial release/Alfred launch',
                'signature_controls': outcomes, 'conversion_oracles': 'not run'},
        'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        'frozen_files': len(snapshot()), 'frozen_tree_unchanged': True}
    (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(f"PASS: {summary['rust_totals']['passed']} Rust tests, 30 mapped groups, 12 structured DSD rejections, unchanged frozen tree")


if __name__ == '__main__':
    main()
