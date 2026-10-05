use audio_forensic::{
    AnalysisOptions, AnalysisProgress, CancellationToken, FileStatus,
    analyze_path_with_tool_statistics, analyze_source_with_byproducts,
    analyze_source_with_tool_statistics,
};
use std::io::Cursor;

fn wav(samples: &[f64], rate: u32) -> Vec<u8> {
    let bytes = samples.len() as u32 * 8;
    let mut data = Vec::new();
    data.extend(b"RIFF");
    data.extend((bytes + 36).to_le_bytes());
    data.extend(b"WAVEfmt ");
    data.extend(16u32.to_le_bytes());
    data.extend(3u16.to_le_bytes());
    data.extend(1u16.to_le_bytes());
    data.extend(rate.to_le_bytes());
    data.extend((rate * 8).to_le_bytes());
    data.extend(8u16.to_le_bytes());
    data.extend(64u16.to_le_bytes());
    data.extend(b"data");
    data.extend(bytes.to_le_bytes());
    for x in samples {
        data.extend(x.to_le_bytes());
    }
    data
}

#[test]
fn shared_prefix_keeps_native_report_and_byproducts_identical() {
    let mut samples = vec![0.25; 8000];
    samples.extend(vec![-0.5; 8000]);
    let data = wav(&samples, 8000);
    let options = AnalysisOptions {
        max_seconds: Some(0.75),
        ..Default::default()
    };
    let old = analyze_source_with_byproducts(
        Box::new(Cursor::new(data.clone())),
        "x.wav",
        &options,
        &CancellationToken::default(),
        |_| {},
    );
    let mut events = Vec::new();
    let p = analyze_source_with_tool_statistics(
        Box::new(Cursor::new(data)),
        "x.wav",
        &options,
        &CancellationToken::default(),
        |e| events.push(e),
    );
    assert_eq!(p.measurement.status, FileStatus::Analyzed);
    assert_eq!(
        serde_json::to_value(p.measurement).unwrap(),
        serde_json::to_value(old.measurement).unwrap()
    );
    assert_eq!(
        serde_json::to_value(p.byproducts).unwrap(),
        serde_json::to_value(old.byproducts).unwrap()
    );
    let t = p.tool_statistics.measurements.unwrap();
    assert_eq!(t.sox.channel_sample_count, 6000);
    assert_eq!(t.astats.channels[0].frames, 6000);
    assert_eq!(t.astats.overall["peak_count"].value, Some(12000.0));
    assert_eq!(t.astats.overall["absolute_peak_count"].value, Some(6000.0));
    assert_eq!(
        t.legacy_loudness_profile["peak_count"].as_deref(),
        Some("6000.00")
    );
    assert_eq!(t.dr.trailing_frames, 6000);
    assert!(t.dr.overall.value.is_none());
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, AnalysisProgress::Finished { .. }))
            .count(),
        1
    );
}

#[test]
fn exact_block_multiple_and_zero_power_do_not_invent_dr() {
    for samples in [vec![0.25; 72000], vec![0.0; 72800]] {
        let p = analyze_source_with_tool_statistics(
            Box::new(Cursor::new(wav(&samples, 8000))),
            "x.wav",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
            |_| {},
        );
        assert_eq!(p.measurement.status, FileStatus::Analyzed);
        let t = p.tool_statistics.measurements.unwrap();
        assert!(t.dr.overall.value.is_none());
        assert!(t.dr.overall.reason.is_some());
        assert!(t.dr.overall_integer_label.is_none());
        assert_eq!(t.dr.legacy_python_label, "N/A");
    }
}

#[test]
fn failures_and_cancellation_during_decode_suppress_products() {
    let data = wav(&vec![0.25; 8000], 8000);
    for status in [
        FileStatus::Cancelled,
        FileStatus::TimedOut,
        FileStatus::Failed,
        FileStatus::Unsupported,
    ] {
        let cancel = CancellationToken::default();
        let options = AnalysisOptions {
            deadline: if status == FileStatus::TimedOut {
                std::time::Duration::ZERO
            } else {
                std::time::Duration::from_secs(600)
            },
            ..Default::default()
        };
        let source = if status == FileStatus::Failed {
            Vec::new()
        } else if status == FileStatus::Unsupported {
            wav(&[0.25; 100], 4000)
        } else {
            data.clone()
        };
        let p = analyze_source_with_tool_statistics(
            Box::new(Cursor::new(source)),
            "x.wav",
            &options,
            &cancel,
            |e| {
                if status == FileStatus::Cancelled && matches!(e, AnalysisProgress::Decoding { .. })
                {
                    cancel.cancel();
                }
            },
        );
        assert_eq!(p.measurement.status, status);
        assert!(p.tool_statistics.measurements.is_none());
        assert!(p.tool_statistics.coverage.is_none());
        assert!(p.byproducts.reference.is_none());
    }
    let p = analyze_path_with_tool_statistics(
        "target/p03a-tool-stats/does-not-exist.wav",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
        |_| {},
    );
    assert_eq!(p.measurement.status, FileStatus::Failed);
    assert!(p.tool_statistics.measurements.is_none());
}
