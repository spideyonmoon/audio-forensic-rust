pub(crate) mod aac;
pub(crate) mod envelope;
pub(crate) mod mdct;
pub(crate) mod mqa;
pub(crate) mod noise;
pub(crate) mod noise_dynamics;
pub(crate) mod noise_floor;
pub(crate) mod preceding_energy;
pub(crate) mod resampling;
pub(crate) mod rolloff;
pub(crate) mod segments;
pub(crate) mod sparsity;
pub(crate) mod spectral_lags;
pub(crate) mod structure;
pub(crate) mod transients;
pub(crate) mod vorbis;

use crate::model::*;

pub(crate) fn append_observations(report: &mut AnalysisReport, rate: u32) {
    for e in &report.preceding_energy {
        let mut measurements = std::collections::BTreeMap::from([
            ("eligible_events".into(), e.eligible_event_count as f64),
            (
                "startup_ineligible_events".into(),
                e.startup_ineligible_count as f64,
            ),
            (
                "history_expired_events".into(),
                e.history_expired_count as f64,
            ),
        ]);
        if let Some(value) = e.above_baseline_fraction {
            measurements.insert("above_baseline_fraction".into(), value);
        }
        report.detectors.push(DetectorResult {
            id: "preceding_event_band_energy".into(), version: 1, family: "transient_measurements".into(),
            status: e.status.clone(), channel_index: Some(e.channel_index), intervals: vec![e.interval.clone()], measurements,
            thresholds: [("absolute_mean_square_floor".into(), preceding_energy::ABSOLUTE_POWER),
                ("median_upper_power_multiplier".into(), preceding_energy::BASELINE_MULTIPLIER)].into(),
            caveats: vec!["Measures causal 10-20 kHz filtered power in complete windows 20-10 ms before the existing high-pass envelope peak centers. Uses fourth-order high-pass followed by fourth-order low-pass, with zero initial states and 20 ms startup exclusion; timestamps are affected by causal filtering and envelope timing.".into(),
                "The baseline is a bounded 0.25 dB power histogram over the first 180 seconds, independent for each native channel. Only context-eligible events enter the fraction denominator; late-detected plateaus whose 250 ms history expired abstain explicitly. All counts include events beyond the first 128 listed.".into(),
                "Preceding musical energy, smooth attacks, noise, edits and filter timing can produce above-baseline observations. This is not a demonstrated codec pre-echo test, a probability, source label or score.".into()],
        });
    }
    for c in &report.channels {
        let mut measurements = std::collections::BTreeMap::new();
        if let Some(value) = c.crest_factor_linear {
            measurements.insert("crest_factor_linear".into(), value);
        }
        if let Some(value) = c.crest_factor_db {
            measurements.insert("crest_factor_db".into(), value);
        }
        report.detectors.push(DetectorResult {
            id: "sample_crest_factor".into(), version: 1, family: "listening_level".into(),
            status: c.crest_factor_status.clone(), channel_index: Some(c.channel_index),
            intervals: vec![AnalysisInterval { start_frame: 0, end_frame: c.samples }], measurements,
            thresholds: Default::default(),
            caveats: vec!["Native-channel sample peak divided by uncentered RMS over every sample in the shared prefix, including DC and silence. Exact silence has no ratio. Tiny amplitudes use scaled sums to avoid squaring underflow.".into(),
                "Crest factor is not true-peak crest, a DR score, a compressor/limiter detector or a mastering grade. Content, DC, fades, editing and the selected interval affect it.".into()],
        });
    }
    if let Some(s) = &report.stereo_correlation {
        let mut measurements =
            std::collections::BTreeMap::from([("paired_frames".into(), s.pair_count as f64)]);
        if let Some(value) = s.coefficient {
            measurements.insert("pearson_coefficient".into(), value);
        }
        report.detectors.push(DetectorResult {
            id: "native_stereo_correlation".into(),
            version: 1,
            family: "channel_measurements".into(),
            status: s.status.clone(),
            channel_index: None,
            intervals: s.interval.iter().cloned().collect(),
            measurements,
            thresholds: [
                ("minimum_pairs".into(), crate::stereo::MIN_PAIRS as f64),
                (
                    "minimum_channel_std_exclusive".into(),
                    crate::stereo::MIN_STD,
                ),
                (
                    "minimum_std_relative_to_peak_exclusive".into(),
                    crate::stereo::MIN_RELATIVE_STD,
                ),
            ]
            .into(),
            caveats: s.caveats.clone(),
        });
    }
    for s in &report.spectral_lags {
        let mut measurements = std::collections::BTreeMap::from([
            ("active_frames".into(), s.active_frames as f64),
            ("selected_bins".into(), s.bin_count as f64),
            ("eligible_bins".into(), s.eligible_bins as f64),
        ]);
        if let Some(value) = s.log_magnitude_std_db {
            measurements.insert("log_magnitude_std_db".into(), value);
        }
        for probe in &s.lags {
            if let Some(value) = probe.target.coefficient {
                measurements.insert(format!("lag_{}_coefficient", probe.multiple), value);
            }
        }
        report.detectors.push(DetectorResult {
            id: "high_band_spectral_lags".into(), version: 1, family: "spectral_measurements".into(),
            status: s.status.clone(), channel_index: Some(s.channel_index),
            intervals: s.interval.iter().cloned().collect(), measurements,
            thresholds: [("minimum_active_frames".into(), spectral_lags::MIN_FRAMES as f64),
                ("minimum_band_bins".into(), spectral_lags::MIN_BINS as f64),
                ("minimum_lag_pairs".into(), spectral_lags::MIN_PAIRS as f64),
                ("absolute_bin_amplitude_floor_exclusive".into(), spectral_lags::ABSOLUTE_AMPLITUDE),
                ("relative_bin_amplitude_floor_exclusive".into(), spectral_lags::RELATIVE_AMPLITUDE),
                ("minimum_log_magnitude_std_db_exclusive".into(), spectral_lags::MIN_STD_DB)].into(),
            caveats: vec!["Signed lag products of the centered 16-20 kHz mean active log-magnitude spectrum, divided by its full squared norm. This is not overlap-centered Pearson correlation or time-domain autocorrelation.".into(),
                "Native channels share the existing activity mask and strict STFT prefix boundary. Each requested separation is a multiple of rate/64; neighbours are three bins away. Incomplete bands, too few pairs, weak bins and nearly flat spectra abstain explicitly.".into(),
                "EQ, comb filtering, noise and musical tones can produce periodic spectra. These coefficients do not identify MP3, filterbank residue, aliasing or authenticity; no hit threshold or score is applied.".into()],
        });
    }
    for p in &report.true_peak {
        let mut measurements = std::collections::BTreeMap::from([
            ("sample_peak".into(), p.sample_peak),
            ("interpolated_peak".into(), p.interpolated_peak),
            ("estimated_peak".into(), p.estimated_peak),
        ]);
        if let Some(db) = p.estimated_peak_dbtp {
            measurements.insert("estimated_peak_dbtp".into(), db);
        }
        report.detectors.push(DetectorResult {
            id: "true_peak_estimate".into(),
            version: 1,
            family: "listening_level".into(),
            status: p.status.clone(),
            channel_index: Some(p.channel_index),
            intervals: vec![p.interval.clone()],
            measurements,
            thresholds: Default::default(),
            caveats: p.caveats.clone(),
        });
    }
    if let Some(l) = &report.loudness {
        let r = &l.range;
        let mut measurements = std::collections::BTreeMap::new();
        for (name, value) in [
            ("range_lu", r.range_lu),
            ("range_lower_lu", r.range_lower_lu),
            ("range_upper_lu", r.range_upper_lu),
            ("relative_gate_lufs", r.relative_gate_lufs),
        ] {
            if let Some(value) = value {
                measurements.insert(name.into(), value);
            }
        }
        report.detectors.push(DetectorResult {
            id: "short_term_loudness_range".into(),
            version: 1,
            family: "listening_level".into(),
            status: r.status.clone(),
            channel_index: None,
            intervals: r.interval.iter().cloned().collect(),
            measurements,
            thresholds: [
                ("absolute_gate_lufs".into(), -70.),
                ("relative_gate_offset_lu".into(), -20.),
                ("minimum_gated_windows".into(), 2.),
            ]
            .into(),
            caveats: r.caveats.clone(),
        });
        let mut measurements = std::collections::BTreeMap::from([
            ("complete_blocks".into(), l.complete_blocks as f64),
            (
                "absolute_gated_blocks".into(),
                l.absolute_gated_blocks as f64,
            ),
            (
                "relative_gated_blocks".into(),
                l.relative_gated_blocks as f64,
            ),
        ]);
        for (name, value) in [
            ("integrated_lufs", l.integrated_lufs),
            ("momentary_max_lufs", l.momentary_max_lufs),
            ("short_term_max_lufs", l.short_term_max_lufs),
            ("relative_gate_lufs", l.relative_gate_lufs),
        ] {
            if let Some(value) = value {
                measurements.insert(name.into(), value);
            }
        }
        report.detectors.push(DetectorResult {
            id: "programme_loudness".into(),
            version: 1,
            family: "listening_level".into(),
            status: l.status.clone(),
            channel_index: None,
            intervals: l.interval.iter().cloned().collect(),
            measurements,
            thresholds: [
                (
                    "absolute_gate_lufs".into(),
                    crate::loudness::ABSOLUTE_GATE_LUFS,
                ),
                ("relative_gate_offset_lu".into(), -10.),
            ]
            .into(),
            caveats: l.caveats.clone(),
        });
    }
    for n in &report.noise_floor {
        let mut measurements = std::collections::BTreeMap::from([
            ("nonzero_blocks".into(), n.nonzero_blocks as f64),
            ("zero_blocks".into(), n.zero_blocks as f64),
        ]);
        if let Some(value) = n.nonzero_rms_p015_dbfs {
            measurements.insert("nonzero_block_rms_p015_dbfs".into(), value);
        }
        if let Some(value) = n.high_minus_low_db {
            measurements.insert("quiet_high_minus_low_db".into(), value);
        }
        report.detectors.push(DetectorResult {
            id: "quiet_block_profile".into(), version: 1, family: "noise_measurements".into(),
            status: n.status.clone(), channel_index: Some(n.channel_index),
            intervals: n.interval.iter().cloned().collect(), measurements,
            thresholds: [("minimum_nonzero_blocks".into(), noise_floor::MIN_NONZERO as f64),
                ("maximum_complete_blocks".into(), noise_floor::MAX_BLOCKS as f64),
                ("color_absolute_mean_bin_power_floor".into(), noise_floor::ABSOLUTE_POWER),
                ("color_relative_to_total_power_floor".into(), noise_floor::RELATIVE_POWER)].into(),
            caveats: vec!["Nonzero-block RMS percentiles and quiet-block spectral color describe signal levels, not isolated noise, source bit depth, dither type, medium or authenticity.".into(),
                "Uses the first 300 complete floor(rate/10)-sample native-channel blocks within the shared prefix. All-zero blocks are counted and excluded from percentiles/selection; partial tails are discarded. Selected original indices are preserved, with chronological tie breaking.".into(),
                "Color has a separate energy-gated status. Music, DC, fades, gain, filtering, numerical leakage and quantization affect these measurements. No flatness label, source-depth verdict or score is inferred.".into()],
        });
    }
    for s in &report.sparsity {
        let mut measurements = std::collections::BTreeMap::from([
            ("eligible_frames".into(), s.eligible_frames as f64),
            (
                "below_peak_floor_frames".into(),
                s.below_peak_floor_frames as f64,
            ),
            ("selected_bins".into(), s.bin_count as f64),
        ]);
        if let Some(value) = s.fraction {
            measurements.insert("sparse_fraction".into(), value);
        }
        report.detectors.push(DetectorResult {
            id: "below_cutoff_sparsity".into(), version: 1, family: "spectral_measurements".into(),
            status: s.status.clone(), channel_index: Some(s.channel_index),
            intervals: s.interval.iter().cloned().collect(), measurements,
            thresholds: [("relative_magnitude_db_exclusive".into(), sparsity::RELATIVE_DB),
                ("absolute_frame_peak_amplitude_floor".into(), sparsity::PEAK_AMPLITUDE_FLOOR),
                ("minimum_eligible_frames".into(), sparsity::MIN_FRAMES as f64),
                ("minimum_selected_bins".into(), sparsity::MIN_BINS as f64)].into(),
            caveats: vec!["Fraction of below-threshold frame/bin observations, not a probability or proof of codec processing. Tones, spectral gaps, filtering and low-level numerical effects can create sparse spectra.".into(),
                "Uses bins strictly below the channel's global active-frame p95 cutoff, excluding DC and Nyquist. That cutoff includes active frames below this measurement's absolute peak floor. Magnitudes are compared with each eligible frame's own peak; the cutoff is not recomputed per frame.".into(),
                "Requires the shared activity gate and a peak above the coherent-amplitude floor. The interval encloses all STFT windows; counts expose active, eligible and rejected-quiet frames. No full spectrogram is retained.".into()],
        });
    }
    for e in &report.envelope {
        let mut measurements =
            std::collections::BTreeMap::from([("active_frames".into(), e.active_frames as f64)]);
        if let Some(value) = e.coefficient {
            measurements.insert("pearson_coefficient".into(), value);
        }
        report.detectors.push(DetectorResult {
            id: "band_envelope_correlation".into(), version: 1, family: "spectral_measurements".into(),
            status: e.status.clone(), channel_index: Some(e.channel_index),
            intervals: e.interval.iter().cloned().collect(), measurements,
            thresholds: [("minimum_active_frames".into(), envelope::MIN_FRAMES as f64),
                ("absolute_mean_band_power_floor".into(), envelope::ABSOLUTE_POWER),
                ("relative_mean_band_power_floor".into(), envelope::RELATIVE_POWER),
                ("absolute_envelope_std_floor".into(), envelope::MIN_STD),
                ("minimum_coefficient_of_variation".into(), envelope::MIN_CV)].into(),
            caveats: vec!["Signed Pearson correlation between 1–8 kHz and 16–22 kHz RMS envelopes across active native-channel STFT frames. Complete bands, usable energy and temporal variation are required; constant or quiet envelopes abstain.".into(),
                "Correlation is a descriptive relationship, not evidence that high-frequency content is authentic or injected. Common modulation, filtering, musical arrangement and gaps can affect it.".into(),
                "Every active frame contributes, including frames where only one band is quiet. Power gates apply to aggregate means; no per-band frame deletion is used. The interval encloses all STFT windows, not just active windows.".into()],
        });
    }
    for r in &report.rolloff {
        let mut measurements = std::collections::BTreeMap::from([
            ("active_frames".into(), r.active_frames as f64),
            (
                "lower_eligible_bins".into(),
                r.lower_band.eligible_bins as f64,
            ),
            (
                "upper_eligible_bins".into(),
                r.upper_band.eligible_bins as f64,
            ),
        ]);
        if let Some(slope) = r.slope_db_per_khz {
            measurements.insert("slope_db_per_khz".into(), slope);
        }
        report.detectors.push(DetectorResult {
            id: "spectral_rolloff".into(), version: 1, family: "spectral_measurements".into(),
            status: r.status.clone(), channel_index: Some(r.channel_index),
            intervals: r.interval.iter().cloned().collect(), measurements,
            thresholds: [("minimum_active_frames".into(), rolloff::MIN_FRAMES as f64),
                ("absolute_bin_amplitude_floor".into(), rolloff::ABSOLUTE_BIN_AMPLITUDE),
                ("relative_bin_amplitude_floor".into(), rolloff::RELATIVE_BIN_AMPLITUDE)].into(),
            caveats: vec!["Descriptive two-band spectral slope only. EQ, filters, musical spectra and processing can produce the same slope; it does not identify cassette or another source medium.".into(),
                "Uses mean log magnitude of the active mean spectrum in complete 500 Hz bands centered at 12 and 18 kHz, divided by their actual bin-center separation. This is an endpoint contrast, not a regression fit or monotonicity test.".into(),
                "Every selected bin must exceed absolute and relative amplitude floors. Amplitudes use 2*mean FFT magnitude/sum(Hann), not integrated band RMS. Only active windows contribute; the reported interval is the support envelope of all windows.".into()],
        });
    }
    for t in &report.transients {
        let mut measurements = std::collections::BTreeMap::new();
        for (name, value) in [
            ("peak_count", t.peak_count.map(|n| n as f64)),
            ("peaks_per_minute", t.peaks_per_minute),
            ("baseline_median_lower", t.baseline_median_lower),
            ("baseline_median_upper", t.baseline_median_upper),
        ] {
            if let Some(value) = value {
                measurements.insert(name.into(), value);
            }
        }
        let mut thresholds = std::collections::BTreeMap::from([
            (
                "absolute_envelope_floor".into(),
                transients::ABSOLUTE_THRESHOLD,
            ),
            ("baseline_multiplier".into(), transients::MEDIAN_MULTIPLIER),
            ("minimum_eligible_seconds".into(), 0.5),
            ("maximum_seconds".into(), transients::CAP_SECONDS as f64),
            (
                "minimum_peak_distance_samples".into(),
                t.minimum_peak_distance_frames as f64,
            ),
        ]);
        if let Some(threshold) = t.envelope_threshold {
            thresholds.insert("envelope_threshold_exclusive".into(), threshold);
        }
        report.detectors.push(DetectorResult {
            id: "highpass_envelope_peaks".into(), version: 1, family: "transient_measurements".into(),
            status: t.status.clone(), channel_index: Some(t.channel_index),
            intervals: t.eligible_peak_interval.iter().cloned().collect(), measurements, thresholds,
            caveats: vec!["Counts high-pass envelope maxima, not proven vinyl clicks. Musical attacks, percussion, clipping, edits and interference can produce the same observation.".into(),
                "Native channels use a causal fourth-order 1 kHz Butterworth high-pass, centered 0.5 ms rectified smoothing and a 20 ms startup exclusion. Peak timestamps include filter/envelope effects; they are not physical impulse onsets.".into(),
                "The threshold uses three times the conservative median upper bound from a 0.25 dB histogram, with an absolute floor. Peaks are selected chronologically with a 10 ms minimum distance, not by global peak height. Only the first 128 events are listed; counts cover the capped prefix.".into()],
        });
    }
    for n in &report.noise {
        report.detectors.push(DetectorResult {
            id: "quiet_passages".into(), version: 1, family: "noise_measurements".into(),
            status: DetectorStatus::Measured, channel_index: Some(n.channel_index),
            intervals: vec![n.interval.clone()],
            measurements: [("quiet_runs".into(), n.quiet_runs as f64),
                ("quiet_samples".into(), n.quiet_samples as f64)].into(),
            thresholds: [("absolute_sample_threshold_exclusive".into(), noise::QUIET_THRESHOLD),
                ("minimum_run_samples".into(), n.minimum_quiet_run_frames as f64),
                ("maximum_listed_intervals".into(), noise::MAX_INTERVALS as f64)].into(),
            caveats: vec!["Quiet means consecutive samples below -40 dBFS peak for at least 0.5 seconds; it is not perceptual silence. Prefix boundaries clip runs. Only the first sixteen intervals are listed; counts and band powers include every qualifying run.".into()],
        });
        for (id, band) in [
            ("high_band_power", &n.high_band),
            ("above_cutoff_band_power", &n.above_cutoff_band),
        ] {
            let mut measurements = std::collections::BTreeMap::from([
                ("stft_frames".into(), n.stft_frames as f64),
                ("quiet_stft_frames".into(), n.quiet_stft_frames as f64),
                ("bin_count".into(), band.bin_count as f64),
            ]);
            for (name, value) in [
                ("mean_square", band.mean_square),
                ("rms_dbfs", band.rms_dbfs),
                ("quiet_mean_square", band.quiet_mean_square),
                ("quiet_rms_dbfs", band.quiet_rms_dbfs),
                ("lower_bin_hz", band.lower_bin_hz),
                ("upper_bin_hz", band.upper_bin_hz),
            ] {
                if let Some(value) = value {
                    measurements.insert(name.into(), value);
                }
            }
            report.detectors.push(DetectorResult {
                id: id.into(), version: 1, family: "noise_measurements".into(),
                status: band.status.clone(), channel_index: Some(n.channel_index),
                intervals: n.stft_interval.iter().cloned().collect(), measurements,
                thresholds: [("minimum_stft_frames_per_estimate".into(), noise::MIN_BAND_FRAMES as f64),
                    ("minimum_band_bins".into(), 2.0)].into(),
                caveats: vec!["Hann-energy-normalized one-sided band power uses all framed samples, without the activity gate. Quiet power uses only windows wholly within qualifying runs; see its separate status in noise. Exact zero power has null dBFS.".into(),
                    "The bands are 16 kHz to min(22 kHz, Nyquist-100 Hz), and channel cutoff+1 kHz to Nyquist-100 Hz. At least two actual FFT bins and four frames are needed per estimate.".into(),
                    "Band power can include music, interference, quantization and window leakage. It does not identify noise origin, codec history, vinyl, cassette or authenticity; no silence ratio or score is inferred.".into()],
            });
        }
    }
    noise_dynamics::append_observations(report);
    for s in &report.spectral_structure {
        let mut measurements = std::collections::BTreeMap::from([
            ("active_frames".into(), s.active_frames as f64),
            ("bound_frames".into(), s.bound_frames as f64),
            ("flat_frames".into(), s.flat_frames as f64),
        ]);
        if let Some(value) = s.average_scatter_bound_hz {
            measurements.insert("average_scatter_bound_hz".into(), value);
        }
        if let Some(value) = s.modal_scatter_bound_hz {
            measurements.insert("modal_scatter_bound_hz".into(), value);
        }
        report.detectors.push(DetectorResult {
            id: "spectral_scatter_bound".into(), version: 1, family: "spectral_wall".into(),
            status: s.scatter_status.clone(), channel_index: Some(s.channel_index),
            intervals: s.interval.iter().cloned().collect(), measurements,
            thresholds: [("minimum_bound_frames".into(), structure::MIN_FRAMES as f64),
                ("magnitude_floor_relative_db".into(), -110.0), ("scatter_threshold_ceiling".into(), 0.6),
                ("scatter_threshold_peak_fraction".into(), 0.25), ("minimum_peak_scatter".into(), structure::MIN_SCATTER)].into(),
            caveats: vec!["Spectral scatter is a measurement, not a lossless/lossy or analog-source label. Filters, tones, noise and codecs can produce related bounds; it shares evidence with spectral walls.".into(),
                "All active native-channel STFT frames are used. Flat scatter abstains. The mode is an exact FFT-bin count, not a probability. The interval covers evaluated frames, with activity-gated gaps.".into()],
        });
        let mut measurements = std::collections::BTreeMap::from([
            ("phase_frame_pairs".into(), s.phase_frame_pairs as f64),
            ("phase_bin_pairs".into(), s.phase_bin_pairs as f64),
        ]);
        if let Some(value) = s.high_band_phase_entropy_bits {
            measurements.insert("high_band_phase_entropy_bits".into(), value);
        }
        if let Some(value) = s.phase_band_start_hz {
            measurements.insert("phase_band_start_hz".into(), value);
        }
        report.detectors.push(DetectorResult {
            id: "high_band_phase_entropy".into(), version: 1, family: "spectral_structure".into(),
            status: s.phase_status.clone(), channel_index: Some(s.channel_index),
            intervals: s.interval.iter().cloned().collect(), measurements,
            thresholds: [("minimum_frame_pairs".into(), 3.0), ("minimum_bins_per_pair".into(), 2.0),
                ("minimum_frequency_hz".into(), 10000.0), ("relative_magnitude_floor".into(), structure::PHASE_RELATIVE_MAG),
                ("absolute_fft_magnitude_floor".into(), structure::PHASE_ABSOLUTE_MAG),
                ("histogram_bins".into(), structure::PHASE_HIST_BINS as f64)].into(),
            caveats: vec!["Phase-difference entropy is a descriptive measurement, not proof of codec disruption or source authenticity. Random noise can have high entropy.".into(),
                "Only adjacent active frames and shared energetic bins at or above 10 kHz contribute. The real-valued Nyquist bin is excluded; inactive gaps are never bridged. Missing energy or unsupported bandwidth yields no entropy value.".into()],
        });
    }
    for a in &report.aac {
        let mut measurements = std::collections::BTreeMap::from([
            ("active_probes".into(), a.probes.len() as f64),
            ("eligible_probes".into(), a.eligible_probes as f64),
        ]);
        if let Some(score) = a.lattice_score {
            measurements.insert("lattice_score".into(), score);
        }
        report.detectors.push(DetectorResult {
            id: format!("aac_quantization_lattice_{}", match a.basis { AacBasis::Mono => "mono", AacBasis::Mid => "mid", AacBasis::Side => "side" }),
            version: 1, family: "transform_grid".into(), status: a.status.clone(),
            channel_index: if a.basis == AacBasis::Mono { Some(0) } else { None },
            intervals: a.probes.iter().map(|p| p.interval.clone()).collect(), measurements,
            thresholds: [
                ("maximum_search_seconds".into(), aac::CAP_SECONDS as f64),
                ("maximum_probes".into(), aac::MAX_ANCHORS as f64),
                ("minimum_active_and_eligible_probes".into(), 4.0),
                ("minimum_anchor_mean_square_s16_scale".into(), 100.0),
                ("relative_anchor_energy_floor".into(), 1e-6),
                ("minimum_band_mean_square_s16_scale".into(), 1.0),
                ("relative_band_energy_floor".into(), 1e-6),
                ("minimum_populated_band_fraction".into(), 0.5),
                ("minimum_scaled_coefficient".into(), 0.5),
                ("minimum_eligible_bands".into(), 16.0),
                ("phase_step_samples".into(), 8.0),
                ("minimum_lattice_score".into(), aac::HIT_SCORE),
            ].into(),
            caveats: vec![
                "Provisional KBD long-window quantization-lattice observation, not a calibrated probability or proof of AAC ancestry. Sparse or quiet spectra abstain.".into(),
                "Mono is native channel 0; stereo mid=(L+R)/2 and side=(L-R)/2 are computed in f64. Each basis has independently selected energy anchors; native-channel reports and PCM hashes are preserved.".into(),
                "Tests 44.1/48 kHz, 2048-sample KBD alpha=4 and phases stepped by eight. TNS, SBR, other windows, short blocks, edits and gain changes can defeat this search.".into(),
                "Only listed spans within the capped prefix are searched. The winning phase/scale is a heuristic maximum over related tests, not independent evidence.".into(),
            ],
        });
    }
    for r in &report.resampling {
        report.detectors.push(DetectorResult {
            id:"resampling_candidates".into(), version:1, family:"sample_rate_history".into(),
            status:r.status.clone(), channel_index:Some(r.channel_index),
            intervals:report.coverage.as_ref().map(|c|vec![AnalysisInterval {start_frame:0,end_frame:c.analyzed_frames}]).unwrap_or_default(),
            measurements:[("active_frames".into(),r.active_frames as f64),
                ("eligible_rate_hypotheses".into(),r.candidates.iter().filter(|c|c.eligible).count() as f64),
                ("matching_rate_hypotheses".into(),r.candidates.iter().filter(|c|!c.modes.is_empty()).count() as f64)].into(),
            thresholds:[("minimum_active_frames".into(),resampling::MIN_FRAMES as f64),
                ("minimum_edge_relative_db".into(),-80.0), ("minimum_notch_depth_db".into(),20.0),
                ("minimum_wall_drop_db".into(),40.0), ("maximum_wall_ceiling_relative_db".into(),-90.0),
                ("minimum_mirror_above_relative_db".into(),-85.0), ("minimum_mirror_correlation".into(),0.35),
                ("required_room_above_foreign_nyquist_hz".into(),1400.0)].into(),
            caveats:vec!["Notches, walls and mirrored spectra near a lower rate's Nyquist are sample-rate hypotheses, not proof of original rate, counterfeit mastering or lossy codec ancestry.".into(),
                "Native channel STFT magnitudes use the shared activity mask. Filters, spectral gaps and subsequent processing can mimic or erase these observations.".into(),
                "Every matching tested rate/mode is retained. Related wall evidence must not be counted again as independent codec evidence.".into()],
        });
    }
    for v in &report.vorbis {
        let mut measurements = std::collections::BTreeMap::from([
            ("sampled_probes".into(), v.probes.len() as f64),
            ("active_probes".into(), v.active_probes as f64),
            ("supporting_probes".into(), v.supporting_probes as f64),
        ]);
        if let Some(score) = v.zero_excess_score {
            measurements.insert("zero_excess_score".into(), score);
        }
        report.detectors.push(DetectorResult {
            id:"vorbis_transform_grid".into(),version:1,family:"transform_grid".into(),
            status:v.status.clone(),channel_index:Some(v.channel_index),
            intervals:v.probes.iter().map(|p|p.interval.clone()).collect(),measurements,
            thresholds:[("maximum_search_seconds".into(),vorbis::CAP_SECONDS as f64),
                ("maximum_probes".into(),12.0), ("minimum_active_probes".into(),4.0),
                ("minimum_anchor_mean_square".into(),1e-7), ("relative_anchor_energy_floor".into(),1e-6),
                ("relative_coefficient_zero_threshold".into(),1e-5), ("absolute_coefficient_zero_threshold".into(),1e-8),
                ("minimum_phase_excess".into(),0.03), ("minimum_baseline_multiple".into(),3.0),
                ("baseline_multiple_offset".into(),0.01), ("minimum_support_fraction".into(),0.5),
                ("minimum_zero_excess_score".into(),vorbis::HIT_SCORE)].into(),
            caveats:vec!["A persistent numerical-zero grid using the Vorbis window is a provisional codec-specific observation; its score is not a probability or unique format identification.".into(),
                "The search covers 2048/256 block geometry at 44.1/48 kHz only, on native channels and listed spans within the capped prefix.".into(),
                "Different block sizes, transitions, resampling, added noise and other processing may erase the pattern. A negative result does not establish lossless history.".into()],
        });
    }
    for s in &report.segments {
        let intervals: Vec<_> = s.probes.iter().map(|p| p.interval.clone()).collect();
        report.detectors.push(DetectorResult {
            id: "segment_wall".into(), version: 1, family: "spectral_wall".into(),
            status: s.status.clone(), channel_index: Some(s.channel_index), intervals: intervals.clone(),
            measurements: [("sampled_probes".into(), s.probes.len() as f64),
                ("active_probes".into(), s.active_probes as f64),
                ("eligible_probes".into(), s.eligible_probes as f64),
                ("wall_probes".into(), s.wall_probes as f64)].into(),
            thresholds: [("probe_seconds".into(), 2.0), ("maximum_probes".into(), segments::MAX_PROBES as f64),
                ("minimum_eligible_probes".into(), 3.0), ("minimum_peak_dbfs".into(), segments::SILENT_PEAK_DB),
                ("cutoff_relative_db".into(), -65.0), ("minimum_cliff_db".into(), segments::MIN_CLIFF_DB),
                ("maximum_above_cutoff_relative_db".into(), segments::MAX_VOID_DB),
                ("minimum_occupied_band_fraction".into(), segments::MIN_OCCUPANCY)].into(),
            caveats: vec!["Provisional gates describe spectral walls, not lossy ancestry; mastering filters can produce the same observation.".into(),
                "Only the listed two-second probes were measured. Probe counts are not duration fractions and do not locate splice boundaries.".into(),
                "Silent, narrow-band, and low-bandwidth probes cannot support this test. Absence of a wall does not establish lossless history.".into()],
        });
        let matched = s
            .probes
            .iter()
            .filter(|p| !p.codec_candidates.is_empty())
            .count();
        let status = if !matches!(rate, 44100 | 48000) {
            DetectorStatus::Unsupported
        } else if matched > 0 {
            DetectorStatus::Measured
        } else if s.eligible_probes < 3 {
            DetectorStatus::Inconclusive
        } else {
            DetectorStatus::NotDetected
        };
        report.detectors.push(DetectorResult {
            id: "codec_wall_candidates".into(), version: 1, family: "spectral_wall".into(), status,
            channel_index: Some(s.channel_index), intervals,
            measurements: [("matched_probes".into(), matched as f64)].into(), thresholds: Default::default(),
            caveats: vec!["All matching legacy Python profiles are retained with their per-profile tolerances. Overlap is expected; none identifies an encoder or bitrate.".into(),
                "Candidate tables are restricted to 44.1/48 kHz analysis; historical sample rate is unknown. Matches share the segment-wall evidence and must not be counted again.".into()],
        });
    }
    if let Some(mqa) = &report.mqa {
        report.detectors.push(DetectorResult {
            id: "mqa_signalling_candidates".into(),
            version: 1,
            family: "mqa_signalling".into(),
            status: mqa.status.clone(),
            channel_index: None,
            intervals: if mqa.scanned_frames == 0 {
                vec![]
            } else {
                vec![AnalysisInterval {
                    start_frame: 0,
                    end_frame: mqa.scanned_frames,
                }]
            },
            measurements: [
                ("sync_matches".into(), mqa.sync_matches as f64),
                ("retained_candidates".into(), mqa.candidates.len() as f64),
                (
                    "complete_retained_payloads".into(),
                    mqa.candidates.iter().filter(|c| c.payload_complete).count() as f64,
                ),
            ]
            .into(),
            thresholds: [
                ("maximum_scan_seconds".into(), 3.0),
                ("sync_word_bits".into(), 36.0),
                (
                    "maximum_retained_candidates".into(),
                    mqa::MAX_CANDIDATES as f64,
                ),
            ]
            .into(),
            caveats: mqa.caveats.clone(),
        });
    }
}
