# Audio Forensic Rust

An offline Rust library and CLI for audio forensic analysis. Version 0.10 adds
**bounded native-channel cross-band envelope correlation**. Every report keeps
ancestry `INCONCLUSIVE` and the evidence index `null` until the detector suite and
aggregation policy are implemented and validated.

Continuing in a new session or tool? Read [HANDOFF.md](HANDOFF.md) for the current
state, checks, known issues and next task, and [AGENTS.md](AGENTS.md) for project
instructions. These files are maintained alongside the code.

## Implemented

- Native WAV/FLAC decoding through Symphonia; no FFmpeg, SoX, MediaInfo or Python
  process is needed by the Rust application.
- Mono/stereo, 8-384 kHz, integer PCM and supported floating-point WAV. Native
  channels are preserved; surround inputs are explicitly unsupported.
- A seekable-source library API (`File`, in-memory `Cursor`, or an application
  implementation of `MediaSource`) and explicit track selection.
- Streaming per-channel peak, RMS, DC offset, silence sample count, full-scale
  sample count, exact exercised integer precision and the reference effective-bit
  statistic. Full-scale sample count is a measurement, not a clipping diagnosis.
- Two-pass streaming Hann/STFT measurements: active-frame selection, 95th-percentile
  cutoff, cutoff variance, cliff depth, sharpness, high-frequency magnitude ratio,
  spectral entropy and above-cutoff FFT magnitude level.
- Conservative integer-padding observations, explicit detector applicability,
  versioned JSON reports and per-file errors that preserve other batch results.
- Per-channel two-second spectral probes with explicit frame intervals, spectral
  walls, repeated/mixed probe patterns, and all overlapping codec-wall candidates.
- Native integer stereo MQA sync-word and payload-field candidates, with bounded
  storage, partial-payload reporting and an explicit three-second scan interval.
  These observations do not confirm MQA or establish source history.
- Streaming foreign-Nyquist notch, wall and mirror-correlation measurements, with
  all matching sample-rate hypotheses retained separately from codec evidence.
- Vorbis-window transform-grid observations at 44.1/48 kHz on each native channel,
  with phase search, active-probe requirements and off-grid baseline comparisons.
- AAC KBD-window quantization-lattice observations at 44.1/48 kHz, using explicitly
  labeled mono or stereo mid/side signals, independent active anchors and band gates.
- Per-native-channel spectral-scatter bounds and high-band phase-difference
  entropy, with explicit activity/energy gates and no source-history label.
- Per-channel quiet-run counts, durations and bounded interval listings, plus
  Hann-normalized power in the high-frequency and above-cutoff bands. Separate
  estimates use only STFT windows wholly inside qualifying quiet passages.
- Signed circular band correlation at two reported lags and band-level variation
  across completed seconds, with explicit energy gates and bounded prefix history.
- High-pass envelope peak counts/rates and bounded event listings, with explicit
  baseline bounds, threshold, startup exclusion and eligible analysis intervals.
- Two-band 12–18 kHz spectral roll-off, using actual frequency-bin coverage and
  explicit energy gates; EQ/filter controls demonstrate that this is not a source label.
- Signed correlation between 1–8 and 16–22 kHz RMS envelopes, with bounded online
  covariance and explicit energy, temporal-variation and full-band coverage gates.
- Shared prefix limits for every pass, cooperative cancellation and deadlines.
- SHA-256 of the decoded interleaved samples, checked across both passes and
  available for comparison with an independent decoder. Integer hashes use
  MSB-aligned signed 32-bit little-endian samples; float hashes use f64le.

The DSP uses bounded spectral accumulators and one decoder block at a time.
The first pass finds each channel's maximum spectral peak; the second applies the
Python reference's relative activity mask. This costs a second decode/FFT pass
and avoids retaining a whole track or spectrogram. Seekable input is required.
The second pass also collects non-overlapping two-second probes across the actual
decoded interval: up to 36 per channel, with one reusable clip FFT workspace.
Their buffers scale with sample rate and channel count, not track duration.
The Vorbis search captures at most twelve 3071-sample spans per channel from the
first 180 analyzed seconds. It searches those spans after decoding, using a
reusable MDCT workspace and cancellation/deadline checks between phase batches.
No full-track audio or coefficient matrix is retained.

