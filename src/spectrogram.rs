//! Bounded display spectrum; separate from detector STFTs and measurement JSON.
use crate::{
    AnalysisReport, CancellationToken, FileStatus,
    decode::Failure,
    model::{AnalysisInterval, Coverage},
    reference_inputs::reference_basis,
};
use rustfft::{Fft, FftPlanner, num_complex::Complex32};
use serde::{Deserialize, Serialize};
use std::{
    fs::{OpenOptions, remove_file},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

pub const ARTIFACT_VERSION: u32 = 1;
pub const METHOD_ID: &str = "hann1024-power-pair-merge-v1";
pub const MAX_COLUMNS: usize = 1280;
pub const FFT_SIZE: usize = 1024;
pub const HOP_FRAMES: usize = 512;
pub const FREQUENCY_ROWS: usize = 513;
pub const FLOOR_DB: f32 = -140.0;
pub const CEILING_DB: f32 = 0.0;
const CELLS: usize = MAX_COLUMNS * FREQUENCY_ROWS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStatus {
    Available,
    Unavailable,
    Failed,
    Unsupported,
    Cancelled,
    TimedOut,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn coverage(frames: u64) -> Coverage {
        Coverage {
            start_seconds: 0.0,
            end_seconds: frames as f64 / 48000.0,
            analyzed_frames: frames,
            reached_end: true,
            requested_max_seconds: None,
            length_matches_header: None,
            decoder_verification: None,
            analysis_passes: 2,
            decoded_pcm_sha256: "0".repeat(64),
            hash_sample_encoding: "f64le".into(),
        }
    }

    #[test]
    fn power_matches_frozen_independent_direct_dft_including_dc_and_nyquist() {
        let oracle: Value =
            serde_json::from_str(include_str!("../tests/fixtures/spectrogram_reference.json"))
                .unwrap();
        for case in oracle["cases"].as_array().unwrap() {
            let mut collector = SpectrogramCollector::new(48000, 1).unwrap();
            for i in 0..=FFT_SIZE {
                let sample = match case["name"].as_str().unwrap() {
                    "tone64" => {
                        (0.5 * (std::f64::consts::TAU * 64.0 * i as f64 / FFT_SIZE as f64).sin())
                            as f32
                    }
                    "dc" => 0.25,
                    "impulse" => {
                        if i == 512 {
                            0.75
                        } else {
                            0.0
                        }
                    }
                    "nyquist" => {
                        if i % 2 == 0 {
                            0.25
                        } else {
                            -0.25
                        }
                    }
                    _ => unreachable!(),
                };
                collector.push(0, f64::from(sample), || Ok(())).unwrap();
            }
            let artifact = collector.finish(&coverage(1025), || Ok(())).unwrap();
            artifact.validate().unwrap();
            let d = artifact.data.unwrap();
            let mut total = 0.0;
            for (db, expected) in d
                .power_db
                .iter()
                .zip(case["power_per_bin"].as_array().unwrap())
            {
                let power = 10f64.powf(f64::from(*db) / 10.0);
                assert!(
                    (power - expected.as_f64().unwrap()).abs() <= 2e-7,
                    "{}: {power} vs {expected}",
                    case["name"]
                );
                total += power;
            }
            assert!((total - case["total_power"].as_f64().unwrap()).abs() < 2e-7);
        }
    }

    #[test]
    fn two_coarsenings_preserve_linear_power_counts_tail_and_bounded_payload() {
        let n = 2561u64;
        let frames = (n - 1) * HOP_FRAMES as u64 + FFT_SIZE as u64 + 1;
        let signal = |i: u64| if i < frames / 3 { 0.25 } else { 0.75 };
        let mut collector = SpectrogramCollector::new(48000, 1).unwrap();
        let allocated = collector.sums.capacity() * 8
            + collector.counts.capacity() * 8
            + collector.window.capacity() * 4
            + collector.pending.capacity() * 4
            + collector.buffer.capacity() * 8
            + collector.scratch.capacity() * 8;
        eprintln!(
            "P06 collector array payload: {allocated} bytes; FFT scratch: {} bytes",
            collector.scratch.capacity() * 8
        );
        assert!(allocated <= 5_300_000);
        let mut expected = 0.0;
        for row in 0..n {
            let mut p = 0.0;
            for (i, &w) in collector.window.iter().enumerate() {
                let x = (signal(row * HOP_FRAMES as u64 + i as u64) as f32 * w) as f64;
                p += x * x;
            }
            expected += p / collector.window_energy;
        }
        for i in 0..frames {
            collector.push(0, signal(i), || Ok(())).unwrap();
        }
        assert_eq!(collector.sums.len(), CELLS);
        assert_eq!(collector.sums.capacity(), CELLS);
        let artifact = collector.finish(&coverage(frames), || Ok(())).unwrap();
        artifact.validate().unwrap();
        let d = artifact.data.unwrap();
        assert_eq!(d.bucket_stft_frames, 4);
        assert_eq!(d.time_columns.len(), 641);
        assert_eq!(d.time_columns.last().unwrap().stft_frames, 1);
        assert_eq!(d.time_columns.iter().map(|c| c.stft_frames).sum::<u64>(), n);
        assert_eq!(d.unwindowed_tail_frames, 1);
        let observed: f64 = d
            .power_db
            .chunks_exact(FREQUENCY_ROWS)
            .zip(&d.time_columns)
            .map(|(bins, c)| {
                bins.iter()
                    .map(|&db| 10f64.powf(f64::from(db) / 10.0))
                    .sum::<f64>()
                    * c.stft_frames as f64
            })
            .sum();
        assert!(
            (observed - expected).abs() / expected < 2e-6,
            "{observed} vs {expected}"
        );
    }

    #[test]
    fn collector_controls_interrupt_fft_merge_and_finalization() {
        let mut c = SpectrogramCollector::new(48000, 1).unwrap();
        for _ in 0..FFT_SIZE {
            c.push(0, 0.0, || Ok(())).unwrap();
        }
        assert!(matches!(
            c.push(0, 0.0, || Err(Failure::Cancelled)),
            Err(Failure::Cancelled)
        ));
        let mut c = SpectrogramCollector::new(48000, 1).unwrap();
        // The full-bucket state exercises the finite merge independently of FFT work.
        c.stft_frames = MAX_COLUMNS as u64;
        c.counts.fill(1);
        c.pending.resize(FFT_SIZE, 0.0);
        let mut checks = 0;
        assert!(matches!(
            c.push(0, 0.0, || {
                checks += 1;
                if checks == 4 {
                    Err(Failure::TimedOut)
                } else {
                    Ok(())
                }
            }),
            Err(Failure::TimedOut)
        ));
        assert_eq!(checks, 4);
        assert!(matches!(
            c.finish(&coverage(10000), || Err(Failure::Cancelled)),
            Err(Failure::Cancelled)
        ));
    }

    #[test]
    fn valid_native_report_survives_artifact_failure_without_phantom_data() {
        let mut report = AnalysisReport::new("generated".into());
        report.status = FileStatus::Analyzed;
        let artifact = SpectrogramArtifact::collector_failure(
            Failure::ResourceLimit("test allocation cap".into()),
            &coverage(1025),
        );
        let result = SpectrogramAnalysis::new(report, Some(artifact));
        assert_eq!(result.measurement.status, FileStatus::Analyzed);
        assert_eq!(result.spectrogram.status, ArtifactStatus::Failed);
        assert!(result.spectrogram.data.is_none());
        assert!(result.spectrogram.coverage.is_some());
        result.spectrogram.validate().unwrap();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DisplayBasis {
    Mono,
    StereoMid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeColumn {
    /// Display bucket, in native PCM frames; final bucket extends to analyzed end.
    pub interval: AnalysisInterval,
    /// Actual overlapping FFT window support (which can extend beyond the bucket).
    pub window_support: AnalysisInterval,
    pub stft_frames: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpectrogramData {
    pub sample_rate_hz: u32,
    pub basis: DisplayBasis,
    pub channel_indices: Vec<usize>,
    pub interval: AnalysisInterval,
    pub spectral_interval: AnalysisInterval,
    pub unwindowed_tail_frames: u64,
    pub fft_size: usize,
    pub hop_frames: usize,
    pub window: String,
    pub frame_boundary: String,
    pub frequency_rows: usize,
    pub frequency_start_hz: f64,
    pub frequency_step_hz: f64,
    pub time_unit: String,
    pub frequency_unit: String,
    pub value_unit: String,
    pub power_reference: f64,
    pub reduction: String,
    pub bucket_stft_frames: u64,
    pub total_stft_frames: u64,
    pub time_columns: Vec<TimeColumn>,
    /// Time-major, then DC-to-Nyquist. Floor applied after linear-power averaging.
    pub power_db: Vec<f32>,
    pub color_floor_db: f32,
    pub color_ceiling_db: f32,
    pub palette: String,
    pub native_peak: f64,
    pub basis_peak: f64,
    pub mid_cancelled_with_native_signal: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpectrogramArtifact {
    pub artifact_version: u32,
    pub contract_version: u32,
    pub method_id: String,
    pub status: ArtifactStatus,
    pub reason: Option<String>,
    pub coverage: Option<Coverage>,
    pub data: Option<SpectrogramData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrogramAnalysis {
    pub measurement: AnalysisReport,
    pub spectrogram: SpectrogramArtifact,
    /// Optional in older saved wrappers; display bitrate never changes measurements.
    #[serde(default)]
    pub presentation: Option<crate::spectrogram_png::SpectrogramPresentation>,
}

impl SpectrogramAnalysis {
    /// Bind a saved artifact to the unchanged native report before display.
    pub fn validate_binding(&self) -> Result<(), RenderError> {
        self.spectrogram.validate()?;
        let invalid =
            || RenderError::Invalid("Spectrogram does not match its measurement report".into());
        if let Some(presentation) = &self.presentation {
            presentation.validate(&self.measurement)?;
        }
        if self.measurement.status != FileStatus::Analyzed {
            if self.spectrogram.data.is_some() || self.spectrogram.coverage.is_some() {
                return Err(invalid());
            }
            return Ok(());
        }
        if let Some(coverage) = &self.spectrogram.coverage {
            if serde_json::to_value(coverage).map_err(|_| invalid())?
                != serde_json::to_value(self.measurement.coverage.as_ref().ok_or_else(invalid)?)
                    .map_err(|_| invalid())?
            {
                return Err(invalid());
            }
        }
        if let Some(data) = &self.spectrogram.data {
            let stream = self.measurement.stream.as_ref().ok_or_else(invalid)?;
            if stream.sample_rate != data.sample_rate_hz
                || stream.channels != data.channel_indices.len()
            {
                return Err(invalid());
            }
        }
        Ok(())
    }

    pub(crate) fn new(measurement: AnalysisReport, artifact: Option<SpectrogramArtifact>) -> Self {
        let spectrogram = if measurement.status == FileStatus::Analyzed {
            artifact.unwrap_or_else(|| {
                SpectrogramArtifact::empty(
                    ArtifactStatus::Failed,
                    "Spectrogram collector did not produce a result".into(),
                )
            })
        } else {
            SpectrogramArtifact::empty(
                match measurement.status {
                    FileStatus::Cancelled => ArtifactStatus::Cancelled,
                    FileStatus::TimedOut => ArtifactStatus::TimedOut,
                    FileStatus::Unsupported => ArtifactStatus::Unsupported,
                    _ => ArtifactStatus::Failed,
                },
                measurement
                    .diagnostics
                    .last()
                    .map(|d| d.message.clone())
                    .unwrap_or_else(|| "Audio analysis did not succeed".into()),
            )
        };
        Self {
            measurement,
            spectrogram,
            presentation: None,
        }
    }

    pub(crate) fn with_presentation(
        mut self,
        presentation: Option<crate::spectrogram_png::SpectrogramPresentation>,
    ) -> Self {
        if self.measurement.status == FileStatus::Analyzed {
            self.presentation = presentation;
        }
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("{0}")]
    Invalid(String),
    #[error("Spectrogram rendering cancelled")]
    Cancelled,
    #[error("Spectrogram rendering exceeded its cooperative deadline")]
    TimedOut,
    #[error("Spectrogram allocation failed")]
    ResourceLimit,
    #[error("PNG encoding failed: {0}")]
    Png(#[from] png::EncodingError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// File identity is host-owned. Only a completed create-new write returns a path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RasterExport {
    pub status: ArtifactStatus,
    pub path: Option<PathBuf>,
    pub reason: Option<String>,
}

pub(crate) fn allocated<T: Clone>(length: usize, value: T) -> Result<Vec<T>, RenderError> {
    let mut out = Vec::new();
    out.try_reserve_exact(length)
        .map_err(|_| RenderError::ResourceLimit)?;
    out.resize(length, value);
    Ok(out)
}

pub(crate) fn render_control(
    cancel: &CancellationToken,
    start: Instant,
    deadline: Duration,
) -> Result<(), RenderError> {
    if cancel.is_cancelled() {
        return Err(RenderError::Cancelled);
    }
    if start.elapsed() >= deadline {
        return Err(RenderError::TimedOut);
    }
    Ok(())
}

impl SpectrogramArtifact {
    pub(crate) fn empty(status: ArtifactStatus, reason: String) -> Self {
        Self {
            artifact_version: ARTIFACT_VERSION,
            contract_version: 1,
            method_id: METHOD_ID.into(),
            status,
            reason: Some(reason),
            coverage: None,
            data: None,
        }
    }

    pub(crate) fn collector_failure(error: Failure, coverage: &Coverage) -> Self {
        let status = match error {
            Failure::Cancelled => ArtifactStatus::Cancelled,
            Failure::TimedOut => ArtifactStatus::TimedOut,
            _ => ArtifactStatus::Failed,
        };
        let mut out = Self::empty(status, error.to_string());
        out.coverage = Some(coverage.clone());
        out
    }

    /// Validate saved descriptors before using their dimensions, axes or pixels.
    pub fn validate(&self) -> Result<(), RenderError> {
        let invalid =
            || RenderError::Invalid("Invalid or unsupported spectrogram descriptor".into());
        if self.artifact_version != ARTIFACT_VERSION
            || self.contract_version != 1
            || self.method_id != METHOD_ID
        {
            return Err(invalid());
        }
        if self.status != ArtifactStatus::Available {
            return if self.data.is_none() && self.reason.as_ref().is_some_and(|s| !s.is_empty()) {
                Ok(())
            } else {
                Err(invalid())
            };
        }
        let d = self.data.as_ref().ok_or_else(invalid)?;
        let c = self.coverage.as_ref().ok_or_else(invalid)?;
        let frames = c.analyzed_frames;
        let n = if frames > FFT_SIZE as u64 {
            (frames - FFT_SIZE as u64 - 1) / HOP_FRAMES as u64 + 1
        } else {
            0
        };
        let mut width = 1u64;
        while n.div_ceil(width) > MAX_COLUMNS as u64 {
            width *= 2;
        }
        let columns = n.div_ceil(width) as usize;
        let channel_ok = match d.basis {
            DisplayBasis::Mono => d.channel_indices == [0] && !d.mid_cancelled_with_native_signal,
            DisplayBasis::StereoMid => d.channel_indices == [0, 1],
        };
        if self.reason.is_some()
            || n == 0
            || !(8000..=384000).contains(&d.sample_rate_hz)
            || !channel_ok
            || c.start_seconds != 0.0
            || c.analysis_passes != 2
            || !matches!(
                c.hash_sample_encoding.as_str(),
                "f64le" | "s32le_msb_aligned"
            )
            || c.end_seconds != frames as f64 / f64::from(d.sample_rate_hz)
            || c.decoded_pcm_sha256.len() != 64
            || !c
                .decoded_pcm_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || d.interval.start_frame != 0
            || d.interval.end_frame != frames
            || d.spectral_interval.start_frame != 0
            || d.spectral_interval.end_frame != (n - 1) * HOP_FRAMES as u64 + FFT_SIZE as u64
            || d.unwindowed_tail_frames != frames - d.spectral_interval.end_frame
            || d.fft_size != FFT_SIZE
            || d.hop_frames != HOP_FRAMES
            || d.window != "symmetric_f32_hann"
            || d.frame_boundary != "window_end_strictly_before_analyzed_end"
            || d.frequency_rows != FREQUENCY_ROWS
            || d.frequency_start_hz != 0.0
            || d.frequency_step_hz != f64::from(d.sample_rate_hz) / FFT_SIZE as f64
            || d.time_unit != "seconds"
            || d.frequency_unit != "Hz"
            || d.value_unit != "dBFS_power_per_bin"
            || d.power_reference != 1.0
            || d.reduction != "linear_power_mean_pairwise_time_merge"
            || d.bucket_stft_frames != width
            || d.total_stft_frames != n
            || d.time_columns.len() != columns
            || d.power_db.len() != columns * FREQUENCY_ROWS
            || d.color_floor_db != FLOOR_DB
            || d.color_ceiling_db != CEILING_DB
            || d.palette != "ember-v1"
            || !d.native_peak.is_finite()
            || !(0.0..=16.0).contains(&d.native_peak)
            || !d.basis_peak.is_finite()
            || !(0.0..=16.0).contains(&d.basis_peak)
            || d.mid_cancelled_with_native_signal
                != (d.basis == DisplayBasis::StereoMid
                    && d.native_peak > 0.0
                    && d.basis_peak == 0.0)
            || d.power_db.iter().any(|v| !v.is_finite() || *v < FLOOR_DB)
        {
            return Err(invalid());
        }
        for (i, col) in d.time_columns.iter().enumerate() {
            let first = i as u64 * width;
            let count = width.min(n - first);
            if col.stft_frames != count
                || col.interval.start_frame != first * HOP_FRAMES as u64
                || col.interval.end_frame
                    != if i + 1 == columns {
                        frames
                    } else {
                        (first + width) * HOP_FRAMES as u64
                    }
                || col.window_support.start_frame != col.interval.start_frame
                || col.window_support.end_frame
                    != (first + count - 1) * HOP_FRAMES as u64 + FFT_SIZE as u64
            {
                return Err(invalid());
            }
        }
        Ok(())
    }

    /// RGB8, row-major, Nyquist at top and time increasing right. Host adds axes
    /// and labels from the descriptor; colors clip to [-140, 0] dBFS/bin.
    pub fn render_rgb(
        &self,
        cancel: &CancellationToken,
        deadline: Duration,
    ) -> Result<Vec<u8>, RenderError> {
        let start = Instant::now();
        render_control(cancel, start, deadline)?;
        self.validate()?;
        let d = self
            .data
            .as_ref()
            .ok_or_else(|| RenderError::Invalid("Spectrogram is unavailable".into()))?;
        let width = d.time_columns.len();
        let mut rgb = allocated(width * FREQUENCY_ROWS * 3, 0u8)?;
        for y in 0..FREQUENCY_ROWS {
            render_control(cancel, start, deadline)?;
            for x in 0..width {
                let color = color(d.power_db[x * FREQUENCY_ROWS + FREQUENCY_ROWS - 1 - y]);
                rgb[(y * width + x) * 3..(y * width + x + 1) * 3].copy_from_slice(&color);
            }
        }
        render_control(cancel, start, deadline)?;
        Ok(rgb)
    }

    /// Portable offline P6 PPM raster. The host selects a new path, and owns PNG
    /// encoding, labels and publication. Existing files are never overwritten.
    pub fn write_ppm_new(
        &self,
        path: impl AsRef<Path>,
        cancel: &CancellationToken,
        deadline: Duration,
    ) -> RasterExport {
        let path = path.as_ref();
        let start = Instant::now();
        let result = self.render_rgb(cancel, deadline).and_then(|rgb| {
            render_control(cancel, start, deadline)?;
            let file = OpenOptions::new().write(true).create_new(true).open(path)?;
            let write_result = (|| {
                let mut writer = BufWriter::new(file);
                let width = self.data.as_ref().unwrap().time_columns.len();
                write!(writer, "P6\n{width} {FREQUENCY_ROWS}\n255\n")?;
                for row in rgb.chunks_exact(width * 3) {
                    render_control(cancel, start, deadline)?;
                    writer.write_all(row)?;
                }
                writer.flush()?;
                render_control(cancel, start, deadline)?;
                Ok::<_, RenderError>(())
            })();
            if write_result.is_err() {
                // Only this invocation's successfully created file is removed.
                let _ = remove_file(path);
            }
            write_result
        });
        match result {
            Ok(()) => RasterExport {
                status: ArtifactStatus::Available,
                path: Some(path.to_owned()),
                reason: None,
            },
            Err(error) => RasterExport {
                status: match error {
                    RenderError::Cancelled => ArtifactStatus::Cancelled,
                    RenderError::TimedOut => ArtifactStatus::TimedOut,
                    _ => ArtifactStatus::Failed,
                },
                path: None,
                reason: Some(error.to_string()),
            },
        }
    }
}

pub(crate) fn color(db: f32) -> [u8; 3] {
    const ANCHORS: [[f32; 3]; 5] = [
        [0., 0., 0.],
        [48., 16., 80.],
        [160., 32., 64.],
        [240., 128., 32.],
        [255., 240., 160.],
    ];
    let p = ((db - FLOOR_DB) / (CEILING_DB - FLOOR_DB)).clamp(0.0, 1.0) * 4.0;
    let i = (p.floor() as usize).min(3);
    let t = p - i as f32;
    std::array::from_fn(|ch| (ANCHORS[i][ch] * (1.0 - t) + ANCHORS[i + 1][ch] * t).round() as u8)
}

pub(crate) struct SpectrogramCollector {
    rate: u32,
    channels: usize,
    left: f64,
    native_peak: f64,
    basis_peak: f64,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    window_energy: f64,
    pending: Vec<f32>,
    buffer: Vec<Complex32>,
    scratch: Vec<Complex32>,
    sums: Vec<f64>,
    counts: Vec<u64>,
    bucket_width: u64,
    stft_frames: u64,
}

impl SpectrogramCollector {
    pub(crate) fn new(rate: u32, channels: usize) -> Result<Self, Failure> {
        let fft = FftPlanner::new().plan_fft_forward(FFT_SIZE);
        let window: Vec<_> = (0..FFT_SIZE)
            .map(|i| {
                (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (FFT_SIZE - 1) as f64).cos())
                    as f32
            })
            .collect();
        let window_energy = window.iter().map(|&x| f64::from(x).powi(2)).sum();
        let alloc = |e: RenderError| Failure::ResourceLimit(e.to_string());
        Ok(Self {
            rate,
            channels,
            left: 0.0,
            native_peak: 0.0,
            basis_peak: 0.0,
            window,
            window_energy,
            pending: Vec::with_capacity(FFT_SIZE),
            buffer: allocated(FFT_SIZE, Complex32::default()).map_err(alloc)?,
            scratch: allocated(fft.get_inplace_scratch_len(), Complex32::default())
                .map_err(alloc)?,
            fft,
            sums: allocated(CELLS, 0.0f64).map_err(alloc)?,
            counts: allocated(MAX_COLUMNS, 0u64).map_err(alloc)?,
            bucket_width: 1,
            stft_frames: 0,
        })
    }

    pub(crate) fn push(
        &mut self,
        ch: usize,
        x: f64,
        mut control: impl FnMut() -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        self.native_peak = self.native_peak.max(x.abs());
        if ch == 0 {
            self.left = x;
        }
        if self.channels == 2 && ch == 0 {
            return Ok(());
        }
        let (mid, _) = reference_basis(self.left, (self.channels == 2).then_some(x));
        self.basis_peak = self.basis_peak.max(f64::from(mid.abs()));
        if self.pending.len() == FFT_SIZE {
            control()?;
            if self.stft_frames / self.bucket_width == MAX_COLUMNS as u64 {
                for dst in 0..MAX_COLUMNS / 2 {
                    control()?;
                    for bin in 0..FREQUENCY_ROWS {
                        self.sums[dst * FREQUENCY_ROWS + bin] = self.sums
                            [2 * dst * FREQUENCY_ROWS + bin]
                            + self.sums[(2 * dst + 1) * FREQUENCY_ROWS + bin];
                    }
                    self.counts[dst] = self.counts[2 * dst] + self.counts[2 * dst + 1];
                }
                self.sums[CELLS / 2..].fill(0.0);
                self.counts[MAX_COLUMNS / 2..].fill(0);
                self.bucket_width = self.bucket_width.checked_mul(2).ok_or_else(|| {
                    Failure::ResourceLimit("Spectrogram time bucket overflow".into())
                })?;
            }
            for (i, &sample) in self.pending.iter().enumerate() {
                self.buffer[i] = Complex32::new(sample * self.window[i], 0.0);
            }
            self.fft
                .process_with_scratch(&mut self.buffer, &mut self.scratch);
            let column = (self.stft_frames / self.bucket_width) as usize;
            for (bin, z) in self.buffer[..FREQUENCY_ROWS].iter().enumerate() {
                let factor = if bin == 0 || bin + 1 == FREQUENCY_ROWS {
                    1.0
                } else {
                    2.0
                };
                let power = (f64::from(z.re).powi(2) + f64::from(z.im).powi(2)) * factor
                    / (FFT_SIZE as f64 * self.window_energy);
                self.sums[column * FREQUENCY_ROWS + bin] += power;
            }
            self.counts[column] += 1;
            self.stft_frames += 1;
            self.pending.copy_within(HOP_FRAMES.., 0);
            self.pending.truncate(FFT_SIZE - HOP_FRAMES);
        }
        self.pending.push(mid);
        Ok(())
    }

    pub(crate) fn finish(
        self,
        coverage: &Coverage,
        mut control: impl FnMut() -> Result<(), Failure>,
    ) -> Result<SpectrogramArtifact, Failure> {
        control()?;
        if self.stft_frames == 0 {
            let mut out = SpectrogramArtifact::empty(
                ArtifactStatus::Unavailable,
                "No complete 1024-frame window strictly before analyzed end".into(),
            );
            out.coverage = Some(coverage.clone());
            return Ok(out);
        }
        let columns = self.stft_frames.div_ceil(self.bucket_width) as usize;
        let mut power_db = allocated(columns * FREQUENCY_ROWS, FLOOR_DB)
            .map_err(|e| Failure::ResourceLimit(e.to_string()))?;
        let mut time_columns = Vec::with_capacity(columns);
        for i in 0..columns {
            control()?;
            for bin in 0..FREQUENCY_ROWS {
                let mean = self.sums[i * FREQUENCY_ROWS + bin] / self.counts[i] as f64;
                power_db[i * FREQUENCY_ROWS + bin] = if mean > 0.0 {
                    (10.0 * mean.log10()).max(f64::from(FLOOR_DB)) as f32
                } else {
                    FLOOR_DB
                };
            }
            let first = i as u64 * self.bucket_width;
            let start_frame = first * HOP_FRAMES as u64;
            time_columns.push(TimeColumn {
                interval: AnalysisInterval {
                    start_frame,
                    end_frame: if i + 1 == columns {
                        coverage.analyzed_frames
                    } else {
                        (first + self.bucket_width) * HOP_FRAMES as u64
                    },
                },
                window_support: AnalysisInterval {
                    start_frame,
                    end_frame: (first + self.counts[i] - 1) * HOP_FRAMES as u64 + FFT_SIZE as u64,
                },
                stft_frames: self.counts[i],
            });
        }
        let spectral_end = (self.stft_frames - 1) * HOP_FRAMES as u64 + FFT_SIZE as u64;
        let mid_cancelled = self.channels == 2 && self.native_peak > 0.0 && self.basis_peak == 0.0;
        let mut warnings = Vec::new();
        if self.channels == 2 {
            warnings.push(
                "Stereo mid combines (L+R)/2; cancellation can hide native channel energy".into(),
            );
        }
        if mid_cancelled {
            warnings.push("Stereo mid is silent while native channels contain signal".into());
        }
        control()?;
        Ok(SpectrogramArtifact {
            artifact_version: ARTIFACT_VERSION,
            contract_version: 1,
            method_id: METHOD_ID.into(),
            status: ArtifactStatus::Available,
            reason: None,
            coverage: Some(coverage.clone()),
            data: Some(SpectrogramData {
                sample_rate_hz: self.rate,
                basis: if self.channels == 1 {
                    DisplayBasis::Mono
                } else {
                    DisplayBasis::StereoMid
                },
                channel_indices: (0..self.channels).collect(),
                interval: AnalysisInterval {
                    start_frame: 0,
                    end_frame: coverage.analyzed_frames,
                },
                spectral_interval: AnalysisInterval {
                    start_frame: 0,
                    end_frame: spectral_end,
                },
                unwindowed_tail_frames: coverage.analyzed_frames - spectral_end,
                fft_size: FFT_SIZE,
                hop_frames: HOP_FRAMES,
                window: "symmetric_f32_hann".into(),
                frame_boundary: "window_end_strictly_before_analyzed_end".into(),
                frequency_rows: FREQUENCY_ROWS,
                frequency_start_hz: 0.0,
                frequency_step_hz: f64::from(self.rate) / FFT_SIZE as f64,
                time_unit: "seconds".into(),
                frequency_unit: "Hz".into(),
                value_unit: "dBFS_power_per_bin".into(),
                power_reference: 1.0,
                reduction: "linear_power_mean_pairwise_time_merge".into(),
                bucket_stft_frames: self.bucket_width,
                total_stft_frames: self.stft_frames,
                time_columns,
                power_db,
                color_floor_db: FLOOR_DB,
                color_ceiling_db: CEILING_DB,
                palette: "ember-v1".into(),
                native_peak: self.native_peak,
                basis_peak: self.basis_peak,
                mid_cancelled_with_native_signal: mid_cancelled,
                warnings,
            }),
        })
    }
}
