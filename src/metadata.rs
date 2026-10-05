//! Bounded, no-decode metadata for the product layer. Editable text is unscored.
use crate::{
    AnalysisOptions, AnalysisReport, CancellationToken, FileStatus,
    model::{AnalysisInterval, Diagnostic},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::File, path::Path, time::Instant};
use symphonia::core::io::MediaSource;

pub const METADATA_VERSION: u32 = 1;
pub const MAX_TAGS: usize = 1024;
pub const MAX_KEY_BYTES: usize = 256;
pub const MAX_VALUE_BYTES: usize = 16 * 1024;
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetadataStatus {
    Available,
    Failed,
    Unsupported,
    Cancelled,
    TimedOut,
}

/// Parsed declarations, not decoded measurements. Future F01/F02 readers use
/// these same optional fields and the bounded Collector below.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TechnicalMetadata {
    pub container: String,
    pub codec: String,
    pub selected_track_id: u32,
    pub file_size_bytes: Option<u64>,
    pub declared_sample_rate_hz: Option<u32>,
    pub declared_channels: Option<u32>,
    pub declared_precision_bits: Option<u32>,
    pub storage_bits_per_sample: Option<u32>,
    pub sample_encoding: Option<String>,
    pub declared_frames: Option<u64>,
    pub declared_duration_seconds: Option<f64>,
    pub declared_bit_rate_bps: Option<u64>,
    pub derived_pcm_bit_rate_bps: Option<u64>,
    pub compression_mode: Option<String>,
    pub unavailable_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagEntry {
    /// Source block/chunk, absolute offset and local order preserve provenance.
    pub source: String,
    pub source_offset: u64,
    pub source_order: usize,
    pub key: String,
    /// None for artwork; its bytes are deliberately not exposed as base64.
    pub value: Option<String>,
    pub artwork_index: Option<usize>,
    pub original_key_bytes: u64,
    pub original_value_bytes: u64,
    pub key_truncated: bool,
    pub value_truncated: bool,
    pub invalid_utf8: bool,
    pub malformed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtworkDescriptor {
    pub source: String,
    pub source_offset: u64,
    pub picture_type: Option<u32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub depth_bits: Option<u32>,
    pub byte_length: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaqueMetadata {
    pub source: String,
    pub source_offset: u64,
    pub byte_length: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextLimits {
    pub total_entries: u64,
    pub omitted_entries: u64,
    pub truncated_entries: u64,
    pub invalid_utf8_entries: u64,
    pub malformed_entries: u64,
    /// Counts decoded UTF-8 tag text, including replacements for invalid bytes.
    /// Artwork's binary/base64 value is excluded and described separately.
    pub total_text_utf8_bytes: u64,
    pub retained_text_utf8_bytes: u64,
    pub omitted_text_utf8_bytes: u64,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataObservations {
    pub encoder_signatures: Vec<String>,
    pub encoder_tag_indices: Vec<usize>,
    pub legacy_extension_eligible: bool,
    pub legacy_encoder_trace: Option<String>,
    pub encoder_summary: Option<String>,
    pub mqa_metadata_claimed: bool,
    pub mqa_tag_indices: Vec<usize>,
    /// A negative text search is incomplete if text was omitted or undecodable.
    pub text_scan_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataReport {
    pub metadata_version: u32,
    pub contract_version: u32,
    pub engine_version: String,
    pub source: String,
    pub status: MetadataStatus,
    pub technical: Option<TechnicalMetadata>,
    pub entries: Vec<TagEntry>,
    /// Named Python fields refer to entries; duplicate text is never copied.
    pub named_tags: BTreeMap<String, Vec<usize>>,
    pub artwork: Vec<ArtworkDescriptor>,
    pub opaque_metadata: Vec<OpaqueMetadata>,
    pub text_limits: TextLimits,
    pub observations: MetadataObservations,
    pub diagnostics: Vec<Diagnostic>,
    pub caveats: Vec<String>,
}

impl MetadataReport {
    fn new(source: &str) -> Self {
        Self {
            metadata_version: METADATA_VERSION,
            contract_version: 1,
            engine_version: env!("CARGO_PKG_VERSION").into(),
            source: source.into(),
            status: MetadataStatus::Failed,
            technical: None,
            entries: vec![],
            named_tags: BTreeMap::new(),
            artwork: vec![],
            opaque_metadata: vec![],
            text_limits: TextLimits { complete: true, ..Default::default() },
            observations: MetadataObservations::default(),
            diagnostics: vec![],
            caveats: vec![
                "Editable tags, encoder text and MQA claims do not establish audio history and never change a score.".into(),
                "Technical metadata is parsed/derived from headers; no audio was decoded or checksum verified.".into(),
                "Named fields select the first retained entry in container order; raw duplicates remain available.".into(),
            ],
        }
    }

    pub fn named_value(&self, field: &str) -> Option<&str> {
        self.named_tags
            .get(field)?
            .first()
            .and_then(|&index| self.entries.get(index))?
            .value
            .as_deref()
    }
}

/// Metadata-only path convenience wrapper. No decoder or external process runs.
pub fn read_metadata_path(
    path: impl AsRef<Path>,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
) -> MetadataReport {
    let path = path.as_ref();
    match File::open(path) {
        Ok(file) => read_metadata_source(Box::new(file), &path.to_string_lossy(), options, cancel),
        Err(error) => {
            let mut report = MetadataReport::new(&path.to_string_lossy());
            report.diagnostics.push(Diagnostic {
                code: "invalid_input".into(),
                message: error.to_string(),
            });
            report
        }
    }
}

/// Seekable app-owned source. Uses the core worker/deadline/source guard, without
/// decoding PCM. max_seconds does not restrict metadata; all tag chunks are read.
/// WAV/FLAC contain one track (0); F01/F02 must bind new formats explicitly.
pub fn read_metadata_source(
    source: Box<dyn MediaSource>,
    name: &str,
    options: &AnalysisOptions,
    cancel: &CancellationToken,
) -> MetadataReport {
    use crate::decode::Failure;
    let mut report = MetadataReport::new(name);
    let start = Instant::now();
    let control = || {
        if cancel.is_cancelled() {
            Err(Failure::Cancelled)
        } else if start.elapsed() >= options.deadline {
            Err(Failure::TimedOut)
        } else {
            Ok(())
        }
    };
    let result = (|| {
        control()?;
        if !source.is_seekable() {
            return Err(Failure::Unsupported(
                "Metadata requires a seekable source".into(),
            ));
        }
        if options.track_id.is_some_and(|id| id != 0) {
            return Err(Failure::Invalid(
                "Requested audio track is not present".into(),
            ));
        }
        let _worker = crate::worker::ANALYSIS_WORKER.acquire(control)?;
        let size = source.byte_len();
        let (mut source, fault) =
            crate::source::GuardedSource::new(source, cancel.clone(), options.deadline, start);
        let mut collector = Collector {
            report: &mut report,
        };
        let inspected = crate::container::inspect(
            &mut source,
            || control().is_ok(),
            &mut collector,
        )
        .map_err(|error| match error {
            crate::container::Error::Unsupported(_) => Failure::Unsupported(error.to_string()),
            crate::container::Error::Interrupted => control().err().unwrap_or(Failure::Cancelled),
            _ => Failure::Decode(error.to_string()),
        });
        fault.check(inspected)?;
        control()?;
        if let Some(technical) = &mut report.technical {
            technical.file_size_bytes = size;
        }
        Ok(())
    })();
    match result {
        Ok(()) => {
            report.status = MetadataStatus::Available;
            report.observations = observations(&report);
        }
        Err(error) => {
            let (status, code) = match error {
                Failure::Cancelled => (MetadataStatus::Cancelled, "cancelled"),
                Failure::TimedOut => (MetadataStatus::TimedOut, "deadline_exceeded"),
                Failure::Unsupported(_) => (MetadataStatus::Unsupported, "unsupported"),
                Failure::Invalid(_) => (MetadataStatus::Failed, "invalid_input"),
                Failure::Decode(_) => (MetadataStatus::Failed, "decode_error"),
            };
            report.status = status;
            report.text_limits.complete = false;
            report.diagnostics.push(Diagnostic {
                code: code.into(),
                message: error.to_string(),
            });
        }
    }
    report
}

fn bounded(text: &str, cap: usize) -> &str {
    let mut end = text.len().min(cap);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn named_field(key: &str) -> Option<&'static str> {
    match key.to_ascii_uppercase().as_str() {
        "TITLE" | "TRACK" | "INAM" => Some("title"),
        "ALBUM" | "IPRD" => Some("album"),
        "DATE" | "YEAR" | "RECORDED_DATE" | "ICRD" => Some("date"),
        "ALBUMARTIST" | "ALBUM_ARTIST" | "ALBUM ARTIST" | "ALBUM_PERFORMER" => Some("album_artist"),
        "ARTIST" | "PERFORMER" | "IART" => Some("artist"),
        "BPM" | "IBPM" => Some("bpm"),
        "COMMENTQUALITY" => Some("comment_quality"),
        "COMMENT" | "COMMENTS" | "DESCRIPTION" | "ICMT" => Some("comments"),
        "REPLAYGAIN_TRACK_GAIN" => Some("replaygain_track_gain"),
        "REPLAYGAIN_ALBUM_GAIN" => Some("replaygain_album_gain"),
        "VENDOR" | "ENCODER" | "ENCODED_LIBRARY" | "WRITING_LIBRARY" | "ISFT" => {
            Some("writing_library")
        }
        "FORMAT_PROFILE" => Some("format_profile"),
        _ => None,
    }
}

pub(crate) struct Collector<'a> {
    pub report: &'a mut MetadataReport,
}
impl Collector<'_> {
    pub fn tag(
        &mut self,
        location: (&str, u64, usize),
        key: &[u8],
        value: &[u8],
        malformed: bool,
        artwork: Option<usize>,
    ) {
        let (source, offset, order) = location;
        let limits = &mut self.report.text_limits;
        limits.total_entries += 1;
        let decoded_key = String::from_utf8_lossy(key);
        let decoded_value = if artwork.is_some() {
            "".into()
        } else {
            String::from_utf8_lossy(value)
        };
        let invalid = matches!(decoded_key, std::borrow::Cow::Owned(_))
            || matches!(decoded_value, std::borrow::Cow::Owned(_));
        let bytes = decoded_key.len() + decoded_value.len();
        limits.total_text_utf8_bytes += bytes as u64;
        limits.invalid_utf8_entries += u64::from(invalid);
        limits.malformed_entries += u64::from(malformed);
        limits.complete &= !invalid && !malformed;
        if self.report.entries.len() >= MAX_TAGS
            || limits.retained_text_utf8_bytes >= MAX_TEXT_BYTES as u64
        {
            limits.omitted_entries += 1;
            limits.omitted_text_utf8_bytes += bytes as u64;
            limits.complete = false;
            return;
        }
        let remaining = MAX_TEXT_BYTES - limits.retained_text_utf8_bytes as usize;
        let retained_key = bounded(&decoded_key, remaining.min(MAX_KEY_BYTES));
        let retained_value = bounded(
            &decoded_value,
            (remaining - retained_key.len()).min(MAX_VALUE_BYTES),
        );
        let key_truncated = retained_key.len() != decoded_key.len();
        let value_truncated = retained_value.len() != decoded_value.len();
        let retained_bytes = retained_key.len() + retained_value.len();
        limits.retained_text_utf8_bytes += retained_bytes as u64;
        limits.omitted_text_utf8_bytes += (bytes - retained_bytes) as u64;
        limits.truncated_entries += u64::from(key_truncated || value_truncated);
        limits.complete &= !key_truncated && !value_truncated;
        if !key_truncated
            && !matches!(decoded_key, std::borrow::Cow::Owned(_))
            && !malformed
            && artwork.is_none()
        {
            if let Some(field) = named_field(retained_key) {
                self.report
                    .named_tags
                    .entry(field.into())
                    .or_default()
                    .push(self.report.entries.len());
            }
        }
        self.report.entries.push(TagEntry {
            source: source.into(),
            source_offset: offset,
            source_order: order,
            key: retained_key.into(),
            value: artwork.is_none().then(|| retained_value.into()),
            artwork_index: artwork,
            original_key_bytes: key.len() as u64,
            original_value_bytes: value.len() as u64,
            key_truncated,
            value_truncated,
            invalid_utf8: invalid,
            malformed,
        });
    }

    pub fn opaque(&mut self, source: &str, offset: u64, len: u64, reason: &str) {
        self.report.opaque_metadata.push(OpaqueMetadata {
            source: source.into(),
            source_offset: offset,
            byte_length: len,
            reason: reason.into(),
        });
    }
}

const SIGNATURES: &[&str] = &[
    "lame",
    "libmp3lame",
    "fraunhofer",
    " fhg",
    "nero aac",
    "fdk-aac",
    "320kbps",
    "320 kbps",
    "v0 (vbr",
    "joint stereo",
    "xing",
];

fn mqa_claim(text: &str) -> bool {
    let lower = text.to_lowercase();
    for (start, _) in lower.match_indices("mqa") {
        if lower[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        {
            continue;
        }
        let tail = &lower[start + 3..];
        let boundary = |s: &str| !s.chars().next().is_some_and(|c| c.is_ascii_alphanumeric());
        if boundary(tail) {
            return true;
        }
        let tail = tail.trim_start();
        if ["encoder", "encode", "studio"]
            .iter()
            .any(|suffix| tail.strip_prefix(suffix).is_some_and(boundary))
        {
            return true;
        }
    }
    false
}

fn observations(report: &MetadataReport) -> MetadataObservations {
    let mut out = MetadataObservations {
        text_scan_complete: report.text_limits.complete && report.opaque_metadata.is_empty(),
        ..Default::default()
    };
    let ext = Path::new(&report.source)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    out.legacy_extension_eligible =
        ["flac", "wav", "alac", "m4a", "ape", "wv", "aiff", "aif"].contains(&ext.as_str());
    let other = |index: usize| {
        !report
            .named_tags
            .values()
            .any(|indices| indices.contains(&index))
    };
    let named = |field: &str, index: usize| {
        report
            .named_tags
            .get(field)
            .is_some_and(|indices| indices.contains(&index))
    };
    let mut encoder_text = String::new();
    let mut encoder_spans = Vec::new();
    for (index, entry) in report.entries.iter().enumerate() {
        let value = entry.value.as_deref().unwrap_or("");
        let unknown = other(index) && !entry.key.to_ascii_lowercase().starts_with("replaygain");
        if unknown
            || named("comments", index)
            || named("comment_quality", index)
            || named("writing_library", index)
        {
            if !encoder_text.is_empty() {
                encoder_text.push(' ');
            }
            let start = encoder_text.len();
            encoder_text.push_str(&value.to_lowercase().replace("mp3tag", ""));
            encoder_spans.push((index, start, encoder_text.len()));
        }
        if (unknown
            || named("writing_library", index)
            || named("format_profile", index)
            || named("comment_quality", index))
            && (mqa_claim(&entry.key) || mqa_claim(value))
        {
            out.mqa_metadata_claimed = true;
            out.mqa_tag_indices.push(index);
        }
    }
    for sig in SIGNATURES {
        for (start, _) in encoder_text.match_indices(sig) {
            out.encoder_signatures.push(sig.trim().to_string());
            out.encoder_tag_indices
                .extend(encoder_spans.iter().filter_map(|&(index, left, right)| {
                    (start < right && start + sig.len() > left).then_some(index)
                }));
        }
    }
    out.encoder_signatures.sort();
    out.encoder_signatures.dedup();
    out.encoder_tag_indices.sort_unstable();
    out.encoder_tag_indices.dedup();
    if !out.encoder_signatures.is_empty() {
        let hits = out.encoder_signatures.join(", ");
        out.encoder_summary = Some(format!(
            "Encoder text in editable metadata: {hits}; audio history is unknown."
        ));
        if out.legacy_extension_eligible {
            out.legacy_encoder_trace = Some(format!(
                "⚠ Lossy encoder fingerprint in metadata: {hits} — tags survived a transcode"
            ));
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayGainStatus {
    Available,
    MissingTag,
    MissingLoudness,
    InvalidTag,
    InvalidLoudness,
    UnavailableInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayGainAudit {
    pub audit_version: u32,
    pub method_id: String,
    pub status: ReplayGainStatus,
    pub stored_raw: String,
    pub stored_gain_db: Option<f64>,
    pub measured_lufs: Option<f64>,
    pub implied_legacy_level_lufs: Option<f64>,
    pub absolute_delta_db: Option<f64>,
    pub comparison: Option<String>,
    pub display_summary: Option<String>,
    /// Exact pinned tuple on finite valid inputs. Raw wording is audit-only.
    pub legacy_outputs: [String; 4],
    pub measurement_method: Option<String>,
    pub measurement_schema_version: Option<String>,
    pub measurement_policy_version: Option<String>,
    pub decoded_pcm_sha256: Option<String>,
    pub sample_rate_hz: Option<u32>,
    pub analyzed_frames: Option<u64>,
    pub loudness_interval: Option<AnalysisInterval>,
    pub caveats: Vec<String>,
}

/// Pinned arithmetic: strip all but decimal digits/dot/minus from the first
/// stored token, compare abs((-18 + stored) - measured), strict 1/3 dB gates.
/// This is a legacy comparison, not ReplayGain compliance or ancestry evidence.
pub fn audit_replaygain(stored_raw: &str, measured_lufs: &str) -> ReplayGainAudit {
    let stored = stored_raw.trim();
    let mut out = ReplayGainAudit {
        audit_version: 1, method_id: "python-reference-c6ecce2-replaygain-v1".into(),
        status: ReplayGainStatus::MissingTag, stored_raw: stored.into(),
        stored_gain_db: None, measured_lufs: None, implied_legacy_level_lufs: None, absolute_delta_db: None,
        comparison: None, display_summary: None,
        legacy_outputs: [stored.into(), measured_lufs.into(), String::new(), String::new()],
        measurement_method: None, measurement_schema_version: None, measurement_policy_version: None,
        decoded_pcm_sha256: None, sample_rate_hz: None, analyzed_frames: None, loudness_interval: None,
        caveats: vec!["D11: pinned -18 + stored arithmetic and strict 1/3 dB boundaries; no standard-compliance or re-encoding conclusion.".into()],
    };
    if stored.is_empty() {
        return out;
    }
    if measured_lufs.is_empty() {
        out.status = ReplayGainStatus::MissingLoudness;
        return out;
    }
    let token = stored.split_whitespace().next().unwrap_or("");
    let filtered: String = token
        .chars()
        .filter_map(|c| {
            if c == '.' || c == '-' {
                Some(c)
            } else {
                decimal_digit(c).map(|d| char::from(b'0' + d))
            }
        })
        .collect();
    let Ok(gain) = filtered.parse::<f64>() else {
        out.status = ReplayGainStatus::InvalidTag;
        return out;
    };
    if !gain.is_finite() {
        out.status = ReplayGainStatus::InvalidTag;
        return out;
    }
    let Ok(measured) = measured_lufs.trim().parse::<f64>() else {
        out.status = ReplayGainStatus::InvalidLoudness;
        return out;
    };
    let implied = -18.0 + gain;
    let delta = (implied - measured).abs();
    if !measured.is_finite() || !delta.is_finite() {
        out.status = ReplayGainStatus::InvalidLoudness;
        return out;
    }
    let (comparison, legacy) = if delta < 1.0 {
        ("matches", "✓ RG tag matches measured loudness".into())
    } else if delta < 3.0 {
        (
            "minor_discrepancy",
            format!("~ {delta:.1} dB mismatch — minor discrepancy"),
        )
    } else {
        (
            "discrepancy",
            format!("⚠ {delta:.1} dB mismatch — file may have been re-encoded after tagging"),
        )
    };
    out.status = ReplayGainStatus::Available;
    out.stored_gain_db = Some(gain);
    out.measured_lufs = Some(measured);
    out.implied_legacy_level_lufs = Some(implied);
    out.absolute_delta_db = Some(delta);
    out.comparison = Some(comparison.into());
    out.display_summary = Some(format!(
        "Reference-method ReplayGain comparison: {comparison}, {delta:.1} dB; cause unknown."
    ));
    out.legacy_outputs = [
        stored.into(),
        format!("{measured:.2} LUFS"),
        format!("{delta:.1} dB"),
        legacy,
    ];
    out
}

// Unicode decimal-digit blocks used by Python's \d/float; no regex dependency.
fn decimal_digit(c: char) -> Option<u8> {
    const STARTS: &[u32] = &[
        0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66,
        0xde6, 0xe50, 0xed0, 0xf20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80, 0x1a90,
        0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0, 0xaa50, 0xabf0,
        0xff10, 0x104a0, 0x10d30, 0x11066, 0x110f0, 0x11136, 0x111d0, 0x112f0, 0x11450, 0x114d0,
        0x11650, 0x116c0, 0x11730, 0x118e0, 0x11950, 0x11c50, 0x11d50, 0x11da0, 0x11f50, 0x16a60,
        0x16ac0, 0x16b50, 0x1d7ce, 0x1d7d8, 0x1d7e2, 0x1d7ec, 0x1d7f6, 0x1e140, 0x1e2f0, 0x1e4f0,
        0x1e950, 0x1fbf0,
    ];
    let n = c as u32;
    STARTS
        .iter()
        .find_map(|&start| (n >= start && n < start + 10).then(|| (n - start) as u8))
}

/// Bind the legacy comparison to the native meter's actual prefix/hash/domain.
/// First duplicate wins; a truncated/malformed selected gain is unavailable.
pub fn audit_metadata_replaygain(
    metadata: &MetadataReport,
    measured: &AnalysisReport,
) -> ReplayGainAudit {
    let raw = metadata.named_value("replaygain_track_gain").unwrap_or("");
    let lufs = measured
        .loudness
        .as_ref()
        .and_then(|l| l.integrated_lufs)
        .map(|v| v.to_string())
        .unwrap_or_default();
    let mut audit = audit_replaygain(raw, &lufs);
    let tag_unusable = metadata
        .named_tags
        .get("replaygain_track_gain")
        .and_then(|v| v.first())
        .and_then(|&i| metadata.entries.get(i))
        .is_some_and(|e| e.value_truncated || e.invalid_utf8 || e.malformed);
    if metadata.status != MetadataStatus::Available
        || measured.status != FileStatus::Analyzed
        || metadata.source != measured.source
        || tag_unusable
        || metadata.metadata_version != METADATA_VERSION
        || metadata.contract_version != 1
        || (!metadata.text_limits.complete && raw.is_empty())
        || measured.schema_version != crate::model::SCHEMA_VERSION
        || measured.policy_version != crate::model::POLICY_VERSION
    {
        audit = audit_replaygain(raw, "");
        audit.status = ReplayGainStatus::UnavailableInput;
        audit.caveats.push("Metadata/measurement binding unavailable, unequal, truncated or unsupported; no stale loudness comparison.".into());
        return audit;
    }
    match (&metadata.technical, &measured.stream, &measured.coverage) {
        (Some(technical), Some(stream), Some(_))
            if technical.selected_track_id == stream.track_id
                && technical.declared_sample_rate_hz == Some(stream.sample_rate)
                && technical.declared_channels == Some(stream.channels as u32) => {}
        _ => {
            audit = audit_replaygain(raw, "");
            audit.status = ReplayGainStatus::UnavailableInput;
            return audit;
        }
    }
    audit.measurement_method = Some("native-rate-bs1770-two-pass-v1".into());
    audit.measurement_schema_version = Some(measured.schema_version.clone());
    audit.measurement_policy_version = Some(measured.policy_version.clone());
    audit.sample_rate_hz = measured.stream.as_ref().map(|s| s.sample_rate);
    if let Some(coverage) = &measured.coverage {
        audit.decoded_pcm_sha256 = Some(coverage.decoded_pcm_sha256.clone());
        audit.analyzed_frames = Some(coverage.analyzed_frames);
    }
    audit.loudness_interval = measured.loudness.as_ref().and_then(|l| l.interval.clone());
    audit.caveats.push("D04: existing native-rate meter on its declared interval, not the Python full-file 48 kHz FFmpeg measurement; metadata itself has no PCM hash. Caller must supply both reports from the same source snapshot; matching names/headers alone cannot verify that identity.".into());
    audit
}
