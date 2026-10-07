//! Calibrated, self-contained offline PNG composition over unchanged P06 data.
use crate::{
    AnalysisReport, CancellationToken, FileStatus,
    model::{AnalysisInterval, Coverage},
    spectrogram::{
        ArtifactStatus, DisplayBasis, RasterExport, RenderError, SpectrogramAnalysis, allocated,
        color, render_control,
    },
};
use fontdue::{Font, FontSettings};
use serde::{Deserialize, Serialize};
use std::{
    fs::{OpenOptions, remove_file},
    io::{BufWriter, Write},
    path::Path,
    sync::OnceLock,
    time::{Duration, Instant},
};

pub const RENDER_METHOD: &str = "alfred-calibrated-png-v1";
pub const MAX_CANVAS_RGB_BYTES: usize = 3840 * 2160 * 3;
static FONT: OnceLock<Result<Font, String>> = OnceLock::new();

/// Additional presentation input, without widening measurement/artifact v1.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrogramPresentation {
    pub presentation_version: u32,
    pub decoded_pcm_sha256: String,
    pub bitrate_bps: Option<f64>,
    pub bitrate_interval: Option<AnalysisInterval>,
    pub bitrate_method: String,
    pub unavailable_reason: Option<String>,
}

impl SpectrogramPresentation {
    pub(crate) fn from_packets(
        c: &Coverage,
        rate: u32,
        bytes: u64,
        frames: u64,
        stable: bool,
    ) -> Self {
        let available = frames > 0 && stable;
        Self {
            presentation_version: 1,
            decoded_pcm_sha256: c.decoded_pcm_sha256.clone(),
            bitrate_bps: available.then(|| bytes as f64 * 8.0 * f64::from(rate) / frames as f64),
            bitrate_interval: available.then_some(AnalysisInterval {
                start_frame: 0,
                end_frame: frames,
            }),
            bitrate_method: "encoded_complete_packet_mean".into(),
            unavailable_reason: (!available).then(|| {
                if stable {
                    "No complete encoded packet inside analyzed interval"
                } else {
                    "Encoded packet sizes changed between passes"
                }
                .into()
            }),
        }
    }

