"""Export separate P04c schema, preserving the native v0.18 report artifact."""
import argparse
import json
import re
from generate_report_schema import ROOT, build_schema


def schema():
    source=(ROOT/'src/model.rs').read_text(encoding='utf-8')+'\n'+(ROOT/'src/reference_inputs.rs').read_text(encoding='utf-8')+'\n'+(ROOT/'src/reference_spectral.rs').read_text(encoding='utf-8')+'\n'+(ROOT/'src/reference_source.rs').read_text(encoding='utf-8')
    source=re.sub(r"crate::(?:reference_spectral|reference_source)::", "", source)
    # The existing strict exporter handles f64 only; this adapter explicitly
    # exports f32 peak fields as finite JSON numbers without altering that tool.
    source=re.sub(r'pub (mid_peak|side_peak): (Option<)?f32(>)?,',
        lambda m:f'pub {m[1]}: '+('Option<f64>' if m[2] else 'f64')+',',source)
    definitions=build_schema(source)['$defs'];used={}
    def include(name):
        if name in used:return
        used[name]=definitions[name]
        for child in re.findall(r'"\$ref": "#/\$defs/(\w+)"',json.dumps(definitions[name])):include(child)
    include('ReferenceAnalysis')
    props=used['ReferenceInputs']['properties']
    props.update(version={'const':2},method={'const':'python-c6ecce2-p04abc-f32-v2'},
        reference_commit={'const':'c6ecce2296256b516709d87088896d1be913908c'},
        processing_passes={'const':3},decoded_pcm_sha256={'type':'string','pattern':'^[0-9a-f]{64}$'},
        pass_pcm_sha256={'type':'array','minItems':3,'maxItems':3,'items':{'type':'string','pattern':'^[0-9a-f]{64}$'}},
        deviation_ids={'const':['D01','D02','D03']})
    used['ReferenceAnalysis']['allOf']=[{
        'if':{'properties':{'measurement':{'properties':{'status':{'const':'analyzed'}}}}},
        'then':{'properties':{'reference_inputs':{'$ref':'#/$defs/ReferenceInputs'}}},
        'else':{'properties':{'reference_inputs':{'type':'null'}}}}]
    used['InputValue']['allOf']=[{
        'if':{'properties':{'value':{'type':'null'}}},
        'then':{'properties':{'unavailable_reason':{'type':'string','minLength':1}}},
        'else':{'properties':{'unavailable_reason':{'type':'null'}}}}]
    for name,field,limit in [('SegmentPlan','offsets',36),('SegmentInputs','probes',36),
        ('ReferenceInputs','aac_bases',2),('ReferenceInputs','vorbis_reconstructed_channels',2)]:
        used[name]['properties'][field]['maxItems']=limit
    used['ScatterInputs']['properties']['legacy_phase_histogram'].update(minItems=36,maxItems=36)
    used['SegmentPlan']['properties']['requested_probes'].update(minimum=9,maximum=36)
    return {'$schema':'https://json-schema.org/draft/2020-12/schema','$id':'urn:audio-forensic:reference-inputs:2',
        '$ref':'#/$defs/ReferenceAnalysis','$defs':used}


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');args=p.parse_args()
    artifact=schema();text=json.dumps(artifact,indent=2,allow_nan=False)+'\n';path=ROOT/'schemas/reference-inputs-2.schema.json'
    if args.check:assert path.read_text(encoding='utf-8')==text,'P04abc schema drift'
    else:
        with path.open('x',encoding='utf-8') as f:f.write(text)
    print(f"PASS: reference-inputs-2 schema, {len(artifact['$defs'])} definitions")
