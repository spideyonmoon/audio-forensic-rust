use audio_forensic::{
    AnalysisOptions, AnalysisProgress, CancellationToken, FileStatus, analyze_path_with_byproducts,
    analyze_source, analyze_source_with_byproducts,
    byproducts::{ByproductAvailability, native_level_display},
};
use sha2::{Digest, Sha256};
use std::io::Cursor;

fn wav(samples: &[f64], channels: u16, rate: u32) -> Vec<u8> {
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
    for x in samples {
        wav.extend(x.to_le_bytes());
    }
    wav
}

#[test]
fn decoded_antiphase_preserves_native_pcm_levels_policy_and_progress() {
    let samples: Vec<_> = (0..8000)
        .flat_map(|i| {
            let a = if i % 2 == 0 { 0.5 } else { -0.5 };
            [a, -a]
        })
        .collect();
    let data = wav(&samples, 2, 8000);
    let mut events = Vec::new();
    let p = analyze_source_with_byproducts(
        Box::new(Cursor::new(data.clone())),
        "anti.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        |e| events.push(e),
    );
    assert_eq!(
        p.measurement.status,
        FileStatus::Analyzed,
        "{:?}",
        p.measurement.diagnostics
    );
    assert_eq!(p.measurement.ancestry_verdict, "INCONCLUSIVE");
    assert_eq!(p.measurement.evidence_index, None);
    assert_eq!(p.measurement.coverage.as_ref().unwrap().analysis_passes, 2);
    let expected = format!(
        "{:x}",
        Sha256::digest(
            samples
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(
        p.byproducts.coverage.as_ref().unwrap().decoded_pcm_sha256,
        expected
    );
    let b = p.byproducts.reference.as_ref().unwrap();
    assert!(b.mid_cancelled_with_native_signal);
    assert_eq!(b.silence.total_percent.value, Some(100.0));
    assert_eq!(b.phase.mean_correlation, Some(-1.0));
    let levels = p.byproducts.native_levels.as_ref().unwrap();
    assert!(levels.integrated_lufs.value.is_some());
    assert_eq!(levels.channels[0].sample_crest_db.value, Some(0.0));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, AnalysisProgress::Finished { .. }))
            .count(),
        1
    );
    assert!(matches!(
        events.last(),
        Some(AnalysisProgress::Finished {
            status: FileStatus::Analyzed
        })
    ));
    let native = analyze_source(
        Box::new(Cursor::new(data)),
        "anti.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    );
    assert_eq!(
        serde_json::to_value(&native).unwrap(),
        serde_json::to_value(&p.measurement).unwrap()
    );
    let json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["byproducts"]["byproduct_version"], 1);
    assert!(json["measurement"].get("reference").is_none());
}

#[test]
fn prefix_silence_uses_analyzed_span_and_does_not_claim_eof() {
    let samples: Vec<_> = std::iter::repeat_n(0.0, 4000)
        .chain(std::iter::repeat_n(0.25, 12000))
        .collect();
    let p = analyze_source_with_byproducts(
        Box::new(Cursor::new(wav(&samples, 1, 8000))),
        "prefix.wav",
        &AnalysisOptions {
            max_seconds: Some(1.0),
            ..Default::default()
        },
        &CancellationToken::default(),
        |_| {},
    );
    let b = p.byproducts.reference.unwrap();
    assert_eq!(b.interval.end_frame, 8000);
    assert_eq!(b.silence.total_percent.value, Some(50.0));
    assert_eq!(b.phase.availability, ByproductAvailability::Inapplicable);
    let p = analyze_source_with_byproducts(
        Box::new(Cursor::new(wav(&samples, 1, 8000))),
        "prefix.wav",
        &AnalysisOptions {
            max_seconds: Some(0.5),
            ..Default::default()
        },
        &CancellationToken::default(),
        |_| {},
    );
    let b = p.byproducts.reference.unwrap();
    assert!(!p.byproducts.coverage.unwrap().reached_end);
    assert!(b.silence.sections[0].display.ends_with(" → analyzed end"));
    assert!(b.silence.sections[0].legacy_display.ends_with(" → EOF"));
}

