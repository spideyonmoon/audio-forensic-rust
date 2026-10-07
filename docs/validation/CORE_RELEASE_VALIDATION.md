# Standalone core differential acceptance and release decision — P08/P09

Date: **2026-10-06**. Engine **0.32.0**, Rust minimum **1.85**.
**P08 DONE.** No unresolved acceptance failure or running job remains.
**P09 DONE — ACCEPT 0.32.0 for the scoped standalone library/CLI release**
(2026-10-06). See the decision below. This does not authorize publication.

## Frozen scope and differential suite

The P01 inventory passed before assembly: **155 fields, 139 methods, 30 groups,
34 rules**, clean Python reference at `c6ecce2296256b516709d87088896d1be913908c`.
Initial formats are WAV, FLAC and ALAC/M4A. Native DSF/DFF conversion and DSD
schema/oracles are **deferred F02 beyond initial release and Alfred launch**.
PCM spectral DSD-like observations remain separate from native DSD support.

`scripts/check_core_release.py` binds the acceptance to SHA-256 hashes of all
production source, manifests, generated fixtures, schemas, rendering assets and
validation helpers. `target/p08/frozen-tree.json` is the final freeze receipt.
Documentation/status updates are outside that computational freeze. The initial
receipts are retained separately: only the acceptance helper's group-to-test map
and Windows log-path normalization were refined. Hash comparison verifies that
Rust source/fixtures/schemas never changed across compilation and execution.

The finite suite consumes existing immutable generated expectations. P08 does
not reopen or regenerate locked experiments. Exact integer PCM hashes, discrete
counts/intervals/gates, and deterministic policy outputs remain exact. Numerical
bounds are those in [RELEASE_CONTRACT.md](../../RELEASE_CONTRACT.md) and each
original validation record. The full regression executes both native controls
and the separate reference-basis adapters; passing native DSP alone would not
establish reference-mid parity. Group coverage is explicitly mapped below.

| Group | Acceptance executable / immutable oracle or contract |
| --- | --- |
| G01 | metadata/alac; bounded tags, claims, duplicates, Unicode, container/PCM controls |
| G02 | metadata and library; frozen ReplayGain arithmetic and strict boundary vectors |
| G03 | core/byproducts/tool_statistics; exact native PCM and qualified display units |
| G04 | tool_statistics and library; frozen FFmpeg/SoX statistics recipes |
| G05 | loudness/listening_levels/byproducts; native meter gates and fixed deltas, D04 |
| G06 | tool_statistics and library; frozen drmeter values, nulls/tails and legacy labels |
| G07 | byproducts and library; frozen reference f32 phase/count/quiet/RMS-p5 vectors |
| G08 | parity/reference_inputs; reference.json and reference_inputs.json, exact PCM/STFT |
| G09 | reference_inputs/library; reference_profiles.json spectral/side/LPF/PCM DSD-like inputs |
| G10 | flac_integrity/wav_formats/library; actual CRC/MD5/header applicability, D07 |
| G11 | detectors/reference_inputs/library; detector_reference.json and seeded segment adapters |
| G12 | transforms/reference_inputs/library; transform_reference.json and reference resampling/fake-hires inputs |
| G13 | structure/reference_inputs; structure_reference.json and matched f32 scatter/phase |
| G14 | aac/reference_inputs/library; aac_reference.json, MDCT/KBD and f32 basis selection |
| G15 | transforms/reference_inputs/library; frozen Vorbis scores/support/reconstructed L/R selection |
| G16 | noise/reference_inputs/library; reference_profiles.json capped FFT/Pearson/temporal profiles |
| G17 | transients/preceding_energy/reference_inputs/library; frozen filters/peaks/clicks/qualified denominator |
| G18 | rolloff/reference_inputs/library; frozen rolloff and cassette source profiles |
| G19 | sparsity/envelope/reference_inputs/library; sparse counts and signed correlation profiles |
| G20 | spectral_lags/reference_inputs/library; frozen MP3 comb geometry and lag predicates |
| G21 | core/noise_floor/reference_inputs/library; integer precision and corrected zero-index profile, float nulls |
| G22 | reference_assessment/library; accepted v2 policy/trace, depth and verdict boundary fixtures |
| G23 | detectors/metadata/reference_assessment/library; provisional MQA candidates/tags, excluded R32 override, D06 |
| G24 | reference_assessment/library; R13 explicitly excluded, raw alias audit without ancestry effect, D05 |
| G25 | spectrogram/spectrogram_png/library; frozen data, tone/bucket bounds, PNG geometry and collision controls |
| G26 | product/product_cli/report_contract; 155 aliases, saved/absent/exact-u64/policy/version controls |
| G27 | product/product_cli/library; 14 original-Python comparison tuples, ties/unavailable/unlike-domain controls |
| G28 | product_cli/cli_contract/jobs/progress/worker_budget; info/no decode, batch/full/fast/cancel/deadline/export |
| G29 | product_cli/source_contract/progress; offline source/lifecycle and JSON/stderr separation, D12 |
| G30 | alac/wav_formats/flac_integrity; exact PCM/format/failure controls; native DSD separately deferred |

