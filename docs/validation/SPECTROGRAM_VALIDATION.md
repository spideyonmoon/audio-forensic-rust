# P06 spectrogram contract and validation

Completed 2026-10-06, engine **0.30.0** including the owner's calibrated PNG
follow-up; artifact/contract **1**, method
`hann1024-power-pair-merge-v1`. Native measurement schema **0.18.0**,
policy, INCONCLUSIVE ancestry and null evidence index are unchanged.

## Source and scope

Pinned Python `generate_spectrogram` at `audio_forensic.py:971` pipes mono
FFmpeg PCM to SoX (1280 × 513, 120 dB range, −20 dB ceiling), falls back to
FFmpeg, and writes an adjacent stem-derived filename. RELEASE_CONTRACT v1
G25/W09/D12 replaces pixels and those side effects with a bounded offline
artifact. No SoX color/FFT parity is asserted. Detector `StreamingStft` stays
4096/hop2048; this separate display FFT is 1024/hop512 to retain DC–Nyquist
in exactly 513 bins without frequency resampling.

`analyze_path_with_spectrogram` and `analyze_source_with_spectrogram` return
`SpectrogramAnalysis { measurement, spectrogram, presentation }`. Optional collection occurs
inside the first existing PCM pass; the second pass verifies the same samples.
No extra decode, subprocess, full-track PCM or duration-sized spectrum is kept.
Source, native track, worker, prefix, cancellation and deadline are shared.
Ordinary APIs allocate no spectrogram collector. Coverage binds the exact
canonical PCM hash/encoding and two-pass interval. A changed/failed source
suppresses the artifact. Short successful audio remains a valid measurement,
with an unavailable artifact when there is no complete display FFT window.
Collector allocation/finalization failure has its own artifact status and
reason; it cannot erase a completed measurement or expose stale pixels.

## DSP and axes for A06

- Basis: mono channel 0; stereo mid `(L+R)/2` using the same pinned f32 operation
  order as `reference_inputs::reference_basis`. Native peak, basis peak and
  exact mid cancellation are explicit. Always label stereo mid; warn that it
  can attenuate native channel energy. A silent mid with nonzero native channels
  carries an additional warning and never implies a silent recording.
- Window: symmetric f32 Hann, 1024 PCM frames; hop 512. A one-sample lookahead
  emits only windows whose end is **strictly before** the analyzed end.
  No zero padding, activity gate, resampling or fabricated final window.
- Linear bin power: `(|FFT[k]|² / (1024 * sum(window²)))`, multiplied by 2 for
  interior positive bins, 1 for DC/Nyquist. `10*log10(power / 1.0)` after time
  averaging, floor −140 dB. Unit: `dBFS_power_per_bin`, not raw FFT dB or
  sinusoidal peak amplitude. Parseval sum equals window-weighted mean square;
  a bin-centered 0.5-peak sine integrates to 0.125 (−9.0309 dBFS). No upper
  clipping is applied to the data; finite float PCM may exceed full scale.
- Frequency row `k` is `k * sample_rate_hz / 1024` Hz, increasing DC→Nyquist.
  Data is flat time-major then frequency-major. Native mono/stereo rate is
  preserved, including 384 kHz; frequency resolution is `rate/1024` Hz.
- Time buckets initially hold one STFT window. On receiving a 1281st bucket,
  merge adjacent bucket **power sums and counts**, halve the occupied count,
  double the bucket width. Repeat as required; partial final buckets use their
  actual count, never mean-of-means or averaging dB. Duration/header length is
  never used to allocate or select buckets. At most 1280 columns remain.
- Each column carries its half-open display interval, overlapping actual
  window support and contributing STFT count. Starts are native frames; divide
  by rate for seconds. The final display interval reaches analyzed end, but
  its actual support may end earlier. `spectral_interval` and
  `unwindowed_tail_frames` expose that difference. For event positions use
  window centers/support, since Hann windows straddle onsets and offsets.
  Axes must reflect those intervals, never an unmeasured container duration.
