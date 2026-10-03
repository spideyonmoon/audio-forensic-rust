"""Validate a core report object or CLI report array entirely offline.

Optional development tool; install report_schema_requirements.txt separately.
JSON Schema checks shape, required nulls, integer widths and report policy.
Small additional checks enforce interval/channel relationships that JSON Schema
does not compare across fields. Input files are read only; no reference fetching.
"""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SCHEMA = ROOT / 'schemas/analysis-report-0.18.0.schema.json'


def strict_json(data):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError(f'Duplicate JSON key: {key}')
            result[key] = value
        return result
    def constant(value):
        raise ValueError(f'Non-finite JSON value: {value}')
    def floating(value):
        parsed = float(value)
        if not math.isfinite(parsed):
            raise ValueError('JSON number is outside the finite f64 range')
        return parsed
    return json.loads(data, object_pairs_hook=pairs, parse_constant=constant, parse_float=floating)


def offline_validator(schema):
    from jsonschema import Draft202012Validator
    from referencing import Registry
    def check_refs(value):
        if isinstance(value, dict):
            for keyword in ('$ref', '$dynamicRef'):
                if keyword in value and not value[keyword].startswith('#/'):
                    raise ValueError('Only local schema references are supported; remote fetching is disabled')
            for child in value.values():
                check_refs(child)
        elif isinstance(value, list):
            for child in value:
                check_refs(child)
    check_refs(schema)
    Draft202012Validator.check_schema(schema)
    def no_retrieval(uri):
        raise ValueError(f'External schema retrieval is disabled: {uri}')
    return Draft202012Validator(schema, registry=Registry(retrieve=no_retrieval))


def validate_report(report, validator):
    validator.validate(report)
    if report['status'] != 'analyzed':
        if not report['diagnostics']:
            raise ValueError('Unsuccessful report lacks structured diagnostics')
        return
    stream, coverage = report['stream'], report['coverage']
    frames, channels = coverage['analyzed_frames'], stream['channels']
    if [c['channel_index'] for c in report['channels']] != list(range(channels)):
        raise ValueError('Native channels are missing, duplicated or reordered')
    expected_encoding = 's32le_msb_aligned' if stream['integer_pcm'] else 'f64le'
    if coverage['hash_sample_encoding'] != expected_encoding:
        raise ValueError('PCM hash encoding disagrees with stream precision')
    if any(c['samples'] != frames for c in report['channels']):
        raise ValueError('Native sample counts disagree with analyzed frames')
    def walk(value):
        if isinstance(value, dict):
            if set(value) == {'start_frame', 'end_frame'}:
                if not 0 <= value['start_frame'] <= value['end_frame'] <= frames:
                    raise ValueError('Analysis interval is reversed or outside the analyzed prefix')
            if 'channel_index' in value and value['channel_index'] is not None:
                if value['channel_index'] >= channels:
                    raise ValueError('Measurement references an unavailable native channel')
            if 'channel_indices' in value and any(i >= channels for i in value['channel_indices']):
                raise ValueError('Joint measurement references an unavailable native channel')
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)
    walk(report)


def validate_document(document, validator):
    reports = document if isinstance(document, list) else [document]
    if not reports:
        raise ValueError('No reports to validate')
    for report in reports:
        validate_report(report, validator)
    return reports


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('inputs', nargs='+', type=Path)
    parser.add_argument('--schema', type=Path, default=DEFAULT_SCHEMA)
    parser.add_argument('--output', type=Path, help='New JSON receipt beneath corpus/local')
    args = parser.parse_args()
    if args.output:
        output = args.output.resolve()
        if not output.is_relative_to((ROOT / 'corpus/local').resolve()) or output.exists():
            raise ValueError('Receipt must be a new file beneath corpus/local')
        output.parent.mkdir(parents=True, exist_ok=True)
    schema_bytes = args.schema.read_bytes()
    validator = offline_validator(strict_json(schema_bytes))
    records, statuses = [], Counter()
    for path in args.inputs:
        data = path.read_bytes()
        try:
            reports = validate_document(strict_json(data), validator)
            for report in reports:
                statuses[report['status']] += 1
            if path.read_bytes() != data:
                raise ValueError('Report changed during validation')
            records.append({'file': str(path), 'sha256': hashlib.sha256(data).hexdigest(),
                            'reports': len(reports), 'passed': True})
        except Exception as error:
            # Diagnostics only; never execute or infer instructions from report text.
            records.append({'file': str(path), 'sha256': hashlib.sha256(data).hexdigest(),
                            'passed': False, 'error': str(error)[:600],
                            'path': list(getattr(error, 'absolute_path', []))})
    if args.schema.read_bytes() != schema_bytes:
        raise ValueError('Schema changed during validation')
    result = {'passed': all(r['passed'] for r in records), 'files': len(records),
              'reports': sum(r.get('reports', 0) for r in records), 'statuses': dict(statuses),
              'schema_sha256': hashlib.sha256(schema_bytes).hexdigest(), 'records': records,
              'scope': 'Serialized shape, observations-only policy and interval/channel consistency; no DSP or accuracy evaluation'}
    if args.output:
        with args.output.open('x', encoding='utf-8') as handle:
            json.dump(result, handle, indent=2, allow_nan=False)
            handle.write('\n')
    print(json.dumps({key: value for key, value in result.items() if key != 'records'}))
    for record in records:
        if not record['passed']:
            print(json.dumps(record))
    return 0 if result['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
