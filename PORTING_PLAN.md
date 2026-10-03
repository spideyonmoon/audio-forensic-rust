# Rust port decisions

Agreed direction: Rust analysis core, entirely offline, with an Android UI after
the core is validated. The initial Rust decoding/measurement library and desktop
CLI are now implemented; see `README.md` for coverage and `VALIDATION.md` for
recorded checks. The full forensic engine remains in progress. Corpus collection
and core development can proceed independently.

## Python reference

- Repository: https://github.com/spideyonmoon/audio-forensic
- Local checkout: `reference/audio-forensic`
- Baseline commit: `c6ecce2296256b516709d87088896d1be913908c`
- Keep this checkout unchanged for reproducible comparisons. Record any future
  baseline update explicitly; do not silently track a moving branch.
- Preserve useful numerical behavior, not known defects or unsupported claims.
  Matching Python is a porting check, not proof that a detector is accurate.
- Do not undertake a Python package/module refactor before the port. If the
  Python CLI remains in active use, small user-facing bug fixes can be backported
  separately with their own tests and commits.

## Where the previous assessment belongs

| Recommendation | Decision |
| --- | --- |
| Real validation corpus | Build once here; run both engines against the same files. |
| Confidence language | Use an evidence index and scoped findings in Rust from the start. Optional small Python backport if continuing to ship that CLI. |
| Correlated evidence | Keep individual measurements, group related evidence, then evaluate aggregation policies on development data. Do not invent calibrated weights. |
| `--fast` | Give every Rust detector an explicit analysis interval and report actual coverage. |
| Failures/timeouts | Return structured per-file and per-detector outcomes; one failed file must not stop the batch. |
| Stream/channel selection | Bind metadata and samples to one explicitly selected stream. Preserve native channels; declare detector applicability. |
| MQA | Preserve integer samples and report sync/payload observations. Strengthen and validate confirmation criteria before exposing a structural-confirmation claim. |
| Module structure, schema, CI | Implement directly in the Rust project. A Python restructuring adds little to the port. |
| Streaming/resource limits | Design now and measure throughout the port, before adding the Android UI. |
| New detector research | Follow initial detector verification and corpus evaluation. |

Assessment correction: the Python bit-depth pass reads a separate 30-second
window, normally around the midpoint of longer tracks. It does not read the whole
track, but it still ignores `--fast`'s advertised first-60-seconds interval.
Loudness, SoX statistics and spectrogram extraction also do not receive that limit.

## Core contracts

1. Separate decoded samples, DSP features, detectors, evidence aggregation and
   presentation. The CLI and eventual Android UI consume the same typed report.
2. Give reports engine, schema and analysis-policy versions. Each detector returns
   its ID/version, status, measurements/units, thresholds, evidence family,
   analyzed stream/channels/time intervals, and caveats/diagnostics.
3. Detector outcomes distinguish a hit, no hit, insufficient evidence,
   unsupported input and processing failure. A skipped or failed detector never
   contributes a fabricated zero measurement or a successful-clear result.
4. Keep lossy-ancestry evidence, sample-rate history, bit-depth observations,
   container corruption and listening-quality measurements separate. Resampling
   is not by itself evidence of lossy codec ancestry; low bandwidth is not proof
   of provenance. A valid container does not establish the history of its audio.
5. Use `NO_INDICATORS`, `WEAK_INDICATORS`, `STRONG_INDICATORS` and `INCONCLUSIVE`
   as candidate ancestry labels; boundaries remain provisional until evaluated.
   Structural findings identify exactly what was observed/confirmed and how.
   There is no generic 100% confidence override.
6. A single MQA sync candidate and editable tags are not sufficient grounds for
   the current unconditional score-of-100 behavior. Do not invent packet spacing
   or consistency rules: establish them from suitable format evidence and
   independently characterized fixtures, then test false matches and truncation.
7. Group wall/cutoff/cliff/void and related segment evidence to limit repeated
   counting of the same phenomenon. Family caps alone do not establish
   statistical independence or calibrated probabilities.
8. Preserve integer PCM for bit-level checks and document float conversion for
   spectral checks. Lossless decoder tests require exact sample agreement;
   floating-point DSP comparisons use explicit per-metric tolerances. Test lossy
   decoder differences separately from DSP port differences.
9. Support bounded buffers, cooperative cancellation, a global worker budget,
   size/allocation limits and explicit decode errors. Avoid allocating from
   untrusted duration fields without limits. Some detectors may require selected
   windows, additional passes or bounded temporary storage; do not assume every
   full-track statistic can be reproduced by a naive streaming average.