    pub fn validate(&self, report: &AnalysisReport) -> Result<(), RenderError> {
        let invalid =
            || RenderError::Invalid("Invalid or mismatched spectrogram presentation".into());
        let c = report.coverage.as_ref().ok_or_else(invalid)?;
        if report.status != FileStatus::Analyzed
            || self.presentation_version != 1
            || self.decoded_pcm_sha256 != c.decoded_pcm_sha256
            || self.bitrate_method != "encoded_complete_packet_mean"
        {
            return Err(invalid());
        }
        match (
            self.bitrate_bps,
            self.bitrate_interval.as_ref(),
            self.unavailable_reason.as_ref(),
        ) {
            (Some(bps), Some(i), None)
                if bps.is_finite()
                    && bps > 0.0
                    && i.start_frame == 0
                    && i.end_frame > 0
                    && i.end_frame <= c.analyzed_frames =>
            {
                Ok(())
            }
            (None, None, Some(reason)) if !reason.is_empty() => Ok(()),
            _ => Err(invalid()),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum CanvasSize {
    Standard,
    #[default]
    Publication,
    Large,
}
impl CanvasSize {
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Standard => (1600, 900),
            Self::Publication => (2560, 1440),
            Self::Large => (3840, 2160),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CanvasOptions {
    pub size: CanvasSize,
    /// Optional host-supplied track title; bounded/sanitized and ellipsized.
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasTick {
    pub value: f64,
    pub pixel: usize,
    pub label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasCutoff {
    pub channel_indices: Vec<usize>,
    pub frequency_hz: f64,
    pub pixel_y: usize,
    pub label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasDescriptor {
    pub render_version: u32,
    pub method_id: String,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub title: String,
    pub title_truncated: bool,
    pub source_filename: String,
    pub stream_line: String,
    pub analysis_line: String,
    pub frequency_ticks: Vec<CanvasTick>,
    pub time_ticks: Vec<CanvasTick>,
    pub cutoffs: Vec<CanvasCutoff>,
    pub legend_min_db: f64,
    pub legend_max_db: f64,
    pub decoded_pcm_sha256: String,
    pub warnings: Vec<String>,
}

/// Full canvas RGB8; raw payload is bounded by MAX_CANVAS_RGB_BYTES.
pub struct SpectrogramCanvas {
    pub descriptor: CanvasDescriptor,
    pub rgb: Vec<u8>,
}

fn font() -> Result<&'static Font, RenderError> {
    FONT.get_or_init(|| {
        Font::from_bytes(
            include_bytes!("../assets/fonts/NotoSans-Regular.ttf").as_slice(),
            FontSettings::default(),
        )
        .map_err(str::to_owned)
    })
    .as_ref()
    .map_err(|e| RenderError::Invalid(format!("Embedded font: {e}")))
}

struct Canvas {
    width: usize,
    height: usize,
    scale: f32,
    rgb: Vec<u8>,
}
impl Canvas {
    fn new(width: u32, height: u32) -> Result<Self, RenderError> {
        let rgb = allocated(width as usize * height as usize * 3, 255u8)?;
        Ok(Self {
            width: width as usize,
            height: height as usize,
            scale: width as f32 / 1280.0,
            rgb,
        })
    }
    fn px(&self, v: f32) -> i32 {
        (v * self.scale).round() as i32
    }
    fn blend(&mut self, x: i32, y: i32, c: [u8; 3], a: u8) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let p = (y as usize * self.width + x as usize) * 3;
        for (k, &v) in c.iter().enumerate() {
            self.rgb[p + k] = ((u32::from(v) * u32::from(a)
                + u32::from(self.rgb[p + k]) * (255 - u32::from(a))
                + 127)
                / 255) as u8;
        }
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 3], a: u8) {
        for row in y.max(0)..(y + h).min(self.height as i32) {
            for col in x.max(0)..(x + w).min(self.width as i32) {
                self.blend(col, row, c, a);
            }
        }
    }
    fn text_width(&self, f: &Font, s: &str, size: f32) -> f32 {
        let px = size * self.scale;
        let mut width = 0.;
        let mut prior = None;
        for ch in s.chars() {
            if let Some(p) = prior {
                width += f.horizontal_kern(p, ch, px).unwrap_or(0.);
            }
            width += f.metrics(ch, px).advance_width;
            prior = Some(ch);
        }
        width
    }
    fn fit(&self, f: &Font, s: &str, size: f32, max_width: f32) -> (String, bool) {
        if self.text_width(f, s, size) <= max_width * self.scale {
            return (s.to_owned(), false);
        }
        let mut out = s.to_owned();
        while !out.is_empty()
            && self.text_width(f, &format!("{out}…"), size) > max_width * self.scale
        {
            out.pop();
        }
        out.push('…');
        (out, true)
    }
    fn text(&mut self, f: &Font, s: &str, x: f32, baseline: f32, size: f32, c: [u8; 3]) {
        let mut pen = x * self.scale;
        let base = self.px(baseline);
        let px = size * self.scale;
        let mut prior = None;
        for ch in s.chars() {
            if let Some(p) = prior {
                pen += f.horizontal_kern(p, ch, px).unwrap_or(0.);
            }
            let (m, bitmap) = f.rasterize(ch, px);
            let left = pen.round() as i32 + m.xmin;
            let top = base - m.ymin - m.height as i32;
            for y in 0..m.height {
                for x in 0..m.width {
                    let a = bitmap[y * m.width + x];
                    if a != 0 {
                        self.blend(left + x as i32, top + y as i32, c, a);
                    }
                }
            }
            pen += m.advance_width;
            prior = Some(ch);
        }
    }
    fn centered(&mut self, f: &Font, s: &str, x: f32, baseline: f32, size: f32, c: [u8; 3]) {
        let start = x - self.text_width(f, s, size) / self.scale / 2.;
        self.text(f, s, start, baseline, size, c);
    }
    fn right(&mut self, f: &Font, s: &str, x: f32, baseline: f32, size: f32, c: [u8; 3]) {
        let start = x - self.text_width(f, s, size) / self.scale;
        self.text(f, s, start, baseline, size, c);
    }
}

fn clean_title(s: &str) -> (String, bool) {
    let text: String = s
        .chars()
        .take(256)
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    (text, s.chars().nth(256).is_some())
}
fn decimal(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}
fn time_label(seconds: f64, fraction: bool) -> String {
    let digits = if seconds < 0.01 { 6 } else { 3 };
    let scale = if digits == 6 { 1_000_000u64 } else { 1000 };
    let rounded = (seconds * scale as f64).round() as u64;
    let whole = if fraction {
        rounded / scale
    } else {
        seconds.floor() as u64
    };
    let mut text = format!("{}:{:02}", whole / 60, whole % 60);
    if fraction {
        let remainder = rounded % scale;
        if remainder > 0 {
            text.push_str(format!(".{remainder:0digits$}").trim_end_matches('0'));
        }
    }
    text
}
fn ticks(max: f64, time: bool) -> Vec<f64> {
    let target = max / 6.;
    let step = if time && target >= 1. {
        [
            1., 2., 5., 10., 15., 30., 60., 120., 300., 600., 900., 1800., 3600., 7200., 14400.,
            43200., 86400.,
        ]
        .into_iter()
        .find(|&s| s >= target)
        .unwrap_or_else(|| nice_step(target))
    } else {
        nice_step(target)
    };
    let mut out = vec![0.];
    for i in 1..=7 {
        let v = i as f64 * step;
        if v >= max || max - v < step * 0.3 {
            break;
        }
        out.push(v);
    }
    out.push(max);
    out
}
fn nice_step(v: f64) -> f64 {
    let scale = 10f64.powf(v.log10().floor());
    [1., 2., 5., 10.]
        .into_iter()
        .find(|x| x * scale >= v)
        .unwrap()
        * scale
}

impl SpectrogramAnalysis {
    /// Compose the complete calibrated canvas in Rust, without touching PCM/DSP.
    pub fn render_canvas(
        &self,
        options: &CanvasOptions,
        cancel: &CancellationToken,
        deadline: Duration,
    ) -> Result<SpectrogramCanvas, RenderError> {
        let start = Instant::now();
        render_control(cancel, start, deadline)?;
        self.validate_binding()?;
        let d = self
            .spectrogram
            .data
            .as_ref()
            .ok_or_else(|| RenderError::Invalid("Spectrogram is unavailable".into()))?;
        let stream = self
            .measurement
            .stream
            .as_ref()
            .ok_or_else(|| RenderError::Invalid("Missing native stream".into()))?;
        let coverage = self
            .measurement
            .coverage
            .as_ref()
            .ok_or_else(|| RenderError::Invalid("Missing native coverage".into()))?;
        let f = font()?;
        render_control(cancel, start, deadline)?;
        let (width, height) = options.size.dimensions();
        let mut c = Canvas::new(width, height)?;
        let dark = [29, 38, 55];
        let muted = [82, 97, 120];
        let accent = [111, 63, 185];
        c.rect(0, 0, width as i32, c.px(114.), [246, 248, 252], 255);
        c.rect(
            c.px(24.),
            c.px(114.),
            c.px(1232.),
            c.px(1.),
            [218, 225, 235],
            255,
        );
        let (filename, _) = clean_title(
            self.measurement
                .source
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&self.measurement.source),
        );
        let (title, title_limited) = clean_title(options.title.as_deref().unwrap_or(&filename));
        let (title, title_fit) = c.fit(f, &title, 24., 1008.);
        c.text(f, &title, 26., 42., 24., dark);
        c.right(f, "ALFRED / SPECTROGRAM", 1252., 40., 11., accent);
        let bitrate = self.presentation.as_ref().and_then(|p| p.bitrate_bps);
        let bitrate_text = bitrate
            .map(|v| format!("{:.0} kbps (audio average)", v / 1000.))
            .unwrap_or_else(|| "bitrate unavailable".into());
        let depth = stream
            .bits_per_sample
            .map(|v| format!("{v}-bit{}", if stream.integer_pcm { "" } else { " float" }))
            .unwrap_or_else(|| "depth unavailable".into());
        let stream_line = format!(
            "{}  /  {} Hz  /  {}  /  {}  /  {} channel{}",
            stream.codec.to_uppercase(),
            stream.sample_rate,
            depth,
            bitrate_text,
            stream.channels,
            if stream.channels == 1 { "" } else { "s" }
        );
        let (line, _) = c.fit(f, &stream_line, 14., 1224.);
        c.text(f, &line, 26., 71., 14., muted);
        let basis = match d.basis {
            DisplayBasis::Mono => "mono channel 1",
            DisplayBasis::StereoMid => "stereo mid (L+R)/2",
        };
        let scope = if coverage.reached_end {
            "FULL STREAM"
        } else {
            "ANALYZED PREFIX"
        };
        let analysis_line = format!(
            "Hann 1024 / hop 512  /  {basis}  /  {scope} 0:00 - {}  /  Nyquist {} kHz",
            time_label(coverage.end_seconds, true),
            decimal(f64::from(stream.sample_rate) / 2000.)
        );
        let (line, _) = c.fit(f, &analysis_line, 13., 1224.);
        c.text(f, &line, 26., 97., 13., muted);
        let left = c.px(112.);
        let right = c.px(1090.);
        let top = c.px(150.);
        let bottom = c.px(590.);
        let plot_w = (right - left + 1) as usize;
        let plot_h = (bottom - top + 1) as usize;
        // Column widths come from their actual intervals, including the partial tail.
        let mut columns = Vec::with_capacity(plot_w);
        let mut at = 0;
        for x in 0..plot_w {
            let frame = (x as f64 / (plot_w - 1) as f64 * coverage.analyzed_frames as f64)
                .min((coverage.analyzed_frames - 1) as f64);
            while at + 1 < d.time_columns.len()
                && frame >= d.time_columns[at].interval.end_frame as f64
            {
                at += 1;
            }
            columns.push(at);
        }
        for y in 0..plot_h {
            render_control(cancel, start, deadline)?;
            let bin = (1. - y as f64 / (plot_h - 1) as f64) * 512.;
            let lo = bin.floor() as usize;
            let hi = (lo + 1).min(512);
            let t = bin - lo as f64;
            for (x, &column) in columns.iter().enumerate() {
                // Smooth only frequency in LINEAR power, never invent time detail.
                let p0 = 10f64.powf(f64::from(d.power_db[column * 513 + lo]) / 10.);
                let p1 = 10f64.powf(f64::from(d.power_db[column * 513 + hi]) / 10.);
                let db = (10. * (p0 * (1. - t) + p1 * t).log10()) as f32;
                c.blend(left + x as i32, top + y as i32, color(db), 255);
            }
        }
        let max_khz = f64::from(stream.sample_rate) / 2000.;
        let mut frequency_ticks = Vec::new();
        c.text(f, "Frequency (kHz)", 112., 133., 13., muted);
        for value in ticks(max_khz, false) {
            let y = bottom - ((bottom - top) as f64 * value / max_khz).round() as i32;
            c.rect(
                left,
                y,
                right - left + 1,
                c.px(0.5).max(1),
                [202, 215, 231],
                25,
            );
            c.rect(left - c.px(6.), y, c.px(6.), c.px(1.).max(1), muted, 255);
            let label = format!("{} kHz", decimal(value));
            c.right(f, &label, 102., y as f32 / c.scale + 4.5, 13., dark);
            frequency_ticks.push(CanvasTick {
                value: value * 1000.,
                pixel: y as usize,
                label,
            });
        }
        let mut time_ticks = Vec::new();
        for value in ticks(coverage.end_seconds, true) {
            let x = left + ((right - left) as f64 * value / coverage.end_seconds).round() as i32;
            c.rect(
                x,
                top,
                c.px(0.5).max(1),
                bottom - top + 1,
                [202, 215, 231],
                25,
            );
            c.rect(x, bottom + 1, c.px(1.).max(1), c.px(7.), muted, 255);
            let label = time_label(value, value == coverage.end_seconds || value.fract() != 0.);
            c.centered(f, &label, x as f32 / c.scale, 619., 13., dark);
            time_ticks.push(CanvasTick {
                value,
                pixel: x as usize,
                label,
            });
        }
        c.centered(f, "Time (M:SS)", 601., 645., 13., muted);
        // Frame and legend use the same calibrated data endpoints.
        for (x, y, w, h) in [
            (left, top, right - left + 1, 1),
            (left, bottom, right - left + 1, 1),
            (left, top, 1, bottom - top + 1),
            (right, top, 1, bottom - top + 1),
        ] {
            c.rect(x, y, w, h, [128, 143, 166], 255);
        }
        let bar_left = c.px(1128.);
        let bar_width = c.px(22.);
        for y in top..=bottom {
            let db = -140. * (y - top) as f32 / (bottom - top) as f32;
            c.rect(bar_left, y, bar_width, 1, color(db), 255);
        }
        c.text(f, "Power", 1127., 113., 13., muted);
        c.text(f, "dBFS / bin", 1127., 134., 12., muted);
        for db in (0..=140).step_by(20) {
            let y = top + ((bottom - top) as f32 * db as f32 / 140.).round() as i32;
            c.rect(
                bar_left + bar_width,
                y,
                c.px(6.),
                c.px(1.).max(1),
                muted,
                255,
            );
            c.text(
                f,
                &format!("{} dB", -db),
                1165.,
                y as f32 / c.scale + 4.5,
                13.,
                dark,
            );
        }
        let mut cutoffs: Vec<CanvasCutoff> = Vec::new();
        for channel in &self.measurement.channels {
            if let Some(hz) = channel
                .spectral
                .cutoff_p95_hz
                .filter(|v| v.is_finite() && *v >= 0. && *v <= max_khz * 1000.)
            {
                if let Some(existing) = cutoffs.iter_mut().find(|v| v.frequency_hz == hz) {
                    existing.channel_indices.push(channel.channel_index);
                } else {
                    cutoffs.push(CanvasCutoff {
                        channel_indices: vec![channel.channel_index],
                        frequency_hz: hz,
                        pixel_y: 0,
                        label: String::new(),
                    });
                }
            }
        }
        cutoffs.sort_by(|a, b| b.frequency_hz.total_cmp(&a.frequency_hz));
        let cutoff_count = cutoffs.len();
        let mut prior_label = top - c.px(24.);
        for (index, cutoff) in cutoffs.iter_mut().enumerate() {
            let y = bottom
                - ((bottom - top) as f64 * cutoff.frequency_hz / (max_khz * 1000.)).round() as i32;
            cutoff.pixel_y = y as usize;
            let channels = cutoff
                .channel_indices
                .iter()
                .map(|v| (v + 1).to_string())
                .collect::<Vec<_>>()
                .join("/");
            cutoff.label = format!(
                "Cutoff p95: {} kHz / native Ch {channels}",
                decimal(cutoff.frequency_hz / 1000.)
            );
            let period = c.px(9.).max(2);
            let dash = c.px(5.).max(1);
            for x in left..=right {
                if (x - left) % period < dash {
                    c.blend(x, y, [244, 237, 211], 145);
                }
            }
            let label_y = (y - c.px(9.))
                .max(top + c.px(20.))
                .max(prior_label + c.px(25.))
                .min(bottom - c.px(6.) - c.px(25.) * (cutoff_count - index - 1) as i32);
            prior_label = label_y;
            let label_width = c.text_width(f, &cutoff.label, 12.) as i32 + c.px(16.);
            c.rect(
                right - label_width - c.px(8.),
                label_y - c.px(16.),
                label_width,
                c.px(21.),
                [25, 32, 45],
                228,
            );
            c.right(
                f,
                &cutoff.label,
                1080.,
                label_y as f32 / c.scale,
                12.,
                [244, 237, 211],
            );
        }
        let cutoff_note = if cutoffs.is_empty() {
            "Cutoff p95 unavailable"
        } else {
            "Cutoff markers: native channel p95"
        };
        let bitrate_note = self
            .presentation
            .as_ref()
            .and_then(|p| p.bitrate_interval.as_ref())
            .map(|i| {
                format!(
                    "Audio average bitrate: complete packets 0:00 - {}",
                    time_label(i.end_frame as f64 / f64::from(stream.sample_rate), true)
                )
            })
            .unwrap_or_else(|| "Encoded bitrate unavailable in this saved result".into());
        let footer = format!("{cutoff_note}  /  {bitrate_note}");
        let (footer, _) = c.fit(f, &footer, 12., 1224.);
        c.text(f, &footer, 26., 673., 12., muted);
        let mut warnings = d.warnings.clone();
        if title.chars().any(|ch| f.lookup_glyph_index(ch) == 0) {
            warnings.push(
                "Embedded font lacks some title glyphs; visible missing-glyph marks are shown"
                    .into(),
            );
        }
        if !coverage.reached_end {
            warnings.push("Prefix image; later audio is outside the analyzed interval".into());
        }
        if title_limited || title_fit {
            warnings.push("Long title truncated to the bounded header".into());
        }
        let warning = if d.mid_cancelled_with_native_signal {
            "WARNING: stereo mid is silent while native channels contain signal"
        } else if d.basis == DisplayBasis::StereoMid {
            "Stereo mid can hide channel energy. Native channel measurements remain separate."
        } else {
            "Display: frequency interpolation in linear power; time buckets retain measured intervals."
        };
        c.text(
            f,
            warning,
            26.,
            699.,
            11.,
            if d.mid_cancelled_with_native_signal {
                [158, 61, 24]
            } else {
                muted
            },
        );
        render_control(cancel, start, deadline)?;
        Ok(SpectrogramCanvas {
            descriptor: CanvasDescriptor {
                render_version: 1,
                method_id: RENDER_METHOD.into(),
                width,
                height,
                dpi: 300,
                title,
                title_truncated: title_limited || title_fit,
                source_filename: filename,
                stream_line,
                analysis_line,
                frequency_ticks,
                time_ticks,
                cutoffs,
                legend_min_db: -140.,
                legend_max_db: 0.,
                decoded_pcm_sha256: coverage.decoded_pcm_sha256.clone(),
                warnings,
            },
            rgb: c.rgb,
        })
    }

    /// Render and stream a lossless PNG to a NEW path. Failure never returns a path.
    pub fn write_png_new(
        &self,
        path: impl AsRef<Path>,
        options: &CanvasOptions,
        cancel: &CancellationToken,
        deadline: Duration,
    ) -> RasterExport {
        let path = path.as_ref();
        let start = Instant::now();
        let result = self
            .render_canvas(options, cancel, deadline)
            .and_then(|image| {
                render_control(cancel, start, deadline)?;
                let file = OpenOptions::new().write(true).create_new(true).open(path)?;
                let result = (|| {
                    let mut buffered = BufWriter::new(file);
                    {
                        let checked = CheckedWriter {
                            writer: &mut buffered,
                            cancel,
                            start,
                            deadline,
                        };
                        let mut encoder = png::Encoder::new(
                            checked,
                            image.descriptor.width,
                            image.descriptor.height,
                        );
                        encoder.set_color(png::ColorType::Rgb);
                        encoder.set_depth(png::BitDepth::Eight);
                        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
                        encoder.set_pixel_dims(Some(png::PixelDimensions {
                            xppu: 11811,
                            yppu: 11811,
                            unit: png::Unit::Meter,
                        }));
                        encoder.add_itxt_chunk(
                            "Alfred canvas".into(),
                            serde_json::to_string(&image.descriptor)
                                .map_err(|e| RenderError::Invalid(e.to_string()))?,
                        )?;
                        let mut writer = encoder.write_header()?;
                        {
                            let mut stream = writer.stream_writer_with_size(64 * 1024)?;
                            for row in image.rgb.chunks_exact(image.descriptor.width as usize * 3) {
                                render_control(cancel, start, deadline)?;
                                stream.write_all(row)?;
                            }
                            stream.finish()?;
                        }
                        writer.finish()?;
                    }
                    buffered.flush()?;
                    render_control(cancel, start, deadline)?;
                    Ok::<_, RenderError>(())
                })();
                if result.is_err() {
                    let _ = remove_file(path);
                }
                result
            });
        match result {
            Ok(()) => RasterExport {
                status: ArtifactStatus::Available,
                path: Some(path.to_owned()),
                reason: None,
            },
            Err(error) => {
                // PNG may wrap a writer control error; restore the control status.
                let status = if cancel.is_cancelled() {
                    ArtifactStatus::Cancelled
                } else if start.elapsed() >= deadline {
                    ArtifactStatus::TimedOut
                } else {
                    ArtifactStatus::Failed
                };
                RasterExport {
                    status,
                    path: None,
                    reason: Some(error.to_string()),
                }
            }
        }
    }
}

struct CheckedWriter<'a, W> {
    writer: W,
    cancel: &'a CancellationToken,
    start: Instant,
    deadline: Duration,
}
impl<W: Write> Write for CheckedWriter<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        render_control(self.cancel, self.start, self.deadline).map_err(std::io::Error::other)?;
        self.writer.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        render_control(self.cancel, self.start, self.deadline).map_err(std::io::Error::other)?;
        self.writer.flush()
    }
}
