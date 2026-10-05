use crate::{
    byproducts::{ByproductAnalysis, ByproductCollector, ReferenceByproducts},
    detectors::{
        self, aac::AacSelector, envelope::EnvelopeStats, mqa::MqaScanner, noise::NoiseStats,
        noise_floor::FloorSurvey, preceding_energy::EnergySurvey, resampling::ResamplingStats,
        rolloff::RolloffStats, segments::SegmentCollector, sparsity::SparsityStats,
        structure::StructureStats, transients::TransientSurvey, vorbis::VorbisCollector,
    },
    dsp::{PcmStats, SpectralStats, StreamingStft},
    loudness::LoudnessMeter,
    model::*,
    progress::{AnalysisProgress, Progress},
    reference_inputs::{ReferenceAnalysis, ReferenceInputs, ReferenceSurvey},
    stereo::StereoStats,
    tool_statistics::{ToolAnalysis, ToolCollector, ToolMeasurements},
    true_peak::TruePeakMeter,
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{ErrorKind, Seek, SeekFrom},
    path::Path,
    time::Instant,
};
use symphonia::core::{
    audio::{AudioBufferRef, SampleBuffer},
    codecs::{
        CODEC_TYPE_PCM_F32BE, CODEC_TYPE_PCM_F32BE_PLANAR, CODEC_TYPE_PCM_F32LE,
        CODEC_TYPE_PCM_F32LE_PLANAR, CODEC_TYPE_PCM_F64BE, CODEC_TYPE_PCM_F64BE_PLANAR,
        CODEC_TYPE_PCM_F64LE, CODEC_TYPE_PCM_F64LE_PLANAR, Decoder, DecoderOptions,
    },
    errors::Error as DecodeError,
    formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
    io::{MediaSource, MediaSourceStream, ReadBytes},
    meta::MetadataOptions,
    probe::Hint,
};

#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum Failure {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    Decode(String),
    #[error("Analysis cancelled")]
    Cancelled,
    #[error("Analysis exceeded its cooperative deadline")]
    TimedOut,
}

impl From<DecodeError> for Failure {
    fn from(err: DecodeError) -> Self {
        match err {
            DecodeError::Unsupported(msg) => Self::Unsupported(msg.into()),
            _ => Self::Decode(err.to_string()),
        }
    }
}

fn failure(report: &mut AnalysisReport, error: Failure) {
    let (status, code) = match &error {
        Failure::Invalid(_) => (FileStatus::Failed, "invalid_input"),
        Failure::Unsupported(_) => (FileStatus::Unsupported, "unsupported"),
        Failure::Decode(_) => (FileStatus::Failed, "decode_error"),
        Failure::Cancelled => (FileStatus::Cancelled, "cancelled"),
        Failure::TimedOut => (FileStatus::TimedOut, "deadline_exceeded"),
    };
    report.status = status;
    report.diagnostics.push(Diagnostic {
        code: code.into(),
        message: error.to_string(),
    });
}

pub fn analyze_path(
    path: impl AsRef<Path>,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
) -> AnalysisReport {
    analyze_path_with_progress(path, options, cancel, |_| {})
}

/// Path-opening convenience wrapper for [`analyze_source_with_progress`].
/// A path-open failure emits only Finished(Failed). Opening a path itself is
/// synchronous and is outside the core's cooperative decode deadline.
pub fn analyze_path_with_progress(
    path: impl AsRef<Path>,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    mut callback: impl FnMut(AnalysisProgress),
) -> AnalysisReport {
    let path = path.as_ref();
    match File::open(path) {
        Ok(file) => analyze_source_with_progress(
            Box::new(file),
            &path.to_string_lossy(),
            options,
            cancel,
            callback,
        ),
        Err(error) => {
            let report = AnalysisReport::input_failure(path.to_string_lossy(), error.to_string());
            callback(AnalysisProgress::Finished {
                status: report.status.clone(),
            });
            report
        }
    }
}

/// Accepts File, Cursor, or a seekable application-provided MediaSource. No path
/// lookup is performed. Two passes require seek support; nonseekable inputs must
/// be staged by the caller. Display names do not determine the actual format.
pub fn analyze_source(
    source: Box<dyn MediaSource>,
    name: &str,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
) -> AnalysisReport {
    analyze_source_with_progress(source, name, options, cancel, |_| {})
}

/// Analyze with synchronous host notifications on the calling thread. Keep the
/// callback short and nonblocking; processing callback time counts toward the deadline. It may
/// request cancellation through a cloned token. Do not reenter analysis from a
/// callback: one worker is held throughout processing. Callback panics unwind
/// normally and no Finished event is promised in that case.
///
/// Decode updates are throttled to at most one per 100 ms, except pass start/end.
/// No overall percentage or detector-time estimate is implied. Finished is sent
/// once for every normal return, after the source and worker permit are released.
/// Cancellation requested in Finished is too late to change the returned report.
pub fn analyze_source_with_progress(
    source: Box<dyn MediaSource>,
    name: &str,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    mut callback: impl FnMut(AnalysisProgress),
) -> AnalysisReport {
    let mut report = AnalysisReport::new(name.into());
    let mut progress = Progress::new(&mut callback);
    if let Err(error) = analyze(
        source,
        options,
        cancel,
        &mut report,
        &mut progress,
        ProductOutputs::default(),
    ) {
        failure(&mut report, error);
    }
    progress.emit(AnalysisProgress::Finished {
        status: report.status.clone(),
    });
    report
}