R01–R34 execute through the accepted P05 v2 fixtures: **166 policy/trace vectors,
224 depth vectors, 30 verdict-precedence cases and 15 rounding values**. These
are vectors inside Rust tests, not additional test-count totals. Rejected v1
P05 fixtures stay preserved and are not used as acceptance expectations. Read-only
Python reproduction checks policy, traces and verdicts; labels and comparison
tuple checks use the pinned source. D01–D12 remain binding, including excluded
R11/R13/R32, reference candidates and missing-applicable-input null propagation.

## Schema, workflows and format applicability

Native measurement schema stays **0.18.0**, `observations-only-v18`, ancestry
`INCONCLUSIVE`, index null. Metadata/byproducts/tool statistics/spectrogram stay
v1; reference inputs v2 and assessment v1 stay explicitly bound. Product dispatch
is `audio-forensic-product-v1`, comparison v1. Old native JSON equality, old
optional artifact presentation absence, saved result rendering and unknown/stale
method/version/domain/coverage rejection are covered by the product/native/API
tests and independent offline schema checks. Historical reference-input v1 is
preserved; v2 does not silently accept v1 as a current adapter.

`check_core_release.py` additionally generates DSF and DFF signature controls.
Each is tested with native and misleading `.wav` extensions, in native JSON,
product JSON and fast mode: **12 structured unsupported cases**. Required exit
is 1; native coverage is null; product inputs/scores/reference label are null;
measurement ancestry/index stay inconclusive/null. The signature diagnostic
identifies the supported FLAC/RIFF/M4A boundary. These small controls verify
honest abstention, not full native DSD parser/packing/conversion correctness.

## Actual execution

Final frozen MSRV optimized regression: **271 passed, zero failed/ignored**,
40 Rust unit/integration executables plus an empty doctest block (41 result
blocks). Rust/Cargo **1.85.0**, host `x86_64-pc-windows-gnu`, LLVM 19.1.7;
Python 3.13.11. Release validation overrides only LTO=false/codegen-units=16;
the production manifest/release profile stays unchanged. This is an optimized
validation run, not a fresh default thin-LTO release build or benchmark.

- All-target MSRV Clippy with warnings denied, no-CLI library/examples check,
  explicit CLI/all-example build and stable formatting passed.
- **16** read-only inventory/schema/field-map/comparison/policy/trace/verdict/
  label/profile/helper checks passed; exact commands/exits in `contracts.json`.
- Fresh generated product checks passed: 9 schema/runtime
  rejections, exact native PCM/JSON equality, saved rendering after audio deletion,
  exact u64 max, comparison, real 61s→60s/480000-frame fast scope,
  FLAC/ALAC product serialization.
- Fresh ALAC checks passed: 72 full + 18 prefix native-schema reports,
  six bounded metadata controls, movie layout/tag variants, actual AAC rejection,
  mixed directory continuation and three-pass reference binding. The prior
  700 MiB control was not rerun.
- Fresh spectrogram checks passed: 10 generated reports, exact PCM/prefix,
  tone/bucket/power/schema/raster/collision controls and eight schema rejections.
- Fresh PNG checks passed: 6 canvases across all three resolutions,
  independent RGB8/CRC/filter/DPI/context checks, unchanged native/artifact v1,
  bitrate/collision/unavailable-path controls. The helper explicitly accounts
  for the already documented legacy caption encoding repair; no oracle changed.
- Two fresh generated product inputs passed independent equivalent-input pinned
  Python policy, exact scores/labels, all 34 rule traces, binding/interval and
  unchanged-receipt checks. Scores remain uncalibrated outputs.
- All **12** native/product/fast DSF/DFF signature controls passed structured
  unsupported/exit/schema/null-policy checks. Frozen 287-file tree remained
  unchanged at final acceptance; binary SHA-256 `9ab5d1aae4ca66a59b9743f350cb5f2af85393e71995c22b4b051f41705bad8b`.
- Final parity inventory, **262** local Markdown links and Git whitespace passed;
  `target/p08/final-doc-checks.json` also verifies the final binary hash and that
  earlier freeze receipts differ only in the acceptance helper.

