//! Minimal host integration without the optional CLI dependencies.
//! cargo run --no-default-features --example background_analysis -- input.wav [wait-ms]
use audio_forensic::{AnalysisJob, AnalysisOptions, FileStatus};
use std::{
    env,
    io::{self, Write},
    process::ExitCode,
    thread,
    time::{Duration, Instant},
};

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let path = args
        .next()
        .ok_or("usage: background_analysis <path> [wait-ms]")?;
    let wait = match args.next() {
        Some(value) => Duration::from_millis(value.to_str().ok_or("invalid wait-ms")?.parse()?),
        None => Duration::from_secs(600),
    };
    if args.next().is_some() {
        return Err("usage: background_analysis <path> [wait-ms]".into());
    }

    // A UI retains this handle and polls from its event loop. Source opening and
    // analysis happen on the worker; dropping the handle requests cancellation.
    let mut job = AnalysisJob::spawn_path(path, AnalysisOptions::default())?;
    let started = Instant::now();
    let mut last_progress = None;
    let report = loop {
        if started.elapsed() >= wait {
            job.cancel();
        }
        let progress = job.progress();
        if progress != last_progress {
            if let Some(event) = &progress {
                eprintln!("{event:?}");
            }
            last_progress = progress;
        }
        if let Some(result) = job.try_finish() {
            break result?;
        }
        // Only this console example sleeps; a UI returns to its event loop.
        thread::sleep(Duration::from_millis(10));
    };
    let status = if report.status == FileStatus::Analyzed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, &report)?;
    writeln!(stdout)?;
    Ok(status)
}

fn main() -> ExitCode {
    match run() {
        Ok(status) => status,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