- RGB renderer: time increases right, Nyquist at top, DC at bottom. `ember-v1`
  interpolates RGB anchors black, (48,16,80), (160,32,64), (240,128,32),
  (255,240,160), at equally spaced values from −140 to 0 dB. Colors clip to
  that range. The calibrated Rust canvas supplies axes, basis/prefix label,
  units, color legend and warnings; A06 displays that complete image. The
  raw renderer remains separate; its last partial column has an actual interval.

## Rendering, export and bounds

`SpectrogramArtifact::validate` rejects unsupported versions/methods, inconsistent
geometry/counts/coverage, nonfinite powers and altered axes before rendering.
The separate schema constrains producer shape; cross-field identity/counts
also require `validate`; `SpectrogramAnalysis::validate_binding` additionally
checks exact wrapper coverage and native rate/channel identity. Never silently
reinterpret a saved artifact using a new method or another file.

`render_rgb` has a fresh cooperative deadline and checks cancellation before
validation, per image row and after processing. Maximum RGB payload is
1,969,920 bytes. `write_ppm_new` is a portable offline P6 PPM export: caller
selects a path, `create_new` prevents any existing file from being overwritten,
and only a completed write/flush returns a path. Failure/cancel/timeout returns
null path and removes this invocation's partial file when possible. Filesystem
I/O remains cooperative; blocking host I/O is not interruptible.
`render_canvas` / `write_png_new` additionally provide the full Rust canvas and
PNG described below. P07/A06 own storage/sharing and collision-safe filename
selection. No input-neighbor filename is invented by the core. The owner's
follow-up supersedes the original P06 app-owned composition/PNG decision.

Collector fixed arrays: 656640 f64 sums (**5,253,120 bytes**), 1280 u64 counts
(10,240), Hann/pending (4,096 each), complex FFT buffer (8,192) and FFT scratch.
The actual Rust 1.85 control measured **8,192 scratch bytes**, making fixed
collector arrays **5,287,936 bytes**. Maximum simultaneous collector/dB/column
record payload is **7,965,696 bytes**, excluding descriptor strings/plans and
the allocator/native components described below.
Final dB payload is at most **2,626,560 bytes**, plus at most 1280 column records
and descriptor strings. Collector and dB payload coexist during finalization;
FFT plans, allocator overhead and existing decoder/native DSP are additional.
Render/export holds dB and RGB after the collector is released; it does not
allocate a second RGB image. No process RSS/Android phone budget is claimed.
Control is checked before FFT, at every merge destination and final time
column. Array storage does not grow with file duration or declared frame count.

## Frozen oracle and intended checks

`tests/fixtures/spectrogram_reference.json` is generated once by an independent
f64 **direct DFT**, from f32 samples/window products. Tone bin 64, DC, impulse
and Nyquist vectors include all 513 expected powers and integrated power.
Absolute normalized bin/total power tolerance **2e-7** was frozen before the
Rust comparison; display colors are not the numerical oracle. `--check`
reproduces without writing the fixture. Time/frequency and long-stream tests
use generated controls; no private recording is analyzed.

Local receipts: ignored `target/p06-spectrogram/`. Source changes remain local
and uncommitted; receipts are not Git history or an external backup.
Android target checking, if run, establishes compilation only.

## Actual results

The initial optimized Rust 1.85/no-CLI focused run passed **76 tests**:
54 library (four new spectrogram units), seven spectrogram integrations,
10 source, four progress and one serialization. No failures/ignored tests.
Four frozen 513-bin direct-DFT vectors met the predeclared **2e-7** absolute
bin/total power bound. Two coarsenings retained all 2561 frame weights,
641 columns with a one-frame last bucket and correct linear power (relative
error below **2e-6**); no duration-based array growth. Low/high-rate tone
frequency, impulse/burst, silence, short strict boundary, antiphase, prefix,
render orientation, collision/error paths, controls and report equality passed.
Frozen oracle reproduction, new seven-definition schema, unchanged 47-definition
measurement schema, helper syntax, parity inventory and whitespace passed.

Final-source tests passed **78 across final runs**: 54 library, nine artifact
integrations, 10 source, four progress and one serialization; no remaining
failures/ignored tests. Additional controls verify one-bucket event bounds,
saved coverage/hash/rate binding, valid-PCM mutation suppression and generated
FLAC with unknown STREAMINFO duration/unknown source byte length.