AAC uses the same two decoding passes: the first retains at most 8436 hop-energy
values per basis from the first 180 seconds; the second captures up to sixteen
3064-sample spans per basis. Its phase search checks cancellation/deadlines every
eight transforms. Retained audio does not grow with file duration.

Spectral-structure measurements reuse the second-pass complex FFT. They retain
fixed per-bin buffers and histograms, adding about 66.4 KiB per channel, without
an extra transform or decode pass. Every active frame contributes; phases across
inactive gaps are never compared as adjacent frames.

Quiet/noise measurements also reuse that FFT, adding three fixed 2049-element
f64 accumulators and at most sixteen quiet intervals per channel (about 48.3 KiB
of payload). Aggregate counts and power include all qualifying runs. Band power
includes inactive frames; quiet passages are never concatenated across gaps.
These descriptive measurements do not identify vinyl, cassette or codec noise.

Band dynamics reuse the same FFT and retain at most 180 per-second spectra,
adding about 2.82 MiB/channel plus bounded bookkeeping/report data. Storage stops
growing after 180 seconds; global band powers and correlation continue over the
analyzed stream. Correlation describes periodic band-filtered Hann windows, not
continuous-stream Pearson correlation. Temporal variation excludes windows that
cross second boundaries and requires every included second to pass energy gates.
See [NOISE_DYNAMICS_VALIDATION.md](NOISE_DYNAMICS_VALIDATION.md) for exact definitions.

Transient measurements reuse the two decode passes with a causal 1 kHz high-pass
filter and centered rectified-envelope smoothing. A fixed histogram estimates
median bounds in the first pass; the second counts qualifying local maxima in
time order, with about 10 ms spacing. Only the first 180 seconds contribute and
only 128 events per channel are listed. Musical attacks and edits can produce
the same observations; counts do not identify physical clicks or a source medium.
See [TRANSIENT_VALIDATION.md](TRANSIENT_VALIDATION.md) for thresholds, exact frame
coverage and independent SciPy checks, including a three-burst control that
produces 60 envelope peaks.

Spectral roll-off averages log magnitudes of the active mean spectrum in two
500 Hz endpoint bands near 12 and 18 kHz, then divides their level difference by
the actual bin-center separation. It adds a fixed 16 KiB/channel accumulator.
Both complete bands and every selected bin must pass applicability gates; silence,
sparse spectra and unavailable bands give null slopes. This is an endpoint
contrast, not a fitted curve or proof of tape origin. Details and numerical checks
are in [ROLLOFF_VALIDATION.md](ROLLOFF_VALIDATION.md).

Cross-band envelope correlation measures how two bands' RMS levels vary together
across active STFT frames. It keeps constant-size running statistics and returns
null for quiet, constant, unavailable or insufficient envelopes. Every active
frame contributes; the result does not identify authentic or injected high-frequency
content. See [ENVELOPE_VALIDATION.md](ENVELOPE_VALIDATION.md) for definitions and
independent inverse-FFT/time-domain checks.

## Build and run

With Rust installed normally:

```text
cargo build --release --locked
cargo run --release -- --json path/to/track.flac
cargo run --release -- --fast path/to/album-directory
```

The published source-only tree omits `tests/`, generated fixtures, validation
scripts and private work files. Public CI builds and lints production targets;
the full validation suite runs in the local research workspace. `cargo test`
and `cargo clippy --all-targets` require that workspace, including the AAC
numerical fixture referenced by an internal test. A source checkout alone is
not a backup of the local validation evidence.

On this Windows workspace a project-local toolchain was installed under `.tools/`
without changing the system PATH. Use:

```powershell
./scripts/cargo.ps1 test --locked
./scripts/cargo.ps1 build --release --locked
./target/release/audio-forensic.exe --json --fast reference/test_files
```

The PowerShell wrapper uses an installed GCC driver for the local GNU toolchain.
Other machines can use standard Cargo with their normal toolchain. `.tools/` is
not a distributable component and is excluded from version control.

