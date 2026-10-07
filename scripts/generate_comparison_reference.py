"""Freeze P07 sort controls by executing the pinned Python comparison functions."""
import argparse
import ast
import json
from pathlib import Path
from types import SimpleNamespace as N
ROOT = Path(__file__).resolve().parents[1]

def fixture():
    source = (ROOT/'reference/audio-forensic/audio_forensic.py').read_text(encoding='utf-8')
    names = {'_bitdepth_confidence','_dr_int','_container_quality','_compare_key'}
    tree = ast.parse(source)
    nodes = [n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names]
    assert len(nodes)==4
    scope = {'ForensicReport':object,'AudioTechnical':object}
    exec(compile(ast.Module(body=nodes,type_ignores=[]),'pinned-comparison-functions','exec'),scope)
    base = {'available':True,'main_score':10,'rate_history_flag':False,'cutoff_hz':20000.,
            'depth_category':0,'legacy_dr_integer':12,'precision_times_rate':16*44100}
    variations = [('base',{}),('tie',{}),('unavailable',{'available':False,'main_score':None}),
        ('lower_main',{'main_score':9}),('rate_history',{'rate_history_flag':True}),
        ('wider',{'cutoff_hz':21000.}),('depth_abstain',{'depth_category':1}),
        ('depth_flag',{'depth_category':2}),('higher_dr',{'legacy_dr_integer':13}),
        ('higher_container',{'precision_times_rate':24*44100}),
        ('missing_dr',{'legacy_dr_integer':None}),('missing_cutoff',{'cutoff_hz':None}),
        ('missing_depth',{'depth_category':None}),('missing_container',{'precision_times_rate':None})]
    cases=[]
    for name,change in variations:
        key=base|change
        # Native PCM rates/precision differ in other controls. Here container
        # multiplication alone drives the last tie-breaker in the pinned tuple.
        bits = key['precision_times_rate']//44100 if key['precision_times_rate'] else 0
        depth = {0:'consistent',1:'~ abstain',2:'⚠ flagged',None:''}[key['depth_category']]
        sp=N(verdict_label='GENUINE' if key['available'] else 'INCONCLUSIVE',main_score=key['main_score'],
            resample_detected=key['rate_history_flag'],fake_hires=False,cutoff_hz=key['cutoff_hz'] or 0.)
        report=N(authenticity=N(spectral=sp,bit_depth_authentic=depth),
            dr_score=f"DR{key['legacy_dr_integer']}" if key['legacy_dr_integer'] is not None else '',
            technical=N(precision=f'{bits}-bit',sample_rate=44100))
        raw=scope['_compare_key'](report)
        cases.append({'name':name,'key':key,'python_tuple':raw})
    return {'reference_commit':'c6ecce2296256b516709d87088896d1be913908c', 'cases':cases,
            'expected_order':sorted(range(len(cases)),key=lambda i:cases[i]['python_tuple'])}

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    data=fixture();text=json.dumps(data,indent=2,ensure_ascii=False)+'\n';path=ROOT/'tests/fixtures/comparison_reference.json'
    if args.check:assert path.read_text(encoding='utf-8')==text,'Comparison oracle drift'
    else:path.write_text(text,encoding='utf-8')
    print(f"PASS: {len(data['cases'])} pinned comparison cases")