| Final check | Actual result |
| --- | --- |
| Independent frozen direct-DFT and weighted Parseval | Passed four 513-bin cases, 2e-7 absolute bin/total bound; two coarsenings conserve all weights/power within 2e-6 relative |
| Optimized no-CLI reader | Built successfully on Rust 1.85 |
| Generated serialized controls | 10 full/prefix reports, exact s16→canonical signed32 PCM/hash/frame counts, unchanged native schema and separate artifact schema passed |
| Raster and failed outputs | Nine PPM/QA PNG images, silent/antiphase/prefix floor, long high-rate burst, impulse, short unavailable, invalid input and no-overwrite collision passed |
| Numeric/event checks | Tone at exact bin 64 across rates; serialized integrated s16 tone maximum error 1.72311e-7 (bound 2e-5); burst onset/offset within one output bucket |
| Saved versions/geometry | Eight Rust malformed descriptor mutations, changed wrapper hash, plus eight JSON schema mutation rejections passed |
| MSRV Clippy | All targets with/without CLI, warnings denied, passed |
| Android ARM64 | Rust 1.85 no-CLI target compilation passed; no linking/APK/device claim |
| Schemas/oracle/syntax/inventory/format | Frozen reproduction, seven-definition artifact and unchanged 47-definition measurement schemas, helper syntax, parity inventory, formatting and whitespace passed |
| Original raw-raster visual QA | Generated long384k burst and impulse inspected: correct time region/broadband impulse, Nyquist top; full canvas added by the follow-up below |

Initial sandbox linker execution was denied;
authorized offline retry reached a compile error in the new reference-basis
import, which was corrected to its actual module. No oracle was changed.
The final combined test attempt then exposed a **test expectation** mismatch:
the existing valid-PCM rejection says `Source changed between analysis passes`,
without the word `PCM`. The corrected exact diagnostic and suppression/recovery
checks pass. No production behavior, reference or tolerance was changed to hide
that failure. Receipts/commands and preservation are in `task-results/P06.md`.

No full CLI/core regression, actual Android linking/device work, private audio,
calibration or new decoder implementation was run for original P06. Its PNG
visual QA used a development stdlib encoder; the follow-up adds runtime Rust PNG.

## Owner-requested calibrated PNG (engine 0.30.0)

`src/spectrogram_png.rs` composes a self-contained image directly in Rust. The
header shows a caller-supplied title or source basename, codec, native Hz,
integer depth or float precision, encoded audio bitrate, channel count,
Hann1024/hop512, mono/stereo-mid basis, full/prefix interval and exact Nyquist.
The left axis has kHz values from DC through Nyquist, ticks and subtle guides;
the bottom has M:SS markers through actual analyzed end (fractional seconds
where required to distinguish short-track ticks). Prefix duration is never
misrepresented as full track duration. The right bar uses exactly ember-v1
from 0 down to −140 dBFS power/bin with 20 dB labels. Native per-channel p95
cutoffs are dashed and labeled with channel identity; equal exact values are
coalesced, distinct values retained, absent/invalid values explicitly unavailable.
These are existing native detector measurements (4096 FFT), not newly measured
mid cutoffs, probabilities or authentication claims. Antiphase-mid silence has
its own visible warning even when native cutoff measurements are available.

The lossless RGB8 presets are Standard **1600×900**, Publication **2560×1440**
(default) and Large **3840×2160**. Labels scale with resolution and use embedded
antialiased Noto Sans Regular through pinned fontdue 0.9.3. Font/OFL provenance
is in `assets/fonts/README.md`; there is no runtime font fetch or system font
dependency. The font covers its supported scripts; per-character kerning does
not implement complex-script shaping, missing title glyphs are visible and
recorded as warnings. Title input is capped at 256 characters, control characters
sanitized, and long headers ellipsized. A06 can supply its stored metadata title;
the renderer does not start another metadata/audio pass.

