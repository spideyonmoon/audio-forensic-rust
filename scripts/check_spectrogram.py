"""Finite generated P06 serialized/schema/hash/raster acceptance; no private audio."""
import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import zlib
import jsonschema

ROOT = Path(__file__).resolve().parents[1]


def wav(words, channels, rate):
    pcm = struct.pack('<'+'h'*len(words), *words)
    return (b'RIFF'+struct.pack('<I',36+len(pcm))+b'WAVEfmt '+struct.pack('<IHHIIHH',16,1,channels,rate,rate*channels*2,channels*2,16)+b'data'+struct.pack('<I',len(pcm))+pcm)


def png_from_ppm(source, target):
    # Development QA encoding only; runtime PPM/RGB comes entirely from Rust.
    magic,dims,maximum,rgb = source.read_bytes().split(b'\n',3)
    assert magic==b'P6' and maximum==b'255'
    width,height = map(int,dims.split())
    assert len(rgb)==width*height*3
    def chunk(kind,payload):
        return struct.pack('>I',len(payload))+kind+payload+struct.pack('>I',zlib.crc32(kind+payload))
    rows=b''.join(b'\0'+rgb[y*width*3:(y+1)*width*3] for y in range(height))
    target.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b''))


def check(binary, output):
    output.mkdir(parents=True, exist_ok=False)
    report_schema=json.loads((ROOT/'schemas/analysis-report-0.18.0.schema.json').read_text())
    schema=json.loads((ROOT/'schemas/spectrogram-1.schema.json').read_text())
    native_validator=jsonschema.Draft202012Validator(report_schema)
    artifact_validator=jsonschema.Draft202012Validator(schema)
    cases=[('tone8k',8000,1,16000,'tone'),('dual48k',48000,2,48000,'tone'),
           ('silent',8000,1,8192,'silent'),('anti',8000,2,8192,'anti'),
           ('burst',8000,1,24000,'burst'),('impulse',8000,1,24000,'impulse'),
           ('long384k',384000,1,1311745,'burst'),('short',8000,1,1024,'silent')]
    receipts=[]
    max_tone_power_error=0.
    for name,rate,channels,frames,kind in cases:
        lo,hi=(frames//3,2*frames//3) if name=='long384k' else (10000,14000)
        words=[]
        for i in range(frames):
            a=round(16384*math.sin(math.tau*64*i/1024))
            if kind=='silent' or kind=='burst' and not lo<=i<hi: a=0
            if kind=='impulse': a=24576 if i==12000 else 0
            words.extend([a] if channels==1 else [a,-a if kind=='anti' else a])
        source=output/(name+'.wav'); source.write_bytes(wav(words,channels,rate))
        for prefix in [None,1.] if kind=='burst' else [None]:
            stem=name+('-prefix' if prefix is not None else '')
            ppm=output/(stem+'.ppm')
            process=subprocess.run([str(binary),str(source),'full' if prefix is None else str(prefix),str(ppm)],capture_output=True,text=True,timeout=180)
            assert process.returncode==0,(name,process.stderr[-2000:])
            p=json.loads(process.stdout); r=p['measurement']; a=p['spectrogram']
            native_validator.validate(r); artifact_validator.validate(a)
            assert r['ancestry_verdict']=='INCONCLUSIVE' and r['evidence_index'] is None
            assert a['coverage']==r['coverage']
            count=frames if prefix is None else min(frames,math.floor(rate*prefix))
            expected=hashlib.sha256(b''.join(struct.pack('<i',w<<16) for w in words[:count*channels])).hexdigest()
            assert a['coverage']['decoded_pcm_sha256']==expected
            assert a['coverage']['analyzed_frames']==count and a['coverage']['analysis_passes']==2
            d=a['data']; export=json.loads(process.stderr.strip())
            if count<=1024:
                assert a['status']=='unavailable' and d is None and export['path'] is None and not ppm.exists()
            else:
                assert a['status']=='available' and export['status']=='available' and Path(export['path'])==ppm
                assert d['interval']=={'start_frame':0,'end_frame':count}
                assert len(d['time_columns'])<=1280 and d['frequency_rows']==513
                assert d['frequency_step_hz']==rate/1024 and d['channel_indices']==list(range(channels))
                assert len(d['power_db'])==513*len(d['time_columns'])
                assert sum(c['stft_frames'] for c in d['time_columns'])==d['total_stft_frames']
                n=(count-1025)//512+1
                assert d['total_stft_frames']==n and d['spectral_interval']['end_frame']==(n-1)*512+1024
                if kind in ['silent','anti']:
                    assert all(v==-140 for v in d['power_db'])
                if kind=='anti': assert d['mid_cancelled_with_native_signal'] and d['native_peak']>0
                if kind=='tone':
                    for off in range(0,len(d['power_db']),513):
                        bins=d['power_db'][off:off+513]
                        assert max(range(513),key=lambda k:bins[k])==64
                        err=abs(sum(10**(v/10) for v in bins)-.125)
                        max_tone_power_error=max(max_tone_power_error,err)
                        assert err<2e-5
                if kind=='burst' and prefix is None:
                    active=[i for i in range(len(d['time_columns'])) if d['power_db'][i*513+64]>-40]
                    first=d['time_columns'][active[0]]['window_support']['start_frame']+512
                    last=d['time_columns'][active[-1]]['window_support']['end_frame']-512
                    bucket=d['bucket_stft_frames']*512
                    assert abs(first-lo)<=bucket and abs(last-hi)<=bucket,(name,first,last,lo,hi,bucket)
                if kind=='burst' and prefix is not None and count<=lo:
                    assert all(v==-140 for v in d['power_db'])
                if name=='long384k' and prefix is None: assert d['bucket_stft_frames']==4
                png_from_ppm(ppm,output/(stem+'.png'))
            (output/(stem+'.json')).write_text(process.stdout,encoding='utf-8')
            receipts.append({'case':stem,'status':a['status'],'frames':count,'hash':expected})
    # Explicit collision and invalid input keep status/path distinct.
    ppm=output/'tone8k.ppm'; before=hashlib.sha256(ppm.read_bytes()).hexdigest()
    process=subprocess.run([str(binary),str(output/'tone8k.wav'),'full',str(ppm)],capture_output=True,text=True,timeout=60)
    assert process.returncode==0
    assert json.loads(process.stderr)['path'] is None
    assert hashlib.sha256(ppm.read_bytes()).hexdigest()==before
    invalid=output/'invalid.wav'; invalid.write_bytes(b'bad')
    process=subprocess.run([str(binary),str(invalid)],capture_output=True,text=True,timeout=60)
    assert process.returncode==1
    p=json.loads(process.stdout); native_validator.validate(p['measurement']); artifact_validator.validate(p['spectrogram'])
    assert p['spectrogram']['data'] is None
    (output/'invalid.json').write_text(process.stdout,encoding='utf-8')
    base=json.loads((output/'tone8k.json').read_text())['spectrogram']
    for mutation in range(8):
        bad=copy.deepcopy(base)
        if mutation==0: bad['artifact_version']=2
        elif mutation==1: bad['method_id']='unknown'
        elif mutation==2: bad['data']['frequency_rows']=514
        elif mutation==3: bad['data']['fft_size']=4096
        elif mutation==4: bad['data']['channel_indices']=[1]
        elif mutation==5: bad['data']['color_floor_db']=-120
        elif mutation==6: bad['status']='failed'
        else: bad['data']['power_db'][0]=-141
        assert list(artifact_validator.iter_errors(bad)),mutation
    summary={'generated_reports':receipts,'invalid_input':True,'collision_preserved':True,
             'schema_mutations_rejected':8,'max_tone_total_power_error':max_tone_power_error}
    (output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(f'PASS: {len(receipts)} generated reports, exact PCM/prefix/schema/raster, collision, invalid input, eight schema rejections; tone power error {max_tone_power_error:.6g}')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    check(args.binary.resolve(),args.output.resolve())
