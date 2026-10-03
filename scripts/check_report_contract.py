"""Positive and negative contract controls against generated Rust report output.

Run tests/report_contract.rs with REPORT_CONTRACT_OUTPUT first. This script never
changes the schema, producer, reports or reference outputs to make a check pass.
"""
import argparse
from collections import Counter
from copy import deepcopy
import hashlib
import json
from pathlib import Path

from jsonschema.exceptions import ValidationError

from generate_report_schema import build_schema
from validate_report_schema import (
    DEFAULT_SCHEMA, ROOT, offline_validator, strict_json, validate_document,
    validate_report,
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reports', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to((ROOT / 'corpus/local').resolve()) or output.exists():
        raise ValueError('Receipt must be a new file beneath corpus/local')
    data = args.reports.read_bytes()
    schema_bytes = DEFAULT_SCHEMA.read_bytes()
    source_bytes = (ROOT / 'src/model.rs').read_bytes()
    schema = strict_json(schema_bytes)
    validator = offline_validator(schema)
    reports = validate_document(strict_json(data), validator)
    assert len(reports) == 8
    assert Counter(r['status'] for r in reports) == {
        'analyzed': 4, 'failed': 1, 'unsupported': 1, 'cancelled': 1, 'timed_out': 1,
    }
    assert reports[0]['stream']['channels'] == 2
    assert reports[1]['stream']['integer_pcm'] is False
    assert reports[1]['coverage']['requested_max_seconds'] == 0.125
    assert reports[2]['channels'][0]['crest_factor_linear'] is None
    assert any(a['status'] == 'hit' for a in reports[3]['aac'])
    controls = []

    def reject(name, action):
        try:
            action()
        except (ValueError, ValidationError) as error:
            controls.append({'name': name, 'passed': True, 'expected': 'rejected',
                             'error_type': type(error).__name__})
        else:
            raise AssertionError(f'Invalid control accepted: {name}')

    def mutate(name, path, value=None, remove=False, base=0):
        report = deepcopy(reports[base])
        target = report
        for key in path[:-1]:
            target = target[key]
        if remove:
            del target[path[-1]]
        else:
            target[path[-1]] = value
        reject(name, lambda: validate_report(report, validator))

    mutate('missing required null', ['evidence_index'], remove=True)
    mutate('unknown root field', ['confidence'], 99)
    mutate('unknown nested field', ['channels', 0, 'clipping_probability'], 0.5)
    mutate('unknown file status', ['status'], 'success')
    mutate('unknown detector status', ['detectors', 0, 'status'], 'clear')
    mutate('numeric evidence index', ['evidence_index'], 0)
    mutate('confident ancestry', ['ancestry_verdict'], 'NO_INDICATORS')
    mutate('stale schema version', ['schema_version'], '0.17.0')
    mutate('stale policy version', ['policy_version'], 'observations-only-v17')
    mutate('empty engine version', ['engine_version'], '')
    mutate('negative unsigned', ['coverage', 'analyzed_frames'], -1)
    mutate('boolean integer', ['coverage', 'analyzed_frames'], True)
    mutate('fractional integer', ['coverage', 'analyzed_frames'], 1.5)
    mutate('u64 overflow', ['stream', 'declared_frames'], 2 ** 64)
    mutate('u32 overflow', ['stream', 'track_id'], 2 ** 32)
    mutate('u8 overflow', ['coverage', 'analysis_passes'], 256)
    mutate('wrong nullable type', ['channels', 0, 'effective_bits'], 'unknown')
    mutate('missing nested nullable', ['channels', 0, 'effective_bits'], remove=True)
    mutate('invalid PCM hash', ['coverage', 'decoded_pcm_sha256'], '0' * 63)
    mutate('wrong fixed array length', ['stereo_correlation', 'mean'], [None])
    mutate('invalid dynamic measurement', ['detectors', 0, 'measurements', 'bad'], 'zero')
    mutate('boolean measurement', ['detectors', 0, 'measurements', 'bad'], True)
    mutate('missing successful coverage', ['coverage'], None)
    mutate('wrong successful pass count', ['coverage', 'analysis_passes'], 1)
    mutate('unsupported successful rate', ['stream', 'sample_rate'], 7999)
    mutate('missing native channel', ['channels'], reports[0]['channels'][:1])
    mutate('duplicated native channel', ['channels', 1, 'channel_index'], 0)
    mutate('sample count mismatch', ['channels', 0, 'samples'], 0)
    mutate('PCM hash precision mismatch', ['coverage', 'hash_sample_encoding'], 'f64le')
    mutate('float hash precision mismatch', ['coverage', 'hash_sample_encoding'], 's32le_msb_aligned', base=1)
    mutate('unavailable detector channel', ['detectors', 0, 'channel_index'], 2)
    mutate('unavailable joint channel', ['stereo_correlation', 'channel_indices'], [0, 2])
    mutate('reversed interval', ['detectors', 0, 'intervals'], [{'start_frame': 2, 'end_frame': 1}])
    mutate('interval outside prefix', ['detectors', 0, 'intervals'], [
        {'start_frame': 0, 'end_frame': reports[0]['coverage']['analyzed_frames'] + 1}])
    mutate('failed without diagnostics', ['diagnostics'], [], base=4)
    reject('empty report batch', lambda: validate_document([], validator))
    reject('non-object report', lambda: validate_document([None], validator))
    for name, text in [
        ('duplicate root key', '{"status":"analyzed","status":"failed"}'),
        ('duplicate nested key', '{"stream":{"channels":1,"channels":2}}'),
        ('NaN JSON', '[NaN]'), ('Infinity JSON', '[Infinity]'),
        ('negative Infinity JSON', '[-Infinity]'), ('f64 overflow JSON', '[1e999]'),
        ('truncated JSON', '{'),
    ]:
        reject(name, lambda text=text: strict_json(text))
    for keyword in ('$ref', '$dynamicRef'):
        reject(f'external {keyword}', lambda keyword=keyword: offline_validator({
            '$schema': schema['$schema'], keyword: 'https://example.invalid/report.json',
        }))
    source = source_bytes.decode('utf-8')
    for name, changed in [
        ('serde field rename', source.replace('pub source: String,', '#[serde(rename = "file")]\n    pub source: String,')),
        ('serde container change', source.replace('pub struct AnalysisReport {', '#[serde(deny_unknown_fields)]\npub struct AnalysisReport {')),
        ('unsupported field type', source.replace('pub source: String,', 'pub source: i128,')),
        ('custom serialization', source + '\nimpl Serialize for Custom {}'),
        ('ambiguous acronym enum', source.replace('    Analyzed,', '    PCM,')),
    ]:
        assert changed != source, f'Control did not mutate source: {name}'
        reject(name, lambda changed=changed: build_schema(changed))
    assert build_schema(source) == schema, 'Artifact drift; do not regenerate to hide it'

    # Positive extremes preserve u64 values above the JavaScript safe-integer
    # range and permit partial metadata outside the analyzed support matrix.
    boundary = deepcopy(reports[5])
    boundary['stream'] = deepcopy(reports[0]['stream'])
    boundary['stream'].update(track_id=2 ** 32 - 1, declared_frames=2 ** 64 - 1,
                              channels=2 ** 64 - 1, sample_rate=2 ** 32 - 1)
    boundary = strict_json(json.dumps(boundary, allow_nan=False))
    validate_report(boundary, validator)
    assert boundary['stream']['declared_frames'] == 2 ** 64 - 1
    controls.append({'name': 'exact u64 and partial unsupported metadata',
                     'passed': True, 'expected': 'accepted'})
    finite = deepcopy(reports[0])
    finite['detectors'][0]['measurements']['positive_boundary'] = 1.7976931348623157e308
    finite['detectors'][0]['measurements']['negative_boundary'] = -1.7976931348623157e308
    validate_report(strict_json(json.dumps(finite, allow_nan=False)), validator)
    controls.append({'name': 'finite f64 endpoints', 'passed': True, 'expected': 'accepted'})
    assert args.reports.read_bytes() == data
    assert DEFAULT_SCHEMA.read_bytes() == schema_bytes
    assert (ROOT / 'src/model.rs').read_bytes() == source_bytes
    result = {'passed': True, 'reports': len(reports), 'controls': len(controls),
              'rejected': sum(c['expected'] == 'rejected' for c in controls),
              'accepted_boundaries': sum(c['expected'] == 'accepted' for c in controls),
              'input_sha256': hashlib.sha256(data).hexdigest(),
              'schema_sha256': hashlib.sha256(schema_bytes).hexdigest(),
              'unchanged_inputs': True, 'records': controls}
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('x', encoding='utf-8') as handle:
        json.dump(result, handle, indent=2, allow_nan=False)
        handle.write('\n')
    print(json.dumps({k: v for k, v in result.items() if k != 'records'}))


if __name__ == '__main__':
    main()