Frequency display interpolation uses **linear power**, not dB averaging; time
uses measured columns at their actual frame interval widths, including a partial
last bucket. Higher canvas pixels do not add measured FFT bins/time detail.
Data v1, frozen DFT oracle, native FFT/detectors and numerical tolerances are
unchanged. No SoX/Spek pixel parity is claimed. The maximum canvas RGB payload
is **24,883,200 bytes**; default is **11,059,200 bytes**. At most 2935 display
column indices (maximum plot width) are temporary. Artifact dB/column records,
resident embedded font/font structures, one glyph raster at a time, PNG/deflate
state, metadata, allocator and existing native buffers are additional. This is
not a total RSS or Android phone budget. Collector is already released before
canvas rendering; PNG encoding streams rows with a 64 KiB stream buffer, without
a second full RGB canvas or full encoded-file buffer.

`write_png_new(path, CanvasOptions, cancel, deadline)` uses pinned pure-Rust
png 0.17.16, sRGB/perceptual intent and pHYs **11811 pixels/metre** (300 dpi).
PNG iTXt `Alfred canvas` stores render version **1**, exact method
`alfred-calibrated-png-v1`, title/metadata, axis values/pixels, cutoff values/
channels, warnings and PCM hash. Embedded context is a descriptor, not a new
measurement report. The caller chooses a new path. `create_new` prevents
overwriting; export success/path is returned only after PNG finish and flush.
Failure/cancel/deadline returns null and removes only this call's partial output
when possible. Cooperative checks run before/after rendering, per image row,
per encoded row and writer write/flush; blocking filesystem calls and font
initialization remain cooperative. Raw `render_rgb`/`write_ppm_new` are retained.

### Bitrate and saved-wrapper contract

Optional `SpectrogramPresentation` version **1**, method
`encoded_complete_packet_mean`, accompanies the existing wrapper only. During
the existing PCM scan, scalar counters sum encoded audio packet bytes and
decoded frames for packets consumed **completely** inside the analyzed interval.
The mean is `bytes * 8 * rate / frames`. A cut final packet is excluded rather
than assigned its full size; the exact contiguous interval is saved and shown
in the footer. Container tags/artwork/overhead are excluded. This is an audio
average, not an instantaneous bitrate, container-size estimate or decoded PCM
rate substituted for compressed audio. Integer/float WAV controls happen to
equal their encoded PCM rate. Both existing analysis passes must give identical
counters; absent complete packets or changed sizes give explicit null/reason.
There is no additional decode pass or packet buffer. PCM/source mutations keep
the existing failed report/artifact suppression.

`validate_binding` binds this presentation to successful measurement PCM hash,
analyzed interval and exact version/method; unknown versions, mismatches and
invalid finite/null combinations fail rendering. Deserialization accepts old
wrappers without the optional field. They render **bitrate unavailable**, not
an invented value. Measurement schema 0.18.0 and spectral artifact schema v1
remain byte-for-byte unchanged. P07 must retain this optional input in saved
product workflows; A06 invokes the engine canvas, displays/shares it and owns
storage/deletion, rather than implementing calibration in Kotlin.

### Follow-up acceptance

Rust 1.85 optimized/no-CLI checks passed **80 tests** (54 library, nine P06
artifact, six calibrated PNG, ten source and one native serialization).
Tests cover full/partial scope, exact native cutoffs and distinct channels,
PNG decoded RGB equality, sRGB/DPI/iTXt, measured WAV bitrate, old wrappers,
saved hash mismatch, title/resolution bounds, silence/antiphase and safe
collision/missing-path/cancel/deadline behavior. Independent stdlib PNG CRC/
filter/RGB/DPI/context checks passed six controls/all presets with exactly
unchanged frozen spectral data and native numeric measurements. Generated
compressed bitrate agrees with FLAC audio-byte count and ALAC stsz sizes;
a cut first FLAC packet shows unavailable bitrate. A 163.968125 s control
verifies 30-second/minute labels. FLAC publication, Large high-rate burst,
antiphase and prefix images were visually inspected. Final-source Clippy
with/without CLI, no-CLI example, Android ARM64 compilation, schemas/helper
syntax/parity inventory/format/whitespace passed. No Android linking/device
claim. Preservation retained 48 private files, pinned reference, schemas,
oracle and unrelated work; no outstanding failures/jobs. Exact commands,
retained initial failures and receipts are in `task-results/P06-PNG.md`.