/// Collect the separate reference byproducts while preserving native analysis.
/// The worker, cancellation, deadline, PCM hashes and progress match the ordinary
/// source API. No extra decode pass or runtime external tool is required.
pub fn analyze_source_with_byproducts(
    source: Box<dyn MediaSource>,
    name: &str,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    mut callback: impl FnMut(AnalysisProgress),
) -> ByproductAnalysis {
    let mut report = AnalysisReport::new(name.into());
    let mut reference = None;
    let mut progress = Progress::new(&mut callback);
    if let Err(error) = analyze(
        source,
        options,
        cancel,
        &mut report,
        &mut progress,
        ProductOutputs {
            byproducts: Some(&mut reference),
            ..Default::default()
        },
    ) {
        failure(&mut report, error);
    }
    progress.emit(AnalysisProgress::Finished {
        status: report.status.clone(),
    });
    ByproductAnalysis::new(report, reference)
}

/// Path-opening convenience wrapper for [`analyze_source_with_byproducts`].
pub fn analyze_path_with_byproducts(
    path: impl AsRef<Path>,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    mut callback: impl FnMut(AnalysisProgress),
) -> ByproductAnalysis {
    let path = path.as_ref();
    match File::open(path) {
        Ok(file) => analyze_source_with_byproducts(
            Box::new(file),
            &path.to_string_lossy(),
            options,
            cancel,
            callback,
        ),
        Err(error) => {
            let report = AnalysisReport::input_failure(path.to_string_lossy(), error.to_string());
            callback(AnalysisProgress::Finished {
                status: report.status.clone(),
            });
            ByproductAnalysis::new(report, None)
        }
    }
}

/// Collect pinned tool statistics and P03 byproducts in the same worker and
/// source snapshot as native analysis. No additional decode pass or subprocess.
pub fn analyze_source_with_tool_statistics(
    source: Box<dyn MediaSource>,
    name: &str,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    mut callback: impl FnMut(AnalysisProgress),
) -> ToolAnalysis {
    let mut report = AnalysisReport::new(name.into());
    let mut reference = None;
    let mut tools = None;
    let mut progress = Progress::new(&mut callback);
    if let Err(error) = analyze(
        source,
        options,
        cancel,
        &mut report,
        &mut progress,
        ProductOutputs {
            byproducts: Some(&mut reference),
            tools: Some(&mut tools),
            ..Default::default()
        },
    ) {
        failure(&mut report, error);
    }
    progress.emit(AnalysisProgress::Finished {
        status: report.status.clone(),
    });
    ToolAnalysis::new(report, reference, tools)
}

/// Path-opening convenience wrapper; failures suppress all product measurements.
pub fn analyze_path_with_tool_statistics(
    path: impl AsRef<Path>,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    mut callback: impl FnMut(AnalysisProgress),
) -> ToolAnalysis {
    let path = path.as_ref();
    match File::open(path) {
        Ok(file) => analyze_source_with_tool_statistics(
            Box::new(file),
            &path.to_string_lossy(),
            options,
            cancel,
            callback,
        ),
        Err(error) => {
            let report = AnalysisReport::input_failure(path.to_string_lossy(), error.to_string());
            callback(AnalysisProgress::Finished {
                status: report.status.clone(),
            });
            ToolAnalysis::new(report, None, None)
        }
    }
}

/// Collect separate P04c reference inputs with three PCM-verified passes.
/// One worker/source snapshot; ordinary native APIs remain two-pass. Missing
/// P04a/P04b adaptive-wall dependencies stay explicitly unavailable.
pub fn analyze_source_with_reference_inputs(
    source: Box<dyn MediaSource>,
    name: &str,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    mut callback: impl FnMut(AnalysisProgress),
) -> ReferenceAnalysis {
    let mut report = AnalysisReport::new(name.into());
    let mut inputs = None;
    let mut progress = Progress::new(&mut callback);
    if let Err(error) = analyze(
        source,
        options,
        cancel,
        &mut report,
        &mut progress,
        ProductOutputs {
            inputs: Some(&mut inputs),
            ..Default::default()
        },
    ) {
        failure(&mut report, error);
    }
    progress.emit(AnalysisProgress::Finished {
        status: report.status.clone(),
    });
    ReferenceAnalysis::new(report, inputs)
}

