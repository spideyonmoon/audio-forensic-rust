use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub const SCHEMA_VERSION: &str = "0.5.0";
pub const POLICY_VERSION: &str = "observations-only-v5";

#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    pub track_id: Option<u32>,
    /// All measurements use [0, max_seconds); None reads the full selected stream.
    pub max_seconds: Option<f64>,
    /// Cooperative deadline between packets/transform batches, not an interruptible I/O timeout.
    pub deadline: Duration,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            track_id: None,
            max_seconds: None,
            deadline: Duration::from_secs(600),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);
impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Analyzed,
    Failed,
    Unsupported,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DetectorStatus {
    Measured,
    Hit,
    NotDetected,
    Inconclusive,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub track_id: u32,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: usize,
    pub bits_per_sample: Option<u32>,
    pub declared_frames: Option<u64>,
    pub integer_pcm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coverage {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub analyzed_frames: u64,
    pub reached_end: bool,
    pub requested_max_seconds: Option<f64>,
    pub length_matches_header: Option<bool>,
    pub decoder_verification: Option<bool>,
    pub analysis_passes: u8,
    pub decoded_pcm_sha256: String,
    pub hash_sample_encoding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMeasurements {
    pub channel_index: usize,
    pub samples: u64,
    pub peak: f64,
    pub rms: f64,
    pub dc_offset: f64,
    pub zero_samples: u64,
    pub full_scale_samples: u64,
    /// Deepest exercised MSB-aligned integer bit; unknown for floats or silence.
    pub exact_used_bits: Option<u32>,
    /// Reference algorithm: >=500 nonzero samples, threshold max(8, floor(n*1e-4)).
    pub effective_bits: Option<u32>,
    pub spectral: SpectralMeasurements,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpectralMeasurements {
    pub frames: u64,
    pub active_frames: u64,
    pub cutoff_p95_hz: Option<f64>,
    pub cutoff_variance_hz2: Option<f64>,
    pub cliff_depth_db: Option<f64>,
    pub sharpness_db_per_bin: Option<f64>,
    /// Magnitude ratio, matching the Python method; not a power ratio.
    pub hf_magnitude_ratio: Option<f64>,
    pub entropy_bits: Option<f64>,
    /// Unnormalized FFT magnitude reference; not calibrated dBFS.
    pub noise_above_cutoff_db: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorResult {
    pub id: String,
    pub version: u32,
    pub family: String,
    pub status: DetectorStatus,
    /// None denotes a joint-channel observation such as the stereo MQA scan.
    pub channel_index: Option<usize>,
    pub intervals: Vec<AnalysisInterval>,
    pub measurements: BTreeMap<String, f64>,
    pub thresholds: BTreeMap<String, f64>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisInterval {
    pub start_frame: u64,
    pub end_frame: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecWallCandidate {
    pub codec: String,
    pub profile: String,
    pub reference_wall_hz: f64,
    pub tolerance_hz: f64,
    pub distance_hz: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentProbe {
    pub interval: AnalysisInterval,
    pub peak_dbfs: Option<f64>,
    pub cutoff_hz: Option<f64>,
    pub cliff_db: Option<f64>,
    pub high_band_relative_db: Option<f64>,
    pub above_cutoff_relative_db: Option<f64>,
    pub occupied_band_fraction: Option<f64>,
    pub active: bool,
    pub eligible: bool,
    pub wall_observed: bool,
    pub codec_candidates: Vec<CodecWallCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentAnalysis {
    pub channel_index: usize,
    pub status: DetectorStatus,
    pub active_probes: usize,
    pub eligible_probes: usize,
    pub wall_probes: usize,
    /// Describes the sampled probes only, not duration coverage or source history.
    pub pattern: String,
    pub probes: Vec<SegmentProbe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqaCandidate {
    pub source_bit_plane: u32,
    /// Inclusive index of the last sample carrying the 36-bit marker.
    pub sync_end_frame: u64,
    pub rate_code: Option<u8>,
    pub rate_field_hz: Option<u32>,
    pub provenance_code: Option<u8>,
    pub studio_flag: Option<bool>,
    pub payload_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqaObservation {
    pub status: DetectorStatus,
    pub scanned_frames: u64,
    pub metadata_evaluated: bool,
    pub sync_matches: u64,
    pub candidates_truncated: bool,
    pub candidates: Vec<MqaCandidate>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResamplingCandidate {
    /// Hypothesis tested, not a verified original recording rate.
    pub source_rate: u32,
    pub edge_relative_db: Option<f64>,
    pub notch_relative_db: Option<f64>,
    pub above_relative_db: Option<f64>,
    pub ceiling_relative_db: Option<f64>,
    pub mirror_correlation: Option<f64>,
    pub eligible: bool,
    pub modes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResamplingAnalysis {
    pub channel_index: usize,
    pub status: DetectorStatus,
    pub active_frames: u64,
    pub candidates: Vec<ResamplingCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VorbisProbe {
    pub interval: AnalysisInterval,
    /// RMS of the initial 2048 samples, used only for anchor selection.
    pub anchor_rms: f64,
    pub active: bool,
    pub baseline_zero_fraction: Option<f64>,
    pub maximum_zero_fraction: Option<f64>,
    pub selected_phase_zero_fraction: Option<f64>,
    pub supports_selected_phase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VorbisAnalysis {
    pub channel_index: usize,
    pub status: DetectorStatus,
    pub search_limit_frames: u64,
    /// Excess zero-coefficient fraction over off-grid baselines; not probability.
    pub zero_excess_score: Option<f64>,
    /// Offset within each anchor, modulo 128 samples; not an absolute file phase.
    pub phase_mod_128: Option<usize>,
    pub supporting_probes: usize,
    pub active_probes: usize,
    pub probes: Vec<VorbisProbe>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AacBasis {
    Mono,
    /// (L + R) / 2, computed in f64 only for this detector.
    Mid,
    /// (L - R) / 2, computed in f64 only for this detector.
    Side,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AacProbe {
    /// Initial 2048-sample energy window starts here, on the 1024-sample grid.
    pub anchor_frame: u64,
    /// Union of the phase-search windows for this anchor.
    pub interval: AnalysisInterval,
    pub anchor_rms: f64,
    pub selected_eligible_bands: Option<usize>,
    pub selected_flagged_bands: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AacAnalysis {
    pub basis: AacBasis,
    pub status: DetectorStatus,
    pub search_limit_frames: u64,
    /// Mean flagged fraction of all 49 bands at the selected phase/scale, not probability.
    pub lattice_score: Option<f64>,
    /// Offset from each probe's interval start (anchor minus 512), stepped by eight.
    pub phase_offset: Option<usize>,
    /// Index 0..7, interpolating 0.3..0.7 of each band's dead-zone scalefactor.
    pub scalefactor_index: Option<usize>,
    pub eligible_probes: usize,
    pub probes: Vec<AacProbe>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralStructureAnalysis {
    pub channel_index: usize,
    pub scatter_status: DetectorStatus,
    pub phase_status: DetectorStatus,
    /// Envelope of evaluated STFT frames, excluding unframed tail samples.
    /// Active-frame and per-bin gates still exclude observations inside this interval.
    pub interval: Option<AnalysisInterval>,
    pub stft_frames: u64,
    pub active_frames: u64,
    pub bound_frames: u64,
    pub flat_frames: u64,
    pub average_scatter_bound_hz: Option<f64>,
    /// Most frequent exact FFT bound bin; ties use the lowest frequency.
    pub modal_scatter_bound_hz: Option<f64>,
    pub phase_band_start_hz: Option<f64>,
    /// Adjacent active STFT pairs with at least two bins above both energy floors.
    pub phase_frame_pairs: u64,
    pub phase_bin_pairs: u64,
    pub high_band_phase_entropy_bits: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub schema_version: String,
    pub engine_version: String,
    pub policy_version: String,
    pub source: String,
    pub status: FileStatus,
    pub stream: Option<StreamInfo>,
    pub coverage: Option<Coverage>,
    pub channels: Vec<ChannelMeasurements>,
    pub detectors: Vec<DetectorResult>,
    pub segments: Vec<SegmentAnalysis>,
    pub mqa: Option<MqaObservation>,
    pub resampling: Vec<ResamplingAnalysis>,
    pub vorbis: Vec<VorbisAnalysis>,
    pub aac: Vec<AacAnalysis>,
    pub spectral_structure: Vec<SpectralStructureAnalysis>,
    pub unimplemented_detectors: Vec<String>,
    pub ancestry_verdict: String,
    pub evidence_index: Option<f64>,
    pub diagnostics: Vec<Diagnostic>,
    pub limitations: Vec<String>,
}

impl AnalysisReport {
    pub(crate) fn new(source: String) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.into(), engine_version: env!("CARGO_PKG_VERSION").into(),
            policy_version: POLICY_VERSION.into(), source, status: FileStatus::Failed,
            stream: None, coverage: None, channels: vec![], detectors: vec![], segments: vec![], mqa: None,
            resampling: vec![], vorbis: vec![], aac: vec![], spectral_structure: vec![],
            unimplemented_detectors: [
                "analog_source", "mqa_confirmation",
                "bit_depth_noise_floor", "loudness", "psychoacoustic_artifacts"]
                .into_iter().map(str::to_owned).collect(),
            ancestry_verdict: "INCONCLUSIVE".into(), evidence_index: None, diagnostics: vec![],
            limitations: vec![
                "Measurements and provisional observations only: the complete forensic detector suite and scoring are not implemented.".into(),
                "Source history is unverified. Neither low bandwidth nor exercised bits proves provenance.".into(),
                "Native channels are measured independently. AAC additionally declares a mono or mid/side analysis basis.".into(),
            ],
        }
    }
}
