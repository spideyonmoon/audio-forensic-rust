use audio_forensic::model::{DetectorStatus, TransientAnalysis};
use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_source,
};

fn run(samples: &[f64], channels: u16, rate: u32, limit: Option<f64>) -> AnalysisReport {
    let bytes = samples.len() as u32 * 8;
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((36 + bytes).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(3u16.to_le_bytes());
    wav.extend(channels.to_le_bytes());
    wav.extend(rate.to_le_bytes());
    wav.extend((rate * u32::from(channels) * 8).to_le_bytes());
    wav.extend((channels * 8).to_le_bytes());
    wav.extend(64u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(bytes.to_le_bytes());
    for &x in samples {
        wav.extend(x.to_le_bytes());
    }
    let r = analyze_source(
        Box::new(std::io::Cursor::new(wav)),
        "transients.wav",
        &AnalysisOptions {
            max_seconds: limit,
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(r.status, FileStatus::Analyzed, "{:?}", r.diagnostics);
    assert_eq!(r.ancestry_verdict, "INCONCLUSIVE");
    assert_eq!(r.evidence_index, None);
    assert!(
        r.detectors
            .iter()
            .filter(|d| d.id == "highpass_envelope_peaks")
            .all(|d| d.status == DetectorStatus::Measured
                || d.status == DetectorStatus::Inconclusive)
    );
    r
}

fn assert_events_inside(t: &TransientAnalysis) {
    let i = t.eligible_peak_interval.as_ref().unwrap();
    assert!(
        t.events
            .iter()
            .all(|e| e.frame >= i.start_frame && e.frame < i.end_frame)
    );
    assert!(
        t.events
            .windows(2)
            .all(|w| w[1].frame - w[0].frame >= u64::from(t.minimum_peak_distance_frames))
    );
}

#[test]
fn silence_and_short_coverage_distinguish_measured_zero_from_unknown() {
    for rate in [8000u32, 8001, 44100, 48000, 96000, 384000] {
        let width = (rate / 2000) | 1;
        let minimum_length = rate.div_ceil(50) + width / 2 + 2 + rate.div_ceil(2);
        for enough in [false, true] {
            let length = minimum_length as usize - usize::from(!enough);
            let r = run(&vec![0.0; length], 1, rate, None);
            let t = &r.transients[0];
            assert_eq!(t.smoothing_frames, width);
            assert_eq!(
                t.status,
                if enough {
                    DetectorStatus::Measured
                } else {
                    DetectorStatus::Inconclusive
                }
            );
            assert_eq!(t.peak_count, enough.then_some(0));
            assert_eq!(t.peaks_per_minute, enough.then_some(0.0));
            assert_eq!(t.envelope_threshold, enough.then_some(1e-4));
            assert_eq!(t.baseline_median_upper, Some(0.0));
            assert!(t.events.is_empty());
        }
    }
    let r = run(&[0.0; 10], 1, 48000, None);
    assert_eq!(
        r.transients[0].eligible_peak_interval.as_ref().map(|_| ()),
        None
    );
    assert_eq!(r.transients[0].baseline_median_upper, None);
}

#[test]
fn isolated_impulses_preserve_native_antiphase_and_silent_channels() {
    let rate = 48000usize;
    let mut mono = vec![0.0; rate];
    for position in [4800, 14400, 24000] {
        mono[position] = 0.5;
    }
    for silent in [false, true] {
        let stereo: Vec<_> = mono
            .iter()
            .flat_map(|&x| [x, if silent { 0.0 } else { -x }])
            .collect();
        let r = run(&stereo, 2, rate as u32, None);
        assert_eq!(r.transients[0].peak_count, Some(3));
        assert_eq!(r.transients[1].peak_count, Some(if silent { 0 } else { 3 }));
        assert_events_inside(&r.transients[0]);
        if !silent {
            assert_eq!(
                serde_json::to_value(&r.transients[0].events).unwrap(),
                serde_json::to_value(&r.transients[1].events).unwrap()
            );
        }
        let roundtrip: AnalysisReport =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(roundtrip.transients[0].peak_count, Some(3));
    }
}

#[test]
fn chronological_spacing_startup_and_list_cap_are_explicit() {
    let rate = 48000usize;
    for (spacing, expected) in [(240, 1), (600, 2)] {
        let mut mono = vec![0.0; rate];
        mono[0] = 0.5; // startup is excluded
        mono[4800] = 0.3;
        mono[4800 + spacing] = 0.6;
        mono[rate - 1] = 0.5; // no complete envelope/right neighbor
        let r = run(&mono, 1, rate as u32, None);
        assert_eq!(r.transients[0].peak_count, Some(expected));
        assert_events_inside(&r.transients[0]);
    }
    let mut mono = vec![0.0; rate * 5];
    for i in 0..200 {
        mono[4800 + i * 960] = 0.5;
    }
    let r = run(&mono, 1, rate as u32, None);
    let t = &r.transients[0];
    assert_eq!(t.peak_count, Some(200));
    assert_eq!(t.events.len(), 128);
    assert!(t.events_truncated);
    assert_events_inside(t);
}

#[test]
fn musical_attacks_can_produce_peaks_without_a_source_label() {
    let rate = 48000usize;
    let steady: Vec<_> = (0..2 * rate)
        .map(|i| {
            (0.6 * (std::f64::consts::TAU * 12000.0 * i as f64 / rate as f64).sin())
                .clamp(-0.2, 0.2)
        })
        .collect();
    let r = run(&steady, 1, rate as u32, None);
    assert_eq!(r.transients[0].peak_count, Some(0));
    let mut attacks = vec![0.0; 2 * rate];
    for onset in [rate / 4, rate, rate * 3 / 2] {
        for i in 0..rate / 5 {
            let t = i as f64 / rate as f64;
            attacks[onset + i] = 0.5
                * (1.0 - (-t / 0.001).exp())
                * (-t / 0.025).exp()
                * (std::f64::consts::TAU * 5000.0 * t).sin();
        }
    }
    let r = run(&attacks, 1, rate as u32, None);
    assert!(r.transients[0].peak_count.unwrap() >= 3);
    assert_events_inside(&r.transients[0]);
}

#[test]
fn requested_prefix_and_180_second_cap_exclude_later_events() {
    let mut mono = vec![0.0; 96000];
    mono[9600] = 0.5;
    mono[57600] = 0.5;
    let r = run(&mono, 1, 48000, Some(0.8));
    assert_eq!(r.transients[0].peak_count, Some(1));
    assert_eq!(r.transients[0].interval.end_frame, 38400);
    assert!(!r.coverage.unwrap().reached_end);
    let mut mono = vec![0.0; 181 * 8000];
    mono[179 * 8000] = 0.5;
    mono[180 * 8000 + 4000] = 0.5;
    let r = run(&mono, 1, 8000, None);
    assert_eq!(r.transients[0].interval.end_frame, 180 * 8000);
    assert_eq!(r.transients[0].peak_count, Some(1));
    assert_eq!(r.coverage.unwrap().analyzed_frames, 181 * 8000);
}