10. An Android decoder must accept app-provided input handles/streams, rather
    than require unrestricted filesystem paths or command-line executables.

## Implementation order and acceptance gates

### 1. Establish a reproducible baseline

Run the Python DSP, accuracy, Vorbis and MQA tests and record actual results,
versions and skipped tests. Export deterministic development fixtures and
intermediate measurements for Rust comparisons. Keep test generators in version
control; keep large/private recordings outside it. The pinned Python baseline
has now been re-run: 79/79 DSP checks and all 22 accuracy/Vorbis/MQA unittest
methods passed. The test suite's synthetic encoder cases ran without skips.

### 2. Build Rust decoding, report types and DSP primitives

Start with WAV/FLAC mono and stereo, a desktop CLI, explicit stream selection,
sample precision, error outcomes and coverage reporting. Verify PCM decoding,
windows, FFT/STFT normalization, frame boundaries and basic features against
shared fixtures. Include short/silent inputs, malformed headers and truncation.
Measure peak memory/runtime with increasing file duration and sample rate.

### 3. Port detectors incrementally

Port basic spectral evidence, bit-depth observations, segment analysis,
resampling evidence and source-profile features before the more delicate
AAC/Vorbis transform searches. Port MQA observations with the conservative
contract above. Preserve regression controls and document deliberate changes
from Python. Extend codec support through an explicit tested support matrix.

Version 0.2 now implements bounded segment-wall observations, overlapping codec
wall candidates and conservative MQA sync/payload candidates. It intentionally
changes the Python segment sampling/gates and omits the unconditional MQA score
override. Numerical clip parity and known processing controls are recorded in
`DETECTOR_VALIDATION.md`. These additions do not complete the detector suite or
justify an ancestry score; resampling/source-profile and transform features remain.

Version 0.3 adds streaming resampling-candidate statistics and the Vorbis transform
grid with bounded native-channel capture, a tested MDCT kernel, and phase-batch
cancellation. `TRANSFORM_VALIDATION.md` records parity checks and controlled
processing experiments. AAC quantization-lattice, source-profile and remaining
features still need their own implementation and validation gates.

Version 0.4 implements AAC KBD long-window quantization-lattice observations at
44.1/48 kHz. Bounded first-pass energy selection and second-pass span capture
replace Python's full-prefix buffers. Reports explicitly identify mono or
mid/side basis, analyzed spans, winning phase/scale and band applicability.
Sparse spectra abstain rather than returning a successful-clear zero. Numerical
comparisons and generated encoding/editing controls are recorded in
`AAC_VALIDATION.md`. The overall ancestry verdict remains inconclusive; this
does not complete codec coverage or validate an aggregation policy.

Version 0.5 adds the pinned auCDtect-style scatter and phase features as streaming
native-channel measurements. It uses centered f64 scatter, an exact-bin mode,
all active frames and energy-gated phase differences between adjacent frames.
Flat scatter, silence and missing high-band coverage abstain. These features
return measurements rather than importing Python's codec/analog interpretations
or score adjustments. See `STRUCTURE_VALIDATION.md` for numerical comparisons and
intentional policy differences. Vinyl/cassette profiling remains separate work.

Version 0.6 adds bounded native-channel quiet-run statistics and normalized
high/above-cutoff band power over all STFT windows and qualifying quiet windows.
Three fixed spectral accumulators replace whole-prefix filtering; no extra FFT
or decode pass is needed. Quiet intervals are capped in the report, with complete
aggregate counts. `NOISE_VALIDATION.md` defines normalization, intervals,
applicability and numerical controls. Correlation, temporal variation, clicks,
source profiles and noise-based scoring remain unimplemented.

Version 0.7 adds explicitly circular band-limited correlation at two lags and
one-second band-level variation over a capped 180-second history. It reuses STFT
power, preserving all-window coverage for global correlations while bounding
temporal spectra. Quiet/insufficient blocks abstain rather than becoming zeros
or being dropped. `NOISE_DYNAMICS_VALIDATION.md` records independent inverse-FFT
time-domain checks and deliberate differences from full-stream Pearson correlation.

