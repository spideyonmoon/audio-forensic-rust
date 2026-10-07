"""Freeze policy oracles from pinned Python AST, without decoding/private audio.

Only DSP input assignments/calls are replaced. Original score branches, order,
vetoes, rounding and labels execute. Header/alias effects are recorded separately
and disabled for the D05/D07 comparison. Exclusive creation; --check is read-only.
"""
import ast, copy, hashlib, inspect, json, sys, textwrap
from pathlib import Path
import numpy as np
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'reference/audio-forensic'))
import audio_forensic as py

BASE=dict(rate=48000,cutoff=23000.,variance=200000.,sharpness=1.,cliff=0.,hf=.1,noise=-30.,entropy=9.,banding=.5,side=.1,dsd_spectrum=0.,comb=0.,resample=0.,resample_wall=0.,bound=22000.,phase=2.,sparsity=.01,envelope=.8,slope=-2.,void=-40.,hiss=-60.,hiss_corr=.5,vinyl_noise=-40.,vinyl_corr=.5,vinyl_variance=8.,preecho=0.,attacks=1.,clicks=0.,silence_seconds=0.,silence_ratio=.2,aac=0.,vorbis=0.,header_duration=0.,header_bitrate=0.,mqa=0.)

def function(name):
 return ast.parse(textwrap.dedent(inspect.getsource(getattr(py.SpectralEngine,name)))).body[0]

def compiled(node,env):
 node.decorator_list=[]
 ns=dict(vars(py));ns.update(env)
 exec(compile(ast.fix_missing_locations(ast.Module(body=[node],type_ignores=[])),'<pinned-policy>','exec'),ns)
 return ns[node.name]

def cassette(e,f):
 # Keep only the original scalar decision statements, including exact thresholds.
 node=function('_cassette_source')
 selected=[]
 for st in node.body:
  if isinstance(st,ast.If) and ast.unparse(st.test) in ['cutoff_hz >= 19000','-6.0 < slope < -3.0','not mp3_detected','50 < cutoff_std < 300']:
   selected.append(st)
  if isinstance(st,ast.If) and ast.unparse(st.test)=='upper_limit > noise_lo':
   selected += [x for x in st.body if isinstance(x,ast.If)]
 node.body=ast.parse('score=0\nreasons=[]\nhiss_found=False').body+selected+[node.body[-1]]
 c=f['cutoff'];geometry=min(20000,f['rate']/2-100)>c+(1000 if c<16000 else 500)
 fun=compiled(node,dict(slope=f['slope'],noise_db=f['hiss'] if geometry else -999,autocorr=f['hiss_corr']))
 comb=f['comb'] if c<21000 else 0
 return fun(e,None,None,None,c,f['variance']**.5,bool(comb))

def silence(e,f):
 node=function('_silence_and_vinyl')
 # Phase 1 scalar gate and Phase 2/3 original policy, omitting extraction.
 phase2=next(s for s in node.body if isinstance(s,ast.If) and 'noise_band is not None or' in ast.unparse(s.test))
 energy_if=next(s for s in phase2.body if isinstance(s,ast.If) and ast.unparse(s.test)=='energy_db < -70.0')
 class Clean(ast.NodeTransformer):
  def visit_Assign(self,n):
   names={x.id for t in n.targets for x in ast.walk(t) if isinstance(x,ast.Name)}
   if names & {'autocorr','variance','hp','env_smooth','peaks','_','clicks_per_min'}: return None
   return n
  def visit_Delete(self,n):return None
 energy_if=Clean().visit(energy_if)
 node.body=ast.parse('score=0\nreasons=[]\nsilence_ratio=-1.0\nvinyl_detected=False').body
 node.body+=ast.parse('if silence_seconds >= 2 and nyquist-100 >= 16000:\n silence_ratio=ratio\n if ratio > .3: return 50, [], ratio, False, 0.').body
 phase2.body=[energy_if];node.body+=[phase2,ast.parse('return score,reasons,silence_ratio,vinyl_detected,clicks_per_min').body[0]]
 c=f['cutoff'];ny=e.nyquist;shared=0<c<ny*.93 and ny-100-(c+800)>=400
 fun=compiled(node,dict(energy_db=f['vinyl_noise'],autocorr=f['vinyl_corr'],variance=f['vinyl_variance'],clicks_per_min=f['clicks'],silence_seconds=f['silence_seconds'],nyquist=ny,ratio=f['silence_ratio']))
 return fun(e,None,c,object() if shared else None,f['cliff'])

