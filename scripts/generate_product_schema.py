"""Export the separately versioned P07 product/comparison contracts, offline."""
import argparse
import json
import re
import generate_report_schema as native
from generate_reference_inputs_schema import schema as reference_schema
from generate_spectrogram_schema import schema as spectrogram_schema


def schema(root):
    modules = ['model', 'metadata', 'byproducts', 'tool_statistics', 'reference_inputs',
               'reference_source', 'reference_spectral', 'reference_assessment',
               'spectrogram', 'spectrogram_png', 'product']
    source = '\n'.join((native.ROOT/f'src/{name}.rs').read_text(encoding='utf-8-sig') for name in modules)
    # Reviewed serde equivalents for this product exporter only. Native exporter
    # and its unchanged artifact remain strict. Optional defaults adjusted below.
    source = source.replace('&impl Serialize', '&impl ExporterIgnoredBound')
    source = source.replace('#[serde(deny_unknown_fields)]', '').replace('#[serde(default)]', '')
    source = re.sub(r'crate::(?:reference_source|reference_spectral|spectrogram_png)::', '', source)
    original = native.type_schema
    def product_type(name, names):
        if name == 'f32': return original('f64', names)
        if name == 'PathBuf': return {'type':'string'}
        if name == 'i32': return {'type':'integer', 'minimum':-2**31, 'maximum':2**31-1}
        # These are existing explicit audit/binding objects. Rust recomputes the
        # assessment from saved typed inputs to check their semantic consistency.
        if name == 'serde_json::Value': return {}
        return original(name, names)
    native.type_schema = product_type
    try: definitions = native.build_schema(source)['$defs']
    finally: native.type_schema = original
    # Reuse stricter reviewed reference/artifact bounds, never widen native v18.
    definitions.update(reference_schema()['$defs'])
    definitions.update(spectrogram_schema()['$defs'])
    definitions['ProductReport']['properties']['product_schema_version'] = {'const':'audio-forensic-product-v1'}
    definitions['ProductReport']['properties']['contract_version'] = {'const':1}
    for optional in ['presentation','title']:
        definitions['ProductArtifacts']['required'].remove(optional)
    for name, props in {
        'MetadataReport': {'metadata_version':1,'contract_version':1},
        'ByproductReport': {'byproduct_version':1,'contract_version':1,'method_id':'python-reference-c6ecce2-byproducts-v1'},
        'ToolStatisticsReport': {'statistics_version':1,'contract_version':1,'method_id':'ffmpeg-7.1.1-sox-14.4.2-v1'},
        'ReferenceAssessment': {'assessment_version':1,'contract_version':1,'method_id':'python-reference-c6ecce2-v1',
            'calibration_status':'uncalibrated','reference_commit':'c6ecce2296256b516709d87088896d1be913908c'},
        'SpectrogramPresentation': {'presentation_version':1,'bitrate_method':'encoded_complete_packet_mean'},
        'ComparisonReport': {'comparison_schema_version':'audio-forensic-comparison-v1', 'method_id':'python-c6ecce2-reference-tuple-v1'},
    }.items():
        definitions[name]['properties'].update({k:{'const':v} for k,v in props.items()})
    definitions['ComparisonReport']['properties']['status'] = {'enum':['available','unavailable','incompatible']}
    for name, field, limit in [('MetadataReport','entries',1024),('ReferenceAssessment','rules',34),('ProductArtifacts','exports',32)]:
        definitions[name]['properties'][field]['maxItems'] = limit
    definitions['FieldAlias']['properties']['status'] = {'enum':['available','unavailable','audit_only']}
    aliases = json.loads((native.ROOT/'assets/product-field-aliases-v1.json').read_text(encoding='utf-8'))
    definitions['ProductReport']['properties']['field_aliases'].update(
        minProperties=155,maxProperties=155,propertyNames={'enum':list(aliases)})
    definitions['ProductReport']['allOf'] = [{
        'if': {'properties': {'measurement_report': {'properties': {'status': {'const':'analyzed'}}}}},
        'else': {'properties': {'reference_inputs': {'type':'null'}}}}]
    used = {}
    def include(name):
        if name in used: return
        used[name] = definitions[name]
        for child in re.findall(r'"\$ref": "#/\$defs/(\w+)"',json.dumps(definitions[name])): include(child)
    include(root)
    name = 'product' if root == 'ProductReport' else 'comparison'
    document = {'$schema':'https://json-schema.org/draft/2020-12/schema', '$id':f'urn:audio-forensic:{name}:1', '$defs':used}
    if root == 'ProductReport':
        document['oneOf'] = [{'$ref':'#/$defs/ProductReport'}, {'type':'array','minItems':1,'items':{'$ref':'#/$defs/ProductReport'}}]
    else: document['$ref'] = '#/$defs/ComparisonReport'
    return document


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for root, name in [('ProductReport','product'),('ComparisonReport','comparison')]:
        data = schema(root)
        text = json.dumps(data,indent=2,allow_nan=False)+'\n'
        path = native.ROOT/f'schemas/{name}-1.schema.json'
        if args.check: assert path.read_text(encoding='utf-8')==text, f'{name} schema drift'
        else: path.write_text(text,encoding='utf-8')
        print(f'PASS: {name} schema, {len(data["$defs"])} definitions')