Version 0.8 adds capped native-channel high-pass envelope peaks using the existing
two decode passes, a bounded median histogram and chronological peak selection.
Reports expose baseline bounds, thresholds, eligible intervals and capped event
lists. Independent SciPy controls and limitations appear in
`TRANSIENT_VALIDATION.md`: three smooth musical bursts produce 60 envelope maxima,
so these measurements must not be described as physical click counts or source
confirmation. Existing cutoff variation must not be relabeled wow/flutter;
analog-source inference remains gated on independent recordings and evaluation.

Version 0.9 adds the cassette profile's useful two-endpoint roll-off measurement,
with actual bin geometry, coherent-amplitude applicability gates and constant
storage. Generated EQ changes demonstrate that the slope cannot identify a medium.
Unsupported/quiet endpoint bands abstain instead of receiving fabricated levels.
See `ROLLOFF_VALIDATION.md`.

Version 0.10 implements signed Pearson correlation of two per-frame band RMS
envelopes with constant-size online covariance. Full-band geometry, mean energy,
temporal variation and frame-count gates replace the reference's fabricated
zero/one fallbacks. `ENVELOPE_VALIDATION.md` records independent time-domain band
power and stored-envelope comparisons. This is not an authenticity or injected-noise
classifier.

Version 0.11 adds below-cutoff sparse-bin fractions using per-frame relative
magnitudes and the channel's final global active-frame p95 cutoff. Per-bin counts
replace a spectrogram; DC/Nyquist, low-amplitude frames and insufficient coverage
are explicitly handled. `SPARSITY_VALIDATION.md` defines geometry and independent
tone/filter controls. Sparsity is not a codec bin-zeroing diagnosis.
Version 0.12 adds the useful quiet-block profile arithmetic: bounded native-channel
RMS percentiles and selected-block spectral color using the existing two decode
passes. Original indices survive silence removal; all-zero/underflow blocks,
partial tails, energy gates and first-300-block coverage are explicit.
`NOISE_FLOOR_VALIDATION.md` records differences from the reference's downmix,
compacted-index bug and independent midpoint window. Measured quietness or
spectral color must not become a source bit-depth or authenticity verdict.
Version 0.13 adds BS.1770 integrated programme loudness using exact two-pass
gating and a fixed power ring. Ungated 400 ms/3 s maxima are sampled every 100 ms.
Native channel powers are summed without a downmix, and shared-prefix, window,
gate and unsupported-rate coverage are explicit. `LOUDNESS_VALIDATION.md` records
the numerical contract and independent controls. True peak, loudness range and
meter conformance are separate work; no mastering-quality score is introduced.
Version 0.14 adds per-native-channel fourfold FIR true-peak estimates and
complete-window loudness range. Fixed history/histogram storage, exact two-pass
range gates, quantization bounds, prefix/tail conventions and limitations are
explicit. `LISTENING_LEVELS_VALIDATION.md` records independent numerical controls.
These measurements do not certify a meter or establish mastering quality.
Version 0.15 implements the reference's high-band spectral-lag arithmetic using
existing mean-spectrum sums. Full 16–20 kHz geometry, every-bin energy/variance
gates, signed products, neighbours and individual lag applicability are explicit.
`SPECTRAL_LAGS_VALIDATION.md` records independent comb/EQ/noise/tone controls and
actual checks. No MP3 verdict or scoring rule is imported.
Version 0.16 adds native sample crest with linear/dB units, scaled underflow-safe
RMS and signed centered stereo PCM correlation over simultaneous pairs. Numerical
variation gates, shared-prefix coverage and independent gain/phase/DC controls
are in `PCM_RELATIONSHIPS_VALIDATION.md`. No compression, fake-stereo or quality
label is imported.
Version 0.17 adds bounded causal 10–20 kHz energy in eligible contexts before
existing envelope peaks. Baseline bounds, startup/history exclusions, complete
aggregate counts and independent impulse/attack/gain controls are recorded in
`PRECEDING_ENERGY_VALIDATION.md`. Smooth musical attacks often exceed the baseline;
this does not establish codec pre-echo.
Version 0.18 adds one active core analysis per process, cooperative waiting that
counts toward deadlines, bounded container preflight and decoded-capacity checks.
Native signatures, metadata limits and PCM WAV geometry are explicit support
conditions. `CORE_ACCEPTANCE_VALIDATION.md` records the audit, actual checks and
remaining engineering/corpus/device gates. No Android app or score is introduced.
Its final 130-test release regression, 69 independent generated numerical cases,
nine duration/rate resource controls, Clippy/formatting and ARM64 target compilation
passed. A private prefix-only compatibility run returned four explicit unsupported
inputs; those are support-limit observations, not provenance labels. Engineering
evidence does not close grouped-corpus accuracy, schema stability or Android
link/device gates.
Fractional loudness hops at rates not divisible by ten remain explicitly unsupported.

