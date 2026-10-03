#![cfg(feature = "cli")]

use audio_forensic::{AnalysisReport, FileStatus};
use std::{fs, path::Path, process::Command};

fn wav() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(b"RIFF");
    bytes.extend(236_u32.to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(8_000_u32.to_le_bytes());
    bytes.extend(16_000_u32.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(200_u32.to_le_bytes());
    for i in 0..100_i16 {
        bytes.extend((i * 17 - 500).to_le_bytes());
    }
    bytes
}

fn run(paths: &[&Path], options: &[&str]) -> (i32, Vec<AnalysisReport>, Vec<u8>) {
    let output = Command::new(env!("CARGO_BIN_EXE_audio-forensic"))
        .arg("--json")
        .args(options)
        .args(paths)
        .output()
        .unwrap();
    let reports: Vec<AnalysisReport> =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "JSON output invalid: {error}; stderr {:?}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
    for report in &reports {
        assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
        assert!(report.evidence_index.is_none());
        if report.status != FileStatus::Analyzed {
            assert!(!report.diagnostics.is_empty());
            assert!(report.coverage.is_none());
            assert!(report.channels.is_empty());
        }
    }
    (output.status.code().unwrap(), reports, output.stdout)
}

#[test]
fn mixed_directory_and_file_inputs_keep_all_results_in_deterministic_order() {
    let root = tempfile::tempdir().unwrap();
    let album = root.path().join("album");
    let empty = root.path().join("empty");
    fs::create_dir(&album).unwrap();
    fs::create_dir(&empty).unwrap();
    let a = album.join("a.FLAC");
    let b = album.join("b.WAV");
    let nested = album.join("nested.wav");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("ignored.wav"), wav()).unwrap();
    // A display extension does not determine the native container format.
    fs::write(&b, wav()).unwrap();
    fs::write(&a, wav()).unwrap();
    fs::write(album.join("notes.txt"), b"not audio").unwrap();
    let malformed = root.path().join("malformed.wav");
    fs::write(&malformed, b"RIFF\x04\0\0\0WAVE").unwrap();
    let missing = root.path().join("missing.wav");
    let (code, reports, _) = run(&[&missing, &empty, &album, &malformed, &b], &[]);
    assert_eq!(code, 1);
    assert_eq!(
        reports.iter().map(|r| r.status.clone()).collect::<Vec<_>>(),
        vec![
            FileStatus::Failed,
            FileStatus::Failed,
            FileStatus::Analyzed,
            FileStatus::Analyzed,
            FileStatus::Failed,
            FileStatus::Analyzed,
        ]
    );
    assert_eq!(
        reports
            .iter()
            .map(|r| r.source.as_str())
            .collect::<Vec<_>>(),
        [&missing, &empty, &a, &b, &malformed, &b].map(|p| p.to_str().unwrap())
    );
    assert!(reports[1].diagnostics[0].message.contains("No WAV/FLAC"));
    assert_eq!(
        reports[2].coverage.as_ref().unwrap().decoded_pcm_sha256,
        reports[3].coverage.as_ref().unwrap().decoded_pcm_sha256
    );
}

#[test]
fn unsuccessful_options_remain_a_complete_json_batch() {
    let root = tempfile::tempdir().unwrap();
    let a = root.path().join("a.wav");
    let b = root.path().join("b.wav");
    fs::write(&a, wav()).unwrap();
    fs::write(&b, wav()).unwrap();
    for (options, expected) in [
        (vec!["--deadline-seconds", "0"], FileStatus::TimedOut),
        (vec!["--max-seconds=-1"], FileStatus::Failed),
        (vec!["--track-id", "99"], FileStatus::Failed),
    ] {
        let (code, reports, _) = run(&[&a, &b], &options);
        assert_eq!(code, 1);
        assert_eq!(reports.len(), 2);
        assert!(reports.iter().all(|r| r.status == expected));
    }
    let (code, reports, _) = run(&[&a, &b], &[]);
    assert_eq!(code, 0);
    assert!(reports.iter().all(|r| r.status == FileStatus::Analyzed));
}
