"""Differential assessment of previously validated generated-audio receipts.
No decode or private recordings; compares pinned policy on equivalent adapters.
"""
import argparse,hashlib,json,subprocess
from pathlib import Path
from generate_reference_assessment import BASE,oracle,py

def main():
 p=argparse.ArgumentParser();p.add_argument('--reader',required=True);p.add_argument('--output',required=True);p.add_argument('receipts',nargs='+');a=p.parse_args()
 output=Path(a.output);output.mkdir(parents=True,exist_ok=False);summary=[]
 for name in a.receipts:
  path=Path(name);raw=path.read_bytes();original=json.loads(raw.decode('utf-8'));r=original['reference_inputs']
  result=subprocess.run([str(Path(a.reader).resolve()),str(path.resolve())],check=True,capture_output=True,encoding='utf-8')
  v=json.loads(result.stdout);assert v['status'] in ('available','partial'),(path,v['status'],v['missing_inputs'],v['display_summary'])
  assert v['assessment_version']==1 and v['method_id']=='python-reference-c6ecce2-v1' and v['calibration_status']=='uncalibrated'
  assert v['input_binding']['coverage']==original['measurement']['coverage']
  assert [t['id'] for t in v['rules']]==[f'R{i:02}' for i in range(1,35)]
  f=BASE.copy();inactive=[]
  for k in BASE:
   value=v['features'][k]['value']
   if value is None:inactive.append(k)
   else:f[k]=value
  # Missing values are placeholders only for source-inactive branches. Rust
  # requires available composite status; any missing applicable input fails above.
  if v['status']=='available':
   expected=oracle(f,r['segments']['probes'])
   for k in v['scores']:assert v['scores'][k]==expected[k],(path,k,v['scores'][k],expected[k])
   assert v['reference_label']==expected['label']
   assert v['legacy_outputs']['primary_verdict']==expected['text']
   assert v['legacy_outputs']['cassette_score']==expected['cassette']
  else:
   assert v['scores'] is None and v['reference_label'] is None and v['missing_inputs']
   assert v['legacy_outputs']['net_confidence_pct'] is None
   assert len([p for p in r['segments']['probes'] if p['eligible']])<3
  for key in ['R11','R13','R32']:
   t=next(t for t in v['rules'] if t['id']==key);assert t['state']=='excluded_by_contract' and t['before']==t['after']
  for feat in v['features'].values():
   for i in feat['intervals']:assert 0<=i['start_frame']<=i['end_frame']<=r['analyzed_frames']
  assert path.read_bytes()==raw
  (output/path.name).write_text(result.stdout,encoding='utf-8')
  summary.append(dict(receipt=str(path),receipt_sha256=hashlib.sha256(raw).hexdigest(),pcm_sha256=r['decoded_pcm_sha256'],status=v['status'],main=v['scores']['main'] if v['scores'] else None,label=v['reference_label'],inactive_placeholder_features=inactive))
 assert any(c['status']=='available' for c in summary),'Need an equivalent-input complete policy comparison, not only abstentions'
 (output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
 print(f'PASS {len(summary)} generated-audio saved policy/binding/interval/unchanged-receipt checks')
if __name__=='__main__':main()