The declared Rust 1.85 minimum now passes all 130 ordinary debug tests,
local desktop all-target compilation/Clippy,
source-only CLI build/Clippy, no-CLI library compilation and ARM64 Android target
compilation with the frozen lockfile. Minimum-compiler desktop/Android jobs are
declared in CI; remote CI and NDK linking remain outstanding. Detailed compiler,
host-linker and execution evidence is in `CORE_ACCEPTANCE_VALIDATION.md`.

Remaining-reference assessment (2026-10-02, pinned source inspected):

- `_psychoacoustic_artifacts` combines three unrelated heuristics. Its 16–20 kHz
  spectral autocorrelation at `sample_rate/64` multiples can reuse a fixed
  mean-spectrum accumulator. Report actual lag/bin geometry and neighbours;
  a periodic spectrum does not establish an MP3 filterbank history. Do not keep
  its outer cutoff/MP3 gate as an undocumented condition on an ordinary measurement.
- Its alleged HF mirror test correlates two time-domain bandpass signals after
  negating one, then takes absolute Pearson correlation. Negation disappears
  under the absolute value and does not reverse frequency order. This cannot be
  ported as a demonstrated frequency-mirroring or codec-aliasing detector.
- Its pre-echo pass selects peaks above an absolute -3 dB envelope threshold,
  counts preceding-band energy above a median baseline, and includes edge-ineligible
  peaks in the denominator. A bounded port needs explicit eligible-event counts,
  filter timing, baseline scope and gain/attack controls. Preceding musical energy
  is not itself evidence of codec pre-echo.
- Remaining stereo correlation and crest-factor presentation are descriptive
  measurements; the reference's fake-stereo/compression/quality labels are not
  accepted. Source-profile verdicts, MQA confirmation and evidence aggregation
  still require independent characterization and grouped evaluation.

Group evidence separately from the numerical port, so a changed score can be
traced to policy rather than accidentally changed arithmetic. Add remaining
loudness/reporting features with suitable reference measurements. A subset of
implemented detectors must not be presented as complete analysis.

### 4. Evaluate and stabilize the core

Use development source groups for algorithm work and grouped validation for
threshold selection. Freeze policy before evaluating the locked test groups.
Report performance per codec/processing family with independent source counts,
false positives, misses, abstentions, coverage and uncertainty. Derivatives of
one recording are related observations, not independent evidence of accuracy.
Unknown-provenance examples are diagnostic cases, not ground-truth labels.

Local intake tooling now accepts explicit provenance/group/split declarations,
fingerprints originals/notes and the binary, and preserves full-file measurements
in immutable private receipts. Parent lineage, declared split consistency and
identical encoded/decoded copies are checked without deriving ancestry labels.
Locked groups are hashed and reserved without analysis; exact decoded-duplicate
checks apply only to measured groups. See `CORPUS_INTAKE_VALIDATION.md` for the
16 stdlib and six generated end-to-end controls, and `TEST_CORPUS.md` for ingestion.
This prepares evaluation; independent source histories still require review.

The 2026-10-02 private follow-up analyzed 37 of 44 supplied originals with exact
independent PCM and preserved seven explicit unsupported/failed results. Added
codec experiments found real-music coverage limits, including Vorbis sensitivity
to final 16/24-bit quantization with exact pinned-Python feature agreement. These
challenge receipts do not supply verified negative ancestry labels or calibration;
see `CORPUS_INTAKE_VALIDATION.md`. Extra voice/capture hardware notes are optional
and do not block core engineering with the supplied music.

The subsequent AAC music audit passed 44 matched-basis comparisons, including
unchanged generated oracles and sparse abstention controls. The missed piano
conversion reproduces in the pinned method and persists across tested TNS/M/S
settings. Do not lower a threshold to make this recording pass; broader excerpt,
stereo and encoder-tool controls are required. The subsequent fixed-excerpt and
intensity-stereo/PNS audit passed 72 matched-basis comparisons (54 new, 18 reused)
with exact PCM and native-parent trims. Those piano cases still miss, while the
other source groups retain hits. A private all-integer-phase diagnostic passed
20 coarse-subset/PCM checks: it recovers the generated 137-sample trim but leaves
all four piano AAC excerpt maxima unchanged. A production phase expansion still
needs resource/cancellation and broader false-hit controls; source-dependent
coverage research must preserve the existing threshold/denominator until grouped
validation supports changes. See `AAC_MUSIC_VALIDATION.md`.

