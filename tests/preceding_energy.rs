use audio_forensic::model::DetectorStatus;
use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_source,
};

fn run(samples: &[f64], rate: u32, limit: Option<f64>) -> AnalysisReport {
    let bytes = samples.len() as u32 * 8;
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((36 + bytes).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(3u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(rate.to_le_bytes());
    wav.extend((rate * 8).to_le_bytes());
    wav.extend(8u16.to_le_bytes());
    wav.extend(64u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(bytes.to_le_bytes());
    for &x in samples {
        wav.extend(x.to_le_bytes());
    }
    let r = analyze_source(
        Box::new(std::io::Cursor::new(wav)),
        "preceding-energy.wav",
        &AnalysisOptions {
            max_seconds: limit,
            ..Default::default()
        },
        &CancellationToken::default(),
    );
    assert_eq!(r.status, FileStatus::Analyzed, "{:?}", r.diagnostics);
    assert_eq!(r.ancestry_verdict, "INCONCLUSIVE");
    assert_eq!(r.evidence_index, None);
    r
}

#[test]
fn isolated_impulses_have_complete_zero_preceding_context() {
    let mut x = vec![0.; 96000];
    for i in [4800, 19200, 57600, 81600] {
        x[i] = 0.5;
    }
    let r = run(&x, 48000, None);
    let e = &r.preceding_energy[0];
    assert_eq!(e.status, DetectorStatus::Measured);
    assert_eq!(e.selected_peak_count, r.transients[0].peak_count);
    assert_eq!(e.eligible_event_count, 4);
    assert_eq!(e.above_baseline_count, Some(0));
    assert_eq!(e.above_baseline_fraction, Some(0.));
    for event in &e.events {
        assert_eq!(event.status, DetectorStatus::Measured);
        assert!(event.mean_square.unwrap() < 1e-12);
        let interval = event.interval.as_ref().unwrap();
        assert_eq!(interval.start_frame, event.frame - 960);
        assert_eq!(interval.end_frame, event.frame - 480);
    }
}

#[test]
fn musical_attack_and_prior_tone_energy_can_exceed_baseline_without_codec_history() {
    let rate = 48000;
    let mut x = vec![0.; 2 * rate];
    for i in 0..rate / 5 {
        let t = i as f64 / rate as f64;
        x[rate / 2 + i] = 0.5
            * (1.0 - (-t / 0.001).exp())
            * (-t / 0.025).exp()
            * (std::f64::consts::TAU * 12000.0 * t).sin();
    }
    let r = run(&x, rate as u32, None);
    let e = &r.preceding_energy[0];
    assert_eq!(e.status, DetectorStatus::Measured);
    assert!(e.above_baseline_count.unwrap() > 0);
    assert!(e.above_baseline_fraction.unwrap() > 0.0);
    assert_eq!(e.selected_peak_count, r.transients[0].peak_count);
    assert!(
        r.detectors
            .iter()
            .find(|d| d.id == "preceding_event_band_energy")
            .unwrap()
            .caveats
            .iter()
            .any(|c| c.contains("musical energy"))
    );
}

#[test]
fn full_band_minimum_coverage_and_zero_events_are_explicit() {
    for rate in [8000, 40000, 40001, 44100, 48000, 96000, 384000] {
        let mut x = vec![0.; rate as usize];
        x[rate as usize / 5] = 0.5;
        let r = run(&x, rate, None);
        let e = &r.preceding_energy[0];
        assert_eq!(
            e.status,
            if rate > 40000 {
                DetectorStatus::Measured
            } else {
                DetectorStatus::Unsupported
            }
        );
        assert_eq!(e.selected_peak_count, Some(1));
        assert_eq!(
            e.above_baseline_fraction,
            if rate > 40000 { Some(0.) } else { None }
        );
        assert_eq!(
            e.history_frames,
            if rate > 40000 {
                u64::from(rate).div_ceil(4)
            } else {
                0
            }
        );
    }
    for frames in [100, 48000] {
        let r = run(&vec![0.; frames], 48000, None);
        let e = &r.preceding_energy[0];
        assert_eq!(e.status, DetectorStatus::Inconclusive);
        assert_eq!(e.above_baseline_fraction, None);
        assert_eq!(
            e.selected_peak_count,
            if frames == 100 { None } else { Some(0) }
        );
    }
}

#[test]
fn prefix_baseline_and_windows_do_not_use_later_content() {
    let mut x = vec![0.; 96000];
    x[1200] = 0.5;
    x[12000] = 0.5;
    x[48000] = 0.5;
    let r = run(&x, 48000, Some(0.8));
    let e = &r.preceding_energy[0];
    assert_eq!(e.interval.end_frame, 38400);
    assert_eq!(e.baseline_samples, 38400 - 960);
    assert!(e.startup_ineligible_count > 0);
    assert_eq!(
        e.eligible_event_count + e.startup_ineligible_count,
        e.selected_peak_count.unwrap()
    );
    x[38400..].fill(16.0);
    let b = run(&x, 48000, Some(0.8));
    assert_eq!(
        r.coverage.as_ref().unwrap().decoded_pcm_sha256,
        b.coverage.as_ref().unwrap().decoded_pcm_sha256
    );
    assert_eq!(
        serde_json::to_value(&r.preceding_energy).unwrap(),
        serde_json::to_value(&b.preceding_energy).unwrap()
    );
}

#[test]
fn retained_events_are_capped_while_all_eligible_events_are_counted() {
    let mut x = vec![0.; 5 * 48000];
    for i in 0..200 {
        x[4800 + i * 960] = 0.5;
    }
    let r = run(&x, 48000, None);
    let e = &r.preceding_energy[0];
    assert_eq!(e.selected_peak_count, Some(200));
    assert_eq!(e.eligible_event_count, 200);
    assert_eq!(e.events.len(), 128);
    assert!(e.events_truncated);
    let rt: AnalysisReport = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(rt.preceding_energy[0].eligible_event_count, 200);
}

#[test]
fn filter_and_history_stop_at_180_seconds_with_whole_stream_coverage() {
    let rate = 44100usize;
    let mut x = vec![0.; 180 * rate + rate / 10];
    x[179 * rate + rate / 2] = 0.5;
    x[180 * rate + rate / 20] = 0.5;
    let r = run(&x, rate as u32, None);
    let e = &r.preceding_energy[0];
    assert_eq!(r.coverage.as_ref().unwrap().analyzed_frames, x.len() as u64);
    assert_eq!(e.interval.end_frame, (180 * rate) as u64);
    assert_eq!(e.selected_peak_count, Some(1));
    assert_eq!(e.eligible_event_count, 1);
}