Commands are recorded in [P08 results](../../task-results/P08.md). Logs and
outputs are local `target/p08/`: `rust-tests-final.log`, `clippy.log`,
`no-cli.log`, `build.log`, `fmt.log`, `contracts.json`, `independent.json`,
`summary.json` and product/alac/spectrogram/png/policy subdirectories. An initial
documentation edit used Windows' default encoding and stopped on ROADMAP;
HANDOFF's affected text was repaired, and subsequent edits use explicit UTF-8.
No production change was needed. Receipts are ignored local files under `target/p08/`,
not Git history or an external backup. The existing process-local
`.tools/msrv-gcc-lld.cmd` linker is used without global configuration changes.
First sandboxed compilation failed because rust-lld execution was denied,
before tests; retained as `rust-tests.log`. Authorized offline retry uses
`rust-tests-final.log`; no assertion/oracle/source correction was made.

## P08 not-run scope at handoff

No private recordings, corpus accuracy/calibration, MQA confirmation, native DSD
conversion or new DSD schemas, fresh worst-case RSS/700 MiB benchmark, Android
target/link/APK/device run, remote CI, commit/push/publication or external backup.
Accepted F01 700 MiB seek/padding and P04 buffer-bound evidence remain separately
documented; P08 does not relabel them as fresh runs. Source blocking-I/O and
cooperative cancellation limits remain; P04 maximum-rate buffers require app
admission budgeting. Android compile evidence from older packets establishes
neither linking nor phone behavior. P08 handed off to the P09 review below;
endgame research stays separate.

## P09 decision — ACCEPT, 2026-10-06

Accept **audio-forensic 0.32.0**, Rust minimum **1.85**, for independent offline
library/desktop CLI use under release-contract v1, delivery scope revision 2.
No unresolved release blocker was found in this finite review. The accepted
computational tree is the same **287 files** hashed by P08; the existing Windows
GNU optimized binary still has SHA-256
`9ab5d1aae4ca66a59b9743f350cb5f2af85393e71995c22b4b051f41705bad8b`.
This is acceptance of the local source and evidenced behavior, not a published
package, a default-profile distribution binary, or acceptance of other hosts.
No production source, dependency, schema, fixture or numerical tolerance changed.

### Supported scope

- Native WAV/FLAC mono/stereo at 8–384 kHz within their existing precision,
  layout and integrity conditions. WAV supports RIFF integer PCM 8/16/24/32,
  float32/64 and validated extensible layouts. FLAC requires native framing;
  16/24-bit support and additional accepted precisions retain decoder limits.
- Actual ALAC in unencrypted, nonfragmented M4A: mono/stereo 16/24-bit,
  8–384 kHz, seekable source, both moov layouts, bounded container tables/tags.
  AAC/Opus/DRM, video/multiple-track ambiguity and other excluded containers
  remain unsupported. A filename extension does not establish codec support.
- Full/shared-prefix measurements, read-only bounded metadata and audit,
  complete required reference fields and qualified source/depth candidates,
  saved product rendering, mixed batches, same-track reference comparison and
  optional bounded spectrogram/PNG exports. Runtime needs no Python, external
  analysis tools, network service or Alfred state. Native report/API remains
  independently usable with the CLI feature disabled.
- Native report **0.18.0** / `observations-only-v18`, ancestry `INCONCLUSIVE`,
  evidence index null; separate reference inputs v2 / assessment v1,
  `audio-forensic-product-v1` and `audio-forensic-comparison-v1`. Scores are
  uncalibrated reference-method outputs. Unknowns, failures and incompatible
  comparison scopes do not become successful-clear verdicts or winners.
- **Native DSD is unavailable.** Owner-approved F02 deferral beyond initial
  standalone release and Alfred launch is accepted as a named scope exclusion.
  The 12 retained DSF/DFF unsupported controls have null coverage/scores and
  exit 1. No conversion, native DSD parity or DSD schema acceptance is claimed.

### Finite risk review and disposition

