"""Export the current report's JSON Schema without adding Rust dependencies.

This deliberately accepts only the plain serde model declarations used here.
New attributes, variants or field types fail instead of guessing wire behavior.
--check compares the artifact; it never rewrites it to make a check pass.
"""
import argparse
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
DECLARATION = re.compile(
    r'#\[derive\(([^)]*)\)\]\s*((?:#\[[^\]]*\]\s*)*)'
    r'pub (struct|enum) (\w+) \{([^{}]*)\}', re.MULTILINE)


def type_schema(name, names):
    if name == 'String':
        return {'type': 'string'}
    if name == 'bool':
        return {'type': 'boolean'}
    if name == 'f64':
        return {'type': 'number', 'minimum': -1.7976931348623157e308,
                'maximum': 1.7976931348623157e308}
    widths = {'u8': 8, 'u32': 32, 'u64': 64, 'usize': 64}
    if name in widths:
        return {'type': 'integer', 'minimum': 0, 'maximum': 2 ** widths[name] - 1}
    match = re.fullmatch(r'(Option|Vec)<(.+)>', name)
    if match:
        inner = type_schema(match[2], names)
        return {'anyOf': [inner, {'type': 'null'}]} if match[1] == 'Option' else {'type': 'array', 'items': inner}
    match = re.fullmatch(r'\[(.+); (\d+)\]', name)
    if match:
        return {'type': 'array', 'items': type_schema(match[1], names),
                'minItems': int(match[2]), 'maxItems': int(match[2])}
    match = re.fullmatch(r'BTreeMap<String, (.+)>', name)
    if match:
        return {'type': 'object', 'additionalProperties': type_schema(match[1], names)}
    if name in names:
        return {'$ref': f'#/$defs/{name}'}
    raise ValueError(f'Unsupported serialized type: {name}')


def build_schema(source):
    # Any unmatched serde declaration or serialization attribute is a hard error.
    if re.search(r'\bimpl\b[^{}]*\bSerialize\b', source):
        raise ValueError('Custom serialization requires an explicit exporter review')
    declarations = [m for m in DECLARATION.finditer(source)
                    if 'Serialize' in [d.strip() for d in m[1].split(',')]]
    all_serialized = re.findall(r'#\[derive\([^)]*\bSerialize\b[^)]*\)\]', source)
    if len(declarations) != len(all_serialized):
        raise ValueError('Serialized declaration outside the supported plain-model syntax')
    for attribute in re.finditer(r'#\[\s*serde\b[^\]]*\]', source):
        if not any(m.start(2) <= attribute.start() < m.end(2) for m in declarations):
            raise ValueError('Serde attribute outside the supported container syntax')
    names = {m[4] for m in declarations}
    if len(names) != len(declarations) or 'AnalysisReport' not in names:
        raise ValueError('Duplicate or missing report model')
    definitions = {}
    for match in declarations:
        attributes, kind, name, body = match[2], match[3], match[4], match[5]
        if attributes.strip() != ('#[serde(rename_all = "snake_case")]' if kind == 'enum' else ''):
            raise ValueError(f'Unsupported serde/container attribute on {name}')
        docs, fields, variants = [], {}, []
        for raw in body.splitlines():
            line = raw.strip()
            if not line:
                continue
            if line.startswith('///'):
                docs.append(line[3:].strip())
                continue
            if kind == 'enum':
                variant = re.fullmatch(r'(\w+),', line)
                if not variant or not re.fullmatch(r'(?:[A-Z][a-z0-9]+)+', variant[1]):
                    raise ValueError(f'Unsupported variant in {name}: {line}')
                variants.append(re.sub(r'(?<=[a-z0-9])(?=[A-Z])', '_', variant[1]).lower())
            else:
                field = re.fullmatch(r'pub (\w+): (.+),', line)
                if not field or field[1] in fields:
                    raise ValueError(f'Unsupported or duplicate field in {name}: {line}')
                schema = type_schema(field[2], names)
                if docs:
                    schema['description'] = ' '.join(docs)
                fields[field[1]] = schema
            docs = []
        if docs:
            raise ValueError(f'Unbound documentation in {name}')
        definitions[name] = ({'type': 'string', 'enum': variants} if kind == 'enum' else
                             {'type': 'object', 'properties': fields, 'required': list(fields),
                              'additionalProperties': False})
    schema_version = re.search(r'pub const SCHEMA_VERSION: &str = "([^"]+)";', source)[1]
    policy_version = re.search(r'pub const POLICY_VERSION: &str = "([^"]+)";', source)[1]
    properties = definitions['AnalysisReport']['properties']
    properties['schema_version'] = {'const': schema_version}
    properties['policy_version'] = {'const': policy_version}
    properties['engine_version']['minLength'] = 1
    properties['ancestry_verdict'] = {'const': 'INCONCLUSIVE'}
    properties['evidence_index'] = {'type': 'null'}
    definitions['Coverage']['properties']['decoded_pcm_sha256'] = {
        'type': 'string', 'pattern': '^[0-9a-f]{64}$'}
    # Successful reports have complete typed stream/coverage. Failed/cancelled
    # reports may retain partial metadata and must not be treated as clear input.
    definitions['AnalysisReport']['allOf'] = [{
        'if': {'properties': {'status': {'const': 'analyzed'}}, 'required': ['status']},
        'then': {'properties': {
            'stream': {'allOf': [{'$ref': '#/$defs/StreamInfo'},
                                {'properties': {'channels': {'minimum': 1, 'maximum': 2},
                                                'sample_rate': {'minimum': 8000, 'maximum': 384000}}}]},
            'coverage': {'allOf': [{'$ref': '#/$defs/Coverage'},
                                  {'properties': {'analysis_passes': {'const': 2}}}]},
            'channels': {'minItems': 1, 'maxItems': 2}}}}]
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema',
            '$id': f'urn:audio-forensic:analysis-report:{schema_version}',
            'title': f'Audio forensic report {schema_version}',
            'description': 'Exact serialized report shape and observations-only policy. '
                           'Required nullable fields preserve abstention. This is not accuracy validation '
                           'or a declaration that the evolving Rust API is stable.',
            '$ref': '#/$defs/AnalysisReport', '$defs': definitions}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    schema = build_schema((ROOT / 'src/model.rs').read_text(encoding='utf-8'))
    version = schema['$defs']['AnalysisReport']['properties']['schema_version']['const']
    path = ROOT / 'schemas' / f'analysis-report-{version}.schema.json'
    content = json.dumps(schema, indent=2, allow_nan=False) + '\n'
    if args.check:
        if not path.is_file() or path.read_text(encoding='utf-8') != content:
            raise SystemExit('Report schema differs from current model; review the contract/version before updating')
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        # A historical versioned artifact is immutable unless an explicit review
        # establishes that the exporter, rather than the model, was incorrect.
        if path.exists():
            raise SystemExit('Versioned schema already exists; use --check and review any drift')
        path.write_text(content, encoding='utf-8')
    print(f'PASS: {len(schema["$defs"])} model definitions; {path.name}')


if __name__ == '__main__':
    main()
