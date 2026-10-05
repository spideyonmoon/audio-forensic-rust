"""Check the saved-report evaluation runner with prior generated-only receipts."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--reports', type=Path, required=True,
                        help='Generated library-consumer receipts: wav/flac/cancel/missing/unsupported.json')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if output.exists() or not output.is_relative_to((ROOT / 'corpus/local').resolve()):
        raise ValueError('Output must be a new directory under corpus/local')
    output.mkdir(parents=True)
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    binary = args.binary.resolve()
    binary_hash = digest(binary)
    oracle_path = ROOT / 'tests/fixtures/reference.json'
    oracle_hash = digest(oracle_path)
    oracle = json.loads(oracle_path.read_text())
    pcm = next(c['pcm_sha256'] for c in oracle['cases'] if c['file'] == 'noise16.wav')
    cases, originals, versions = [], {}, set()
    for name in ['wav', 'flac', 'cancel', 'missing', 'unsupported']:
        path = args.reports / f'{name}.json'
        originals[path] = digest(path)
        report = json.loads(path.read_bytes())
        versions.add(report['engine_version'])
        generated = name in ['wav', 'flac', 'cancel']
        if name in ['wav', 'flac']:
            assert report['coverage']['decoded_pcm_sha256'] == pcm
            target = [d for d in report['detectors'] if d['id'].startswith('aac_quantization_lattice_')]
            assert len(target) == 2 and all(d['status'] == 'not_detected' for d in target)
        cases.append(dict(id=name, source_group='noise_source' if generated else name,
                          split='challenge', codec='aac', label='absent' if generated else 'unknown',
                          provenance='generated_control' if generated else 'unknown_history',
                          processing='native', evidence_ref=str(oracle_path), evidence_sha256=oracle_hash,
                          expected_pcm_sha256=pcm if generated else None))
    assert len(versions) == 1
    reserved = dict(cases[0], id='unseen', source_group='reserved', split='locked_test',
                    expected_pcm_sha256='f' * 64)
    manifest = dict(manifest_version=1, report_engine_version=versions.pop(), cases=cases + [reserved])
    manifest_path, plan_path = output / 'manifest.json', output / 'plan.json'
    manifest_path.write_text(json.dumps(manifest, indent=2) + '\n')
    checks = []

    def run(name, arguments, success):
        process = subprocess.run([str(binary), *map(str, arguments)], capture_output=True, timeout=120)
        (output / f'{name}.stdout').write_bytes(process.stdout)
        (output / f'{name}.stderr').write_bytes(process.stderr)
        assert (process.returncode == 0) == success, (name, process.returncode, process.stderr)
        checks.append(dict(check=name, exit=process.returncode, expected_success=success))

    run('freeze', ['freeze', manifest_path, plan_path], True)
    frozen_hash = digest(plan_path)
    result = output / 'evaluation.json'
    run('selected', ['run', plan_path, 'challenge', args.reports, result], True)
    summary = json.loads(result.read_text())
    assert len(summary['cases']) == 5
    assert summary['unknown_cases_excluded'] == 2
    assert len(summary['labeled_strata']) == 1
    stratum = summary['labeled_strata'][0]
    assert len(stratum['groups']) == 1
    assert stratum['file_counts'] == dict(hit=0, no_hit=2, abstain=1)
    assert stratum['group_mean_no_hit_fraction'] == 2 / 3
    assert stratum['group_mean_abstain_fraction'] == 1 / 3
    assert {c['file_status'] for c in summary['cases']} == {'analyzed', 'cancelled', 'failed', 'unsupported'}
    result_hash = digest(result)
    run('no-overwrite-plan', ['freeze', manifest_path, plan_path], False)
    run('no-overwrite-summary', ['run', plan_path, 'challenge', args.reports, result], False)
    run('missing-locked', ['run', plan_path, 'locked_test', args.reports, output / 'missing-output.json'], False)
    assert not (output / 'missing-output.json').exists()
    original_plan = json.loads(plan_path.read_text())
    for field in ['manifest', 'executable']:
        changed = json.loads(json.dumps(original_plan))
        if field == 'manifest':
            changed['evaluation']['manifest']['cases'][0]['processing'] = 'changed'
        else:
            changed['runner_sha256'] = '0' * 64
        changed_path = output / f'changed-{field}.json'
        changed_path.write_text(json.dumps(changed))
        destination = output / f'invalid-{field}.json'
        run(f'reject-{field}', ['run', changed_path, 'challenge', args.reports, destination], False)
        assert not destination.exists()
    assert digest(plan_path) == frozen_hash and digest(result) == result_hash
    assert digest(binary) == binary_hash and digest(oracle_path) == oracle_hash
    assert all(digest(path) == expected for path, expected in originals.items())
    receipt = dict(passed=True, checks=checks, runner_sha256=binary_hash, frozen_plan_sha256=frozen_hash,
                   original_report_sha256={str(p): h for p, h in originals.items()},
                   note='Five saved generated-control reports; no new audio decode or accuracy claim.')
    (output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt))


if __name__ == '__main__':
    main()
