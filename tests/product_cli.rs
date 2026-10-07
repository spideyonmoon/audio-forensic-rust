#![cfg(feature = "cli")]
use serde_json::Value;
use std::{fs, process::Command};
fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/clip_noise.wav")
}
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_audio-forensic"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn product_mixed_batch_saved_without_audio_and_native_json_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let audio = dir.path().join("audio.wav");
    fs::copy(fixture(), &audio).unwrap();
    let missing = dir.path().join("missing.wav");
    let result = run(&[
        "--product-json",
        "--progress",
        audio.to_str().unwrap(),
        missing.to_str().unwrap(),
    ]);
    assert_eq!(result.status.code(), Some(1));
    let p: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(p.as_array().unwrap().len(), 2);
    assert_eq!(p[0]["product_schema_version"], "audio-forensic-product-v1");
    assert_eq!(p[1]["measurement_report"]["status"], "failed");
    assert!(p[1]["reference_inputs"].is_null());
    assert!(p[1]["reference_assessment"]["scores"].is_null());
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(stderr.contains("decode pass 3/3"));
    let native = run(&["--json", audio.to_str().unwrap()]);
    assert!(native.status.success());
    let n: Value = serde_json::from_slice(&native.stdout).unwrap();
    assert_eq!(n[0], p[0]["measurement_report"]);
    let saved = dir.path().join("saved.json");
    fs::write(&saved, &result.stdout).unwrap();
    fs::remove_file(audio).unwrap();
    let result = run(&["--saved", "--product-json", saved.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(serde_json::from_slice::<Value>(&result.stdout).unwrap(), p);
    let result = run(&["--saved", saved.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(1));
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("Reference method (uncalibrated)"));
    assert!(text.contains("Batch summary"));
    assert!(text.contains("missing.wav"));
}

#[test]
fn info_directory_prefix_fast_and_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.wav");
    let b = dir.path().join("b.FLAC");
    fs::copy(fixture(), &b).unwrap();
    fs::copy(fixture(), &a).unwrap();
    fs::create_dir(dir.path().join("nested.wav")).unwrap();
    fs::write(dir.path().join("ignored.txt"), b"ignore").unwrap();
    let result = run(&["--info", "--json", dir.path().to_str().unwrap()]);
    assert!(result.status.success());
    let p: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(p.as_array().unwrap().len(), 2);
    assert_eq!(p[0]["source"], a.to_str().unwrap());
    assert_eq!(p[0]["status"], "available");
    assert!(p[0].get("measurement_report").is_none());
    let result = run(&[
        "--product-json",
        "--max-seconds",
        "0.125",
        a.to_str().unwrap(),
    ]);
    assert!(result.status.success());
    let p: Value = serde_json::from_slice(&result.stdout).unwrap();
    let rate = p[0]["measurement_report"]["stream"]["sample_rate"]
        .as_u64()
        .unwrap();
    assert_eq!(
        p[0]["measurement_report"]["coverage"]["analyzed_frames"],
        ((rate as f64 * 0.125).floor() as u64)
    );
    assert_eq!(
        p[0]["reference_inputs"]["analyzed_frames"],
        p[0]["measurement_report"]["coverage"]["analyzed_frames"]
    );
    let result = run(&["--product-json", "--fast", a.to_str().unwrap()]);
    assert!(result.status.success());
    let p: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        p[0]["measurement_report"]["coverage"]["requested_max_seconds"],
        60.
    );
    let empty = tempfile::tempdir().unwrap();
    let result = run(&["--product-json", empty.path().to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(1));
    let p: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(p[0]["measurement_report"]["status"], "failed");
}

#[test]
fn saved_export_presets_collision_and_all_unavailable_compare() {
    let dir = tempfile::tempdir().unwrap();
    let saved = dir.path().join("spectral.json");
    let png = dir.path().join("new.png");
    let result = run(&[
        "--product-json",
        "--collect-spectrogram",
        "--title",
        "Saved title",
        fixture().to_str().unwrap(),
    ]);
    assert!(result.status.success());
    fs::write(&saved, &result.stdout).unwrap();
    let result = run(&[
        "--saved",
        "--product-json",
        "--spectrogram",
        png.to_str().unwrap(),
        "--spectrogram-preset",
        "standard",
        saved.to_str().unwrap(),
    ]);
    assert!(result.status.success());
    let p: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(p[0]["artifacts"]["title"], "Saved title");
    assert_eq!(p[0]["artifacts"]["exports"][0]["status"], "available");
    let bytes = fs::read(&png).unwrap();
    assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 1600);
    assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 900);
    let result = run(&[
        "--saved",
        "--product-json",
        "--spectrogram",
        png.to_str().unwrap(),
        saved.to_str().unwrap(),
    ]);
    assert_eq!(result.status.code(), Some(1));
    let p: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(p[0]["measurement_report"]["status"], "analyzed");
    assert!(p[0]["artifacts"]["exports"][0]["path"].is_null());
    assert_eq!(bytes, fs::read(png).unwrap());
    let result = run(&[
        "--saved",
        "--compare",
        "--comparison-json",
        saved.to_str().unwrap(),
        saved.to_str().unwrap(),
    ]);
    assert!(result.status.success());
    let p: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(p["status"], "unavailable");
    assert!(p["winner_input_index"].is_null());
    let result = run(&["--compare", fixture().to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(2));
    let mut p: Value = serde_json::from_slice(&fs::read(&saved).unwrap()).unwrap();
    p[0]["product_schema_version"] = "future".into();
    fs::write(&saved, p.to_string()).unwrap();
    let result = run(&["--saved", saved.to_str().unwrap()]);
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
}
