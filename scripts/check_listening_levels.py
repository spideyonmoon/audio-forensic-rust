"""Independent FIR convolution and dense LRA oracle; generated signals only.

Fixed tolerances: FIR amplitude 1e-13 + 1e-11*expected; dB 1e-8;
exact gate counts, percentiles within .00500001 LU and range within .01000001 LU.
EBU-style peak tones: -0.4/+0.2 dB; long LRA level steps: +/-1 LU.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import wave
import numpy as np
from scipy.signal import lfilter

ROOT = Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--case-pattern',default='',help='Only compare case names containing this text')
args=parser.parse_args()
AUDIO = ROOT/'corpus/local/generated/listening-levels-v14'
OUT = ROOT/'corpus/local/results/listening-levels-v14'
AUDIO.mkdir(parents=True, exist_ok=True)
OUT.mkdir(parents=True, exist_ok=True)
# BS.1770 Annex 2 table, kept independent of the Rust source. Full convolution
# of each phase is a different implementation of the streaming FIR history.
H = np.array([
    [.001708984375,-.0291748046875,-.0189208984375,-.00830078125],
    [.010986328125,.029296875,.0330810546875,.014892578125],
    [-.0196533203125,-.0517578125,-.0582275390625,-.026611328125],
    [.033203125,.089111328125,.1015625,.047607421875],
    [-.0594482421875,-.16650390625,-.2003173828125,-.102294921875],
    [.1373291015625,.465087890625,.77978515625,.97216796875],
    [.97216796875,.77978515625,.465087890625,.1373291015625],
    [-.102294921875,-.2003173828125,-.16650390625,-.0594482421875],
    [.047607421875,.1015625,.089111328125,.033203125],
    [-.026611328125,-.0582275390625,-.0517578125,-.0196533203125],
    [.014892578125,.0330810546875,.029296875,.010986328125],
    [-.00830078125,-.0189208984375,-.0291748046875,.001708984375]])


def immutable(path, expected):
    text = json.dumps(expected,indent=2,allow_nan=False)+'\n'
    if path.exists(): assert path.read_text() == text, ('changed oracle',path)
    else: path.write_text(text)


def save(name,x,rate):
    if x.ndim == 1: x=x[:,None]
    words=np.rint(x*2**23).astype(np.int32)
    assert np.max(np.abs(words)) < 2**23
    raw=((words[...,None]>>(8*np.arange(3)))&255).astype(np.uint8).tobytes()
    path=AUDIO/(name+'.wav')
    if path.exists():
        with wave.open(str(path),'rb') as w:
            assert w.getframerate()==rate and w.getnchannels()==x.shape[1]
            assert w.readframes(w.getnframes())==raw
    else:
        with wave.open(str(path),'wb') as w:
            w.setparams((x.shape[1],3,rate,0,'NONE','not compressed')); w.writeframes(raw)
    return path


def tp(x):
    sample=float(np.max(np.abs(x)))
    interpolated=float(max(np.max(np.abs(np.convolve(x,h))) for h in H.T))
    peak=max(sample,interpolated)
    return dict(sample_peak=sample,interpolated_peak=interpolated,estimated_peak=peak,
                estimated_peak_dbtp=float(20*np.log10(peak)) if peak>0 else None)


def lra(x,rate):
    if rate!=48000: return None
    # Independent published 48 kHz coefficients and full-array filtering.
    y=lfilter([1.53512485958697,-2.69169618940638,1.19839281085285],
              [1.,-1.69065929318241,.73248077421585],x,axis=0)
    y=lfilter([1.,-2.,1.],[1.,-1.99004745483398,.99007225036621],y,axis=0)
    power=np.sum(y*y,axis=1)
    powers=np.array([power[i:i+144000].mean() for i in range(0,len(x)-144000+1,4800)])
    absolute=powers[powers>=10**((-70+.691)/10)]
    gate=absolute.mean()*.01 if len(absolute) else None
    gated=absolute[absolute>=gate] if gate is not None else np.array([])
    levels=np.sort(-.691+10*np.log10(gated))
    low=high=width=None
    if len(gated)>=2:
        low=float(levels[int(np.floor((len(gated)-1)*.1+.5))])
        high=float(levels[int(np.floor((len(gated)-1)*.95+.5))])
        width=high-low
    return dict(absolute_gated_windows=len(absolute),relative_gated_windows=len(gated),
        relative_gate_lufs=float(-.691+10*np.log10(gate)) if gate is not None else None,
        p10_lufs=low,p95_lufs=high,range_lu=width,
        status='measured' if width is not None else 'inconclusive',
        interval=dict(start_frame=0,end_frame=len(x)//4800*4800) if len(powers) else None)


def tone(rate,seconds,divisor,phase,amplitude=.5):
    n=round(rate*seconds)
    fade=np.minimum(1,np.minimum(np.arange(n),np.arange(n)[::-1])/(rate*.01))
    return amplitude*fade*np.sin(2*np.pi*np.arange(n)/divisor+np.deg2rad(phase))


# Paths only are retained; large reference arrays are processed one file at a time.
cases=[]
for rate in [8000,11025,44100,48000,96000,192000,384000]:
    x=tone(rate,.12,4,45)
    cases.append((save('rate_'+str(rate),np.c_[x,-x*.5],rate),None,None,None))
for divisor,phase,gain in [(4,0,.5),(4,45,.5),(6,60,.5),(8,67.5,.5),(4,45,1.41)]:
    x=tone(48000,.12,divisor,phase,gain)
    cases.append((save(f'peak_{divisor}_{phase}_{gain}',x,48000),None,gain,None))
for name,x in [('silence',np.zeros(2000)),('one_sample',np.array([.5])),
               ('last_impulse',np.r_[np.zeros(5000),.5]),('dc',np.full(2000,.2)),
               ('alternating',np.tile([.4,.4,-.4,-.4],600)),
               ('noise',np.random.default_rng(20261002).uniform(-.7,.7,(10000,2)))]:
    cases.append((save(name,x,48000),None,None,None))
base=save('prefix',np.r_[np.zeros(300),[.4,.4,-.4,-.4],np.full(100,.9)],48000)
cases.append((base,304,None,None))
for index,levels in enumerate([[-20,-30],[-20,-15],[-40,-20],[-50,-35,-20,-35,-50]]):
    waveforms=[10**(db/20)*np.sin(2*np.pi*1000*np.arange(48000*20)/48000) for db in levels]
    x=np.concatenate(waveforms)
    path=save('lra_steps_'+str(index),np.c_[x,-x],48000)
    cases.append((path,None,None,[10,5,20,15][index]))
    if index==0:
        for frames in [143999,144000,148799,148800,960000]:
            cases.append((path,frames,None,None))
        cases.append((save('lra_gain',np.c_[x*.5,-x*.5],48000),None,None,10))
for name, x in [('lra_silence',np.zeros(48000*4)),
                ('lra_sub_gate',1e-6*np.sin(2*np.pi*1000*np.arange(48000*4)/48000))]:
    cases.append((save(name,x,48000),None,None,None))
rng=np.random.default_rng(20261003)
t=np.arange(48000*12)/48000
envelope=np.where(t<4,.1,np.where(t<8,.015,.065))*(1+.7*np.sin(2*np.pi*.13*t))
x=rng.uniform(-1,1,(len(t),2))*envelope[:,None]
cases.append((save('varying_noise',x,48000),None,None,None))
cases.append((save('varying_noise_gain',x*.5,48000),None,None,None))

summary=[]
max_tp=max_range=max_percentile=max_gate=0.
for path,frames,peak_target,range_target in cases:
    name=path.stem+(f'_prefix_{frames}' if frames is not None else '')
    if args.case_pattern not in name: continue
    with wave.open(str(path),'rb') as w:
        rate=w.getframerate(); channels=w.getnchannels()
    raw=subprocess.check_output(['ffmpeg','-nostdin','-v','error','-i',str(path),'-c:a','pcm_s32le','-f','s32le','pipe:1'])
    if frames is not None: raw=raw[:frames*channels*4]
    x=np.frombuffer(raw,dtype='<i4').reshape(-1,channels).astype(float)/2**31
    expected=dict(true_peak=[tp(x[:,c]) for c in range(channels)],range=lra(x,rate))
    immutable(OUT/(name+'.expected.json'),expected)
    command=[str(ROOT/'target/release/audio-forensic.exe'),'--json']
    if frames is not None: command+=['--max-seconds',str((frames+.1)/rate)]
    actual=json.loads(subprocess.check_output(command+[str(path)]))[0]
    (OUT/(name+'.actual.json')).write_text(json.dumps(actual,indent=2))
    assert actual['status']=='analyzed',actual['diagnostics']
    assert actual['ancestry_verdict']=='INCONCLUSIVE' and actual['evidence_index'] is None
    assert hashlib.sha256(raw).hexdigest()==actual['coverage']['decoded_pcm_sha256']
    for c,expected_peak in enumerate(expected['true_peak']):
        a=actual['true_peak'][c]
        assert a['interval']==dict(start_frame=0,end_frame=len(x))
        assert a['oversampled_rate_hz']==rate*4 and a['zero_padding_frames']==11
        for k,v in expected_peak.items():
            if v is None: assert a[k] is None
            else:
                error=abs(a[k]-v)
                assert error<=(1e-8 if k.endswith('dbtp') else 1e-13+1e-11*abs(v)),(name,k,a[k],v)
                if not k.endswith('dbtp'): max_tp=max(max_tp,error)
    if expected['range'] is not None:
        a=actual['loudness']['range']
        for k,v in expected['range'].items():
            if isinstance(v,float):
                error=abs(a[k]-v)
                if k=='range_lu':
                    assert a['range_lower_lu']-1e-10<=v<=a['range_upper_lu']+1e-10
                    max_range=max(max_range,error); tolerance=.01000001
                elif k in ['p10_lufs','p95_lufs']: max_percentile=max(max_percentile,error); tolerance=.00500001
                else: max_gate=max(max_gate,error); tolerance=1e-6
                assert error<=tolerance,(name,k,a[k],v)
            else: assert a[k]==v,(name,k,a[k],v)
    ff=None
    if peak_target is not None or range_target is not None:
        log=subprocess.run(['ffmpeg','-nostdin','-i',str(path),'-af','ebur128=peak=true','-f','null','-'],capture_output=True,text=True,check=True).stderr
        (OUT/(name+'.ffmpeg.log')).write_text(log)
        ff={}
        if peak_target is not None:
            expected_db=20*np.log10(peak_target)
            ff['true_peak_dbtp']=float(re.findall(r'Peak:\s*([-\d.]+) dBFS',log)[-1])
            for reading in [ff['true_peak_dbtp'],actual['true_peak'][0]['estimated_peak_dbtp']]:
                assert -.4<=reading-expected_db<=.2,(name,reading,expected_db)
        if range_target is not None:
            ff['range_lu']=float(re.findall(r'LRA:\s*([-\d.]+) LU',log)[-1])
            for reading in [ff['range_lu'],actual['loudness']['range']['range_lu']]:
                assert abs(reading-range_target)<=1,(name,reading,range_target)
    summary.append(dict(case=name,pcm_exact=True,ffmpeg=ff))
    print(name,'PASS',flush=True)
result=dict(cases=summary,max_peak_error=max_tp,max_range_error_lu=max_range,
            max_percentile_error_lu=max_percentile,max_gate_error_lu=max_gate)
suffix='-'+re.sub(r'[^a-zA-Z0-9_-]','_',args.case_pattern) if args.case_pattern else ''
(OUT/f'summary{suffix}.json').write_text(json.dumps(result,indent=2))
print({k:v for k,v in result.items() if k!='cases'},len(summary),'cases passed')
