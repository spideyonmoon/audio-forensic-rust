use audio_forensic::{AnalysisOptions, CancellationToken, analyze_path_with_tool_statistics};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: read_tool_statistics PATH [PREFIX_SECONDS]");
    let mut options = AnalysisOptions::default();
    if let Some(seconds) = args.next() {
        options.max_seconds = Some(seconds.parse().expect("prefix seconds"));
    }
    assert!(args.next().is_none(), "unexpected argument");
    let product =
        analyze_path_with_tool_statistics(path, &options, &CancellationToken::default(), |_| {});
    println!(
        "{}",
        serde_json::to_string_pretty(&product).expect("finite result")
    );
    if product.measurement.status != audio_forensic::FileStatus::Analyzed {
        std::process::exit(1);
    }
}