| Area inspected | Evidence and disposition |
| --- | --- |
| Required P01 rows and policy deviations | All G01–G30 and R01–R34 map to the P08 ledger above; inventory still finds 155 fields/139 methods. Native/reference basis separation, exact PCM binding, D01–D12 and unchanged accepted fixtures remain intact. Deferred native DSD is explicit within G30. No missing required implementation identified. |
| Interpretation and saved results | Inspected `assess_reference`, candidate/summary construction, `ProductReport::validate`, `read_product_json` and `render_product`. Unsuccessful analysis emits null scores; missing applicable inputs suppress composite scores; saved scores/traces are recomputed from stored bound inputs. Headlines/candidates remain qualified; D05/D06/D07 exclusions remain visible. P08 boundary/stale/version/domain controls and fresh saved rendering support acceptance. |
| Comparison and export | Inspected `compare_products`, compatibility and tuple ordering, product export and existing P06 create-new/cancellation paths. Same-track identity is caller asserted, missing values stay null, incompatible inputs have no winner, ties retain input order. Artifact failures stay separate from measurements. P08 tests/independent PNG controls support acceptance; fresh saved comparison passed. |
| Format and source boundaries | Inspected selected-stream checks, ALAC preflight limits/codec checks, decoder `catch_unwind` and guarded source controls. Exact PCM and cross-pass evidence are retained; unsupported codecs/layouts remain structured outcomes. ALAC requires the supported unwind build; no panic=abort compatibility or exhaustive hostile-input audit is claimed. Closed parser experiments were not reopened. |
| Resource and host integration | Bounded DSP/profile captures, a single core analysis worker and documented cooperative cancellation remain. Desktop limits are accepted with the qualifications below. A01/A04/A07 own Android admission, staging, enumeration and actual device evidence; those tasks are not prerequisites for this standalone gate. |
| Evidence identity/builds | Recomputed P08 tree and binary hashes; parsed the actual 41 result blocks (271 passed, zero failed/ignored), checked required executables, 16 contract exits, six independent-command exits, successful build/Clippy/no-CLI logs and all 12 saved DSD outcomes. No computation changed, so a repeated full regression was unnecessary. |

### Explicit limitations and follow-through

The P04 maximum-rate reference profile has a conservative combined
capture/silence/FFT/result payload bound of **2,346,057,728 bytes at 384 kHz**;
FFT plans/twiddles, native/P04c state and allocator overhead are additional.
This is not measured peak RSS, a low-memory guarantee or evidence of phone
fitness. Native measurement-only APIs remain separate. A01 must budget the
full product path explicitly before A07 tests it on the Redmi device.

The desktop product/info CLI accepts at most 32 inputs/results and 64 MiB per
saved file, with 32 recorded export attempts. Inspection clarified that directory
candidates are collected/sorted **before** enforcing the 32-input limit; the
directory walk itself is not bounded by 32 and is not covered by the analysis
deadline. Oversized batches are rejected before analysis. This is an accepted
desktop limitation, not a bounded Android folder implementation; A01/A04 already
require bounded folder enumeration. The product guide now states this precisely.
Library callers own input/result batch and serialized-size admission; the CLI
limits are not universal limits on `read_product_json` or `compare_products`.

Saved runtime validation checks supported versions, identity and derived
assessment/artifact bindings; it is not an authenticity signature or the entire
development JSON Schema validator. No claim is made that edited saved measurements
prove audio provenance. Blocking caller I/O and individual compute operations
limit cancellation responsiveness. Keep the ALAC unwind boundary before any
Android panic strategy change. F01's prior 700 MiB seek/padding control remains
historical evidence; no fresh worst-case memory/large-file benchmark ran here.

No calibration, original-depth/source proof, perceptual ranking, MQA confirmation,
or stronger-than-reference detector accuracy is accepted by this decision.
Those remain E04–E09 (and E01–E03 detector work) under their existing gates.
No new research question was made a release blocker. Android target/link/APK/
device, default thin-LTO distribution build, remote CI, publication and external
backup remain unperformed in P09; prior target compilation is not phone evidence.

### P09 checks and handoff

Fresh `python target/p09/review_checks.py` passed tree/binary and receipt checks,
read-only parity inventory, CLI version and saved render/comparison against P08's
generated products. The exact four subprocess commands/exits and reused receipt
counts are in ignored `target/p09/review-checks.json`; stdout/stderr are retained
alongside it. These are receipt verification and smoke checks, **not 271 newly
executed Rust tests**. Final documentation links and whitespace are recorded in
[P09 results](../../task-results/P09.md). One exploratory read of a nonexistent
`target/p08/independent.log` failed; actual evidence is `independent.json` and
per-helper outputs. The first documentation check stopped on historical Windows-1252 text; a
read-only encoding fallback passed all 271 local Markdown file targets on
2026-10-07. No runtime acceptance check failed or evidence was replaced.

**Next eligible task: A01**, the Alfred architecture/integration contract.
It has not started. No job remains running. Existing dirty/untracked user work,
private recordings and the pinned Python checkout remain preserved. P08/P09
receipts are ignored local files, not Git history or an external backup; no
commit, push, upload or publication was performed or authorized by this gate.