Library consumers can disable the default `cli` feature to omit Clap and Ctrl+C
process handlers. The core itself exposes a cancellation token and performs no
signal-handler setup. A compile check for the Android library is:

```text
rustup target add aarch64-linux-android
cargo check --lib --no-default-features --target aarch64-linux-android --locked
```

This checks target compatibility; it does not build an APK, link an Android
shared library, or establish device runtime behavior.

Directories scan their immediate `.wav`/`.flac` children in sorted order. Explicit
file arguments always produce a result, including unsupported or missing files.
Progress is written to stderr, JSON to stdout. Exit codes are 0 for successful
measurements, 1 for any failed/unsupported file, 2 for command/setup errors and
130 for cancellation. Successful measurements do not mean authentic audio.

`--fast` limits all passes to the first 60 seconds. `--max-seconds N` chooses a
different prefix; they are mutually exclusive. A prefix result never claims
full-file integrity. `--track-id N` binds metadata and decoding to that track.
`--deadline-seconds N` defaults to 600 per file and is checked between packets;
it cannot interrupt a blocking operating-system read or an individual decoder
call. Transform searches also check cancellation/deadlines between phase batches.
Ctrl+C requests cooperative cancellation.

## Numerical conventions and limits

- FFT length 4096, hop 2048, symmetric Hann, f32 unnormalized forward transform.
  A frame must end strictly before EOF, matching the pinned Python reference's
  boundary behavior; trailing partial frames are not padded.
- Activity uses a channel's maximum frame magnitude and a -60 dB relative gate.
  Cutoff is the last bin above -65 dB relative to that frame's maximum.
- Fixed histograms reproduce linear percentile interpolation over cutoff bins.
  Spectral reductions use f64 accumulation, so small floating-point differences
  from Python's f32 reductions are expected and tested with explicit tolerances.
- High-frequency ratio is a **magnitude** ratio; above-cutoff level is based on
  unnormalized FFT magnitudes and must not be displayed as calibrated dBFS.
- The separate `noise` fields use one-sided power `2 * sum(|X[k]|^2) /
  (4096 * sum(Hann^2))`, then average over windows. Their dBFS values describe
  window-weighted band RMS, not the earlier unnormalized magnitude field.
  A band needs at least two bins and four windows per estimate. Exact zero has
  linear power zero and null dBFS; unavailable or insufficient estimates are null.
- Quiet runs require every sample's absolute value to be strictly below 0.01
  for at least `ceil(sample_rate / 2)` samples. Runs are clipped to analysis
  coverage. This peak threshold is not a perceptual-silence test. Actual band
  edges, frame counts and separate quiet applicability appear in the report.
- Exact bit usage and the reference thresholded effective-bit statistic are
  separate. A padding observation requires exact unused bits in the analyzed
  interval. Exercised low bits do not prove recording depth.
- Float PCM must be finite with absolute amplitude <=16. Bit-level source-depth
  checks are unsupported for float input. There is no silent float-to-integer
  conversion for these checks.
- Full-stream decoded counts are compared with declared counts when present.
  Decoder verification is reported when available. These checks are not a
  comprehensive malformed-container validator. Prefixes and files without known
  counts/checksums have weaker integrity evidence.
- Algorithm-owned buffers are bounded by channel/block/window sizes. Decoder and
  container/metadata allocations are owned by the dependency; this is not yet a
  hard process-wide memory cap or a security-audited hostile-input parser.
- Python measures many features on a stereo mid signal. This implementation
  exposes each original channel and explicitly labels AAC's separate mid/side
  basis. Numerical parity is tested;
  complete report/score parity is neither implemented nor claimed.

### Provisional detectors

Segment analysis runs at 32 kHz and above. Each active probe uses a two-second
f64 Hann transform. Eligibility excludes low bandwidth and sparsely occupied
spectra, including isolated tones. A wall requires a sharp cliff and a quiet band
above its own cutoff. The legacy 18.5 kHz band measurement is retained separately;
it cannot evaluate a wall above 18.5 kHz on its own. These gates are engineering
choices awaiting corpus evaluation, not calibrated likelihoods.

