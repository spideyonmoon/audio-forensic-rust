use crate::{
    storage::{self, Output},
    transport::{Result, Scope, State, fail},
};
use audio_forensic::{
    AnalysisOptions, AnalysisProgress,
    metadata::{MetadataStatus, TechnicalMetadata},
    product::{ProductReport, compare_products, read_product_json},
    spectrogram::SpectrogramAnalysis,
    spectrogram_png::{CanvasOptions, CanvasSize},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    time::{Duration, Instant},
};

/// Checked reservation bound using the core's 2/3/5/7/11-smooth FFT geometry. Includes 32*N profile bytes, silence scratch,
/// and a 512 MiB reserve for other collectors/serialization/allocator overhead.
pub fn admit_product(
    rate: u32,
    max_seconds: Option<f64>,
    available: u64,
    low: bool,
) -> Result<u64> {
    if rate == 0 || rate > 384_000 || low || available < 1536 * 1024 * 1024 {
        return Err(fail(
            "resource_limit",
            "Product requires 1.5 GiB available memory and a supported rate",
        ));
    }
    // Never trust declared duration to bound an unbounded decode: malformed
    // headers may understate actual length. Prefix is the enforced frame bound.
    let seconds = max_seconds.unwrap_or(180.0).min(180.0);
    let frames = (seconds * f64::from(rate)).ceil() as u64;
    if frames > 8_640_000 {
        return Err(fail(
            "resource_limit",
            &format!(
                "Choose an explicit prefix at most {} seconds",
                8_640_000.0 / f64::from(rate)
            ),
        ));
    }
    let mut padded = frames.max(1);
    loop {
        let mut q = padded;
        for factor in [2, 3, 5, 7, 11] {
            while q % factor == 0 {
                q /= factor;
            }
        }
        if q == 1 {
            break;
        }
        padded = padded
            .checked_add(1)
            .ok_or_else(|| fail("resource_limit", "FFT length overflow"))?;
    }
    let silence = frames
        .min(30 * u64::from(rate))
        .max(1)
        .checked_next_power_of_two()
        .ok_or_else(|| fail("resource_limit", "Silence length overflow"))?;
    let bytes = padded
        .checked_mul(32)
        .and_then(|n| n.checked_add(silence * 8))
        .and_then(|n| n.checked_add(512 * 1024 * 1024))
        .ok_or_else(|| fail("resource_limit", "Memory bound overflow"))?;
    if bytes > 1024 * 1024 * 1024 {
        return Err(fail(
            "resource_limit",
            "FFT bound exceeds reservation; choose a smaller prefix",
        ));
    }
    Ok(bytes)
}
fn options(s: &State, start: Instant) -> Result<AnalysisOptions> {
    let deadline = Duration::from_millis(s.request.deadline_ms)
        .checked_sub(start.elapsed())
        .filter(|v| !v.is_zero())
        .ok_or_else(|| fail("interrupted", "Operation deadline expired"))?;
    Ok(AnalysisOptions {
        max_seconds: match s.request.scope {
            Scope::Full => None,
            Scope::Prefix { seconds } => Some(seconds),
        },
        deadline,
        track_id: None,
    })
}
fn check(s: &State, start: Instant) -> Result<()> {
    if s.cancel.is_cancelled() {
        return Err(fail("interrupted", "Cancellation requested"));
    }
    options(s, start).map(|_| ())
}
fn open_snapshot(s: &State, start: Instant) -> Result<File> {
    let lease = &s.request.leases[0];
    let mut file = File::open(&lease.path).map_err(|e| {
        fail(
            match e.kind() {
                std::io::ErrorKind::NotFound => "input_missing",
                std::io::ErrorKind::PermissionDenied => "permission_denied",
                _ => "io_error",
            },
            "Cannot open snapshot",
        )
    })?;
    if !file.metadata().map_err(storage::io)?.is_file() {
        return Err(fail(
            "invalid_request",
            "Snapshot must be a regular private file",
        ));
    }
    let mut buf = [0u8; 256 * 1024];
    let mut h = Sha256::new();
    let mut count = 0u64;
    loop {
        check(s, start)?;
        let n = file.read(&mut buf).map_err(storage::io)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > lease.bytes {
            return Err(fail("input_changed", "Snapshot size changed"));
        }
        h.update(&buf[..n]);
    }
    if count != lease.bytes || format!("{:x}", h.finalize()) != lease.sha256 {
        return Err(fail("input_changed", "Snapshot hash changed"));
    }
    file.seek(SeekFrom::Start(0)).map_err(storage::io)?;
    Ok(file)
}
fn progress(s: &State, p: AnalysisProgress) {
    s.progress(match p {
        AnalysisProgress::Decoding { pass, processed_frames, expected_frames } => json!({"phase":"decoding","pass":pass,"frames":processed_frames,"expected_frames":expected_frames}),
        AnalysisProgress::WaitingForWorker => json!({"phase":"waiting"}),
        AnalysisProgress::ReadingMetadata => json!({"phase":"metadata"}),
        AnalysisProgress::AnalyzingDetectors => json!({"phase":"detectors"}),
        _ => json!({"phase":"finalizing"}),
    });
}
fn technical(t: &Option<TechnicalMetadata>) -> Value {
    t.as_ref().map(|t| json!({"codec":t.codec,"sample_rate":t.declared_sample_rate_hz,"channels":t.declared_channels,"precision":t.declared_precision_bits,"declared_frames":t.declared_frames})).unwrap_or(Value::Null)
}
fn saved_product(s: &crate::transport::SavedInput) -> Result<ProductReport> {
    if s.payload.kind != "product" || s.payload.version != "audio-forensic-product-v1" {
        return Err(fail("unsupported_version", "Expected product v1"));
    }
    let bytes = storage::load(&s.root, &s.payload)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| fail("invalid_request", "Payload is not UTF-8"))?;
    #[derive(serde::Deserialize)]
    struct Version {
        product_schema_version: String,
    }
    let version: Version = serde_json::from_str(text)
        .map_err(|_| fail("invalid_request", "Invalid saved product header"))?;
    if version.product_schema_version != "audio-forensic-product-v1" {
        return Err(fail(
            "unsupported_version",
            "Unsupported saved product version",
        ));
    }
    let mut products = read_product_json(text)
        .map_err(|_| fail("invalid_request", "Invalid saved product binding"))?;
    if products.len() != 1 {
        return Err(fail(
            "invalid_request",
            "One product per descriptor is required",
        ));
    }
    Ok(products.remove(0))
}
pub fn run(s: &State) -> Result<Value> {
    let r = &s.request;
    let start = Instant::now();
    check(s, start)?;
    let mut out = Output::new(&r.output_dir, &r.attempt_id, r.reserved_output_bytes)?;
    let mut summary = Value::Null;
    let mut image: Option<SpectrogramAnalysis> = None;
    let payload;
    match r.feature.as_deref() {
        Some("compare") => {
            if r.low_memory || r.available_memory_bytes < 1536 * 1024 * 1024 {
                return Err(fail(
                    "resource_limit",
                    "Insufficient memory for saved comparison",
                ));
            }
            let mut products = Vec::new();
            for input in &r.saved {
                check(s, start)?;
                products.push(saved_product(input)?);
            }
            let report = compare_products(&products)
                .map_err(|_| fail("invalid_request", "Invalid comparison inputs"))?;
            payload = out.json(
                "comparison.json",
                "comparison",
                "audio-forensic-comparison-v1",
                &report,
            )?;
        }
        Some("render") => {
            if r.low_memory || r.available_memory_bytes < 512 * 1024 * 1024 {
                return Err(fail(
                    "resource_limit",
                    "Rendering requires 512 MiB available memory",
                ));
            }
            let input = &r.saved[0];
            image = Some(if input.payload.kind == "product" {
                let p = saved_product(input)?;
                SpectrogramAnalysis {
                    measurement: p.measurement_report,
                    spectrogram: p
                        .artifacts
                        .spectrogram
                        .ok_or_else(|| fail("invalid_request", "Product has no spectrogram"))?,
                    presentation: p.artifacts.presentation,
                }
            } else {
                if input.payload.kind != "spectrogram" || input.payload.version != "1" {
                    return Err(fail("unsupported_version", "Expected spectrogram v1"));
                }
                serde_json::from_slice(&storage::load(&input.root, &input.payload)?)
                    .map_err(|_| fail("invalid_request", "Invalid saved spectrogram"))?
            });
            image
                .as_ref()
                .unwrap()
                .validate_binding()
                .map_err(|_| fail("invalid_request", "Spectrogram binding mismatch"))?;
            payload = out.json(
                "spectrogram.json",
                "spectrogram",
                "1",
                image.as_ref().unwrap(),
            )?;
        }
        _ => {
            s.progress(json!({"phase":"verifying_snapshot"}));
            let mut file = open_snapshot(s, start)?;
            let lease = &r.leases[0];
            s.progress(json!({"phase":"metadata"}));
            // try_clone shares a cursor; probe finishes and drops its clone before rewind.
            let metadata = audio_forensic::metadata::read_metadata_source(
                Box::new(file.try_clone().map_err(storage::io)?),
                &lease.name,
                &options(s, start)?,
                &s.cancel,
            );
            let reason = metadata
                .diagnostics
                .first()
                .map(|d| fail(&d.code, &d.message));
            summary = json!({"status":metadata.status,"technical":technical(&metadata.technical),"reason":reason.map(|e| json!({"code":e.code,"message":e.message}))});
            if r.kind == "probe" {
                drop(file);
                payload = out.json("metadata.json", "metadata", "1", &metadata)?;
            } else {
                file.seek(SeekFrom::Start(0)).map_err(storage::io)?;
                if r.feature.as_deref() == Some("forensics") {
                    if metadata.status == MetadataStatus::Available {
                        let rate = metadata
                            .technical
                            .as_ref()
                            .and_then(|t| t.declared_sample_rate_hz)
                            .ok_or_else(|| {
                                fail("resource_limit", "Cannot bound unknown sample rate")
                            })?;
                        admit_product(
                            rate,
                            options(s, start)?.max_seconds,
                            r.available_memory_bytes,
                            r.low_memory,
                        )?;
                    }
                    let p = audio_forensic::analyze_source_product(
                        Box::new(file),
                        &lease.name,
                        &options(s, start)?,
                        &s.cancel,
                        r.spectrogram || r.png,
                        |p| progress(s, p),
                    );
                    summary = json!({"status":p.measurement_report.status});
                    payload =
                        out.json("product.json", "product", "audio-forensic-product-v1", &p)?;
                    if r.png {
                        image = p
                            .artifacts
                            .spectrogram
                            .map(|spectrogram| SpectrogramAnalysis {
                                measurement: p.measurement_report,
                                spectrogram,
                                presentation: p.artifacts.presentation,
                            });
                    }
                } else {
                    if r.low_memory || r.available_memory_bytes < 512 * 1024 * 1024 {
                        return Err(fail(
                            "resource_limit",
                            "Spectrogram requires 512 MiB available memory",
                        ));
                    }
                    let analysis = audio_forensic::analyze_source_with_spectrogram(
                        Box::new(file),
                        &lease.name,
                        &options(s, start)?,
                        &s.cancel,
                        |p| progress(s, p),
                    );
                    summary = json!({"status":analysis.measurement.status});
                    payload = out.json("spectrogram.json", "spectrogram", "1", &analysis)?;
                    if r.png {
                        image = Some(analysis);
                    }
                }
            }
        }
    }
    // Core calls have dropped sources and DSP collectors before PNG allocation.
    s.progress(json!({"phase":"finalizing"}));
    let mut artifact = Value::Null;
    if let Some(image) = image {
        artifact = if r.low_memory
            || r.available_memory_bytes < 512 * 1024 * 1024
            || !out.png_space()
        {
            json!({"status":"failed","reason":"resource_limit","payload":null})
        } else {
            let canvas = CanvasOptions {
                size: match r.preset.as_str() {
                    "standard" => CanvasSize::Standard,
                    "large" => CanvasSize::Large,
                    _ => CanvasSize::Publication,
                },
                title: None,
            };
            let path = storage::child(&out.dir, "spectrogram.png.partial")?;
            let export = image.write_png_new(
                &path,
                &canvas,
                &s.cancel,
                options(s, start)
                    .map(|o| o.deadline)
                    .unwrap_or(Duration::ZERO),
            );
            if export.path.is_some() {
                let publication = (|| -> Result<_> {
                    // Windows FlushFileBuffers requires write access too.
                    std::fs::OpenOptions::new()
                        .write(true)
                        .open(&path)
                        .map_err(storage::io)?
                        .sync_all()
                        .map_err(storage::io)?;
                    std::fs::rename(&path, out.dir.join("spectrogram.png")).map_err(storage::io)?;
                    out.descriptor("spectrogram.png", "png", "1")
                })();
                match publication {
                    Ok(d) => {
                        json!({"status":export.status,"payload":d,"decoded_pcm_sha256":image.measurement.coverage.as_ref().map(|c| &c.decoded_pcm_sha256),"preset":r.preset})
                    }
                    Err(e) => {
                        let _ = std::fs::remove_file(&path);
                        let _ = std::fs::remove_file(out.dir.join("spectrogram.png"));
                        json!({"status":"failed","reason":e.code,"payload":null})
                    }
                }
            } else {
                json!({"status":export.status,"reason":export.reason,"payload":null})
            }
        };
    }
    // Shared history maps these stable item IDs to retained input jobs. Avoid
    // persisting absolute storage paths or copying/reinterpreting input scores.
    let inputs: Vec<_> = r
        .saved
        .iter()
        .zip(&r.item_ids)
        .map(|(saved, item)| json!({"item_id":item,"payload":saved.payload}))
        .collect();
    let manifest = json!({"version":1,"job_id":r.job_id,"attempt_id":r.attempt_id,"feature":r.feature,"selection_id":r.selection_id,"generation":r.generation,"item_ids":r.item_ids,"host_state":"completed","payload":payload,"artifact":artifact,"retained_inputs":inputs});
    let descriptor = out.commit(&manifest)?;
    Ok(
        json!({"directory":r.attempt_id,"manifest":descriptor,"payload":payload,"artifact":artifact,"summary":summary}),
    )
}
