use audio_forensic::{
    AnalysisOptions, AnalysisProgress, AnalysisReport, CancellationToken, FileStatus, analyze_path,
    analyze_path_with_progress, assess_evidence,
};
use clap::Parser;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Parser)]
#[command(
    version,
    about = "Offline WAV/FLAC measurements and provisional forensic observations"
)]
struct Args {
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    /// Emit a versioned JSON array, including per-file failures.
    #[arg(long)]
    json: bool,
    /// Show grouped non-MQA evidence and inference limits instead of detailed measurements.
    #[arg(long, conflicts_with = "json")]
    summary: bool,
    /// Show live stages and per-pass frame counts on stderr (JSON stays on stdout).
    #[arg(long)]
    progress: bool,
    /// Analyze the first 60 seconds in every pass.
    #[arg(long, conflicts_with = "max_seconds")]
    fast: bool,
    /// Analyze only this many seconds from the start of each selected stream.
    #[arg(long)]
    max_seconds: Option<f64>,
    /// Select this track ID; metadata and samples refer to the same track.
    #[arg(long)]
    track_id: Option<u32>,
    /// Cooperative per-file deadline, checked between decoder packets and transform batches.
    #[arg(long, default_value_t = 600)]
    deadline_seconds: u64,
}

enum BatchInput {
    Audio(PathBuf),
    Failed(PathBuf, String),
}

impl BatchInput {
    fn path(&self) -> &Path {
        match self {
            Self::Audio(path) | Self::Failed(path, _) => path,
        }
    }

    fn analyze(&self, options: &AnalysisOptions, cancel: &CancellationToken) -> AnalysisReport {
        match self {
            Self::Audio(path) => analyze_path(path, options, cancel),
            Self::Failed(path, message) => {
                AnalysisReport::input_failure(path.to_string_lossy(), message)
            }
        }
    }
}

fn directory_inputs(
    directory: PathBuf,
    entries: io::Result<impl IntoIterator<Item = io::Result<PathBuf>>>,
) -> Vec<BatchInput> {
    let entries = match entries {
        Ok(entries) => entries,
        Err(error) => {
            return vec![BatchInput::Failed(
                directory,
                format!("Cannot enumerate directory: {error}"),
            )];
        }
    };
    let mut inputs = Vec::new();
    for entry in entries {
        let path = match entry {
            Ok(path) => path,
            Err(error) => {
                inputs.push(BatchInput::Failed(
                    directory.clone(),
                    format!("Cannot read directory entry: {error}"),
                ));
                continue;
            }
        };
        if !path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("wav") || s.eq_ignore_ascii_case("flac"))
        {
            continue;
        }
        match fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => inputs.push(BatchInput::Audio(path)),
            Ok(_) => {}
            Err(error) => inputs.push(BatchInput::Failed(
                path,
                format!("Cannot inspect directory entry: {error}"),
            )),
        }
    }
    if inputs.is_empty() {
        inputs.push(BatchInput::Failed(
            directory,
            "No WAV/FLAC files found in directory".into(),
        ));
    }
    inputs.sort_by(|left, right| left.path().cmp(right.path()));
    inputs
}

fn print_progress(event: AnalysisProgress) {
    match event {
        AnalysisProgress::WaitingForWorker => eprintln!("  waiting for analysis slot"),
        AnalysisProgress::ReadingMetadata => eprintln!("  reading metadata"),
        AnalysisProgress::Decoding {
            pass,
            processed_frames,
            expected_frames,
        } => {
            if let Some(expected) = expected_frames {
                eprintln!(
                    "  decode pass {pass}/2: {processed_frames} frames (expected {expected})"
                );
            } else {
                eprintln!("  decode pass {pass}/2: {processed_frames} frames (length unknown)");
            }
        }
        AnalysisProgress::AnalyzingDetectors => eprintln!("  analyzing detector observations"),
        AnalysisProgress::Finished { status } => eprintln!("  finished: {status:?}"),
        _ => {}
    }
}

