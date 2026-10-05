//! Interpret saved reports without decoding the recordings again.
//! cargo run --no-default-features --example explain_report -- saved-report.json
use audio_forensic::{AnalysisReport, assess_evidence};
use std::{
    env,
    fs::File,
    io::{self, Read, Write},
    process::ExitCode,
};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let path = args
        .next()
        .ok_or("usage: explain_report <saved-report.json>")?;
    if args.next().is_some() {
        return Err("usage: explain_report <saved-report.json>".into());
    }
    // Bound this example's input storage. The report producer's schema should be
    // validated separately when accepting reports from an untrusted producer.
    const MAX_BYTES: u64 = 64 * 1024 * 1024;
    let mut bytes = vec![];
    File::open(path)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("report input exceeds 64 MiB".into());
    }
    let document: serde_json::Value = serde_json::from_slice(&bytes)?;
    let reports: Vec<AnalysisReport> = if document.is_array() {
        serde_json::from_value(document)?
    } else {
        vec![serde_json::from_value(document)?]
    };
    if reports.is_empty() {
        return Err("report array is empty".into());
    }
    // Preserve report order for positional association with the input array.
    let assessments: Vec<_> = reports.iter().map(assess_evidence).collect();
    let mut out = io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, &assessments)?;
    writeln!(out)?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
