# Rust port decisions

Alfred now lives in its [separate repository](https://github.com/spideyonmoon/alfred/tree/codex/extract-android).
Owner requested extraction on 2026-10-09; Android continuation and UI scratchpad
coordination belong there. Core engine ownership and historical acceptance remain here.

A06a Spectrogram and A06b live/saved Compare completed 2026-10-09. Accepted
compiled source 5c5ce00 passed both ABI APK builds and full Android 11–16 generated
worker/Compose/input/lifecycle acceptance in [CI 37820435450](https://github.com/spideyonmoon/audio-forensic-rust/actions/runs/37820435450). Spectrogram
retains P06 presets/canvas/original export; saved Compare retains original coverage,
order, admission/native pins and independently owned product bytes. Native terminal
failure codes survive the shared adapter. Rust computation/policy/schemas/oracles
remain unchanged. Next A07 physical resources/device gate; no jobs running.
See [A06a](task-results/A06a.md), [A06b](task-results/A06b.md) and current handoff.

## A06 Forensics integration — 2026-10-08

DONE. Workspace Forensics uses shared input/jobs/storage with unchanged Rust
products, qualified uncalibrated reference summaries, complete read-only exact
field inspection, mixed outcomes, fresh retry, history/delete and original-byte
SAF export/FileProvider share. Both ABI builds and Android 11–16 generated
feature/Compose controls passed across full 37806954007 and verified-APK retry
37809483892. Compiled source is 8094a01; later repairs are harness-only.
No Kotlin scoring, core/native Rust change or physical acceptance. Next A06a
Spectrogram and A06b Compare; A07 owns physical resources. [A06](task-results/A06.md).

## A05 shared jobs — 2026-10-08

DONE. Implemented feature-neutral serial foreground jobs, fresh persisted attempts,
bounded progress/cancellation, rotation/process recovery, result quotas/leases,
grant ownership and explicit export/share. Both ABI builds and A05 generated
lifecycle/storage checks passed on Android 11–16 across source-bound runs. Final
API-34 verified-APK retry 37777694067 passed picker before lifecycle, retaining
every check. No jobs running; next A06. Feature execution/results stay A06/a/b;
physical resource acceptance stays A07. [A05 evidence](task-results/A05.md).

## A04 shared input — 2026-10-08

DONE. Shared SAF identities/grants, bounded enumeration/staging and native capability checks passed both ABI builds and runtime controls API 30–36, automated picker API 31–36 and owner manual Hot 11S/Android-11 picker checks. Real-file ALAC diagnosis required a narrow unreleased core preflight correction for all-zero dependency tables and generic 16-bit sample-entry fields with 24-bit cookies. Eight ALAC tests, Clippy, exact private one-second PCM checks and replacement Android/core CI passed. Owner approved closure without waiting for the replacement-APK physical ALAC retest; that retest was not run. No DSP/scoring/schema/dependency changes. Next A05. [A04 details](task-results/A04.md).


## A03 native adapter — 2026-10-08

DONE: CI 37673381546 passed both ABI builds and all API 30–36 JNI smoke jobs.

The app-owned `apps/alfred/native/adapter` now provides JNI byte-array transport,
one serial cancellable worker, exact payload descriptors and atomic attempt
publication around the unchanged core probe/product/Spectrogram/Compare APIs.
Kotlin preserves integer tokens as BigInteger, decimal tokens as BigDecimal and
original payload bytes. Shared source/storage/history ownership remains A04/A05;
this does not make the scaffold a complete analysis UI. See
[adapter contract details](https://github.com/spideyonmoon/audio-forensic-rust/blob/79470a3/apps/alfred/NATIVE_ADAPTER.md) and
[A03 checks/current acceptance](task-results/A03.md).

Product admission checks native rate, capture frames, padded FFT and silence
scratch against the reservation. Full high-rate requests conservatively require
an explicit prefix even for a header-declared short file, since that declaration
does not bound actual decoded frames. The accepted core semantics are unchanged;
A07 still measures physical memory/runtime and validates engineering margins.

## Android compatibility amendment — 2026-10-07

Owner authorized Android 11–16 (min/native API 30, compile/target 36), including
Infinix Hot 11S/Helio G88, and GitHub Actions for heavy builds/emulator checks.
Primary phone ABI stays ARM64; x86_64 is a separate CI emulator artifact.
Shared memory admission/frame limits stay unchanged; no device-specific chunking
or throttling. Build/emulator evidence is separate from physical A07 acceptance.
See [compatibility packet](task-results/ANDROID-11-CI.md).
P09/A01–A05 are complete; A06 Forensics results/history/export is next.

## Current delivery priority — 2026-10-06

Validation records now live under `docs/validation/`; see its README index for
bare historical filenames used below. Host guidance is `docs/CORE_INTEGRATION.md`.

Product correction: finish the standalone Audio Forensics library/CLI through
P07/P08/P09, then begin Alfred at A01. Alfred is a file/folder-first Android
workspace consuming this independent library, initially co-located and later
extracted to its own repository. Shared input/jobs/navigation/storage are app
infrastructure; Forensics is one feature. Spectrogram and Compare are independent
features reusing P06/P07 now without duplication/refactoring. Spectrogram's
broader competitive viewing/exploration scope is not exhausted by P06 images;
A06a is its initial slice. A01 must define long-term reusable spectrogram and
comparison ownership/contracts/tests when Alfred moves repositories, preserving
independent core consumers. Future expansion needs named requirements packets.
No other feature implementation is authorized now.
See ALFRED_ARCHITECTURE.md and revised A cards; earlier application assumptions
are superseded, historical validation remains intact.

P05 in engine 0.31.0 implements a separate reference assessment v1 from bound
P04 v2 inputs: all 34 rule rows, source/depth candidates, selective vetoes,
Python rounding, excluded D05/D06/D07 effects and missing-input propagation.
The pure saved-result API preserves native measurement JSON. Exact boundary
parsing uses serde_json float_roundtrip. See REFERENCE_ASSESSMENT_VALIDATION.md
and task-results/P05.md for evidence and the rejected/corrected oracle history.
P07 implements product reporting/comparison in 0.32.0. P08 acceptance is complete:
271 Rust tests and finite differential/schema/workflow/unsupported-DSD controls
passed; see docs/validation/CORE_RELEASE_VALIDATION.md and task-results/P08.md.
P09 accepted the scoped standalone 0.32.0 library/CLI on 2026-10-06 after
verifying the unchanged frozen tree/binary and P08 receipts and reviewing product,
policy and host/resource risks. Fresh inventory/version/saved-render/comparison
checks passed; no production change or repeated full regression. See
task-results/P09.md and the release ledger for accepted limits. A01 completed
2026-10-07: ANDROID_CONTRACT.md freezes app-owned JNI, shared input/jobs/storage,
conservative native-rate resource admission and extraction with P06/P07 backends
retained here. A02 scaffold, A03 native adapter, A04 shared input and A05
jobs/retention acceptance are complete; next A06. No device acceptance is implied.


Follow `ROADMAP.md` and `ROADMAP_TASKS.md`: faithful rewrite and offline Alfred
Android app first, endgame improvements separately. Android is now authorized;
older deferral entries are historical. Initial required formats are FLAC/WAV and ALAC/M4A. The owner explicitly
deferred F02/DSD beyond the first release and Alfred launch on 2026-10-06.
The frozen DSD scope remains a later requirement, not shipped support.
P01 freezes numerical/output fidelity and deviations; P05 implements a
separate uncalibrated reference assessment, keeping measurement ancestry/index
unchanged. P01 is complete: `PYTHON_PARITY.md` maps 155 fields/139 methods and
`RELEASE_CONTRACT.md` v1 freezes domains, tolerances, deviations and format scope.
Required child packets P03a/P04c close missing tool statistics/DR and reference
input adapters. Consult the owner-edited `OWNER_CHECKLIST.md` for actual priorities,
device and fresh-on-demand evaluation policy. P02 implements separate bounded
FLAC/WAV metadata, encoder/MQA text claims and the pinned ReplayGain comparison;
see `METADATA_VALIDATION.md` and `task-results/P02.md`. Raw keys/duplicates and
truncation remain explicit; no tag changes measurement ancestry or score.
Native tag mapping preserves ReplayGain/IART that MediaInfo 24.01's Python
projection misses. Metadata/audit version 1 freezes collector hooks for F01/F02;
opaque ancillary structures and absent header fields remain explicit. Engine
0.23.0 retains measurement schema/policy unchanged. P03 in engine 0.24.0 adds
separate reference byproducts and native-level display, collected optionally
inside the existing two-pass source/worker contract. See BYPRODUCT_VALIDATION.md:
mean valid 100 ms reconstructed L/R correlation, fixed 16-bit-ceiling sample
counts, strict mid quiet runs and exact positive-block RMS p5. Native channels,
precision full-scale counts, meters and measurement schema/policy are unchanged.
Silence percentage uses analyzed frames (D02), native methods and separate
linear/dB crest remain explicit (D04); -16/-14 deltas are fixed method targets
(D11). Bounded RMS scalars/quiet-section output have explicit resource-limit/
omission states; no duration-sized PCM, external tools or new decode pass.
P03a in engine 0.25.0 adds optional native-lane astats, all canonical SoX stat
keys and a distinct f32 drmeter implementation in the same two passes/worker.
Separate version-1 tool statistics carry methods, input domains, exact counts,
coverage/hash, finite/null applicability, corrected crest units and legacy audit
selections. Default APIs/measurement JSON are unchanged. Histograms and 50 ms
arrays have fixed stereo payload ceiling 1,794,080 bytes, excluding native/P03
buffers. No duration-sized PCM or runtime tool. Pinned FFmpeg 7.1.1/SoX 14.4.2
source/controls precede Rust implementation; undefined tails/windows and upstream
queue quirks are explicit. Overall DR labels differ from Python's first channel
label; retain both with attribution for P05/P07. See TOOL_STATISTICS_VALIDATION.md.
P03/P03a acceptance is complete; see task-results/P03.md and task-results/P03a.md.
P04a/P04b are implemented in engine 0.27.0; reference-input schema v2 adds
spectral/header/source-profile adapters and closes adaptive-wall dependencies.
See REFERENCE_PROFILES_VALIDATION.md for pinned comparisons, deviations and
384kHz storage bounds. Native measurement schema/policy remains unchanged.
F01 in engine 0.28.0 adds offline native ALAC in M4A to the existing APIs,
16/24-bit mono/stereo at 8–384 kHz, with both moov layouts. Bounded preflight
checks native cookies and table/mdat geometry before the locked MP4 reader;
unknown source length is discovered by seeking to end. P02 metadata version 1
exposes ilst/freeform text, numeric tags and artwork/opaque descriptors. Native
measurement schema/policy stays unchanged; AAC/DRM/fragmentation/ambiguous tracks
remain unsupported. ALAC has no embedded PCM checksum; exact independent and
cross-pass PCM hashes are separate checks. See ALAC_VALIDATION.md and
task-results/F01.md for actual acceptance results and explicit limits.
F01 acceptance completed 2026-10-06: exact generated PCM, bounded metadata,
structured source/failure controls, no-CLI/MSRV and Android target checks passed.
Android linking and device validation remain separate gates.
Next delivery task: A01; P09 accepted 0.32.0 and F02/DSD remains deferred beyond first release/launch.
P06 completed 2026-10-06 after separate request: engine 0.29.0 adds bounded optional
same-two-pass spectrogram collection and offline rendering, artifact v1/method
`hann1024-power-pair-merge-v1`. Hann1024/hop512 replaces Python's SoX pixels,
with explicit mono/stereo-mid basis, power normalization, scope and time-bucket
coarsening. Native measurement schema/policy is unchanged; artifact/export
failure is separate. See SPECTROGRAM_VALIDATION.md for A06 axes/resource/export
contract and actual acceptance results. The owner requested a follow-up in
engine **0.30.0**: composition and lossless PNG encoding now belong to Rust,
with embedded antialiased text, full stream/FFT header including same-pass
scoped bitrate, calibrated frequency/time ticks/grid, ember-v1 dB legend and
native-channel p95 overlays. Bounded canvases are 1600×900 / 2560×1440 default /
3840×2160, 300-dpi metadata. Pixel quality changes do not alter v1 spectral data
or FFT precision. P07/A06 reuse the engine image and own workflow/storage/sharing.
See `task-results/P06-PNG.md`. This supersedes the original app-owned PNG decision.
Focused final-source Rust checks passed 78 tests across two runs; serialized
generated full/prefix/schema/hash/raster controls, MSRV Clippy with/without CLI
and Android target compilation passed. Android linking/device checks remain
separate. Next default delivery task is A01; P09 accepted 0.32.0 and F02 is deferred.
The owner supplied independent MQA work during P01; see `task-results/P01-MQA.md`.
E09 reviews that existing prototype/evidence and does not block the app.

Agreed direction: Rust analysis core, entirely offline, with an Android UI after
the core is validated. The initial Rust decoding/measurement library and desktop
CLI are now implemented; see `README.md` for coverage and `VALIDATION.md` for
recorded checks. The full forensic engine remains in progress. Corpus collection
and core development can proceed independently.

P04c in engine 0.26.0 adds separate reference inputs version 1, method
`python-c6ecce2-p04c-f32-v1`, and source/path APIs. The shared f32 basis, base
STFT, source sampled scatter/20-bin mode, qualified phase, Random(42) segment
probes and transform winner/support adapters preserve native report/API values.
Actual pinned Vorbis reconstructs L/R after widening f32 M/S. The opt-in third
pass verifies the same PCM/prefix/source; ordinary APIs remain two-pass. Segment
PCM uses a two-second ring rather than a full track or all requested clips.
In 0.27.0 adaptive-wall dependencies consume P04a/P04b inputs; no STFT noise
substitute or ancestry score is added. Read REFERENCE_INPUTS_VALIDATION.md and task-results/P04c.md.

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

Version 0.22.0 adds a scoped codec-stage label contract and offline Rust
evaluation infrastructure. `FrozenEvaluation` binds declarations, groups, splits,
independent PCM hashes and versions; `EvaluationSession` processes saved reports
one at a time. The CLI-free example freezes a plan and runs only one requested
split. AAC/Vorbis pattern hits/no-hits/abstentions are counted once per file and
weighted equally by declared source group within processing/provenance strata.
Unknown histories remain challenge-only and outside labeled rates. This
implements evaluation accounting, not independently reviewed labels, a corpus,
population uncertainty estimates or calibrated ancestry classification. See
`EVALUATION_VALIDATION.md`. Do not treat the intake's `documented` declaration as
verified negative history or tune against exposed locked results.

The subsequent control-assembly milestone supplies six procedural families,
48 generated known-chain inputs and 96 codec-specific cases across development,
validation and reserved locked splits. Commands, lossy intermediates, recipe
hashes, exact independent PCM and copied binaries are retained locally, with the
evaluation plan frozen before detector reports. See
`GROUPED_CONTROLS_VALIDATION.md` for actual coverage results and limitations.
These controls support scoped engineering sensitivity/false-hit observations;
they do not provide a representative independently reviewed recording corpus.

The user explicitly deferred both Android and MQA confirmation on 2026-10-04.
Version 0.21.0 adds the non-MQA evidence grouping layer through `assess_evidence`
and CLI `--summary`. It groups related spectral/noise observations, deduplicates
named method matches across channels/bases, retains every record's status and
scope via source indices, and keeps sample-rate/bit-depth/source-profile findings
separate from ancestry. Existing measurement JSON and detector arithmetic remain
unchanged. See `EVIDENCE_INTERPRETATION.md` for the finite implementation boundary
and exact outstanding inference/data requirements. This does not close calibrated
classification, known detector misses or source-medium/depth validation.

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
passed with exact independent PCM and 732 schema-valid reports. The final MSRV
regression passed all 161 tests; production-only validation also passed. A later
generated header-equivalence audit passed all ten FLAC integration tests,
including 48 new exact-PCM successful reports and 138 rejected header controls.
Read `FLAC_INTEGRITY_VALIDATION.md`.
Schema, dependencies and detector policy are unchanged; Android stays deferred.

### 5. Build the Android application

Version 0.20.0 implements a portable background-job lifecycle around the core:
`AnalysisJob` accepts sources or paths, retains one latest progress value and
one result, supports polling/cancellation, and requests cancellation on drop.
Process-wide admission rejects additional unfinished background jobs as Busy;
dropping a handle cannot admit new work while the old source remains blocked.
Worker panics remain host errors. The background example now uses this API.
This completes the reusable Rust worker wrapper, not Android bindings or
device lifecycle validation. See `JOB_VALIDATION.md` for actual checks.

The 0.19.0 continuation implements the planned core progress interface:
`analyze_source_with_progress` emits typed stages and bounded-frequency per-pass
frame updates, supports cancellation from callbacks, and reports terminal status
after releasing the source/worker. Existing entry points remain available and
report schema/policy are unchanged. Android bindings remain a separate step.

The first integration target is the 0.18.3 observations-only measurement core
defined in `CORE_INTEGRATION.md`, with a no-CLI background consumer example.
Its desktop closure passed 164 release tests, MSRV all-target Clippy and the
CLI-free consumer's build/report/PCM checks on 2026-10-03. This is the completed
initial measurement baseline, not a completed classifier or Android release.
Do not extend this milestone indefinitely with speculative audits or require
calibrated ancestry/source profiles before displaying supported measurements.
Those claims retain their separate corpus gates. Further hardening is follow-up
work unless a concrete defect blocks the documented measurement scope.

After P09, follow A01–A08 for Alfred shared workspace/input/jobs and separate
feature adapters. The Android app consumes the core; its home is selection and
applicable operations, with Forensics, Spectrogram and Compare independently
reachable. Keep Kotlin/native adapters and local app storage outside the core.
Then validate on actual Android hardware for memory, runtime, lifecycle and
thermal behavior. Desktop benchmarks cannot establish phone performance.

## Collection

See [TEST_CORPUS.md](docs/TEST_CORPUS.md) for the shopping list and collection notes.
Begin with a small development collection; do not wait for a large corpus to
start engineering. Reserve unseen source groups before detector tuning begins.

Validation references:

- [Grouped evaluation](https://scikit-learn.org/stable/modules/cross_validation.html#cross-validation-iterators-for-grouped-data)
- [Avoiding test-data leakage](https://scikit-learn.org/stable/common_pitfalls.html#data-leakage)

## P07 product workflows — 2026-10-06

Engine 0.32.0 exposes `audio-forensic-product-v1` and comparison v1, keeping
measurement schema/policy/ancestry unchanged. Combined collectors share one
source/worker/deadline and three verified PCM passes; native APIs stay two-pass.
Actual no-decode metadata info, deterministic mixed batch, prefix/fast, saved
rendering, all 155 field aliases and collision-safe P06 PNG workflow are
implemented. Comparison uses the pinned qualified tuple; unlike versions,
reference domains or duration/EOF coverage have no winner. Product/info limits
are 32 inputs/results and saved documents 64 MiB; exports retain 32 attempts.
See [workflows](docs/PRODUCT_REPORT.md), task-results/P07.md and
docs/validation/PRODUCT_WORKFLOW_VALIDATION.md for checks and limitations.
The former proposed `alfred-product-v1` is not emitted; this independent feature
envelope contains no Alfred shared workspace/job/storage state.