def oracle(f,probes):
 e=py.SpectralEngine(Path('generated.wav'),int(f['rate']),channels=1)
 node=function('analyse')
 start=next(i for i,s in enumerate(node.body) if isinstance(s,ast.Assign) and 'lossy_score' in ast.unparse(s.targets))
 node.body=node.body[start:]
 class Patch(ast.NodeTransformer):
  def visit_Delete(self,n):return None
  def visit_AugAssign(self,n):
   if isinstance(n.target,ast.Name) and n.target.id=='main':
    return [n,ast.parse(f'TRACE.append(({n.lineno},main))').body[0]]
   return n
  def visit_Assign(self,n):
   # Read the exact supplied filtered void instead of redoing an FFT.
   if len(n.targets)==1 and isinstance(n.targets[0],ast.Name) and n.targets[0].id=='void_db':
    n.value=ast.parse("F['void']",mode='eval').body
   n=self.generic_visit(n)
   if len(n.targets)==1 and isinstance(n.targets[0],ast.Name) and n.targets[0].id=='main':
    return [n,ast.parse(f'TRACE.append(({n.lineno},main))').body[0]]
   return n
 node=Patch().visit(node)
 e._resample_check=lambda *a:(int(f['resample']),'wall' if f['resample_wall'] else 'notch',30.) if f['resample'] else None
 e._check_header_integrity=lambda *a:(False,False,[])
 def psycho(*a):
  if f['cutoff']>=21000:return 0,[],0.,0.,False
  p=f['preecho'] if f['attacks'] else 0
  return (15 if p>10 else 10 if p>=5 else 0)+10*bool(f['comb']),[],p,0.,bool(f['comb'])
 e._psychoacoustic_artifacts=psycho
 e._cassette_source=lambda *a:cassette(e,f)
 e._fft_band_extract=lambda *a:np.ones(1)
 def segments(audio,n_segments,wall_hz):
  ps=[p for p in probes if p['eligible']]
  w=sum(p['cutoff_hz']<=wall_hz for p in ps);n=len(ps)
  return w,n,(w>0 and w>=n/2),[(p['interval']['start_frame']/f['rate'],p['cutoff_hz'],p['cliff_db'],p['high_band_relative_db'],p['peak_dbfs']) for p in ps]
 e._segment_voting=segments
 e._silence_and_vinyl=lambda *a,**kw:silence(e,f)
 e._aucdtect_features=lambda *a:(f['bound'],f['bound'],f['phase'])
 e._spectral_sparsity=lambda *a:f['sparsity']
 e._ultrasonic_envelope_correlation=lambda *a:f['envelope']
 e._mdct_quant_error=lambda *a:f['aac']
 e._vorbis_grid=lambda *a:(f['vorbis'],1,3,0)
 trace=[]
 env=dict(TRACE=trace,F=f,result=py.SpectralAnalysis(),st=lambda *a:None,audio=np.ones(10000),side=None,frames=None,phase_act=None,bins=None,cutoff_hz=f['cutoff'],cutoff_var=f['variance'],cutoff_std=f['variance']**.5,sharpness=f['sharpness'],cliff_depth=f['cliff'],hf_ratio=f['hf'],nf_above=f['noise'],banding=f['banding'],side_anomaly=f['side'],entropy=f['entropy'],dsd_detected=bool(f['dsd_spectrum']),lpf_detected=False,lpf_s='')
 r=compiled(node,env)(e)
 result=dict(lossy=r.lossy_score,natural=r.natural_score,net=r.net_score,max=r.max_score,main=r.main_score,heuristic=r.heuristic_score,raw_lossy_pct=r.raw_lossy_pct,label=r.verdict_label,text=r.primary_verdict,cassette=r.cassette_score,cassette_hiss=cassette(e,f)[2],vinyl=r.vinyl_noise_detected)
 if '--traces' in sys.argv:result['trace']=[(line+inspect.getsourcelines(py.SpectralEngine.analyse)[1]-1,value) for line,value in trace]
 return result

def probes(cut=23000,n=9):
 return [dict(interval=dict(start_frame=i*96000,end_frame=(i+1)*96000),peak_dbfs=-3.,cutoff_hz=cut,cliff_db=0.,high_band_relative_db=-20.,above_cutoff_relative_db=-20.,occupied_band_fraction=.5,active=True,eligible=True,wall_observed=False,codec_candidates=[]) for i in range(n)]

