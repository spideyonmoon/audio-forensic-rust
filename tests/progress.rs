use audio_forensic::{
    AnalysisOptions, AnalysisProgress as P, CancellationToken, FileStatus, analyze_source,
    analyze_source_with_progress,
};
use std::{io::Cursor, time::Duration};

fn wav() -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(16036u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(16000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(16000u32.to_le_bytes());
    for i in 0..8000 {
        bytes.extend(((i * 71 % 32000) as i16 - 16000).to_le_bytes());
    }
    bytes
}

#[test]
fn notifications_preserve_report_and_exact_prefix_coverage() {
    let options = AnalysisOptions {
        max_seconds: Some(0.25),
        ..Default::default()
    };
    let plain = analyze_source(
        Box::new(Cursor::new(wav())),
        "memory",
        &options,
        &CancellationToken::default(),
    );
    let mut events = vec![];
    let report = analyze_source_with_progress(
        Box::new(Cursor::new(wav())),
        "memory",
        &options,
        &CancellationToken::default(),
        |e| events.push(e),
    );
    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        serde_json::to_value(plain).unwrap()
    );
    assert_eq!(events[0], P::WaitingForWorker);
    assert_eq!(events[1], P::ReadingMetadata);
    assert_eq!(
        events.last(),
        Some(&P::Finished {
            status: FileStatus::Analyzed
        })
    );
    let mut previous_phase = 0;
    for pass in [1, 2] {
        let frames: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                P::Decoding {
                    pass: p,
                    processed_frames,
                    expected_frames,
                } if *p == pass => {
                    assert_eq!(*expected_frames, Some(2000));
                    Some(*processed_frames)
                }
                _ => None,
            })
            .collect();
        assert_eq!(frames.first(), Some(&0));
        assert_eq!(frames.last(), Some(&2000));
        assert!(frames.windows(2).all(|w| w[0] <= w[1]));
    }
    for e in events {
        let phase = match e {
            P::WaitingForWorker => 0,
            P::ReadingMetadata => 1,
            P::Decoding { pass, .. } => pass + 1,
            P::AnalyzingDetectors => 4,
            P::Finished { .. } => 5,
            _ => unreachable!(),
        };
        assert!(phase >= previous_phase);
        previous_phase = phase;
    }
}

#[test]
fn callbacks_can_cancel_each_processing_stage_and_worker_recovers() {
    for target in [
        P::WaitingForWorker,
        P::ReadingMetadata,
        P::Decoding {
            pass: 1,
            processed_frames: 0,
            expected_frames: Some(8000),
        },
        P::Decoding {
            pass: 2,
            processed_frames: 0,
            expected_frames: Some(8000),
        },
        P::AnalyzingDetectors,
    ] {
        let cancel = CancellationToken::default();
        let mut events = vec![];
        let r = analyze_source_with_progress(
            Box::new(Cursor::new(wav())),
            "cancel",
            &AnalysisOptions::default(),
            &cancel,
            |event| {
                if event == target {
                    cancel.cancel();
                }
                events.push(event);
            },
        );
        assert_eq!(r.status, FileStatus::Cancelled, "{target:?}");
        assert!(r.coverage.is_none() && r.detectors.is_empty());
        assert_eq!(
            events.last(),
            Some(&P::Finished {
                status: FileStatus::Cancelled
            })
        );
    }
    let r = analyze_source(
        Box::new(Cursor::new(wav())),
        "recovery",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    );
    assert_eq!(r.status, FileStatus::Analyzed);
}

#[test]
fn early_failures_have_one_terminal_event_and_callback_time_counts() {
    for (bytes, options, expected) in [
        (
            wav(),
            AnalysisOptions {
                max_seconds: Some(-1.),
                ..Default::default()
            },
            FileStatus::Failed,
        ),
        (
            b"not audio".to_vec(),
            AnalysisOptions::default(),
            FileStatus::Unsupported,
        ),
        (
            wav(),
            AnalysisOptions {
                deadline: Duration::ZERO,
                ..Default::default()
            },
            FileStatus::TimedOut,
        ),
    ] {
        let mut events = vec![];
        let r = analyze_source_with_progress(
            Box::new(Cursor::new(bytes)),
            "early",
            &options,
            &CancellationToken::default(),
            |e| events.push(e),
        );
        assert_eq!(r.status, expected);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, P::Finished { .. }))
                .count(),
            1
        );
        assert_eq!(events.last(), Some(&P::Finished { status: expected }));
    }
    let r = analyze_source_with_progress(
        Box::new(Cursor::new(wav())),
        "slow callback",
        &AnalysisOptions {
            deadline: Duration::from_millis(10),
            ..Default::default()
        },
        &CancellationToken::default(),
        |e| {
            if e == P::WaitingForWorker {
                std::thread::sleep(Duration::from_millis(20));
            }
        },
    );
    assert_eq!(r.status, FileStatus::TimedOut);
}

#[test]
fn callback_panic_releases_worker() {
    let result = std::panic::catch_unwind(|| {
        analyze_source_with_progress(
            Box::new(Cursor::new(wav())),
            "panic",
            &AnalysisOptions::default(),
            &CancellationToken::default(),
            |e| {
                if e == P::ReadingMetadata {
                    panic!("host callback");
                }
            },
        )
    });
    assert!(result.is_err());
    let r = analyze_source(
        Box::new(Cursor::new(wav())),
        "recovery",
        &AnalysisOptions::default(),
        &CancellationToken::default(),
    );
    assert_eq!(r.status, FileStatus::Analyzed);
}
