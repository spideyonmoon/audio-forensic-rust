//! No-CLI adapter; main product/report workflows belong to P07.
use audio_forensic::spectrogram_png::{CanvasOptions, CanvasSize};
use audio_forensic::{AnalysisOptions, CancellationToken, analyze_path_with_spectrogram};
use std::time::Duration;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: read_spectrogram PATH [MAX_SECONDS|full] [NEW_PNG_OR_PPM_PATH] [standard|publication|large]");
    let max_seconds = args
        .next()
        .filter(|v| v != "full")
        .map(|v| v.parse().expect("invalid MAX_SECONDS"));
    let export_path = args.next();
    let size = match args.next().as_deref() {
        None | Some("publication") => CanvasSize::Publication,
        Some("standard") => CanvasSize::Standard,
        Some("large") => CanvasSize::Large,
        _ => panic!("invalid canvas size"),
    };
    assert!(args.next().is_none(), "unexpected argument");
    let cancel = CancellationToken::default();
    let result = analyze_path_with_spectrogram(
        path,
        &AnalysisOptions {
            max_seconds,
            ..Default::default()
        },
        &cancel,
        |_| {},
    );
    if let Some(path) = export_path {
        let export = if std::path::Path::new(&path)
            .extension()
            .is_some_and(|v| v.eq_ignore_ascii_case("ppm"))
        {
            result
                .spectrogram
                .write_ppm_new(path, &cancel, Duration::from_secs(30))
        } else {
            result.write_png_new(
                path,
                &CanvasOptions {
                    size,
                    ..Default::default()
                },
                &cancel,
                Duration::from_secs(30),
            )
        };
        eprintln!(
            "{}",
            serde_json::to_string(&export).expect("export serialization")
        );
    }
    println!("{}", serde_json::to_string(&result).expect("serialization"));
    if result.measurement.status != audio_forensic::FileStatus::Analyzed {
        std::process::exit(1);
    }
}
