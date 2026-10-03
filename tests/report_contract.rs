use audio_forensic::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus, analyze_source,
};
use sha2::{Digest, Sha256};
use std::{fs::OpenOptions, io::Cursor, io::Write, path::Path, time::Duration};

fn wav(format: u16, bits: u16, channels: u16, rate: u32, data: &[u8]) -> Vec<u8> {
    let alignment = channels * (bits / 8);
    let mut bytes = Vec::with_capacity(44 + data.len());
    bytes.extend(b"RIFF");
    bytes.extend((36 + data.len() as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(format.to_le_bytes());
    bytes.extend(channels.to_le_bytes());
    bytes.extend(rate.to_le_bytes());
    bytes.extend((rate * u32::from(alignment)).to_le_bytes());
    bytes.extend(alignment.to_le_bytes());
    bytes.extend(bits.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((data.len() as u32).to_le_bytes());
    bytes.extend(data);
    bytes
}

fn run(bytes: Vec<u8>, name: &str, options: &AnalysisOptions) -> AnalysisReport {
    analyze_source(
        Box::new(Cursor::new(bytes)),
        name,
        options,
        &CancellationToken::default(),
    )
}

fn compare_roundtrip(expected: &serde_json::Value, actual: &serde_json::Value, path: &str) -> u64 {
    use serde_json::Value;
    match (expected, actual) {
        (Value::Object(left), Value::Object(right)) => {
            assert_eq!(left.len(), right.len(), "object field count at {path}");
            left.iter()
                .map(|(key, value)| {
                    compare_roundtrip(
                        value,
                        right.get(key).expect("missing report field"),
                        &format!("{path}/{key}"),
                    )
                })
                .max()
                .unwrap_or(0)
        }
        (Value::Array(left), Value::Array(right)) => {
            assert_eq!(left.len(), right.len(), "array length at {path}");
            left.iter()
                .zip(right)
                .enumerate()
                .map(|(index, (left, right))| {
                    compare_roundtrip(left, right, &format!("{path}/{index}"))
                })
                .max()
                .unwrap_or(0)
        }
        (Value::Number(left), Value::Number(right)) if left.is_f64() && right.is_f64() => {
            let left = left.as_f64().unwrap();
            let right = right.as_f64().unwrap();
            // serde_json's default decimal parser need not recover the same f64
            // bits. This contract check permits at most four adjacent values;
            // exact integer PCM hashes and integer JSON fields remain exact.
            assert!(
                left.is_finite() && right.is_finite(),
                "non-finite value at {path}"
            );
            assert_eq!(
                left.is_sign_negative(),
                right.is_sign_negative(),
                "float sign at {path}"
            );
            let distance = left.to_bits().abs_diff(right.to_bits());
            assert!(
                distance <= 4,
                "float roundtrip changed {path}: {left:?} -> {right:?}, {distance} ULP"
            );
            distance
        }
        _ => {
            assert!(
                expected == actual,
                "non-float JSON changed at {path}: {expected:?} -> {actual:?}"
            );
            0
        }
    }
}

#[test]
fn serialized_contract_covers_all_outcomes_and_null_applicability() {
    let options = AnalysisOptions::default();
    let words: Vec<i16> = (0_u32..88_200)
        .map(|index| (index.wrapping_mul(16_807).wrapping_add(3) >> 8) as i16)
        .collect();
    let data: Vec<u8> = words.iter().flat_map(|value| value.to_le_bytes()).collect();
    let integer = run(
        wav(1, 16, 2, 44_100, &data),
        "generated-stereo.wav",
        &options,
    );
    assert_eq!(integer.status, FileStatus::Analyzed);
    let hash: Sha256 = words.iter().fold(Sha256::new(), |mut hash, value| {
        hash.update((i32::from(*value) << 16).to_le_bytes());
        hash
    });
    assert_eq!(
        integer.coverage.as_ref().unwrap().decoded_pcm_sha256,
        format!("{:x}", hash.finalize())
    );

    let floats: Vec<f32> = [0.125, -0.25, 0.0625, -0.5]
        .into_iter()
        .cycle()
        .take(5_500)
        .collect();
    let data: Vec<u8> = floats
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let prefix_options = AnalysisOptions {
        max_seconds: Some(0.125),
        ..Default::default()
    };
    let float = run(
        wav(3, 32, 1, 11_025, &data),
        "float-prefix.wav",
        &prefix_options,
    );
    assert_eq!(float.status, FileStatus::Analyzed);
    assert!(!float.stream.as_ref().unwrap().integer_pcm);
    let coverage = float.coverage.as_ref().unwrap();
    assert!(!coverage.reached_end);
    assert_eq!(coverage.requested_max_seconds, Some(0.125));
    let mut hash = Sha256::new();
    for value in floats.iter().take(coverage.analyzed_frames as usize) {
        hash.update(f64::from(*value).to_le_bytes());
    }
    assert_eq!(
        coverage.decoded_pcm_sha256,
        format!("{:x}", hash.finalize())
    );

    let silence = run(
        wav(1, 16, 1, 8_000, &[0; 128]),
        "short-silence.wav",
        &options,
    );
    assert_eq!(silence.status, FileStatus::Analyzed);
    let aac = run(
        include_bytes!("fixtures/aac_256_44100.flac").to_vec(),
        "generated-aac.flac",
        &options,
    );
    assert_eq!(aac.status, FileStatus::Analyzed);
    assert!(
        aac.aac
            .iter()
            .any(|a| a.status == audio_forensic::model::DetectorStatus::Hit)
    );

    let failed = run(b"RIFF\x04\0\0\0WAVE".to_vec(), "truncated.wav", &options);
    assert_eq!(failed.status, FileStatus::Failed);
    let unsupported = run(b"ID3generated-control".to_vec(), "wrapper.flac", &options);
    assert_eq!(unsupported.status, FileStatus::Unsupported);
    let cancel = CancellationToken::default();
    cancel.cancel();
    let cancelled = analyze_source(
        Box::new(Cursor::new(Vec::<u8>::new())),
        "cancelled.wav",
        &options,
        &cancel,
    );
    assert_eq!(cancelled.status, FileStatus::Cancelled);
    let timed_out = run(
        Vec::new(),
        "deadline.wav",
        &AnalysisOptions {
            deadline: Duration::ZERO,
            ..Default::default()
        },
    );
    assert_eq!(timed_out.status, FileStatus::TimedOut);

    let reports = vec![
        integer,
        float,
        silence,
        aac,
        failed,
        unsupported,
        cancelled,
        timed_out,
    ];
    let mut max_roundtrip_ulp = 0;
    for report in &reports {
        assert_eq!(report.schema_version, audio_forensic::model::SCHEMA_VERSION);
        assert_eq!(report.policy_version, audio_forensic::model::POLICY_VERSION);
        assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
        assert!(report.evidence_index.is_none());
        let bytes = serde_json::to_vec(report).unwrap();
        let roundtrip: AnalysisReport = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(roundtrip.status, report.status);
        max_roundtrip_ulp = max_roundtrip_ulp.max(compare_roundtrip(
            &serde_json::to_value(report).unwrap(),
            &serde_json::to_value(roundtrip).unwrap(),
            "",
        ));
        let value = serde_json::to_value(report).unwrap();
        for name in [
            "stream",
            "coverage",
            "mqa",
            "loudness",
            "stereo_correlation",
            "evidence_index",
        ] {
            assert!(
                value.as_object().unwrap().contains_key(name),
                "nullable field must remain present: {name}"
            );
        }
        if report.status != FileStatus::Analyzed {
            assert!(!report.diagnostics.is_empty());
        }
    }
    println!("8 report outcomes passed; maximum f64 roundtrip distance: {max_roundtrip_ulp} ULP");
    // Optional export for the independent JSON Schema validator. Generated-only
    // receipts stay private and are never silently overwritten by a later run.
    if let Some(path) = std::env::var_os("REPORT_CONTRACT_OUTPUT") {
        let path = Path::new(&path);
        let private = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("corpus/local")
            .canonicalize()
            .unwrap();
        assert!(
            path.parent()
                .unwrap()
                .canonicalize()
                .unwrap()
                .starts_with(private)
        );
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(&serde_json::to_vec_pretty(&reports).unwrap())
            .unwrap();
        file.write_all(b"\n").unwrap();
    }
}
