"""P07 reviewed field aliases/locations; no reference audio or oracle is modified."""
import argparse,json,re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def field_map():
    fields=re.findall(r'`field:([^`]+)`', (ROOT/'PYTHON_PARITY.md').read_text(encoding='utf-8'))
    assert len(fields)==len(set(fields))==155
    result={}
    def put(prefix, mapping, domain, scope, qualification):
        for name,(paths,unit) in mapping.items():
            key=prefix+'.'+name
            assert key in fields,key
            result[key]={'value_paths':paths.split('|') if paths else [],'unit':unit,'domain':domain,
                'scope_paths':scope.split('|') if scope else [],'qualification':qualification,'status':'unavailable'}
    tags={n:('tag:'+n,'text') for n in ['title','album','date','album_artist','artist','bpm','comment_quality','comments','replaygain_track_gain','replaygain_album_gain']}
    tags['other']=('/metadata/entries','tag entries')
    put('AudioTags',tags,'metadata','','Read-only editable text; named first retained entry, duplicates remain in entries.')
    tech={'bit_rate':('declared_bit_rate_bps','bps'),'channels':('declared_channels','channels'),
        'precision':('declared_precision_bits','bits'),'sample_rate':('declared_sample_rate_hz','Hz'),
        'sample_encoding':('sample_encoding','text'),'duration':('declared_duration_seconds','s'),
        'duration_sec':('declared_duration_seconds','s'),'compression_mode':('compression_mode','text'), 'codec':('codec','text')}
    tech={n:('/metadata/technical/'+path,unit) for n,(path,unit) in tech.items()}
    tech.update(writing_library=('tag:writing_library','text'),format_profile=('tag:format_profile','text'))
    put('AudioTechnical',tech,'metadata','','Declared container fields, not decoded duration/bitrate; missing remains unavailable.')
    loud={n:('/tool_statistics/measurements/legacy_loudness_profile/'+n,unit) for n,unit in {
        'peak_db':'dBFS','rms_db':'dBFS','rms_peak_db':'dBFS','rms_trough_db':'dBFS','noise_floor_db':'dBFS',
        'dynamic_range_db':'dB','crest_factor_db':'linear ratio (legacy mislabel)','flat_factor':'dB (astats flat factor)',
        'peak_count':'count','sox_entropy':'normalized entropy','dc_offset':'normalized amplitude','zero_crossings_rate':'crossings/sample'}.items()}
    # Actual original dictionary aliases are tested against P03a; all remain
    # present in full tool_statistics even when an unavailable alias is not numeric.
    native={'lufs_integrated':('integrated_lufs','LUFS'),'lufs_range':('range_lu','LU'),
        'lufs_momentary_max':('momentary_max_lufs','LUFS'),'lufs_shortterm_max':('short_term_max_lufs','LUFS'),
        'apple_music_delta':('fixed_minus16_delta_db','dB'),'spotify_delta':('fixed_minus14_delta_db','dB')}
    loud.update({n:('/byproducts/native_levels/'+path+'/value',unit) for n,(path,unit) in native.items()})
    loud['true_peak_dbtp']=('/measurement_report/true_peak/0/estimated_peak_dbtp','dBTP')
    put('LoudnessProfile',loud,'native PCM / signed-maximum tool normalization',
        '/measurement_report/coverage|/byproducts/native_levels|/tool_statistics/coverage',
        'D04/D11: native meter and explicitly attributed tool/legacy fields; crest alias is raw linear, fixed deltas -16/-14.')
    feature={'cutoff_hz':('cutoff','Hz'),'cutoff_hz_str':('cutoff','Hz'),'cutoff_variance':('variance','Hz squared'),
        'cutoff_sharpness_db':('sharpness','dB/bin'),'cliff_depth_db':('cliff','dB relative'),'hf_energy_ratio':('hf','magnitude ratio'),
        'banding_score':('banding','ratio'),'nf_above_cutoff_db':('noise','raw FFT magnitude dB'),'side_anomaly_score':('side','ratio'),
        'entropy':('entropy','bits'),'spectral_sparsity':('sparsity','fraction'),'hf_envelope_correlation':('envelope','Pearson coefficient'),
        'preecho_pct':('preecho','percent'),'silence_ratio':('silence_ratio','ratio'),'vinyl_clicks_per_min':('clicks','candidates/minute'),
        'resample_src_rate':('resample','Hz (zero is no hit)'), 'auc_avg_bound_freq':('bound','Hz'),
        'auc_prob_bound_freq':('mode','Hz'),'auc_phase_entropy':('phase','bits'),'mdct_quant_score':('aac','ratio'),
        'vorbis_grid_score':('vorbis','ratio')}
    spectral={n:('/reference_assessment/features/'+path+'/value',unit) for n,(path,unit) in feature.items()}
    interp={'cutoff_variance_interp':'variance','cutoff_sharpness_interp':'sharpness','hf_energy_interp':'hf','banding_interp':'banding',
        'nf_interp':'noise','side_interp':'side','entropy_interp':'entropy','sparsity_interp':'sparsity',
        'hf_env_corr_interp':'envelope','auc_bound_interp':'bound','auc_phase_interp':'phase','mdct_quant_interp':'aac'}
    spectral.update({n:('/reference_assessment/legacy_outputs/interpretations/'+path,'audit text') for n,path in interp.items()})
    score={'lossy_score':'lossy','natural_score':'natural','net_score':'net','max_score':'max','raw_lossy_pct':'raw_lossy_pct','main_score':'main','heuristic_score':'heuristic'}
    spectral.update({n:('/reference_assessment/scores/'+path,'uncalibrated score' if path!='raw_lossy_pct' else 'uncalibrated percent') for n,path in score.items()})
    extra={
        'lpf_detected':('/reference_inputs/spectral/lpf_detected','boolean'), 'lpf_cutoff_str':('/reference_inputs/spectral/lpf_legacy_label','audit text'),
        'dsd_detected':('/reference_inputs/spectral/dsd_like_spectrum','boolean spectral observation; no native DSD support'),
        'net_confidence_pct':('/reference_assessment/legacy_outputs/net_confidence_pct','uncalibrated score (legacy alias, not confidence)'),
        'known_lossy_codec':('/reference_assessment/legacy_outputs/known_lossy_codec','codec text; MQA override excluded'),
        'verdict_label':('/reference_assessment/reference_label','audit label'), 'primary_verdict':('/reference_assessment/legacy_outputs/primary_verdict','audit text'),
        'evidence':('/reference_assessment/rules','rule traces; effects/states/operands'), 'natural_evidence':('/reference_assessment/rules','rule traces; effects/states/operands'),
        'caveats':('/reference_assessment/caveats','text list'), 'aliasing_corr':('/measurement_report/resampling','excluded mirroring claim; raw native observations'),
        'mp3_noise_pattern_detected':('/reference_inputs/spectral/comb_pattern','boolean observation'),
        'cassette_score':('/reference_assessment/legacy_outputs/cassette_score','uncalibrated score'),
        'vinyl_noise_detected':('/reference_assessment/source_candidates/1/matched','boolean candidate'),
        'header_duration_mismatch':('/reference_inputs/header/duration_mismatch','boolean integrity observation'),
        'header_bitrate_mismatch':('/reference_inputs/header/legacy_bitrate_mismatch','boolean integrity observation'),
        'segment_walled':('/reference_inputs/segments/classic_vote|/reference_inputs/segments/probes|/reference_inputs/segments/adaptive_wall_hz','scoped counts/votes, adaptive threshold and probes (D03)'), 'segment_total':('/reference_inputs/segments/plan/requested_probes','count'),
        'segment_wall_hz':('/reference_inputs/segments/adaptive_wall_hz/value','Hz'), 'segment_map':('/reference_inputs/segments/probes','scoped probes'),
        'codec_fingerprint':('/reference_inputs/segments/nearest_reference_wall','candidate observation'),
        'resample_detected':('/reference_inputs/spectral/ordered_resampling_hit','candidate observation'),
        'fake_hires':('/reference_inputs/source/fake_hires_bandwidth_candidate','boolean candidate'),
        'vorbis_grid_support':('/reference_inputs/vorbis_winner/supporting_probes','count'),
        'vorbis_grid_tested':('/reference_inputs/vorbis_winner/tested_probes','count'), 'vorbis_grid_channel':('/reference_inputs/vorbis_winner/basis','basis text'),
        'vorbis_grid_interp':('/reference_inputs/vorbis_winner/score','availability/value/reason'),
        'scipy_available':('/reference_inputs','native adapter availability (no SciPy runtime)'),
    }
    spectral.update(extra)
    put('SpectralAnalysis',spectral,'reference mono/mid/side; see referenced field domain',
        '/reference_inputs/basis|/reference_assessment/features',
        'Uncalibrated pinned method; raw labels audit-only; D01–D12. D05 frequency mirroring and D06 MQA certainty excluded.')
    auth={
        'spectral':('/reference_assessment','reference method object'), 'spectral_cutoff_hz':spectral['cutoff_hz'],
        'spectral_cutoff_verdict':('/reference_assessment/display_summary','qualified reference text'),
        'lpf_detected':spectral['lpf_detected'], 'lpf_cutoff_hz':('/reference_inputs/spectral/lpf_last_bin_hz/value','Hz'),
        'bit_depth_authentic':('/reference_assessment/depth_candidates|/reference_assessment/legacy_outputs/depth','qualified candidates plus audit text'),
        'phase_correlation':('/byproducts/reference/phase/mean_correlation','Pearson coefficient'), 'phase_verdict':('/byproducts/reference/phase/legacy_verdict','audit text'),
        'clipped_samples':('/byproducts/reference/ceiling_count/samples_at_or_above_threshold','channel samples'),
        'clipping_verdict':('/byproducts/reference/ceiling_count/legacy_verdict','audit text'),
        'silence_total_pct':('/byproducts/reference/silence/total_percent/value','percent analyzed frames'),
        'silence_sections':('/byproducts/reference/silence/sections','half-open native frame intervals'),
        'rg_stored':('/replaygain_audit/stored_raw','text'), 'rg_measured_lufs':('/replaygain_audit/measured_lufs','LUFS'),
        'rg_delta':('/replaygain_audit/absolute_delta_db','dB'), 'rg_verdict':('/replaygain_audit/display_summary','audit comparison'),
        'cassette_rip_detected':('/reference_assessment/source_candidates/0/matched','boolean candidate'),
        'vinyl_rip_detected':('/reference_assessment/source_candidates/1/matched','boolean candidate'),
        'mqa_detected':('/measurement_report/mqa','signalling candidates; no confirmation'),
        'mqa_metadata_claimed':('/metadata/observations/mqa_metadata_claimed','boolean editable text claim'),
        'mqa_studio':('/measurement_report/mqa/candidates/0/studio_flag','unverified payload candidate'),
        'mqa_original_sample_rate':('/measurement_report/mqa/candidates/0/rate_field_hz','Hz; unverified signalling'),
        'mqa_bit_plane':('/measurement_report/mqa/candidates/0/source_bit_plane','bit plane'),
        'mqa_sync_sample':('/measurement_report/mqa/candidates/0/sync_end_frame','native frame index'),
        'mqa_evidence':('/measurement_report/mqa','scoped signalling observations'),
        'mqa_scan_status':('/measurement_report/mqa/status','status'), 'mqa_scan_error':('/measurement_report/diagnostics','structured diagnostics'),
        'side_channel_analysis':('/reference_inputs/spectral/side_hf_ratio|/reference_inputs/spectral/side_anomaly','ratios'),
        'header_integrity':('/reference_inputs/header','integrity observations'), 'encoder_trace':('/metadata/observations','editable encoder text observations'),
    }
    put('AuthenticityReport',auth,'mixed; follow native/reference/metadata paths',
        '/measurement_report/coverage|/reference_inputs/basis|/byproducts/coverage|/replaygain_audit/loudness_interval',
        'Candidate observations only. Raw diagnoses are qualified audits; original source/depth/MQA confirmation remain unverified.')
    forensic={'filepath':('/measurement_report/source','display source'), 'tags':('/metadata/entries|/metadata/named_tags','raw/named text'),
        'technical':('/metadata/technical','declared metadata'),'sox_stats':('/tool_statistics/measurements/sox','scoped tool fields'),
        'loudness':('/byproducts/native_levels|/tool_statistics/measurements/legacy_loudness_profile','native levels and tool audit'),
        'authenticity':('/reference_assessment|/measurement_report','reference interpretation and unchanged native observations'),
        'dr_score':('/tool_statistics/measurements/dr/legacy_python_label|/tool_statistics/measurements/dr/overall_integer_label','distinct legacy/overall DR labels'),
        'spectrogram_path':('/artifacts/exports','export status/path/reason; no file existence guarantee for saved paths'),
        'analysis_seconds':('/analysis_seconds','wall-clock s; includes worker wait; no ETA')}
    put('ForensicReport',forensic,'product / declarations / native and reference PCM',
        '/measurement_report/coverage','Versioned independent product; no calibrated authenticity/provenance claim.')
    mqa={'detected':auth['mqa_detected'],'metadata_claimed':auth['mqa_metadata_claimed'], 'studio':auth['mqa_studio'],
        'original_sample_rate':auth['mqa_original_sample_rate'],'bit_plane':auth['mqa_bit_plane'],
        'sync_sample':auth['mqa_sync_sample'],'error':auth['mqa_scan_error']}
    put('MQADetection',mqa,'native integer PCM signalling / metadata',
        '/measurement_report/mqa','D06: signalling and payload candidates only; excluded certainty/100 override. Missing candidate payload remains unavailable.')
    assert set(result)==set(fields),set(fields)-set(result)
    return result

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    data=field_map();text=json.dumps(data,ensure_ascii=False,indent=2)+'\n';path=ROOT/'assets/product-field-aliases-v1.json'
    if args.check:assert path.read_text(encoding='utf-8')==text,'Alias map drift'
    else:path.write_text(text,encoding='utf-8')
    print(f'PASS: {len(data)} reviewed Python field aliases')