/// Opens the path once. All subsequent passes reuse the same guarded source.
pub fn analyze_path_with_reference_inputs(
    path: impl AsRef<Path>,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    mut callback: impl FnMut(AnalysisProgress),
) -> ReferenceAnalysis {
    let path = path.as_ref();
    match File::open(path) {
        Ok(file) => analyze_source_with_reference_inputs(
            Box::new(file),
            &path.to_string_lossy(),
            options,
            cancel,
            callback,
        ),
        Err(error) => {
            let report = AnalysisReport::input_failure(path.to_string_lossy(), error.to_string());
            callback(AnalysisProgress::Finished {
                status: report.status.clone(),
            });
            ReferenceAnalysis::new(report, None)
        }
    }
}

#[derive(Default)]
struct ProductOutputs<'a> {
    byproducts: Option<&'a mut Option<ReferenceByproducts>>,
    tools: Option<&'a mut Option<ToolMeasurements>>,
    inputs: Option<&'a mut Option<ReferenceInputs>>,
}

fn check_control(
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    start: Instant,
) -> Result<(), Failure> {
    if cancel.is_cancelled() {
        return Err(Failure::Cancelled);
    }
    if start.elapsed() >= options.deadline {
        return Err(Failure::TimedOut);
    }
    Ok(())
}

struct ScanOutcome {
    frames: u64,
    packet_bytes: u64,
    reached_end: bool,
    integer: Option<bool>,
    pcm_hash: String,
}

struct ScanContext<'a> {
    info: &'a StreamInfo,
    flac: Option<crate::container::FlacInfo>,
    limit: Option<u64>,
    options: &'a AnalysisOptions,
    cancel: &'a CancellationToken,
    start: Instant,
}

fn scan(
    format: &mut dyn FormatReader,
    decoder: &mut dyn Decoder,
    context: &ScanContext<'_>,
    mut progress: impl FnMut(u64, bool),
    mut visit: impl FnMut(usize, f64, Option<i32>),
) -> Result<ScanOutcome, Failure> {
    let mut outcome = ScanOutcome {
        frames: 0,
        packet_bytes: 0,
        reached_end: false,
        integer: None,
        pcm_hash: String::new(),
    };
    let mut hash = Sha256::new();
    progress(0, true);
    let info = context.info;
    let padding_mask = info
        .bits_per_sample
        .filter(|&bits| bits > 0 && bits < 32)
        .map(|bits| (1u32 << (32 - bits)) - 1)
        .unwrap_or(0);
    loop {
        check_control(context.options, context.cancel, context.start)?;
        if context.limit.is_some_and(|n| outcome.frames >= n) {
            break;
        }
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(DecodeError::IoError(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                outcome.reached_end = true;
                break;
            }
            Err(e) => return Err(e.into()),
        };
        if packet.track_id() != info.track_id {
            continue;
        }
        if context.flac.is_some() && packet.ts != outcome.frames {
            return Err(Failure::Decode(format!(
                "FLAC packet discontinuity: expected sample {}, got {}",
                outcome.frames, packet.ts
            )));
        }
        if context.flac.is_some() {
            crate::flac_frame::validate(
                &packet.data,
                packet.dur,
                info.channels,
                info.bits_per_sample.unwrap_or(0),
                || check_control(context.options, context.cancel, context.start),
            )?;
        }
        let decoded = decoder.decode(&packet)?;
        if context.flac.is_some()
            && (decoded.frames() == 0 || packet.dur != decoded.frames() as u64)
        {
            return Err(Failure::Decode(
                "FLAC packet duration does not match decoded frames".into(),
            ));
        }
        outcome.packet_bytes = outcome
            .packet_bytes
            .checked_add(packet.data.len() as u64)
            .ok_or_else(|| Failure::Decode("Packet byte count overflow".into()))?;
        if decoded.spec().rate != info.sample_rate
            || decoded.spec().channels.count() != info.channels
        {
            return Err(Failure::Unsupported(
                "Stream parameters changed during decode".into(),
            ));
        }
        if decoded.frames() > 1_048_576 || decoded.capacity() > 1_048_576 {
            return Err(Failure::Unsupported(
                "Decoder block frames/capacity exceed the supported limit".into(),
            ));
        }
        let integer = !matches!(&decoded, AudioBufferRef::F32(_) | AudioBufferRef::F64(_));
        if outcome.integer.is_some_and(|prior| prior != integer) {
            return Err(Failure::Decode("PCM representation changed".into()));
        }
        outcome.integer = Some(integer);
        let take = decoded.frames().min(
            context
                .limit
                .map(|n| usize::try_from(n.saturating_sub(outcome.frames)).unwrap_or(usize::MAX))
                .unwrap_or(usize::MAX),
        );
        if integer {
            let mut samples = SampleBuffer::<i32>::new(decoded.capacity() as u64, *decoded.spec());
            samples.copy_interleaved_ref(decoded);
            for frame in samples.samples().chunks_exact(info.channels).take(take) {
                for (ch, word) in frame.iter().enumerate() {
                    if *word as u32 & padding_mask != 0 {
                        return Err(Failure::Decode(
                            "Nonzero integer PCM bits below the declared precision".into(),
                        ));
                    }
                    hash.update(word.to_le_bytes());
                    visit(ch, *word as f64 / 2147483648.0, Some(*word));
                }
            }
        } else {
            let mut samples = SampleBuffer::<f64>::new(decoded.capacity() as u64, *decoded.spec());
            samples.copy_interleaved_ref(decoded);
            for frame in samples.samples().chunks_exact(info.channels).take(take) {
                for (ch, x) in frame.iter().enumerate() {
                    if !x.is_finite() || x.abs() > 16.0 {
                        return Err(Failure::Invalid(
                            "Nonfinite or unsupported float PCM amplitude (absolute limit 16)"
                                .into(),
                        ));
                    }
                    hash.update(x.to_le_bytes());
                    visit(ch, *x, None);
                }
            }
        }
        outcome.frames += take as u64;
        progress(outcome.frames, false);
    }
    if outcome.frames == 0 {
        return Err(Failure::Invalid("No audio samples decoded".into()));
    }
    if outcome.reached_end && info.declared_frames.is_some_and(|n| n != outcome.frames) {
        return Err(Failure::Decode(format!(
            "Decoded {} frames, header declares {}; stream may be truncated",
            outcome.frames,
            info.declared_frames.unwrap()
        )));
    }
    outcome.pcm_hash = format!("{:x}", hash.finalize());
    progress(outcome.frames, true);
    check_control(context.options, context.cancel, context.start)?;
    Ok(outcome)
}

