"""Independent offline P07 schema/PCM/workflow controls on generated audio only."""
import argparse,copy,hashlib,json,struct,subprocess
from pathlib import Path
from validate_report_schema import strict_json,offline_validator,validate_report
ROOT=Path(__file__).resolve().parents[1]

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--binary',type=Path,required=True);parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
    args.output.mkdir(parents=True,exist_ok=False)
    def run(name,flags,code=0,json_output=True):
        command=[str(args.binary.resolve()),*map(str,flags)]
        p=subprocess.run(command,capture_output=True)
        (args.output/(name+'.stdout')).write_bytes(p.stdout);(args.output/(name+'.stderr')).write_bytes(p.stderr)
        assert p.returncode==code,(name,p.returncode,p.stderr.decode('utf-8'))
        return strict_json(p.stdout) if json_output else p.stdout.decode('utf-8')
    product_validator=offline_validator(strict_json((ROOT/'schemas/product-1.schema.json').read_bytes()))
    comparison_validator=offline_validator(strict_json((ROOT/'schemas/comparison-1.schema.json').read_bytes()))
    native_validator=offline_validator(strict_json((ROOT/'schemas/analysis-report-0.18.0.schema.json').read_bytes()))
    source=(ROOT/'tests/fixtures/clip_noise.wav').read_bytes();assert source[36:40]==b'data'
    bits=struct.unpack_from('<H',source,34)[0];width=bits//8;assert bits in (16,24,32)
    pcm=source[44:]*9
    data=bytearray(source[:44]+pcm);struct.pack_into('<I',data,4,36+len(pcm));struct.pack_into('<I',data,40,len(pcm))
    audio=args.output/'generated.wav';audio.write_bytes(data)
    reduced=bytearray(data)
    for offset in range(44,len(reduced),width):
        word=int.from_bytes(data[offset:offset+width],'little',signed=True)//2
        reduced[offset:offset+width]=word.to_bytes(width,'little',signed=True)
    gain=args.output/'generated-half.wav';gain.write_bytes(reduced)
    exact=hashlib.sha256(b''.join((int.from_bytes(pcm[i:i+width],'little',signed=True)<<(32-bits)).to_bytes(4,'little',signed=True) for i in range(0,len(pcm),width))).hexdigest()
    docs=run('products',['--product-json','--collect-spectrogram',audio,gain])
    product_validator.validate(docs)
    for p in docs:validate_report(p['measurement_report'],native_validator)
    assert docs[0]['measurement_report']['coverage']['decoded_pcm_sha256']==exact
    assert len(docs[0]['field_aliases'])==155
    alias=docs[0]['field_aliases']
    for a in alias.values():
        assert a['status'] in ('available','unavailable','audit_only')
        assert a['unit'] and a['domain'] and a['qualification']
    native=run('native',['--json',audio])[0]
    assert native==docs[0]['measurement_report']
    saved=args.output/'saved.json';saved.write_text(json.dumps(docs,ensure_ascii=False),encoding='utf-8')
    audio.unlink();gain.unlink()
    text=run('saved-text',['--saved',saved],json_output=False)
    assert 'Reference method (uncalibrated)' in text and 'Batch summary' in text
    comparison=run('comparison',['--saved','--compare','--comparison-json',saved]);comparison_validator.validate(comparison)
    assert comparison['status']=='available' and comparison['winner_input_index'] is not None
    # Check actual saved-product tuples against Python's frozen sort mechanics.
    def key(entry):
        k=entry['key'];return(not k['available'],k['main_score'] if k['main_score'] is not None else 999,
            k['rate_history_flag'] or False,-(k['cutoff_hz'] or 0),k['depth_category'] or 0,
            -(k['legacy_dr_integer'] if k['legacy_dr_integer'] is not None else -1),-(k['precision_times_rate'] or 0))
    assert comparison['ranking']==sorted(comparison['ranking'],key=key)
    # Absence is explicit; old optional presentation/title are readable. Preserve
    # exact u64 above JavaScript Number range and no input decode on saved paths.
    absent=copy.deepcopy(docs[0]);absent['artifacts'].pop('presentation');absent['artifacts'].pop('title')
    absent['byproducts']=None;absent['tool_statistics']=None
    absent['metadata']['technical']['file_size_bytes']=2**64-1
    product_validator.validate(absent)
    absent_file=args.output/'absent.json';absent_file.write_text(json.dumps(absent),encoding='utf-8')
    absent_result=run('absent',['--saved','--product-json',absent_file])[0]
    assert absent_result['metadata']['technical']['file_size_bytes']==2**64-1
    product_validator.validate(absent_result)
    negatives=0
    for path,value in [(['product_schema_version'],'future'),(['contract_version'],2),
            (['reference_assessment','method_id'],'native-lookalike'),(['reference_inputs','version'],3),
            (['measurement_report','ancestry_verdict'],'GENUINE'),(['measurement_report','evidence_index'],100),
            (['metadata','metadata_version'],2),(['artifacts','presentation','presentation_version'],2)]:
        invalid=copy.deepcopy(docs[0]);target=invalid
        for k in path[:-1]:target=target[k]
        target[path[-1]]=value
        assert list(product_validator.iter_errors(invalid)),path
        file=args.output/f'negative-{negatives}.json';file.write_text(json.dumps(invalid),encoding='utf-8')
        run(f'negative-{negatives}',['--saved',file],code=2,json_output=False);negatives+=1
    unknown=copy.deepcopy(docs[0]);unknown['unexpected']=True
    assert list(product_validator.iter_errors(unknown))
    file=args.output/'negative-unknown.json';file.write_text(json.dumps(unknown),encoding='utf-8')
    run('negative-unknown',['--saved',file],code=2,json_output=False);negatives+=1
    failure=run('missing',['--product-json',args.output/'missing.wav'],code=1)
    product_validator.validate(failure)
    assert failure[0]['reference_assessment']['scores'] is None
    # A generated 61-second mono PCM control exercises the real 60s alias across
    # every collector rather than only checking the requested option on EOF.
    import wave
    long=args.output/'generated-61s.wav'
    samples=bytearray();state=7
    for _ in range(61*8000):
        state=(1664525*state+1013904223)&0xffffffff
        samples.extend(struct.pack('<h',((state>>16)&0xffff)-32768))
    with wave.open(str(long),'wb') as wav:
        wav.setnchannels(1);wav.setsampwidth(2);wav.setframerate(8000);wav.writeframes(samples)
    fast=run('fast',['--product-json','--fast','--collect-spectrogram',long])[0]
    product_validator.validate(fast)
    coverage=fast['measurement_report']['coverage']
    assert coverage['analyzed_frames']==480000 and coverage['end_seconds']==60 and not coverage['reached_end']
    for branch in ['byproducts','tool_statistics']:
        assert fast[branch]['coverage']==coverage
    assert fast['reference_inputs']['analyzed_frames']==480000
    assert fast['reference_inputs']['source']['captured_interval']['end_frame']==480000
    assert fast['artifacts']['spectrogram']['coverage']==coverage
    compressed=run('compressed',['--product-json','--collect-spectrogram',ROOT/'tests/fixtures/rate_wall.flac',
        ROOT/'tests/fixtures/alac/44100-16-2-front.m4a'])
    product_validator.validate(compressed)
    assert all(p['measurement_report']['status']=='analyzed' and p['metadata']['status']=='available' for p in compressed)
    summary={'product_schema_definitions':len(product_validator.schema['$defs']), 'generated_products':len(docs),
        'negative_schema_and_runtime_controls':negatives,'exact_pcm_sha256':exact,
        'comparison_status':comparison['status'],'winner_input_index':comparison['winner_input_index'],
        'preserved_u64':absent_result['metadata']['technical']['file_size_bytes'],
        'source_audio_deleted_before_saved_checks':True,'fast_frames':coverage['analyzed_frames'], 'compressed_products':len(compressed)}
    (args.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(summary,indent=2))

if __name__=='__main__':main()
