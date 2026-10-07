"""Separate v1 spectrogram schema; native report schema stays unchanged."""
import argparse
import json
import re
from generate_report_schema import ROOT, build_schema


def schema():
    src = (ROOT/'src/spectrogram.rs').read_text(encoding='utf-8')
    src += '\n' + (ROOT/'src/spectrogram_png.rs').read_text(encoding='utf-8')
    assert 'pub const ARTIFACT_VERSION: u32 = 1;' in src
    assert 'pub const METHOD_ID: &str = "hann1024-power-pair-merge-v1";' in src
    # The existing exporter emits additionalProperties:false for plain structs.
    # Explicit reviewed equivalents: deny_unknown_fields, f32 number, PathBuf text.
    src = src.replace('#[serde(deny_unknown_fields)]', '').replace(': f32,', ': f64,')
    # Reviewed backward-compatible optional wrapper presentation field.
    src = src.replace('#[serde(default)]', '')
    src = src.replace('crate::spectrogram_png::SpectrogramPresentation', 'SpectrogramPresentation')
    src = src.replace('Vec<f32>', 'Vec<f64>').replace('Option<PathBuf>', 'Option<String>')
    defs = build_schema((ROOT/'src/model.rs').read_text(encoding='utf-8')+'\n'+src)['$defs']
    used = {}
    def include(name):
        if name in used:
            return
        used[name] = defs[name]
        for child in re.findall(r'"\$ref": "#/\$defs/(\w+)"', json.dumps(defs[name])):
            include(child)
    include('SpectrogramArtifact')
    used['Coverage']['properties']['analysis_passes'] = {'const':2}
    used['Coverage']['properties']['hash_sample_encoding'] = {'enum':['s32le_msb_aligned','f64le']}
    used['Coverage']['properties']['start_seconds'] = {'const':0}
    used['Coverage']['properties']['analyzed_frames'].update(minimum=1)
    p = used['SpectrogramArtifact']['properties']
    p.update(artifact_version={'const':1}, contract_version={'const':1}, method_id={'const':'hann1024-power-pair-merge-v1'})
    used['SpectrogramArtifact']['allOf'] = [{
        'if':{'properties':{'status':{'const':'available'}}},
        'then':{'properties':{'data':{'$ref':'#/$defs/SpectrogramData'}, 'coverage':{'$ref':'#/$defs/Coverage'}, 'reason':{'type':'null'}}},
        'else':{'properties':{'data':{'type':'null'}, 'reason':{'type':'string','minLength':1}}}
    }]
    d = used['SpectrogramData']['properties']
    for key,val in {'fft_size':1024, 'hop_frames':512, 'frequency_rows':513,
                    'frequency_start_hz':0, 'window':'symmetric_f32_hann',
                    'frame_boundary':'window_end_strictly_before_analyzed_end',
                    'time_unit':'seconds', 'frequency_unit':'Hz', 'value_unit':'dBFS_power_per_bin',
                    'power_reference':1, 'reduction':'linear_power_mean_pairwise_time_merge',
                    'color_floor_db':-140, 'color_ceiling_db':0, 'palette':'ember-v1'}.items():
        d[key] = {'const':val}
    d['sample_rate_hz'].update(minimum=8000, maximum=384000)
    d['channel_indices'].update(minItems=1, maxItems=2, uniqueItems=True)
    d['channel_indices']['items'].update(maximum=1)
    d['time_columns'].update(minItems=1, maxItems=1280)
    d['power_db'].update(minItems=513, maxItems=1280*513)
    d['power_db']['items'].update(minimum=-140, maximum=3.4028234663852886e38)
    d['bucket_stft_frames'].update(minimum=1)
    d['total_stft_frames'].update(minimum=1)
    for key in ['native_peak', 'basis_peak']:
        d[key].update(minimum=0, maximum=16)
    used['TimeColumn']['properties']['stft_frames'].update(minimum=1)
    used['SpectrogramData']['allOf'] = [{
        'if':{'properties':{'basis':{'const':'mono'}}},
        'then':{'properties':{'channel_indices':{'const':[0]}, 'mid_cancelled_with_native_signal':{'const':False}}},
        'else':{'properties':{'channel_indices':{'const':[0,1]}}}
    }]
    return {'$schema':'https://json-schema.org/draft/2020-12/schema', '$id':'urn:audio-forensic:spectrogram:1',
            '$ref':'#/$defs/SpectrogramArtifact', '$defs':used}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    artifact = schema()
    data = json.dumps(artifact, indent=2, allow_nan=False)+'\n'
    path = ROOT/'schemas/spectrogram-1.schema.json'
    if args.check:
        assert path.read_text(encoding='utf-8') == data, 'spectrogram schema drift'
    else:
        with path.open('x', encoding='utf-8') as out:
            out.write(data)
    print(f"PASS: spectrogram-1.schema.json, {len(artifact['$defs'])} definitions")