Fewer than three eligible probes gives an inconclusive aggregate. Otherwise the
report distinguishes no walls, one isolated wall, repeated walls (at least two,
and at least half of eligible probes) and mixed probes. These are counts of
sampled windows, not proportions of a recording or inferred splice boundaries.
Mastering filters can also produce walls. All matching legacy codec profiles are
retained at 44.1/48 kHz, with per-profile distances/tolerances. They neither identify
an encoder nor prove a bitrate. Wall and candidate results share one evidence family.

MQA scanning supports native 16/24-bit integer stereo. It searches three bit
planes for the reference's 36-bit marker, retains up to 64 candidates, and counts
all matches within the first three analyzed seconds. Missing payload fields stay
null. The rate field is a reverse-engineered interpretation, not a verified
original recording rate. Tags, repeated-packet consistency and actual payload
decoding are not evaluated. Negative results apply only to the scanned interval.

Schema 0.2 added `segments`, `mqa`, and detector `intervals`. Frame ranges use
zero-based indices and an exclusive end; MQA `sync_end_frame` is explicitly
inclusive. Detector `channel_index` is now nullable for joint stereo observations.
Clients of schema 0.1 must update their report types. Ancestry remains inconclusive
and `evidence_index` remains null even when an individual detector reports a hit.

Schema 0.3 additionally exposes `resampling` and `vorbis`. Resampling checks the
foreign Nyquists of 44.1/48/88.2/96 kHz where the analyzed rate leaves enough
room above them. Inactive or insufficient frames abstain. Mirror correlation is
unavailable when either comparison band is too quiet. A filter can mimic these
patterns; they do not prove an original recording rate or lossy encoding.

The Vorbis search follows the pinned 2048/256-block heuristic: it compares
near-zero MDCT coefficient occupancy at every long-block phase, pools equivalent
phases modulo 128, and requires persistent support above off-grid baselines.
At least four active probes are required. A hit is a provisional observation;
the excess-zero score is not a probability. Different block sizes, noise,
transitions or later processing can erase the pattern. The search uses native
channels directly rather than reconstructing them from a Python mid/side buffer.

Schema 0.4 adds `aac`, one result for native mono or separate stereo `mid=(L+R)/2`
and `side=(L-R)/2` results. These transforms use f64 copies; exact PCM hashing and
native-channel measurements are unchanged. Each basis selects its own highest-energy
anchors on a 1024-sample grid, separated by more than 2048 samples, within the
analyzed prefix capped at 180 seconds. Every captured interval is reported.

AAC tests 2048-sample KBD alpha=4 windows at offsets 0..1016 stepped by eight.
It evaluates rounding-error energy across 49 scalefactor bands and eight scaling
levels. At least four active anchors and four usable probes at a shared phase/scale
are required; a usable probe needs sixteen energetic, populated bands. Sparse
tones and quiet signals abstain with a null score. `lattice_score` is the mean
flagged fraction of all 49 bands at the selected phase/scale, not a probability.
The provisional hit threshold is 0.10; it has not been calibrated on independent
recording groups. TNS, SBR, short blocks, other windows, off-grid trims and gain
changes may hide the pattern. A hit does not prove AAC ancestry, and a miss does
not establish lossless history. See [AAC_VALIDATION.md](AAC_VALIDATION.md).

Schema 0.5 adds `spectral_structure` for each native channel. A scatter bound is
the last bin above an adaptive threshold in a smoothed five-bin standard deviation
of clamped log power. The report gives the mean bound and the most frequent exact
FFT bound bin (lowest bin wins a tie). At least four non-flat active frames are
required; an entirely flat scatter profile yields null, not a Nyquist bound.
This is not an authenticity or analog-source classifier.

High-band phase entropy uses a 36-bin histogram of wrapped phase differences
between adjacent active frames. Only shared bins at or above 10 kHz with magnitude
above both `1e-4 * frame_peak` and `1e-8` unnormalized FFT units contribute; the
real-valued Nyquist bin is excluded. At least three frame pairs with two usable
bins each are required. Inapplicable or insufficient results remain null. White
noise can have high entropy without any codec processing. These features report
`measured`, `inconclusive` or `unsupported`, never a provenance hit. Their interval
is the envelope of evaluated STFT frames, with explicit counts and gated gaps,
excluding the unframed tail. See [STRUCTURE_VALIDATION.md](STRUCTURE_VALIDATION.md).

