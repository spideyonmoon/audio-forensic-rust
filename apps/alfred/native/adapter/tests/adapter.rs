use alfred_native::{
    operation::admit_product,
    storage,
    transport::{Bridge, Descriptor, Request},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};
static NEXT: AtomicU64 = AtomicU64::new(1);
fn id(n: u64) -> String {
    format!("00000000-0000-0000-0000-{n:012x}")
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "alfred-a03-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../tests/fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}
fn request(root: &Path, input: &Path, feature: Option<&str>) -> Value {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let bytes = fs::read(input).unwrap();
    json!({"version":1,"kind":if feature.is_some(){"feature"}else{"probe"},"job_id":id(n),"attempt_id":id(n),"selection_id":id(3),"generation":u64::MAX,"item_ids":[id(4)],"feature":feature,
        "leases":[{"id":id(5),"attempt_id":id(n),"path":input,"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(&bytes)),"name":input.file_name().unwrap().to_str().unwrap()}],
        "scope":{"kind":"full"},"deadline_ms":120000,"output_dir":root,"available_memory_bytes":2147483648u64,"low_memory":false,"reserved_output_bytes":134217728})
}
fn run(b: &Bridge, r: &Value) -> Value {
    let h = b.start(&serde_json::to_vec(r).unwrap()).unwrap();
    let end = Instant::now() + Duration::from_secs(120);
    loop {
        let p = b.poll(h).unwrap();
        if p["terminal"] != Value::Null {
            assert_eq!(p, b.poll(h).unwrap());
            assert_eq!(p["leases_released"], true);
            b.close(h);
            return p["terminal"].clone();
        }
        assert!(Instant::now() < end);
        thread::sleep(Duration::from_millis(10));
    }
}
fn read(root: &Path, t: &Value) -> Value {
    assert_eq!(t["status"], "completed", "{t}");
    let result = &t["result"];
    let d: Descriptor = serde_json::from_value(result["payload"].clone()).unwrap();
    let dir = root.join(result["directory"].as_str().unwrap());
    let manifest: Descriptor = serde_json::from_value(result["manifest"].clone()).unwrap();
    storage::load(&dir, &manifest).unwrap();
    serde_json::from_slice(&storage::load(&dir, &d).unwrap()).unwrap()
}
#[test]
fn malformed_version_size_exact_integer_and_admission() {
    let b = Bridge::default();
    assert_eq!(b.start(&vec![b' '; 65537]).unwrap_err().code, "size_limit");
    assert_eq!(b.start(b"{").unwrap_err().code, "invalid_request");
    assert_eq!(
        b.start(br#"{"version":2,"future":true}"#).unwrap_err().code,
        "unsupported_version"
    );
    let t = Temp::new();
    let mut r = request(&t.0, &fixture("noise16.wav"), None);
    r["version"] = json!(2);
    assert_eq!(
        b.start(&serde_json::to_vec(&r).unwrap()).unwrap_err().code,
        "unsupported_version"
    );
    r["version"] = json!(1);
    let parsed = Request::parse(&serde_json::to_vec(&r).unwrap()).unwrap();
    assert_eq!(parsed.generation, u64::MAX);
    let bytes = alfred_native::encode(Ok(json!({"max":u64::MAX,"null":null,"unknown":{"a":1}})));
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["value"]["max"].as_u64(), Some(u64::MAX));
    assert!(v["value"]["null"].is_null());
    for (rate, seconds) in [
        (48000, 180.0),
        (96000, 90.0),
        (192000, 45.0),
        (384000, 22.5),
    ] {
        assert!(
            admit_product(rate, Some(seconds), 2 * 1024 * 1024 * 1024, false).unwrap()
                <= 1024 * 1024 * 1024
        );
        assert!(
            admit_product(rate, Some(seconds + 0.1), 2 * 1024 * 1024 * 1024, false).is_err()
                || rate == 48000
        );
    }
    assert!(admit_product(384000, None, 2 * 1024 * 1024 * 1024, false).is_err());
    assert!(admit_product(48000, None, 1024, true).is_err());
    r["deadline_ms"] = json!(1200001);
    assert!(Request::parse(&serde_json::to_vec(&r).unwrap()).is_err());
}
#[test]
fn generated_exact_pcm_products_png_and_saved_operations() {
    let t = Temp::new();
    let b = Bridge::default();
    let mut saved = Vec::new();
    for (name, hash) in [
        (
            "noise16.wav",
            "1981c82a788cf0be394f6c6326e6e7576a788076b51e7f7834286cada2b0f0e4",
        ),
        (
            "noise16.flac",
            "1981c82a788cf0be394f6c6326e6e7576a788076b51e7f7834286cada2b0f0e4",
        ),
        (
            "alac/8000-16-1-tail.m4a",
            "8211292db8b4da19fb6b60bc1b1896c20a8108dbd3eb59f5ee9b7ee9c2790f89",
        ),
    ] {
        let mut r = request(&t.0, &fixture(name), Some("forensics"));
        r["png"] = json!(true);
        let terminal = run(&b, &r);
        let p = read(&t.0, &terminal);
        assert_eq!(p["measurement_report"]["status"], "analyzed");
        assert_eq!(
            p["measurement_report"]["coverage"]["decoded_pcm_sha256"],
            hash
        );
        assert_eq!(p["product_schema_version"], "audio-forensic-product-v1");
        let result = &terminal["result"];
        assert_eq!(result["artifact"]["status"], "available");
        let dir = t.0.join(result["directory"].as_str().unwrap());
        let png = fs::read(dir.join("spectrogram.png")).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 2560);
        saved.push(json!({"root":dir,"payload":result["payload"]}));
    }
    let mut compare = request(&t.0, &fixture("noise16.wav"), Some("compare"));
    compare["leases"] = json!([]);
    compare["saved"] = json!(&saved[..2]);
    compare["item_ids"] = json!([id(8), id(9)]);
    assert_eq!(
        b.start(&serde_json::to_vec(&compare).unwrap())
            .unwrap_err()
            .code,
        "invalid_request"
    );
    compare["same_track_asserted"] = json!(true);
    read(&t.0, &run(&b, &compare));
    let mut render = request(&t.0, &fixture("noise16.wav"), Some("render"));
    render["leases"] = json!([]);
    render["saved"] = json!([saved[0]]);
    render["png"] = json!(true);
    render["preset"] = json!("standard");
    let rendered = run(&b, &render);
    read(&t.0, &rendered);
    assert_eq!(rendered["result"]["artifact"]["status"], "available");
    let mut spec = request(&t.0, &fixture("noise16.wav"), Some("spectrogram"));
    spec["png"] = json!(true);
    spec["preset"] = json!("large");
    let rendered = run(&b, &spec);
    let p = read(&t.0, &rendered);
    assert_eq!(p["measurement"]["status"], "analyzed");
    assert_eq!(rendered["result"]["artifact"]["status"], "available");
}
#[test]
fn full_metadata_larger_than_control_and_snapshot_failure() {
    let t = Temp::new();
    let b = Bridge::default();
    let mut wav = fs::read(fixture("noise16.wav")).unwrap();
    let mut info = b"INFO".to_vec();
    for _ in 0..10 {
        info.extend(b"ICMT");
        info.extend(12000u32.to_le_bytes());
        info.extend(vec![b'x'; 12000]);
    }
    let mut list = b"LIST".to_vec();
    list.extend((info.len() as u32).to_le_bytes());
    list.extend(info);
    wav.splice(12..12, list);
    let size = wav.len() as u32 - 8;
    wav[4..8].copy_from_slice(&size.to_le_bytes());
    let input = t.0.join("tags.wav");
    fs::write(&input, wav).unwrap();
    let r = request(&t.0, &input, None);
    let terminal = run(&b, &r);
    let p = read(&t.0, &terminal);
    assert!(terminal["result"]["payload"]["bytes"].as_u64().unwrap() > 65536);
    assert_eq!(p["text_limits"]["retained_text_utf8_bytes"], 120000 + 40);
    assert!(serde_json::to_vec(&terminal).unwrap().len() < 65536);
    let mut changed = request(&t.0, &input, None);
    changed["leases"][0]["sha256"] = json!("0".repeat(64));
    assert_eq!(run(&b, &changed)["error"]["code"], "input_changed");
    assert!(!t.0.join(changed["attempt_id"].as_str().unwrap()).exists());
}
#[test]
fn unsupported_dsd_aac_and_storage_rejections() {
    let t = Temp::new();
    let b = Bridge::default();
    // Generated marker containers use the same explicit unsupported sniffing path.
    let mut aac = fs::read(fixture("alac/8000-16-1-tail.m4a")).unwrap();
    let pos = aac.windows(4).position(|p| p == b"alac").unwrap();
    aac[pos..pos + 4].copy_from_slice(b"mp4a");
    for (name, bytes) in [
        ("test.dsf", b"DSD     ".as_slice()),
        ("test.m4a", aac.as_slice()),
    ] {
        let path = t.0.join(name);
        fs::write(&path, bytes).unwrap();
        let r = request(&t.0, &path, Some("forensics"));
        let p = read(&t.0, &run(&b, &r));
        assert_eq!(p["measurement_report"]["status"], "unsupported");
    }
    assert!(storage::child(&t.0, "../outside").is_err());
    assert!(storage::child(&t.0, "/outside").is_err());
    let mut r = request(&t.0, &fixture("noise16.wav"), None);
    r["reserved_output_bytes"] = json!(1);
    assert_eq!(run(&b, &r)["error"]["code"], "storage_full");
    assert!(!t.0.join(r["attempt_id"].as_str().unwrap()).exists());
}

#[test]
fn metadata_survives_product_admission_rejection_and_png_failure_keeps_report() {
    let t = Temp::new();
    let b = Bridge::default();
    let mut r = request(&t.0, &fixture("noise16.wav"), Some("forensics"));
    r["low_memory"] = json!(true);
    assert_eq!(run(&b, &r)["error"]["code"], "resource_limit");
    let mut probe = request(&t.0, &fixture("noise16.wav"), None);
    probe["low_memory"] = json!(true);
    assert_eq!(read(&t.0, &run(&b, &probe))["status"], "available");
    let mut r = request(&t.0, &fixture("noise16.wav"), Some("forensics"));
    r["png"] = json!(true);
    r["reserved_output_bytes"] = json!(16 * 1024 * 1024);
    let terminal = run(&b, &r);
    assert_eq!(
        read(&t.0, &terminal)["measurement_report"]["status"],
        "analyzed"
    );
    assert_eq!(terminal["result"]["artifact"]["status"], "failed");
    assert!(terminal["result"]["artifact"]["payload"].is_null());
}
