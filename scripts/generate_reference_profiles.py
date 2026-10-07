"""Generated P04a/b oracle. Exclusive creation; --check never rewrites expectations."""
import sys,json,hashlib
from pathlib import Path
import numpy as np
from scipy import signal,fft
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'reference/audio-forensic'))
from audio_forensic import SpectralEngine,calculate_autocorrelation,calculate_temporal_variance,highpass_filter,bandpass_filter

def rows(rate,kind):
 a=np.empty((32,2049),np.float32)
 for t in range(32):
  for k in range(2049):
   v=np.float32(0.05+((k*17+t*31+k*t*3)%997)/997)
   hz=k*rate/4096
   if kind=='wall' and hz>16000:v*=np.float32(1e-6)
   if kind=='notch' and abs(hz-22050)<350:v*=np.float32(1e-4)
   if kind=='constant':v=np.float32(1)
   a[t,k]=v
 return a

def samples(rate,seconds,kind):
 n=int(rate*seconds);i=np.arange(n,dtype=np.int64)
 # Integer deterministic PRNG; identical f32 input without trigonometric generation.
 u=((i*1664525+1013904223)&0xffffffff).astype(np.float64)/2**32-.5
 x=(u*.1).astype(np.float32)
 if kind=='zero':x[:]=0
 if kind=='events':
  x*=np.float32(.001)
  for p in [int(rate*.01),int(rate*.7),int(rate*1.5),int(rate*2.3)]:x[p:p+int(rate*.004)]=np.float32(.9)
 return x

def generate():
 out={'reference_sha256':hashlib.sha256((ROOT/'reference/audio-forensic/audio_forensic.py').read_bytes()).hexdigest(),'spectral':[],'profiles':[]}
 for rate,kind in [(8000,'constant'),(48000,'wall'),(96000,'notch'),(96000,'constant'),(192000,'wall')]:
  e=SpectralEngine(Path('generated.wav'),rate,channels=2);a=rows(rate,kind);b=e._freq_bins();cut=float(np.percentile(e._cutoff_per_frame(a,b),95));avg=a.mean(axis=0);db=20*np.log10(avg/(avg.max()+1e-12)+1e-12)
  lo,hi=int(16000/(rate/4096)),min(2049,int(min(20000,rate/2-100)/(rate/4096)))
  spec=20*np.log10(a[:,lo:hi].mean(axis=0)+1e-12) if hi>lo else np.array([]);spec-=spec.mean() if len(spec) else 0
  den=float(np.sum(spec**2))+1e-12
  lags=[float(np.sum(spec[:-l]*spec[l:]))/den if 0<l<len(spec)-1 else None for l in [64,128,192,61,67,125,131,189,195]]
  out['spectral'].append(dict(rate=rate,kind=kind,cutoff=cut,banding=e._banding_score(a,b,cut),lpf=list(e._lpf_scan(a,b)),dsd=bool(e._dsd_scan(a,b)),resample=e._resample_check(a,b),sparsity=e._spectral_sparsity(a,b,cut),envelope=e._ultrasonic_envelope_correlation(a,b),lags=lags))
 for rate,seconds,kind in [(8000,3.2,'noise'),(48000,3.2,'noise'),(48000,3.2,'events'),(96000,3.2,'noise'),(8000,61,'noise'),(8000,181,'noise'),(48000,3.2,'zero')]:
  x=samples(rate,seconds,kind);e=SpectralEngine(Path('generated.wav'),rate,channels=1)
  c=x[:rate*180];lo,hi=rate*.30,rate*.45;y=e._fft_band_extract(c,lo,hi)
  lag=max(25,round(50*rate/44100));rms=float(np.sqrt(np.mean(y*y)))
  env=e._smooth_envelope(c,.001);peaks,_=signal.find_peaks(env,height=10**(-3/20),distance=int(.05*rate))
  hp=highpass_filter(c,1000,rate);smooth=e._smooth_envelope(hp,.0005);clicks,_=signal.find_peaks(smooth,height=float(np.median(smooth))*3,distance=int(.01*rate))
  pre=None;baseline=None;affected=eligible=0
  if rate/2-100>10000:
   hf=bandpass_filter(c,10000,min(20000,rate/2-100),rate);baseline=float(np.median(hf**2))
   for p in peaks:
    if p<int(.03*rate):continue
    eligible+=1
    affected+=float(np.mean(hf[p-int(.02*rate):p-int(.01*rate)]**2))>baseline*3
   if eligible:pre=100*affected/eligible
  out['profiles'].append(dict(rate=rate,seconds=seconds,kind=kind,frames=len(c),fft_length=fft.next_fast_len(len(c)),band=[lo,hi],rms_db=20*np.log10(rms+1e-12),std_db=20*np.log10(float(np.std(y))+1e-12),autocorrelation=calculate_autocorrelation(y,lag),temporal_std=calculate_temporal_variance(y,rate),peaks=peaks.tolist(),clicks=len(clicks),baseline=baseline,affected=int(affected),eligible=eligible,preceding_pct=pre))
 return out
if __name__=='__main__':
 p=ROOT/'tests/fixtures/reference_profiles.json';s=json.dumps(generate(),indent=2,allow_nan=False)+'\n'
 if '--check' in sys.argv:assert p.read_text()==s
 else:p.open('x').write(s)
 print('PASS frozen P04a/b oracle')