## Validation

`cargo test --locked` uses generated audio and checked-in Python numerical
oracles. It does not require private music, Python or FFmpeg. To regenerate the
oracles, install the reference requirements and FFmpeg and run:

```text
python scripts/generate_reference.py
python scripts/generate_detector_reference.py
python scripts/generate_transform_reference.py
python scripts/generate_aac_reference.py
python scripts/generate_structure_reference.py
```

The oracle source is pinned in `PORTING_PLAN.md`. Generated fixture WAVs contain
only synthetic noise and derived controls. The matching FLAC fixtures are encoded
by FFmpeg, with exact original PCM hashes independently computed in Python.
Do not regenerate oracles merely to
make a failing Rust test pass; investigate the discrepancy first.

`python scripts/check_detectors.py` additionally creates known synthetic controls
and FFmpeg MP3/AAC/Opus transcodes, runs the release executable, and checks decoded
PCM against FFmpeg. This optional experiment requires FFmpeg encoders; ordinary
Cargo tests do not. Commands, processing histories and results are saved locally.

`python scripts/check_transforms.py` exercises actual Vorbis quality levels,
transient block switching, trimming/gain, and FFmpeg SWR/SoXR rate conversion.
It requires NumPy and FFmpeg with those components. Outputs stay local and ignored.

`python scripts/check_aac.py` generates stereo AAC, transient, mid/side, trim/gain
and cross-codec controls, then compares decoded PCM exactly with FFmpeg. It accepts
`--work` and `--output` for preserving prior experiment directories. This optional
experiment requires NumPy and FFmpeg; ordinary Rust tests do not.

`python scripts/check_structure.py` exercises generated noise, filters, tones,
silence gaps, native-channel isolation and unsupported high-band coverage. It
also accepts `--work` and `--output` and keeps all experiment results local.

The user-supplied CD/vinyl collection under `reference/test_files` is local
diagnostic material. Described origin is recorded as a claim, not ground truth;
it is excluded from public tests and accuracy estimates.

To measure the release executable on local files and compare each decoded PCM
hash with FFmpeg:

```text
python scripts/validate_local.py reference/test_files
```

Results stay in `corpus/local/results/local-validation`. On Windows the script
also samples the process's OS-reported peak working set. This is a local
engineering check, not a source-provenance or accuracy evaluation.

## Remaining work

Analog profiling, MQA confirmation, bit-depth noise-floor
analysis, psychoacoustic artifacts, loudness, scoring/calibration and Android bindings/UI remain to be
implemented. Reports enumerate this missing coverage. The initial core is useful
for validating decoding, numerical behavior and resource use while the real
validation corpus is collected.

See [PORTING_PLAN.md](PORTING_PLAN.md) and [TEST_CORPUS.md](TEST_CORPUS.md).
Actual results and known input-file errors are recorded in [VALIDATION.md](VALIDATION.md).
The new detector checks are recorded in [DETECTOR_VALIDATION.md](DETECTOR_VALIDATION.md).
The resampling/Vorbis milestone is recorded in [TRANSFORM_VALIDATION.md](TRANSFORM_VALIDATION.md).
The AAC milestone is recorded in [AAC_VALIDATION.md](AAC_VALIDATION.md).
The spectral-structure milestone is recorded in [STRUCTURE_VALIDATION.md](STRUCTURE_VALIDATION.md).
The transient milestone is recorded in [TRANSIENT_VALIDATION.md](TRANSIENT_VALIDATION.md).
The spectral roll-off milestone is recorded in [ROLLOFF_VALIDATION.md](ROLLOFF_VALIDATION.md).
The cross-band envelope milestone is recorded in [ENVELOPE_VALIDATION.md](ENVELOPE_VALIDATION.md).

## License and attribution

MIT; see `LICENSE`. DSP conventions and portions of the algorithms are ported
from Bishal Das's Python audio-forensic project at the pinned reference commit.
Dependencies have their own licenses, including Symphonia's MPL-2.0.
