"""Freeze/check additional generated P04c streaming controls, never private audio.

--freeze refuses an existing oracle. --reader runs the Rust no-CLI example and
checks frozen numerical values, source intervals/hashes, nulls and schema.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

import numpy as np
from generate_reference_inputs import ROOT, features, decoded, COMMIT, SpectralEngine

ORACLE=ROOT/'tests/fixtures/reference_inputs_streaming.json'
WORK=ROOT/'target/p04c-reference-inputs/streaming'


def signals():
    _,noise,_=decoded('clip_noise.wav')
    _,wall,_=decoded('clip_wall.wav')
    # Noninteger stereo relation tests rounding before M/S (not native aliases).
    x=noise[:22050,0]
    stereo=np.column_stack((x*np.float32(.731),x*np.float32(-.239)+np.roll(x,37)*np.float32(.121)))
    yield 'unequal_stereo',44100,stereo
    # Exact source duration eligibility, random overlap, final interval and splice
    # inputs. No ancestry labels attach to these generated controls.
    yield 'segments_splice',44100,np.tile(np.concatenate((wall[:,0],noise[:,0],np.zeros_like(noise[:,0]))),3)[:,None]
    # Over 2500 STFT frames, unsupported transform rate keeps the control focused.
    n=np.arange(96_000*55,dtype=np.uint64)
    words=(n*1664525+1013904223)&np.uint64(0xffffffff)
    x=((words.astype(np.float64)/2**32-.5)*.5).astype(np.float32)
    x[2*96000:3*96000]=0
    yield 'long_active_gap',96000,x[:,None]


def write_wav(path,audio,rate):
    # IEEE-float RIFF; decoder preserves f32 exactly when widening to f64.
    import struct
    raw=audio.astype('<f4').tobytes(); channels=audio.shape[1]
    path.write_bytes(b'RIFF'+struct.pack('<I',36+len(raw))+b'WAVEfmt '+struct.pack('<IHHIIHH',16,3,channels,rate,rate*channels*4,channels*4,32)+b'data'+struct.pack('<I',len(raw))+raw)


def expected(name,rate,audio):
    mid=audio[:,0] if audio.shape[1]==1 else (audio[:,0]+audio[:,1])/np.float32(2)
    eng=SpectralEngine(Path('generated.wav'),rate,channels=audio.shape[1])
    count=int(min(36,max(9,len(mid)/rate//15)))
    wall,total,majority,segments=eng._segment_voting(mid,n_segments=count)
    return {'name':name,'rate':rate,'frames':len(audio),'channels':audio.shape[1],
        'decoded_pcm_sha256':hashlib.sha256(audio.astype('<f8').tobytes()).hexdigest(),
        'features':features(audio,rate),'segments':segments,'legacy_vote':[wall,total,majority]}


def near(a,b,tol,label):
    assert abs(a-b)<=tol,f'{label}: {a} vs {b} (tol {tol})'


def check(actual,e):
    r=actual['reference_inputs'];m=actual['measurement'];f=e['features']
    assert m['status']=='analyzed',m['diagnostics']
    assert m['ancestry_verdict']=='INCONCLUSIVE' and m['evidence_index'] is None
    assert r['decoded_pcm_sha256']==e['decoded_pcm_sha256']
    assert r['pass_pcm_sha256']==[e['decoded_pcm_sha256']]*3
    assert r['analyzed_frames']==e['frames'] and r['reached_end']
    assert r['base']['stft_frames']==f['frames'] and r['base']['active_frames']==f['active_frames']
    for key,tol in [('cutoff_p95_hz',.01),('cutoff_variance_hz2',.01),('sharpness_db_per_bin',.02),
        ('cliff_depth_db',.02),('hf_magnitude_ratio',1e-6),('entropy_bits',1e-5),('noise_above_cutoff_db',.02)]:
        near(r['base'][key]['value'],f[key],tol,key)
    sc=f['scatter'];a=r['scatter'];assert a['sampling_stride']==sc['stride'] and a['sampled_frames']==sc['sampled_frames']
    near(a['legacy_average_bound_hz'],sc['average_bound_hz'],e['rate']/4096,'scatter mean')
    near(a['legacy_histogram_mode_hz'],sc['histogram_mode_hz'],e['rate']/4096,'scatter mode')
    near(a['legacy_phase_entropy_bits'],sc['legacy_phase_entropy_bits'],.001,'phase')
    probes=[p for p in r['segments']['probes'] if p['active']]
    assert len(probes)==len(e['segments'])
    for p,s in zip(probes,e['segments']):
        off,cutoff,cliff,void,peak=s
        assert p['interval']['start_frame']==round(off*e['rate'])
        assert p['interval']['end_frame']==round(off*e['rate'])+2*e['rate']
        for key,value,tol in [('cutoff_hz',cutoff,.001),('cliff_db',cliff,.0001),('high_band_relative_db',void,.0001),('peak_dbfs',peak,1e-10)]:
            if p[key] is None:assert value==0.0,key
            else:near(p[key],value,tol,key)
    raw=r['segments']['classic_vote'];wall,total,majority=e['legacy_vote']
    if wall>=0:
        assert raw['legacy_walled_probes']==wall and raw['active_probes']==total and raw['legacy_majority']==majority



def check_profiles(actual,audio,rate):
    from audio_forensic import calculate_autocorrelation,calculate_temporal_variance
    r=actual['reference_inputs'];p=r['source'];mid=audio[:,0] if audio.shape[1]==1 else (audio[:,0]+audio[:,1])/np.float32(2)
    eng=SpectralEngine(Path('generated.wav'),rate,channels=audio.shape[1]);frames=eng._compute_frames(mid);active=frames[eng._active_frame_mask(frames)];bins=eng._freq_bins()
    s=r['spectral'];cut=r['base']['cutoff_p95_hz']['value']
    if len(active)>=4:
        if s['banding']['value'] is not None:near(s['banding']['value'],eng._banding_score(active,bins,cut),.001,'banding')
        if s['sparsity']['value'] is not None:near(s['sparsity']['value'],eng._spectral_sparsity(active,bins,cut),.001,'sparsity')
        if s['envelope_correlation']['value'] is not None:near(s['envelope_correlation']['value'],eng._ultrasonic_envelope_correlation(active,bins),.001,'envelope')
        assert s['lpf_detected']==bool(eng._lpf_scan(active,bins)[0])
        if audio.shape[1]==2 and s['side_anomaly']['value'] is not None:
            side=(audio[:,0]-audio[:,1])/np.float32(2)
            near(s['side_anomaly']['value'],eng._side_channel_anomaly(frames,side,bins),.001,'side anomaly')
    assert p['captured_interval']['end_frame']==min(len(mid),180*rate)
    assert p['cassette_profile']['interval']['end_frame']==min(len(mid),60*rate)
    assert p['effective_bits_interval']['end_frame']==min(len(mid),30*rate)
    for field,cap in [('void_profile',180),('cassette_profile',60),('vinyl_profile',180)]:
        v=p[field]
        if v['rms_dbfs']['value'] is None:continue
        y=eng._fft_band_extract(mid[:cap*rate],v['lower_hz'],v['upper_hz'])
        near(v['rms_dbfs']['value'],20*np.log10(float(np.sqrt(np.mean(y*y)))+1e-12),.05,field+' RMS')
        near(v['std_dbfs']['value'],20*np.log10(float(np.std(y))+1e-12),.05,field+' std')
        if v['absolute_lag_correlation']['value'] is not None:near(v['absolute_lag_correlation']['value'],calculate_autocorrelation(y,v['lag_frames']),.003,field+' lag')
        if v['temporal_rms_std_db']['value'] is not None:near(v['temporal_rms_std_db']['value'],calculate_temporal_variance(y,rate),.05,field+' temporal')
    assert p['quiet_profile']['complete_blocks']<=300
    assert all(v['value'] is None for v in p['effective_bits_by_channel']) # generated float WAV

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--freeze',action='store_true');parser.add_argument('--reader');args=parser.parse_args()
    WORK.mkdir(parents=True,exist_ok=True)
    if args.freeze:
        if ORACLE.exists():raise SystemExit('Refuse to overwrite frozen oracle')
        out={'reference_commit':COMMIT,'cases':[expected(*case) for case in signals()]}
        ORACLE.write_text(json.dumps(out,indent=2,allow_nan=False)+'\n',encoding='utf-8');print('Frozen streaming oracle')
    if args.reader:
        from validate_report_schema import offline_validator,validate_report
        v=offline_validator(json.loads((ROOT/'schemas/reference-inputs-2.schema.json').read_text(encoding='utf-8')))
        native=offline_validator(json.loads((ROOT/'schemas/analysis-report-0.18.0.schema.json').read_text(encoding='utf-8')))
        cases=json.loads(ORACLE.read_text(encoding='utf-8'))['cases']
        summary=[]
        for case,(name,rate,audio) in zip(cases,signals()):
            assert case['name']==name
            path=WORK/f'{name}.wav';write_wav(path,audio,rate)
            result=subprocess.run([str(Path(args.reader).resolve()),str(path)],capture_output=True,text=True,timeout=300,check=True)
            actual=json.loads(result.stdout);(WORK/f'{name}.json').write_text(result.stdout,encoding='utf-8')
            v.validate(actual);validate_report(actual['measurement'],native);check(actual,case);check_profiles(actual,audio,rate)
            summary.append({'name':name,'passed':True,'frames':len(audio),'active':actual['reference_inputs']['base']['active_frames'],
                'stride':actual['reference_inputs']['scatter']['sampling_stride']})
            # Schema rejection controls: forbid native lookalikes, wrong methods,
            # silent missing-input zeros and successful payloads on failed calls.
            from copy import deepcopy
            mutations=[]
            bad=deepcopy(actual);bad['reference_inputs']['method']='native-channel-observations';mutations.append(bad)
            bad=deepcopy(actual);bad['reference_inputs']['version']=1;mutations.append(bad)
            bad=deepcopy(actual);bad['reference_inputs']['processing_passes']=2;mutations.append(bad)
            bad=deepcopy(actual);bad['reference_inputs']['base']['cutoff_p95_hz'].update(value=None,unavailable_reason=None);mutations.append(bad)
            bad=deepcopy(actual);bad['measurement']['status']='cancelled';mutations.append(bad)
            bad=deepcopy(actual);bad['reference_inputs']['pass_pcm_sha256'].pop();mutations.append(bad)
            for bad in mutations:
                assert list(v.iter_errors(bad)), 'Invalid reference result passed schema'
            summary[-1]['schema_mutations_rejected']=len(mutations)
            print(name,'PASS',flush=True)
        (WORK/'summary.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
