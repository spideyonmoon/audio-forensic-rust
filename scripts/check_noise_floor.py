"""Independent dense NumPy quiet-block oracle; generated local audio only."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import wave
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
AUDIO = ROOT/'corpus/local/generated/noise-floor-v12'
OUT = ROOT/'corpus/local/results/noise-floor-v12'
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)

def save(name, x, rate):
    words = np.rint(x*2**23).astype(np.int32)
    assert np.max(np.abs(words)) < 2**23
    data = ((words[..., None] >> (8*np.arange(3))) & 255).astype(np.uint8).tobytes()
    path = AUDIO/(name+'.wav')
    if path.exists():
        with wave.open(str(path),'rb') as w:
            assert w.readframes(w.getnframes()) == data
    else:
        with wave.open(str(path),'wb') as w:
            w.setparams((x.shape[1],3,rate,0,'NONE','not compressed'))
            w.writeframes(data)
    return path

def oracle(x, rate):
    n = rate//10
    seen = min(len(x), 300*n)
    count = seen//n
    blocks = x[:count*n].reshape(count,n)
    rms = np.sqrt(np.mean(blocks**2,axis=1))
    nonzero = np.flatnonzero(rms > 0)
    enough = len(nonzero) >= 20
    selected = []
    low = high = None
    if enough:
        low, high = map(float,np.percentile(rms[nonzero],[1.5,99]))
        ordered = nonzero[np.argsort(rms[nonzero],kind='stable')]
        selected = sorted(map(int,ordered[:max(3,len(nonzero)//10)]))
    freq = np.fft.rfftfreq(n,1/rate)
    w = np.hanning(n)
    spec = np.mean(np.abs(np.fft.rfft(blocks[selected]*w,axis=1))**2,axis=0)*2/(n*np.sum(w*w)) if selected else None
    if selected:
        # Independent time-domain total Hann-weighted power via Parseval.
        total = np.mean(np.sum((blocks[selected]*w)**2,axis=1)/np.sum(w*w))
        floor = max(1e-24,1e-12*total)
    bands = []
    bins = np.arange(len(freq),dtype=np.int64)
    for high_band,(lower,upper) in enumerate([(150.,2000.),(.33*rate,.45*rate)]):
        mask = ((bins*100>33*n)&(bins*100<45*n)) if high_band else ((bins*rate>150*n)&(bins*rate<2000*n))
        power = float(np.mean(spec[mask])) if spec is not None and mask.sum()>=2 else None
        bands.append(dict(requested_lower_hz=lower,requested_upper_hz=upper,
            lower_bin_hz=float(freq[mask][0]),upper_bin_hz=float(freq[mask][-1]),bin_count=int(mask.sum()),
            mean_bin_power=power,energy_eligible=bool(power is not None and power>floor)))
    color = all(b['energy_eligible'] for b in bands)
    return dict(status='measured' if enough else 'inconclusive',
        interval=dict(start_frame=0,end_frame=count*n) if count else None,
        block_frames=n,inspected_frames=seen,trailing_frames=seen%n,coverage_capped=len(x)>seen,
        complete_blocks=count,zero_blocks=count-len(nonzero),rms_underflow_blocks=0,nonzero_blocks=len(nonzero),
        nonzero_rms_p015=low,nonzero_rms_p99=high,
        nonzero_rms_p015_dbfs=float(20*np.log10(low)) if low else None,
        nonzero_rms_p99_dbfs=float(20*np.log10(high)) if high else None,
        selected_blocks=[dict(block_index=i,rms=float(rms[i])) for i in selected],
        color_status='measured' if color else 'inconclusive',low_band=bands[0],high_band=bands[1],
        high_minus_low_db=float(10*np.log10(bands[1]['mean_bin_power']/bands[0]['mean_bin_power'])) if color else None)

# Fixed before comparing: RMS absolute 1e-14 + relative 1e-10;
# f64 FFT power absolute 1e-25 + relative 1e-8; dB error 1e-6.
# Selection, counts, geometry and statuses must match; no threshold allowances.
def compare(a,e,path=''):
    if isinstance(e,dict):
        for k,v in e.items(): compare(a[k],v,path+'/'+k)
    elif isinstance(e,list):
        assert len(a)==len(e),(path,len(a),len(e))
        for i,v in enumerate(e): compare(a[i],v,path+'/'+str(i))
    elif isinstance(e,float):
        tol = 1e-6 if ('db' in path) else (1e-25+1e-8*abs(e) if 'power' in path else 1e-14+1e-10*abs(e))
        assert abs(a-e)<=tol,(path,a,e,tol)
    else: assert a==e,(path,a,e)

cases=[]
rng=np.random.default_rng(20261002)
for rate in [8000,8001,44100,48000,96000,383990,384000]:
    n=rate//10
    x=rng.uniform(-1,1,(25,n))*np.linspace(.002,.02,25)[:,None]
    x[[0,4,9]]=0
    x=x.ravel()
    cases.append((save('noise_gaps_'+str(rate),np.c_[x,-x*.5],rate),None))
rate=8000
n=800
levels=np.array([0 if i%2==0 else (25-i//2)/128 for i in range(50)])
x=np.repeat(levels,n)
cases.append((save('original_index_regression',np.c_[x,-x],rate),None))
x=np.zeros(20*n)
cases.append((save('silence',np.c_[x,x],rate),None))
x=np.full(20*n,.125)
x[-n:]=0
cases.append((save('nineteen_nonzero',np.c_[x,np.zeros_like(x)],rate),None))
x=np.full(301*n+7,.125); x[300*n:]=.001
cap=save('cap_and_ties',np.c_[x,-x],rate)
cases.extend([(cap,None),(cap,1.99),(cap,2.000125),(cap,0.01)])
rate=48000
n=rate//10
t=np.arange(22*n)/rate
levels=np.repeat(np.linspace(.001,.02,22),n)
x=levels*(np.sin(2*np.pi*1000*t)+.5*np.sin(2*np.pi*18000*t))
cases.append((save('two_band_tones',np.c_[x,-x*.5],rate),None))
x=levels*np.sin(2*np.pi*1000*t)
cases.append((save('low_tone_missing_high',np.c_[x,x*1e-4],rate),None))
x=rng.uniform(-1,1,22*n)*levels
freq=np.fft.rfftfreq(len(x),1/rate)
y=np.fft.irfft(np.fft.rfft(x)/(1+(freq/3000)**4),n=len(x))
cases.append((save('colored_and_white',np.c_[y,x],rate),None))
records=[]
for path,limit in cases:
    with wave.open(str(path),'rb') as w: rate,channels=w.getframerate(),w.getnchannels()
    raw=subprocess.check_output(['ffmpeg','-nostdin','-v','error','-i',str(path),'-c:a','pcm_s32le','-f','s32le','pipe:1'])
    x=np.frombuffer(raw,dtype='<i4').reshape(-1,channels)
    if limit is not None: x=x[:int(limit*rate)]; raw=x.tobytes()
    samples=x.astype(float)/2**31
    expected=[oracle(samples[:,ch],rate) for ch in range(channels)]
    name=path.stem+('' if limit is None else '_prefix_'+str(limit))
    saved=json.dumps(expected,indent=2,allow_nan=False)+'\n'
    reference=OUT/(name+'.expected-rational-bounds.json')
    if reference.exists(): assert reference.read_text()==saved,'Refusing oracle overwrite: '+name
    else: reference.write_text(saved)
    if '--prepare' in sys.argv:
        print('prepared',name,flush=True)
        continue
    command=[str(ROOT/'target/release/audio-forensic.exe'),'--json']
    if limit is not None: command+=['--max-seconds',str(limit)]
    output=subprocess.check_output(command+[str(path)])
    (OUT/(name+'.json')).write_bytes(output)
    report=json.loads(output)[0]
    assert report['status']=='analyzed' and report['schema_version']=='0.12.0'
    assert report['coverage']['decoded_pcm_sha256']==hashlib.sha256(raw).hexdigest()
    assert report['ancestry_verdict']=='INCONCLUSIVE' and report['evidence_index'] is None
    for ch,e in enumerate(expected): compare(report['noise_floor'][ch],e,name+'/ch'+str(ch))
    records.append(dict(name=name,pcm_hash_matches=True,numerical_checks_passed=True,
        color_db=[p['high_minus_low_db'] for p in report['noise_floor']]))
    print(records[-1],flush=True)
if '--prepare' not in sys.argv:
    (OUT/'summary.json').write_text(json.dumps(dict(numpy=np.__version__,controls=records),indent=2)+'\n')
