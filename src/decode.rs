use crate::{
    detectors::{
        self, aac::AacSelector, envelope::EnvelopeStats, mqa::MqaScanner, noise::NoiseStats,
        resampling::ResamplingStats, rolloff::RolloffStats, segments::SegmentCollector,
        sparsity::SparsityStats, structure::StructureStats, transients::TransientSurvey,
        vorbis::VorbisCollector,
    },
    dsp::{PcmStats, SpectralStats, StreamingStft},
    model::*,
};
use sha2::{Digest, Sha256};
use std::{fs::File, io::ErrorKind, path::Path, time::Instant};
use symphonia::core::{
    audio::{AudioBufferRef, SampleBuffer},
    codecs::{Decoder, DecoderOptions},
    errors::Error as DecodeError,
    formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
    io::{MediaSource, MediaSourceStream},
    meta::MetadataOptions,
    probe::Hint,
};

#[derive(Debug, thiserror::Error)]
enum Failure {
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
    let path = path.as_ref();
    match File::open(path) {
        Ok(file) => analyze_source(Box::new(file), &path.to_string_lossy(), options, cancel),
        Err(error) => {
            let mut report = AnalysisReport::new(path.to_string_lossy().into_owned());
            failure(&mut report, Failure::Invalid(error.to_string()));
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
    let mut report = AnalysisReport::new(name.into());
    if let Err(error) = analyze(source, options, cancel, &mut report) {
        failure(&mut report, error);
    }
    report
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
    reached_end: bool,
    integer: Option<bool>,
    pcm_hash: String,
}

struct ScanContext<'a> {
    info: &'a StreamInfo,
    limit: Option<u64>,
    options: &'a AnalysisOptions,
    cancel: &'a CancellationToken,
    start: Instant,
}

fn scan(
    format: &mut dyn FormatReader,
    decoder: &mut dyn Decoder,
    context: &ScanContext<'_>,
    mut visit: impl FnMut(usize, f64, Option<i32>),
) -> Result<ScanOutcome, Failure> {
    let mut outcome = ScanOutcome {
        frames: 0,
        reached_end: false,
        integer: None,
        pcm_hash: String::new(),
    };
    let mut hash = Sha256::new();
    let info = context.info;
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
        let decoded = decoder.decode(&packet)?;
        if decoded.spec().rate != info.sample_rate
            || decoded.spec().channels.count() != info.channels
        {
            return Err(Failure::Unsupported(
                "Stream parameters changed during decode".into(),
            ));
        }
        if decoded.frames() > 1_048_576 {
            return Err(Failure::Unsupported(
                "Decoder block exceeds the supported frame limit".into(),
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
    Ok(outcome)
}

fn analyze(
    source: Box<dyn MediaSource>,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
    report: &mut AnalysisReport,
) -> Result<(), Failure> {
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
    let mut hint = Hint::new();
    if let Some(ext) = Path::new(&report.source)
        .extension()
        .and_then(|x| x.to_str())
    {
        hint.with_extension(ext);
    }
    let stream = MediaSourceStream::new(source, Default::default());
    let probed = symphonia::default::get_probe().format(
        &hint,
        stream,
        &FormatOptions::default(),
        &MetadataOptions::default(),
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
        integer_pcm: true,
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
    let mut left_word = None;
    let context = ScanContext {
        info: &info,
        limit,
        options,
        cancel,
        start,
    };
    let first = scan(&mut *format, &mut *decoder, &context, |ch, x, word| {
        aac_selector.push(ch, x);
        transient_surveys[ch].push(x);
        if ch == 0 {
            left_word = word;
        } else {
            mqa.push(left_word, word);
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
    })?;
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
    let seeked = format.seek(
        SeekMode::Accurate,
        SeekTo::TimeStamp {
            ts: 0,
            track_id: info.track_id,
        },
    )?;
    if seeked.actual_ts != 0 {
        return Err(Failure::Unsupported(
            "Cannot seek exactly to stream start".into(),
        ));
    }
    // reset() does not reset every decoder's running verification checksum.
    // A fresh decoder makes the second pass an independent verification run.
    decoder = symphonia::default::get_codecs().make(&params, &DecoderOptions { verify: true })?;
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
    let mut transients: Vec<_> = transient_surveys
        .into_iter()
        .enumerate()
        .map(|(ch, survey)| survey.into_detector(ch))
        .collect();
    let second = scan(&mut *format, &mut *decoder, &context, |ch, x, _| {
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
        transients[ch].push(x);
        segments.push(ch, x);
        vorbis.push(ch, x);
        aac.push(ch, x);
    })?;
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
    detectors::append_observations(report, rate);
    report.stream = Some(info);
    if !first.reached_end {
        report.limitations.push("Only the requested prefix was analyzed; full-file integrity and later content were not checked.".into());
    }
    report.status = FileStatus::Analyzed;
    Ok(())
}