Validate runtime/memory limits, corrupt-input handling and cancellation. Stabilize
the public API/report schema and support matrix. A small ARM64 build/link check
during core work may catch portability issues; it is not Android app development.

The 2026-10-03 report-contract milestone exports 47 serialized model definitions
as versioned JSON Schema and adds optional offline validation. All 125 fresh/saved
v0.18 reports passed shape, policy and interval/channel checks; 51 invalid
controls were rejected and two numeric boundary controls passed. A generated-only
Rust 1.85 integration passed all five file outcomes with exact PCM hashes.
See `REPORT_SCHEMA_VALIDATION.md` and `schemas/README.md`. This adds validation
without changing wire output or declaring the evolving API stable. The user
deferred Android. The following engine 0.18.1 continuation fixes interrupted
reads, probe-masked I/O errors, failed float precision and batch directory error
handling. Focused source/CLI controls and a 788-case generated header/truncation
campaign passed; all 147 final Rust 1.85 debug tests, all-target Clippy,
no-CLI library check and offline production-only snapshot build also passed. Read
`SOURCE_API_VALIDATION.md`. Schema and detector policy remain unchanged.
The 0.18.2 WAV follow-up characterizes integer/float container widths, valid-bit
padding, channel masks, GUIDs, unknown lengths and RF64 with generated exact PCM.
It rejects unknown data lengths consistently in full/prefix mode, checks the
whole subtype GUID and supported mono/stereo mask, validates header geometry and
rejects nonzero PCM padding within the measured interval. RF64/BW64/RIFX remain
explicitly unsupported; unknown source byte length is distinct from an unknown
WAV data length. See `WAV_FORMAT_VALIDATION.md` for actual checks and the retained
FFmpeg 24-in-32-bit auto-detection disagreement. No decoder dependency, report
schema, detector threshold or ancestry policy changes in this patch.
The expanded generated WAV audit covers every valid integer precision within
8/16/24/32-bit containers plus float amplitude and underflow boundaries. All 220
cases / 440 full-prefix reports and 180 independent PCM comparisons passed;
invalid measured floats fail while errors outside a requested prefix remain
outside its scope. The production-only 0.18.2 Rust 1.85 build/Clippy/no-CLI checks
and its 128-report format matrix passed. The deterministic 788-case mutation
regression also passed, with stricter byte-rate failures recorded explicitly.
The final frozen-source Rust 1.98.1 release regression passed all 153 tests,
zero failed/ignored; 23 focused Rust 1.85 tests and all-target MSRV Clippy passed.
The interrupted MSRV debug run is retained as incomplete, not a full-suite pass.
These validate engineering contracts, not source ancestry or detector accuracy.

The 0.18.3 FLAC follow-up replaces reader rewind with a bounded fresh probe,
checks unchanged STREAMINFO, continuous sample timestamps and complete accounted
packet framing. Generated controls exposed retained reader sequence state and
valid-CRC packets with extra ignored bytes; both are now explicitly checked.
Unknown source length, unknown total samples and absent MD5 remain supported
availability conditions. Prefixes do not validate later frames or whole-stream
checksums, and a byte-complete shorter stream without total/MD5 cannot establish
missing final frames. The 270-case integrity matrix and 96 compressed controls
passed with exact independent PCM and 732 schema-valid reports; MSRV regression
and production-only validation are in progress. Read `FLAC_INTEGRITY_VALIDATION.md`.
Schema, dependencies and detector policy are unchanged; Android stays deferred.

### 5. Build the Android application

After the core gates pass, add Kotlin bindings and the UI: file selection,
background analysis, progress/cancellation, findings and local report storage.
Then validate on actual Android hardware for memory, runtime, lifecycle and
thermal behavior. Desktop benchmarks cannot establish phone performance.

## Collection

See [TEST_CORPUS.md](TEST_CORPUS.md) for the shopping list and collection notes.
Begin with a small development collection; do not wait for a large corpus to
start engineering. Reserve unseen source groups before detector tuning begins.

Validation references:

- [Grouped evaluation](https://scikit-learn.org/stable/modules/cross_validation.html#cross-validation-iterators-for-grouped-data)
- [Avoiding test-data leakage](https://scikit-learn.org/stable/common_pitfalls.html#data-leakage)
