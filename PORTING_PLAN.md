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

Validate runtime/memory limits, corrupt-input handling and cancellation. Stabilize
the public API/report schema and support matrix. A small ARM64 build/link check
during core work may catch portability issues; it is not Android app development.

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