fn validate_container(
    source: &mut dyn MediaSource,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    start: Instant,
) -> Result<Option<crate::container::FlacInfo>, Failure> {
    crate::container::validate(source, || check_control(options, cancel, start).is_ok()).map_err(
        |error| match error {
            crate::container::Error::Unsupported(_) => Failure::Unsupported(error.to_string()),
            crate::container::Error::Interrupted => check_control(options, cancel, start)
                .err()
                .unwrap_or(Failure::Cancelled),
            _ => Failure::Decode(error.to_string()),
        },
    )
}

fn flac_stream(
    format: Box<dyn FormatReader>,
    scan: &ScanOutcome,
    info: crate::container::FlacInfo,
) -> Result<MediaSourceStream, Failure> {
    let source = format.into_inner();
    let expected = info
        .audio_offset
        .checked_add(scan.packet_bytes)
        .ok_or_else(|| Failure::Decode("FLAC byte position overflow".into()))?;
    // The locked parser may silently discard corrupt fragments before returning
    // a packet or EOF. Every consumed byte must belong to an accepted packet.
    // Use the logical buffered position, not the underlying read-ahead position.
    if source.pos() != expected {
        return Err(Failure::Decode(format!(
            "FLAC parser skipped or left unaccounted bytes: expected offset {expected}, got {}",
            source.pos()
        )));
    }
    Ok(source)
}