def cases():
 result=[]
 def add(name,changes,ps=None):
  f=BASE|changes;ps=probes() if ps is None else ps
  result.append(dict(name=name,features=f,probes=ps,expected=oracle(f,ps)))
 add('full_band',{})
 # Both sides and equality of all scalar score gates.
 gates={'cutoff':[16500,18500,19000,21000,22500,20400,22320,22800], 'variance':[900,1000,2500,10000,90000,100000], 'sharpness':[5,8,15], 'cliff':[20,25,30,35], 'hf':[.005,.05], 'noise':[-70,-50,-40], 'banding':[.92], 'side':[.2,.6], 'entropy':[8.5], 'slope':[-10,-6,-3], 'hiss':[-55], 'hiss_corr':[.2], 'preecho':[5,10], 'phase':[4.5], 'sparsity':[.3], 'envelope':[.15], 'aac':[.06,.10], 'vorbis':[.03], 'silence_ratio':[.15,.3], 'vinyl_noise':[-70], 'vinyl_corr':[.3], 'vinyl_variance':[5], 'clicks':[5,50], 'void':[-85,-80]}
 context=dict(cutoff=16800.,variance=40000.,sharpness=10.,cliff=36.,hf=.003,noise=-75.,bound=14000.,slope=-4.,silence_seconds=3.)
 for key,values in gates.items():
  for v in values:
   for suffix,x in [('below',float(np.nextafter(float(v),-np.inf))),('equal',float(v)),('above',float(np.nextafter(float(v),np.inf)))]:
    d=context|{key:x}
    if key.startswith('vinyl') or key=='clicks':d|=dict(vinyl_noise=-50.,vinyl_corr=.1,vinyl_variance=1.,clicks=10.,hiss=-60.,slope=-2.,cliff=0.);d[key]=x
    if key=='void':d|=dict(rate=96000,cutoff=20000.);d[key]=x
    add(f'{key}_{v}_{suffix}',d)
 add('cassette_without_hiss',context|dict(hiss=-60.))
 add('cassette_veto',context|dict(hiss=-40.,hiss_corr=.1,aac=.2,vorbis=.05,phase=5,sparsity=.5,envelope=0.,resample=44100))
 add('dsd_spectrum_variance_else',dict(dsd_spectrum=1,variance=200000))
 add('resample_wall',context|dict(resample=44100,resample_wall=1))
 for n in [1,2,4]:
  ps=probes();
  for p in ps[:n]:p.update(cutoff_hz=15400.,cliff_db=40.,high_band_relative_db=-120.)
  add(f'splice_{n}',{},ps)
 ps=probes(n=4)
 for p in ps[:2]:p.update(cutoff_hz=15000.,cliff_db=40.)
 add('even_half',context,ps)
 add('round_half_net7',dict(cutoff=18000,sharpness=16,cliff=0,hf=.001,noise=-45,side=.3,variance=5000,banding=.5))
 return result

def generate():
 # Independent execution of the complete original method verifies the harness
 # kept Phase 2, rather than accidentally selecting its preceding FFT guard.
 e=py.SpectralEngine(Path('generated.wav'),48000,channels=1)
 direct=compiled(function('_silence_and_vinyl'),dict(calculate_autocorrelation=lambda *a,**k:.1,calculate_temporal_variance=lambda *a:1.,highpass_filter=lambda x,*a:np.zeros_like(x)))
 assert direct(e,np.full(48000,.1,dtype=np.float32),16800,np.full(48000,.001,dtype=np.float32),0)[0]==-40
 assert silence(e,BASE|dict(cutoff=16800.,vinyl_noise=-60.,vinyl_corr=.1,vinyl_variance=1.,clicks=0.))[0]==-40
 depth=[]
 for claimed in [16,20,24,32]:
  for effective in [0,claimed-8,claimed-1,claimed]:
   for floor in [None,-103.,-102.,-93.,-89.,-86.,-85.]:
    for flat in [False,True]:
     p=None if floor is None else dict(floor_db=floor,flat=flat)
     depth.append(dict(claimed=claimed,effective=effective,floor=floor,flat=flat,text=py._bit_depth_verdict(claimed,effective,p)))
 return dict(reference_sha256=hashlib.sha256((ROOT/'reference/audio-forensic/audio_forensic.py').read_bytes()).hexdigest(),cases=cases(),depth=depth)

def verdict_boundaries():
 e=py.SpectralEngine(Path('generated.wav'),96000,channels=1);cases=[]
 for main in [0,10,11,30,31,54,55,85,86,100]:
  for resample,fake in [(0,False),(44100,False),(0,True)]:
   label,text,_=e._verdict(main,0,20000,False,resampled_from=resample,fake_hires='96 kHz container but bandwidth ends at 20.0 kHz' if fake else '')
   cases.append(dict(main=main,resample=resample,fake=fake,label=label,text=text))
 f=BASE|dict(cutoff=18000.,sharpness=16.,cliff=0.,hf=.001,noise=-45.,side=.3,variance=5000.,banding=.5,dsd_spectrum=1.)
 return dict(cases=cases,rounding=[round(n*45/14) for n in range(15)],net7=oracle(f,probes()))

if __name__=='__main__':
 p=ROOT/('tests/fixtures/reference_assessment_traces_v2.json' if '--traces' in sys.argv else 'tests/fixtures/reference_assessment_v2.json')
 data=verdict_boundaries() if '--verdicts' in sys.argv else generate()
 if '--verdicts' in sys.argv:p=ROOT/'tests/fixtures/reference_verdict_boundaries.json'
 if '--traces' in sys.argv:data={c['name']:c['expected']['trace'] for c in data['cases']}
 text=json.dumps(data,ensure_ascii=False,indent=2,allow_nan=False)+'\n'
 if '--check' in sys.argv:assert p.read_text(encoding='utf-8')==text
 else:p.open('x',encoding='utf-8').write(text)
 print('PASS frozen P05 Python policy oracle')