#[test]
fn short_silence_and_unsupported_meter_keep_nulls_and_explicit_units() {
    for (rate, count) in [(8000, 100), (8000, 8000), (11025, 11025)] {
        let p = analyze_source_with_byproducts(
            Box::new(Cursor::new(wav(&vec![0.; count], 1, rate))),
            "quiet.wav",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
            |_| {},
        );
        assert_eq!(p.measurement.status, FileStatus::Analyzed);
        let l = p.byproducts.native_levels.unwrap();
        assert_eq!(l.integrated_lufs.value, None);
        assert_eq!(l.fixed_minus16_delta_db.value, None);
        assert_eq!(l.fixed_minus14_delta_db.display, None);
        assert_eq!(l.channels[0].sample_peak_dbfs.unit, "dBFS");
        assert_eq!(l.channels[0].sample_peak_dbfs.value, None);
        assert_eq!(l.channels[0].dc_offset_amplitude.value, Some(0.0));
        assert_eq!(
            p.byproducts
                .reference
                .unwrap()
                .noise_floor_fallback
                .percentile_rms,
            None
        );
    }
}

#[test]
fn fixed_level_targets_and_ties_to_even_display_preserve_native_method() {
    let p = analyze_source_with_byproducts(
        Box::new(Cursor::new(wav(&vec![0.25; 8000], 1, 8000))),
        "dc.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        |_| {},
    );
    let mut r = p.measurement;
    for (lufs, a, s) in [
        (-18.0, "+2.0 dB", "+4.0 dB"),
        (-16.0, "+0.0 dB", "+2.0 dB"),
        (-12.0, "-4.0 dB", "-2.0 dB"),
        (-16.125, "+0.1 dB", "+2.1 dB"),
        (-16.25, "+0.2 dB", "+2.2 dB"),
        (-16.75, "+0.8 dB", "+2.8 dB"),
    ] {
        r.loudness.as_mut().unwrap().integrated_lufs = Some(lufs);
        let l = native_level_display(&r).unwrap();
        assert_eq!(l.fixed_minus16_delta_db.value, Some(-16.0 - lufs));
        assert_eq!(l.fixed_minus16_delta_db.display.as_deref(), Some(a));
        assert_eq!(l.fixed_minus14_delta_db.display.as_deref(), Some(s));
    }
    r.loudness.as_mut().unwrap().integrated_lufs = Some(f64::NAN);
    assert_eq!(
        native_level_display(&r)
            .unwrap()
            .fixed_minus16_delta_db
            .value,
        None
    );
    r.schema_version = "unknown".into();
    assert!(native_level_display(&r).is_none());
}

#[test]
fn failed_cancelled_timed_out_and_missing_inputs_never_expose_byproducts() {
    let data = wav(&vec![0.25; 8000], 1, 8000);
    let token = CancellationToken::default();
    token.cancel();
    let cancelled = analyze_source_with_byproducts(
        Box::new(Cursor::new(data.clone())),
        "x.wav",
        &AnalysisOptions::default(),
        &token,
        |_| {},
    );
    let timed = analyze_source_with_byproducts(
        Box::new(Cursor::new(data)),
        "x.wav",
        &AnalysisOptions {
            deadline: std::time::Duration::ZERO,
            ..Default::default()
        },
        &CancellationToken::default(),
        |_| {},
    );
    let failed = analyze_source_with_byproducts(
        Box::new(Cursor::new(Vec::new())),
        "x.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        |_| {},
    );
    let missing = analyze_path_with_byproducts(
        "target/p03-byproducts/does-not-exist.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        |_| {},
    );
    assert_eq!(cancelled.measurement.status, FileStatus::Cancelled);
    assert_eq!(timed.measurement.status, FileStatus::TimedOut);
    assert_eq!(missing.measurement.status, FileStatus::Failed);
    for p in [cancelled, timed, failed, missing] {
        assert!(p.byproducts.reference.is_none());
        assert!(p.byproducts.native_levels.is_none());
        assert!(p.byproducts.coverage.is_none());
        assert!(native_level_display(&p.measurement).is_none());
    }
}
