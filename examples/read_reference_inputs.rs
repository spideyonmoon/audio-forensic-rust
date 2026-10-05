//! No-CLI example; P07 owns production workflows.
use audio_forensic::{AnalysisOptions, CancellationToken, analyze_path_with_reference_inputs};
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: read_reference_inputs PATH [MAX_SECONDS]");
    let max_seconds = args
        .next()
        .map(|x| x.parse::<f64>().expect("invalid MAX_SECONDS"));
    assert!(args.next().is_none(), "unexpected argument");
    let result = analyze_path_with_reference_inputs(
        path,
        &AnalysisOptions {
            max_seconds,
            ..Default::default()
        },
        &CancellationToken::default(),
        |_| {},
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&result).expect("serialization")
    );
    if result.measurement.status != audio_forensic::FileStatus::Analyzed {
        std::process::exit(1);
    }
}