fn run(args: Args) -> Result<i32, Box<dyn std::error::Error>> {
    let options = AnalysisOptions {
        track_id: args.track_id,
        max_seconds: if args.fast {
            Some(60.0)
        } else {
            args.max_seconds
        },
        deadline: Duration::from_secs(args.deadline_seconds),
    };
    let cancel = CancellationToken::default();
    let token = cancel.clone();
    ctrlc::set_handler(move || token.cancel())?;
    let mut inputs = vec![];
    for input in args.inputs {
        if input.is_dir() {
            let entries = fs::read_dir(&input)
                .map(|entries| entries.map(|entry| entry.map(|entry| entry.path())));
            inputs.extend(directory_inputs(input, entries));
        } else {
            inputs.push(BatchInput::Audio(input));
        }
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    let mut failed = false;
    if args.json {
        write!(out, "[")?;
    }
    for (i, input) in inputs.iter().enumerate() {
        let path = input.path();
        eprintln!("[{}/{}] {}", i + 1, inputs.len(), path.display());
        let report = if args.progress {
            match input {
                BatchInput::Audio(path) => {
                    analyze_path_with_progress(path, &options, &cancel, print_progress)
                }
                BatchInput::Failed(_, _) => {
                    let report = input.analyze(&options, &cancel);
                    print_progress(AnalysisProgress::Finished {
                        status: report.status.clone(),
                    });
                    report
                }
            }
        } else {
            input.analyze(&options, &cancel)
        };
        failed |= report.status != FileStatus::Analyzed;
        if args.json {
            if i > 0 {
                write!(out, ",")?;
            }
            serde_json::to_writer_pretty(&mut out, &report)?;
        } else if args.summary {
            writeln!(out, "{}: {:?}", path.display(), report.status)?;
            write!(out, "{}", assess_evidence(&report))?;
            for d in &report.diagnostics {
                writeln!(out, "  {}: {}", d.code, d.message)?;
            }
        } else {
            writeln!(
                out,
                "{}: {:?} | ancestry INCONCLUSIVE (provisional observations)",
                path.display(),
                report.status
            )?;
            if let (Some(info), Some(coverage)) = (&report.stream, &report.coverage) {
                writeln!(
                    out,
                    "  {} Hz, {} channels, {:.3}s measured, full stream: {}",
                    info.sample_rate, info.channels, coverage.end_seconds, coverage.reached_end
                )?;
                if let Some(l) = &report.loudness {
                    writeln!(
                        out,
                        "  programme loudness: {:?}; integrated {:?} LUFS, momentary max {:?} LUFS, short-term max {:?} LUFS (100 ms grid; {} gated / {} complete blocks)",
                        l.status,
                        l.integrated_lufs,
                        l.momentary_max_lufs,
                        l.short_term_max_lufs,
                        l.relative_gated_blocks,
                        l.complete_blocks
                    )?;
                    writeln!(
                        out,
                        "  loudness range: {:?}, {:?} LU, quantization bounds {:?}..{:?} LU (complete windows)",
                        l.range.status,
                        l.range.range_lu,
                        l.range.range_lower_lu,
                        l.range.range_upper_lu
                    )?;
                }
                if let Some(s) = &report.stereo_correlation {
                    writeln!(
                        out,
                        "  native stereo correlation: {:?}, {:?} (zero-lag centered PCM; {} pairs)",
                        s.status, s.coefficient, s.pair_count
                    )?;
                }
                for c in &report.channels {
                    writeln!(
                        out,
                        "  channel {} sample crest factor: {:?}, {:?} dB (measurement only)",
                        c.channel_index, c.crest_factor_status, c.crest_factor_db
                    )?;
                }
                for p in &report.true_peak {
                    writeln!(
                        out,
                        "  channel {} true-peak estimate: {:?} dBTP, sample peak {:.6} (4x FIR, zero-extended prefix)",
                        p.channel_index, p.estimated_peak_dbtp, p.sample_peak
                    )?;
                }
                for ch in &report.channels {
                    writeln!(
                        out,
                        "  channel {}: peak {:.6}, RMS {:.6}, used bits {:?}, cutoff {:?} Hz",
                        ch.channel_index,
                        ch.peak,
                        ch.rms,
                        ch.exact_used_bits,
                        ch.spectral.cutoff_p95_hz
                    )?;
                }
                for segment in &report.segments {
                    let candidates: std::collections::BTreeSet<_> = segment
                        .probes
                        .iter()
                        .flat_map(|p| &p.codec_candidates)
                        .map(|c| c.codec.as_str())
                        .collect();
                    writeln!(
                        out,
                        "  channel {}: {} wall probes / {} eligible / {} sampled ({})",
                        segment.channel_index,
                        segment.wall_probes,
                        segment.eligible_probes,
                        segment.probes.len(),
                        segment.pattern
                    )?;
                    if !candidates.is_empty() {
                        writeln!(
                            out,
                            "    overlapping wall candidates: {} (not codec identification)",
                            candidates.into_iter().collect::<Vec<_>>().join(", ")
                        )?;
                    }
                }
                if let Some(mqa) = &report.mqa {
                    writeln!(
                        out,
                        "  MQA signalling scan: {:?}, {} sync matches in {} frames (candidates only)",
                        mqa.status, mqa.sync_matches, mqa.scanned_frames
                    )?;
                }
                for r in &report.resampling {
                    let matches: Vec<_> = r
                        .candidates
                        .iter()
                        .filter(|c| !c.modes.is_empty())
                        .map(|c| format!("{} Hz: {}", c.source_rate, c.modes.join("/")))
                        .collect();
                    writeln!(
                        out,
                        "  channel {} resampling: {:?}{}",
                        r.channel_index,
                        r.status,
                        if matches.is_empty() {
                            String::new()
                        } else {
                            format!("; hypotheses {}", matches.join(", "))
                        }
                    )?;
                }
                for v in &report.vorbis {
                    writeln!(
                        out,
                        "  channel {} Vorbis grid: {:?}, excess {:?}, support {}/{} probes",
                        v.channel_index,
                        v.status,
                        v.zero_excess_score,
                        v.supporting_probes,
                        v.active_probes
                    )?;
                }
                for a in &report.aac {
                    writeln!(
                        out,
                        "  AAC {:?} lattice: {:?}, score {:?}, eligible {}/{} probes (provisional)",
                        a.basis,
                        a.status,
                        a.lattice_score,
                        a.eligible_probes,
                        a.probes.len()
                    )?;
                }
                for n in &report.noise {
                    writeln!(
                        out,
                        "  channel {} quiet passages: {} runs, {} samples; high-band {:?}, {:?} dBFS; quiet band {:?}, {:?} dBFS (measurements only)",
                        n.channel_index,
                        n.quiet_runs,
                        n.quiet_samples,
                        n.high_band.status,
                        n.high_band.rms_dbfs,
                        n.high_band.quiet_status,
                        n.high_band.quiet_rms_dbfs
                    )?;
                    writeln!(
                        out,
                        "    high-band correlation {:?}; temporal variation {:?}, {:?} dB over {} complete seconds (measurements only)",
                        n.high_band
                            .correlations
                            .iter()
                            .map(|c| (c.lag_frames, c.coefficient))
                            .collect::<Vec<_>>(),
                        n.high_band.temporal_variation.status,
                        n.high_band.temporal_variation.level_std_db,
                        n.high_band.temporal_variation.blocks.len()
                    )?;
                }
                for t in &report.transients {
                    writeln!(
                        out,
                        "  channel {} high-pass envelope peaks: {:?}, count {:?}, {:?}/minute (transient measurements only)",
                        t.channel_index, t.status, t.peak_count, t.peaks_per_minute
                    )?;
                }
                for e in &report.preceding_energy {
                    writeln!(
                        out,
                        "  channel {} preceding-event band energy: {:?}, {} eligible / {:?} selected, fraction {:?} (context measurement only)",
                        e.channel_index,
                        e.status,
                        e.eligible_event_count,
                        e.selected_peak_count,
                        e.above_baseline_fraction
                    )?;
                }
                for s in &report.spectral_lags {
                    let coefficients: Vec<_> =
                        s.lags.iter().map(|p| p.target.coefficient).collect();
                    writeln!(
                        out,
                        "  channel {} high-band spectral lags: {:?}, {:?} (measurements only)",
                        s.channel_index, s.status, coefficients
                    )?;
                }
                for r in &report.rolloff {
                    writeln!(
                        out,
                        "  channel {} spectral roll-off: {:?}, {:?} dB/kHz (measurement only)",
                        r.channel_index, r.status, r.slope_db_per_khz
                    )?;
                }
                for e in &report.envelope {
                    writeln!(
                        out,
                        "  channel {} band-envelope correlation: {:?}, {:?} (measurement only)",
                        e.channel_index, e.status, e.coefficient
                    )?;
                }
                for n in &report.noise_floor {
                    writeln!(
                        out,
                        "  channel {} quiet-block profile: {:?}, nonzero RMS p1.5 {:?} dBFS; color {:?}, HF-LF {:?} dB (measurements only)",
                        n.channel_index,
                        n.status,
                        n.nonzero_rms_p015_dbfs,
                        n.color_status,
                        n.high_minus_low_db
                    )?;
                }
                for s in &report.sparsity {
                    writeln!(
                        out,
                        "  channel {} below-cutoff sparsity: {:?}, {:?} (measurement only)",
                        s.channel_index, s.status, s.fraction
                    )?;
                }
                for s in &report.spectral_structure {
                    writeln!(
                        out,
                        "  channel {} spectral structure: scatter {:?}, average bound {:?} Hz; phase {:?}, entropy {:?} bits (measurements only)",
                        s.channel_index,
                        s.scatter_status,
                        s.average_scatter_bound_hz,
                        s.phase_status,
                        s.high_band_phase_entropy_bits
                    )?;
                }
            }
            for d in &report.diagnostics {
                writeln!(out, "  {}: {}", d.code, d.message)?;
            }
        }
        out.flush()?;
    }
    if args.json {
        writeln!(out, "]")?;
    }
    out.flush()?;
    Ok(if cancel.is_cancelled() {
        130
    } else if failed {
        1
    } else {
        0
    })
}

fn main() {
    let code = match run(Args::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error}");
            2
        }
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_enumeration_failure_is_a_reportable_input() {
        let input = PathBuf::from("generated-inaccessible-directory");
        let items = directory_inputs(
            input.clone(),
            Err::<Vec<io::Result<PathBuf>>, _>(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "generated permission error",
            )),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path(), input);
        let report = items[0].analyze(&AnalysisOptions::default(), &CancellationToken::default());
        assert_eq!(report.status, FileStatus::Failed);
        assert!(
            report.diagnostics[0]
                .message
                .contains("generated permission error")
        );
        assert_eq!(report.diagnostics[0].code, "invalid_input");
        assert!(report.stream.is_none() && report.coverage.is_none());
        assert!(report.channels.is_empty() && report.detectors.is_empty());
        assert_eq!(report.ancestry_verdict, "INCONCLUSIVE");
        assert!(report.evidence_index.is_none());
    }

    #[test]
    fn entry_and_metadata_failures_keep_sorted_valid_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.FLAC");
        let b = dir.path().join("b.WAV");
        let vanished = dir.path().join("vanished.wav");
        let ignored = dir.path().join("notes.txt");
        let nested = dir.path().join("nested.wav");
        fs::write(&a, []).unwrap();
        fs::write(&b, []).unwrap();
        fs::create_dir(&nested).unwrap();
        let items = directory_inputs(
            dir.path().into(),
            Ok(vec![
                Ok(b.clone()),
                Err(io::Error::other("generated entry error")),
                Ok(vanished.clone()),
                Ok(ignored),
                Ok(nested),
                Ok(a.clone()),
            ]),
        );
        assert_eq!(items.len(), 4);
        assert!(
            matches!(&items[0], BatchInput::Failed(_, message) if message.contains("generated entry error"))
        );
        assert!(matches!(&items[1], BatchInput::Audio(path) if path == &a));
        assert!(matches!(&items[2], BatchInput::Audio(path) if path == &b));
        assert!(
            matches!(&items[3], BatchInput::Failed(path, message) if path == &vanished && message.contains("Cannot inspect"))
        );
    }

    #[test]
    fn empty_directory_has_an_explicit_failure() {
        let items = directory_inputs(
            PathBuf::from("empty"),
            Ok(Vec::<io::Result<PathBuf>>::new()),
        );
        assert!(
            matches!(&items[..], [BatchInput::Failed(_, message)] if message.contains("No WAV/FLAC"))
        );
    }
}