fn analyze(
    source: Box<dyn MediaSource>,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    report: &mut AnalysisReport,
    progress: &mut Progress<'_>,
    outputs: ProductOutputs<'_>,
) -> Result<(), Failure> {
    let ProductOutputs {
        byproducts: reference,
        tools,
        inputs,
    } = outputs;
    let start = Instant::now();
    check_control(options, cancel, start)?;
    if options
        .max_seconds
        .is_some_and(|x| !x.is_finite() || x <= 0.0 || x > 86400.0)
    {
        return Err(Failure::Invalid(
            "max_seconds must be finite and in (0, 86400]".into(),
        ));
    }
    if !source.is_seekable() {
        return Err(Failure::Unsupported(
            "Two-pass analysis requires a seekable source".into(),
        ));
    }
    // Acquire before probing or allocating decoder/DSP state. Deadline includes
    // time queued behind another call; cancellation is checked while waiting.
    progress.emit(AnalysisProgress::WaitingForWorker);
    let _worker =
        crate::worker::ANALYSIS_WORKER.acquire(|| check_control(options, cancel, start))?;
    progress.emit(AnalysisProgress::ReadingMetadata);
    let (mut source, source_fault) =
        crate::source::GuardedSource::new(source, cancel.clone(), options.deadline, start);
    let flac = source_fault.check(validate_container(&mut source, options, cancel, start))?;
    let mut hint = Hint::new();
    if let Some(ext) = Path::new(&report.source)
        .extension()
        .and_then(|x| x.to_str())
    {
        hint.with_extension(ext);
    }
    let stream = MediaSourceStream::new(Box::new(source), Default::default());
    let probed = source_fault.check(
        symphonia::default::get_probe()
            .format(
                &hint,
                stream,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(Failure::from),
    )?;
    let mut format = probed.format;
    let track = match options.track_id {
        Some(id) => format.tracks().iter().find(|t| t.id == id),
        None => format.default_track(),
    }
    .ok_or_else(|| Failure::Invalid("Requested audio track is not present".into()))?;
    let params = track.codec_params.clone();
    let rate = params
        .sample_rate
        .ok_or_else(|| Failure::Unsupported("Missing sample rate".into()))?;
    let channels = params
        .channels
        .ok_or_else(|| Failure::Unsupported("Missing channel layout".into()))?
        .count();
    if !(8000..=384000).contains(&rate) || !(1..=2).contains(&channels) {
        return Err(Failure::Unsupported(
            "Initial support is mono/stereo at 8-384 kHz; no implicit downmix is performed".into(),
        ));
    }
    let codec_name = symphonia::default::get_codecs()
        .get_codec(params.codec)
        .map(|c| c.short_name.to_owned())
        .unwrap_or_else(|| format!("{:?}", params.codec));
    let mut info = StreamInfo {
        track_id: track.id,
        codec: codec_name,
        sample_rate: rate,
        channels,
        bits_per_sample: params.bits_per_sample,
        declared_frames: params.n_frames,
        // Preserve known float precision even when decode fails before a scan
        // completes. The supported native codecs are integer FLAC/PCM or these
        // explicit float PCM types; successful output also checks decoded data.
        integer_pcm: !matches!(
            params.codec,
            CODEC_TYPE_PCM_F32LE
                | CODEC_TYPE_PCM_F32BE
                | CODEC_TYPE_PCM_F32LE_PLANAR
                | CODEC_TYPE_PCM_F32BE_PLANAR
                | CODEC_TYPE_PCM_F64LE
                | CODEC_TYPE_PCM_F64BE
                | CODEC_TYPE_PCM_F64LE_PLANAR
                | CODEC_TYPE_PCM_F64BE_PLANAR
        ),
    };
    if info.bits_per_sample.is_some_and(|n| n == 0 || n > 64) {
        return Err(Failure::Unsupported("Unsupported sample precision".into()));
    }
    let mut decoder =
        symphonia::default::get_codecs().make(&params, &DecoderOptions { verify: true })?;
    report.stream = Some(info.clone());
    let limit = options
        .max_seconds
        .map(|s| (s * rate as f64).floor() as u64);
    if limit == Some(0) {
        return Err(Failure::Invalid(
            "Requested interval is shorter than one sample".into(),
        ));
    }
    let mut pcm: Vec<PcmStats> = (0..channels).map(|_| PcmStats::default()).collect();
    let mut peaks = vec![0.0f32; channels];
    let mut stfts: Vec<_> = (0..channels).map(|_| StreamingStft::new()).collect();
    let mut mqa = MqaScanner::new(rate, channels, info.bits_per_sample);
    let mut aac_selector = AacSelector::new(rate, channels);
    let mut transient_surveys: Vec<_> = (0..channels).map(|_| TransientSurvey::new(rate)).collect();
    let mut energy_surveys: Vec<_> = (0..channels).map(|_| EnergySurvey::new(rate)).collect();
    let mut floor_surveys: Vec<_> = (0..channels).map(|_| FloorSurvey::new(rate)).collect();
    let mut loudness_survey = LoudnessMeter::new(rate, channels, None, None);
    let mut true_peak: Vec<_> = (0..channels).map(|_| TruePeakMeter::default()).collect();
    let mut left_word = None;
    let mut left_sample = 0.0;
    let mut stereo = StereoStats::default();
    let mut byproducts = reference
        .as_ref()
        .map(|_| ByproductCollector::new(rate, channels));
    let mut tool_collector = tools.as_ref().map(|_| ToolCollector::new(&info));
    let mut input_survey = inputs
        .as_ref()
        .map(|_| ReferenceSurvey::new(rate, channels));
    let context = ScanContext {
        info: &info,
        flac,
        limit,
        options,
        cancel,
        start,
    };
    let first = source_fault.check(scan(
        &mut *format,
        &mut *decoder,
        &context,
        |frames, force| {
            progress.frames(
                1,
                frames,
                match (info.declared_frames, limit) {
                    (Some(total), Some(limit)) => Some(total.min(limit)),
                    (total, None) => total,
                    (None, Some(limit)) => Some(limit),
                },
                force,
            )
        },
        |ch, x, word| {
            if let Some(r) = &mut input_survey {
                r.push(ch, x);
            }
            if let Some(t) = &mut tool_collector {
                t.push(ch, x, word);
            }
            loudness_survey.push(ch, x);
            true_peak[ch].push(x);
            aac_selector.push(ch, x);
            transient_surveys[ch].push(x);
            energy_surveys[ch].push(x);
            floor_surveys[ch].push(x);
            if ch == 0 {
                left_word = word;
                left_sample = x;
                if channels == 1 {
                    if let Some(b) = &mut byproducts {
                        b.push(x, None);
                    }
                }
            } else {
                mqa.push(left_word, word);
                stereo.push(left_sample, x);
                if let Some(b) = &mut byproducts {
                    b.push(left_sample, Some(x));
                }
            }
            pcm[ch].push(
                x,
                word,
                if word.is_some() {
                    info.bits_per_sample
                } else {
                    None
                },
            );
            stfts[ch].push(x as f32, |mags| {
                peaks[ch] = peaks[ch].max(mags.iter().copied().fold(0.0, f32::max));
            });
        },
    ))?;
    drop(stfts);
    let verified = if first.reached_end {
        decoder.finalize().verify_ok
    } else {
        None
    };
    if verified == Some(false) {
        return Err(Failure::Decode(
            "Decoder checksum verification failed".into(),
        ));
    }
    if let Some(flac_info) = flac {
        let mut stream = flac_stream(format, &first, flac_info)?;
        source_fault.check(
            stream
                .seek(SeekFrom::Start(0))
                .map_err(|error| Failure::Decode(format!("FLAC source rewind: {error}"))),
        )?;
        let second_info =
            source_fault.check(validate_container(&mut stream, options, cancel, start))?;
        if second_info != flac {
            return Err(Failure::Decode(
                "FLAC STREAMINFO or audio offset changed between analysis passes".into(),
            ));
        }
        // A seek on the locked FLAC reader can retain the last frame sequence,
        // merge earlier frames into one packet, and require a known byte_len.
        // Re-probe from zero with bounded metadata and a fresh reader instead.
        format = source_fault
            .check(
                symphonia::default::get_probe()
                    .format(
                        &hint,
                        MediaSourceStream::new(Box::new(stream), Default::default()),
                        &FormatOptions::default(),
                        &MetadataOptions::default(),
                    )
                    .map_err(Failure::from),
            )?
            .format;
    } else {
        let seeked = source_fault.check(
            format
                .seek(
                    SeekMode::Accurate,
                    SeekTo::TimeStamp {
                        ts: 0,
                        track_id: info.track_id,
                    },
                )
                .map_err(Failure::from),
        )?;
        if seeked.actual_ts != 0 {
            return Err(Failure::Unsupported(
                "Cannot seek exactly to stream start".into(),
            ));
        }
    }
    // reset() does not reset every decoder's running verification checksum.
    // A fresh decoder makes the second pass an independent verification run.
    decoder = symphonia::default::get_codecs().make(&params, &DecoderOptions { verify: true })?;
    let mut input_second = input_survey.map(|r| r.into_second(first.frames));
    let mut spectral: Vec<_> = peaks
        .into_iter()
        .map(|p| SpectralStats::new(rate, p))
        .collect();
    let mut stfts: Vec<_> = (0..channels).map(|_| StreamingStft::new()).collect();
    let mut segments = SegmentCollector::new(first.frames, rate, channels);
    let mut resampling: Vec<_> = (0..channels).map(|_| ResamplingStats::new(rate)).collect();
    let mut vorbis = VorbisCollector::new(first.frames, rate, channels);
    let mut aac = aac_selector.into_collector(first.frames);
    let mut structure: Vec<_> = (0..channels).map(|_| StructureStats::new(rate)).collect();
    let mut rolloff: Vec<_> = (0..channels).map(|_| RolloffStats::new(rate)).collect();
    let mut envelope: Vec<_> = (0..channels).map(|_| EnvelopeStats::new(rate)).collect();
    let mut sparsity: Vec<_> = (0..channels).map(|_| SparsityStats::new(rate)).collect();
    let mut noise: Vec<_> = (0..channels).map(|_| NoiseStats::new(rate)).collect();
    let mut noise_floor: Vec<_> = floor_surveys
        .into_iter()
        .enumerate()
        .map(|(ch, s)| s.into_collector(ch, first.frames))
        .collect();
    let mut transients: Vec<_> = transient_surveys
        .into_iter()
        .enumerate()
        .map(|(ch, survey)| survey.into_detector(ch))
        .collect();
    let mut loudness = LoudnessMeter::new(
        rate,
        channels,
        loudness_survey.relative_gate(),
        loudness_survey.range_gate(),
    );
    let mut preceding_energy: Vec<_> = energy_surveys
        .into_iter()
        .enumerate()
        .map(|(ch, s)| s.into_collector(ch, transients[ch].selection_available()))
        .collect();
    let second = source_fault.check(scan(
        &mut *format,
        &mut *decoder,
        &context,
        |frames, force| progress.frames(2, frames, Some(first.frames), force),
        |ch, x, _| {
            if let Some(r) = &mut input_second {
                r.push(ch, x);
            }
            loudness.push(ch, x);
            stfts[ch].push_spectrum(x as f32, |mags, spectrum| {
                noise[ch].push_spectrum(spectrum);
                let active = spectral[ch].push(mags);
                rolloff[ch].push(mags, active);
                envelope[ch].push(mags, active);
                sparsity[ch].push(mags, active);
                structure[ch].push(mags, spectrum, active);
                if active {
                    resampling[ch].push(mags);
                }
            });
            noise[ch].push_sample(x);
            noise_floor[ch].push(x);
            preceding_energy[ch].push(x);
            if let Some(event) = transients[ch].push(x) {
                preceding_energy[ch].observe(&event);
            }
            segments.push(ch, x);
            vorbis.push(ch, x);
            aac.push(ch, x);
        },
    ))?;
    if first.frames != second.frames
        || first.integer != second.integer
        || first.pcm_hash != second.pcm_hash
    {
        return Err(Failure::Decode(
            "Source changed between analysis passes".into(),
        ));
    }
    if second.reached_end && decoder.finalize().verify_ok == Some(false) {
        return Err(Failure::Decode(
            "Decoder checksum verification failed on second pass".into(),
        ));
    }
    drop(stfts);
    let mut input_record = None;
    if let Some(r) = input_second {
        progress.emit(AnalysisProgress::AnalyzingDetectors);
        let mut third_inputs = r.into_third(|| check_control(options, cancel, start))?;
        if let Some(flac_info) = flac {
            let mut stream = flac_stream(format, &second, flac_info)?;
            source_fault.check(
                stream
                    .seek(SeekFrom::Start(0))
                    .map_err(|error| Failure::Decode(format!("FLAC source rewind: {error}"))),
            )?;
            let third_info =
                source_fault.check(validate_container(&mut stream, options, cancel, start))?;
            if third_info != flac {
                return Err(Failure::Decode(
                    "FLAC STREAMINFO or audio offset changed between analysis passes".into(),
                ));
            }
            // A seek on the locked FLAC reader can retain the last frame sequence,
            // merge earlier frames into one packet, and require a known byte_len.
            // Re-probe from zero with bounded metadata and a fresh reader instead.
            format = source_fault
                .check(
                    symphonia::default::get_probe()
                        .format(
                            &hint,
                            MediaSourceStream::new(Box::new(stream), Default::default()),
                            &FormatOptions::default(),
                            &MetadataOptions::default(),
                        )
                        .map_err(Failure::from),
                )?
                .format;
        } else {
            let seeked = source_fault.check(
                format
                    .seek(
                        SeekMode::Accurate,
                        SeekTo::TimeStamp {
                            ts: 0,
                            track_id: info.track_id,
                        },
                    )
                    .map_err(Failure::from),
            )?;
            if seeked.actual_ts != 0 {
                return Err(Failure::Unsupported(
                    "Cannot seek exactly to stream start".into(),
                ));
            }
        }
        decoder =
            symphonia::default::get_codecs().make(&params, &DecoderOptions { verify: true })?;
        let third = source_fault.check(scan(
            &mut *format,
            &mut *decoder,
            &context,
            |frames, force| progress.frames(3, frames, Some(first.frames), force),
            |ch, x, _| third_inputs.push(ch, x),
        ))?;
        if third.frames != first.frames
            || third.integer != first.integer
            || third.pcm_hash != first.pcm_hash
        {
            return Err(Failure::Decode(
                "Source changed on reference input pass".into(),
            ));
        }
        if third.reached_end && decoder.finalize().verify_ok == Some(false) {
            return Err(Failure::Decode(
                "Decoder checksum verification failed on reference input pass".into(),
            ));
        }
        if let Some(flac_info) = flac {
            flac_stream(format, &third, flac_info)?;
        }
        check_control(options, cancel, start)?;
        let mut record = third_inputs.finish();
        record.decoded_pcm_sha256 = first.pcm_hash.clone();
        record.pass_pcm_sha256 = vec![
            first.pcm_hash.clone(),
            second.pcm_hash.clone(),
            third.pcm_hash,
        ];
        record.reached_end = first.reached_end;
        record.hash_sample_encoding = if first.integer == Some(true) {
            "s32le_msb_aligned"
        } else {
            "f64le"
        }
        .into();
        input_record = Some(record);
    } else if let Some(flac_info) = flac {
        flac_stream(format, &second, flac_info)?;
    }
    progress.emit(AnalysisProgress::AnalyzingDetectors);
    check_control(options, cancel, start)?;
    let segments = segments.finish();
    let vorbis = vorbis.finish(|| check_control(options, cancel, start))?;
    let aac = aac.finish(|| check_control(options, cancel, start))?;
    info.integer_pcm = first.integer.unwrap_or(false);
    report.coverage = Some(Coverage {
        start_seconds: 0.0,
        end_seconds: first.frames as f64 / rate as f64,
        analyzed_frames: first.frames,
        reached_end: first.reached_end,
        requested_max_seconds: options.max_seconds,
        length_matches_header: if first.reached_end {
            info.declared_frames.map(|n| n == first.frames)
        } else {
            None
        },
        decoder_verification: verified,
        analysis_passes: 2,
        decoded_pcm_sha256: first.pcm_hash,
        hash_sample_encoding: if info.integer_pcm {
            "s32le_msb_aligned"
        } else {
            "f64le"
        }
        .into(),
    });
    report.spectral_lags = spectral
        .iter()
        .enumerate()
        .map(|(ch, s)| s.spectral_lags(ch))
        .collect();
    report.channels = pcm
        .into_iter()
        .zip(spectral)
        .enumerate()
        .map(|(ch, (p, s))| p.finish(ch, s.finish()))
        .collect();
    for channel in &report.channels {
        let (status, caveat) = if !info.integer_pcm {
            (
                DetectorStatus::Unsupported,
                "Integer precision checks do not apply to floating-point PCM.",
            )
        } else if channel.effective_bits.is_none() || info.bits_per_sample.is_none() {
            (
                DetectorStatus::Inconclusive,
                "Insufficient nonzero samples or unknown source precision.",
            )
        } else if channel
            .exact_used_bits
            .zip(info.bits_per_sample)
            .is_some_and(|(used, declared)| used.saturating_add(8) <= declared)
        {
            (
                DetectorStatus::Hit,
                "At least eight low-order source bits are unused in the analyzed interval; original recording depth remains unverified.",
            )
        } else {
            (
                DetectorStatus::NotDetected,
                "No eight-bit integer padding observed; dither or processing can exercise low bits without establishing source depth.",
            )
        };
        let mut measures = std::collections::BTreeMap::new();
        if let Some(bits) = channel.effective_bits {
            measures.insert("effective_bits".into(), bits as f64);
        }
        if let Some(bits) = channel.exact_used_bits {
            measures.insert("exact_used_bits".into(), bits as f64);
        }
        report.detectors.push(DetectorResult {
            id: "integer_precision".into(),
            version: 1,
            family: "bit_depth".into(),
            status,
            channel_index: Some(channel.channel_index),
            intervals: vec![AnalysisInterval {
                start_frame: 0,
                end_frame: first.frames,
            }],
            measurements: measures,
            thresholds: [
                ("unused_bits_for_hit".into(), 8.0),
                ("minimum_nonzero_samples".into(), 500.0),
            ]
            .into(),
            caveats: vec![caveat.into()],
        });
        report.detectors.push(DetectorResult { id: "spectral_features".into(), version: 1, family: "spectral_measurements".into(),
            status: if channel.spectral.active_frames > 0 { DetectorStatus::Measured } else { DetectorStatus::Inconclusive },
            channel_index: Some(channel.channel_index),
            intervals: vec![AnalysisInterval { start_frame: 0, end_frame: first.frames }],
            measurements: [("active_frames".into(), channel.spectral.active_frames as f64)].into(),
            thresholds: [("cutoff_db_relative_to_frame_peak".into(), -65.0), ("activity_db_relative_to_channel_peak".into(), -60.0)].into(),
            caveats: vec!["Spectral measurements alone do not identify a lossy codec or prove source history.".into()] });
    }
    report.segments = segments;
    report.mqa = Some(mqa.finish());
    report.resampling = resampling
        .into_iter()
        .enumerate()
        .map(|(ch, r)| r.finish(ch))
        .collect();
    report.vorbis = vorbis;
    report.aac = aac;
    report.spectral_structure = structure
        .into_iter()
        .enumerate()
        .map(|(ch, s)| s.finish(ch))
        .collect();
    for (ch, n) in noise.into_iter().enumerate() {
        check_control(options, cancel, start)?;
        report
            .noise
            .push(n.finish(ch, report.channels[ch].spectral.cutoff_p95_hz));
    }
    report.transients = transients.into_iter().map(|t| t.finish()).collect();
    report.preceding_energy = preceding_energy.into_iter().map(|e| e.finish()).collect();
    report.rolloff = rolloff
        .into_iter()
        .enumerate()
        .map(|(ch, r)| r.finish(ch))
        .collect();
    report.envelope = envelope
        .into_iter()
        .enumerate()
        .map(|(ch, e)| e.finish(ch))
        .collect();
    report.sparsity = sparsity
        .into_iter()
        .enumerate()
        .map(|(ch, s)| s.finish(ch, report.channels[ch].spectral.cutoff_p95_hz))
        .collect();
    report.noise_floor = noise_floor.into_iter().map(|s| s.finish()).collect();
    report.loudness = Some(loudness.finish(first.frames));
    report.true_peak = true_peak
        .into_iter()
        .enumerate()
        .map(|(ch, t)| t.finish(ch, rate))
        .collect();
    report.stereo_correlation = Some(stereo.finish(channels));
    if let (Some(output), Some(collector)) = (reference, byproducts) {
        *output =
            Some(collector.finish(first.reached_end, || check_control(options, cancel, start))?);
    }
    if let (Some(output), Some(collector)) = (tools, tool_collector) {
        *output = Some(collector.finish(|| check_control(options, cancel, start))?);
    }
    detectors::append_observations(report, rate);
    report.stream = Some(info);
    if !first.reached_end {
        report.limitations.push("Only the requested prefix was analyzed; full-file integrity and later content were not checked.".into());
    }
    if let Some(output) = inputs {
        *output = input_record;
    }
    report.status = FileStatus::Analyzed;
    Ok(())
}
