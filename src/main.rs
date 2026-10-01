use audio_forensic::{AnalysisOptions, CancellationToken, FileStatus, analyze_path};
use clap::Parser;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
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
    let mut paths = vec![];
    for input in args.inputs {
        if input.is_dir() {
            let mut entries: Vec<_> = fs::read_dir(&input)?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file()
                        && p.extension().and_then(|s| s.to_str()).is_some_and(|s| {
                            s.eq_ignore_ascii_case("wav") || s.eq_ignore_ascii_case("flac")
                        })
                })
                .collect();
            entries.sort();
            if entries.is_empty() {
                paths.push(input);
            } else {
                paths.extend(entries);
            }
        } else {
            paths.push(input);
        }
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    let mut failed = false;
    if args.json {
        write!(out, "[")?;
    }
    for (i, path) in paths.iter().enumerate() {
        eprintln!("[{}/{}] {}", i + 1, paths.len(), path.display());
        let report = analyze_path(path, &options, &cancel);
        failed |= report.status != FileStatus::Analyzed;
        if args.json {
            if i > 0 {
                write!(out, ",")?;
            }
            serde_json::to_writer_pretty(&mut out, &report)?;
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
