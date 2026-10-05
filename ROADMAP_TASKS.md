# Executable roadmap task cards

Updated **2026-10-05**. Read [ROADMAP.md](ROADMAP.md) for order/status and
[OWNER_CHECKLIST.md](OWNER_CHECKLIST.md) for the owner's current requirements.
Sol = GPT-6.1 Sol; Astra = GPT-6 Astra. Assignments are recommendations.

## Shared instructions for every task

1. Read AGENTS.md, HANDOFF.md, PORTING_PLAN.md, this card and the relevant
   validation documents. Inspect actual code and `git status`; preserve existing
   work, recordings and the pinned Python reference. Owner edits take precedence
   over obsolete planning assumptions.
2. Check dependencies in ROADMAP.md. Complete only the requested packet. On a
   bare “resume”, finish the active delivery packet or earliest eligible one.
   Do not substitute endgame experiments for delivery work.
3. Set the task IN_PROGRESS and create `task-results/<ID>.md`. Record the exact
   scope, relevant source symbols, intended checks and any missing input. New
   output files named below are deliverables, not files presumed to exist.
4. Implement and verify the stated behavior; reuse existing tests/oracles.
   Run focused checks, not repeated full matrices. Never regenerate a reference
   merely to hide disagreement. Do not change Rust during a running Rust build.
5. Finish with files changed, exact check outcomes, unresolved limitations,
   any running jobs and next action. Update the task table and HANDOFF.md.
   DONE requires the acceptance criteria; hardware/data blocks stay explicit.
   Large newly discovered scope becomes named child cards with dependencies,
   not an unbounded session. Do not silently waive required owner features.
6. Conserve usage: inspect targeted files, keep updates/results concise, do not
   re-audit completed milestones without a concrete reason. Escalate ambiguous
   DSP/policy decisions to an Astra packet with a minimal reproducer. No automatic
   subagents, network uploads, pushes, public releases or model switching.

Standard prompt: **“Complete <ID> from ROADMAP_TASKS.md, including acceptance
checks and roadmap/handoff updates.”** This contract applies to every card.

## P01 — fidelity and release contract

**Astra; DONE 2026-10-05.** See `task-results/P01.md`, `PYTHON_PARITY.md` and
`RELEASE_CONTRACT.md` v1. Inputs: pinned `reference/audio-forensic/audio_forensic.py`,
its tests/README, Rust `src/`, PORTING_PLAN.md and OWNER_CHECKLIST.md.
Produce `PYTHON_PARITY.md` and `RELEASE_CONTRACT.md`: map every user-facing
Python field/workflow and relevant method to Rust code, current tests, exact
parity, deliberate difference or missing implementation, and an owning task.
Include tags, SoX/DR mappings, ReplayGain, spectrogram, comparison, all scoring
and veto paths, bit-depth/source profiles, side-channel/banding/LPF/DSD inputs.
Freeze required outputs, sampling/channel semantics, numerical tolerances,
versioned reference-assessment shape and explicit unsupported states. FLAC, WAV,
ALAC/M4A and DSD are owner requirements; settle DSF/DFF/rate scope explicitly.
Also record a proposed commit/checkpoint policy for owner approval, respecting
the existing source-only publication history; do not commit/push by assumption.
**Done:** every required row has a finite task and acceptance oracle; conflicts
and unsafe claims have explicit disposition. This is the scope/spec packet,
not another broad detector campaign. Update downstream cards if inspection finds
missing work. Do not call this a complete port merely because measurements exist.

## P02 — tags, encoder traces and ReplayGain

**Sol; DONE 2026-10-05.** See `task-results/P02.md` and
`METADATA_VALIDATION.md` for implemented adapters, actual checks and unavailable
container fields. Read Python `extract_mediainfo`, `detect_encoder_trace`,
`audit_replaygain`; Rust container/decode/model and schema validation documents.
Implement bounded metadata exposure and unknown-tag retention for supported
containers, declared technical metadata, and reference ReplayGain comparison.
Separate editable tags from measured evidence. Use P01's result contract; avoid
embedding unbounded artwork or retaining unlimited tag values.
**Done:** generated missing/duplicate/malformed/large-tag controls, ReplayGain
unit/reference fixtures and serialization checks pass. Metadata never becomes
proof of ancestry. Document container-specific unavailable fields.

P01 implementation constraints:

Use PYTHON_PARITY G01/G02/G23 and RELEASE_CONTRACT limits/D06/D11/D12.
Preserve raw keys, duplicates, truncation markers, tag-only MQA claims and actual
selected-stream codec identity. Freeze container hooks for F01/F02; their parsers
may be added later without reopening metadata semantics. Use existing native
LUFS with its declared method in the legacy audit, not fabricated full-file LUFS.

## P03 — byproduct measurements and display mapping

**Sol; DONE 2026-10-05.** See `task-results/P03.md` and
`BYPRODUCT_VALIDATION.md`. Read Python `measure_phase_correlation`, `detect_clipping`,
`map_silence`, `_noise_floor_from_audio`, loudness/SoX extraction and DR mapping;
inspect current stereo/PCM/loudness/quiet measurements and their validation docs.
Implement only the P01 missing rows: preserve native existing measurements and
add explicit reference-compatible window/threshold calculations where needed.
Do not label whole-stream correlation as Python's mean 100 ms correlation or
full-scale sample count as a plateau detector without matching definitions.
**Done:** silence, DC, mono/antiphase, plateaus, fades, short/prefix inputs have
documented expected values; output units and unavailable conditions are exact.

P01 implementation constraints:

Own G03/G05/G07 and level-display deltas; P03a owns missing external-tool
statistics/DR. The inspected `detect_clipping` is a fixed 16-bit-ceiling sample
counter, not a plateau detector. Preserve that definition separately from native
precision full-scale counts. Follow D02/D04 and typed availability.

## P03a — bounded tool statistics and DR

**Status: DONE (2026-10-05).** See [P03a results](task-results/P03a.md) and
[tool-statistics validation](TOOL_STATISTICS_VALIDATION.md).

**Sol; after P03.** P01 discovered required outputs without numerical equivalents
in the core. Own PYTHON_PARITY G04/G06: FFmpeg astats RMS peak/trough, noise floor,
dynamic range, flat factor, peak count, entropy, zero crossings; the complete
canonical SoX `stat` key list; and a separate FFmpeg drmeter-compatible DR value.
Read `extract_loudness`, `extract_sox_stats`, the frozen tool-stat inventory and
native PCM/loudness validations. Record exact tool versions and fixtures before
implementation; inspect tool algorithms where needed. Define each aggregate's
channel/window/tail semantics, worst-case memory and cancellation. Correct the
linear astats crest value mislabeled `crest_factor_db` (D04), preserving raw
legacy audit output and explicit linear/dB units. No subprocess
at runtime; never substitute crest/LRA for DR or call drmeter EBU loudness range.
**Done:** finite generated constant/sine/impulse/plateau/noise/unequal-stereo and
short/tail/prefix controls meet RELEASE_CONTRACT bounds; null/nonfinite cases and
all output keys have explicit dispositions. Publish the algorithm/input schema
for P07. If a tool field is nonsensical for a domain, mark it inapplicable with
reason; missing implementation is not inapplicability. DSP ambiguity gets a
minimal reproducer and focused review, not an invented formula.

## P04c — reference basis, spectral and transform adapters

**Astra; after P01.** P01 identified incompatible native/reference inputs.
Own PYTHON_PARITY G08/G11/G13/G14/G15: shared pinned f32 mono/mid/side basis,
base features, adaptive segment inputs, auCDtect sampling/mode/phase,
AAC best-basis adapter and Vorbis reconstructed-L/R winner with support/tested
counts (mono M; pinned code widens f32 M/S before reconstruction).
Read the existing per-method validations and preserve native APIs/measurements.
Use bounded captures/accumulators or additional cancellable passes, strict
reference frame boundaries and explicit energy/coverage states. Do not weaken
current applicability gates or perform E01/E02 improvements. Use the frozen
deviations when reference defaults/gates conflict with meaningful availability.
**Done:** existing immutable oracles plus matched f32 stereo/antiphase/active-gap,
>2500-frame scatter, duration-adaptive segment/Random(42), winning transform
basis/tie and insufficient-input controls pass. Inputs carry versions, domains,
intervals and deviation IDs; P05 cannot consume a native lookalike by accident.

## P04a — missing spectral inputs

**Astra; after P04c.** Inspect Python `_banding_score`, `_side_channel_anomaly`,
`_lpf_scan`, `_dsd_scan`, `_score`, `_resample_check` and Rust DSP/detectors.
Implement the missing P01 reference-input rows with bounded storage, explicit
channel transforms, windows, energy gates and cancellation. A DSD-like spectrum
is distinct from native DSD container identity. Preserve known useful fixes.
**Done:** focused generated comparisons pass frozen tolerances; differences
have named adapters or exclusions, not invented zeros. P05 can consume the
defined inputs without pretending unequal calculations are identical.

P01 implementation constraints:

Own G09/G10/G12: include separate header mismatch observations, ordered
resampling-candidate adapter and fake-hires bandwidth inputs. Header issues do
not add ancestry points (D07); no extension proves a codec. Use P04c's basis.

## P04b — source-profile inputs

**Astra; after P03/P04a.** Read Python `_silence_and_vinyl`, `_cassette_source`,
`_psychoacoustic_artifacts`, `_noise_floor_profile`, sparsity/envelope methods;
compare corresponding Rust modules and validation records.
Implement missing bounded reference inputs/adapters from P01, including any
necessary distinctions between filtered time-domain and circular STFT statistics.
Keep musical attacks, noise profiles and preceding energy descriptive inputs.
**Done:** generated counterexamples and numerical controls cover required rows;
no alleged mirror detector is produced by negating absolute correlation, and
source-history certainty is not added. Save inputs needed by P05's candidate rules.

P01 implementation constraints:

Own G16–G21 and D02/D03/D05: corrected first-30s bit/noise-floor inputs,
60/180s filtered profiles, source rolloff, sparse/envelope and comb inputs;
specify a bounded FFT/filter/storage plan and worst-case bytes at 384 kHz.
Retain source-profile candidate rules for P05; unavailable numerical inputs must
not silently become clear zeros or “no artifact” evidence.

## F01 — ALAC in M4A

**Sol; after P01.** Add offline ALAC/M4A decoding, bounded container metadata,
source seeking and format applicability through the existing analysis API.
Read current decode/container/source contracts and inspect decoder support before
adding dependencies. Detect the actual codec: M4A containing AAC is not ALAC.
Integrate P02 metadata when available; no phone-only hidden decoder dependency.
**Done:** generated 16/24-bit mono/stereo and supported rates match independent
integer PCM exactly; malformed/truncated, prefix, seek and cancellation cases
remain structured. Update format/schema support and no-CLI/Android build checks.

P01 implementation constraints:

Required scope is RELEASE_CONTRACT v1's nonfragmented unencrypted 16/24-bit
mono/stereo matrix, including moov before/after mdat. Preserve old measurement
schema; use versioned dispatch if new format fields require an extension.

## F02 — DSD input and declared conversion

**Astra; after P01.** Implement the frozen DSF/DFF/rate scope with bounded parsing
and explicit DSD-to-PCM analysis semantics. Record original DSD format/rate,
conversion filter/output rate/delay and the domain of every applied detector.
Never label converted PCM precision as original integer recording depth or apply
integer/MQA tests to converted samples as though they were native PCM.
**Done:** generated native bitstream/packing checks, independent conversion
comparisons with declared tolerances, anti-alias/level/delay controls and resource/
cancellation checks pass. Unsupported DSD encodings remain explicit. If the
frozen scope cannot be delivered, record a blocker; do not silently drop DSD.

P01 implementation constraints:

RELEASE_CONTRACT v1 requires mono/stereo DSF and uncompressed DFF at
2.8224/5.6448/11.2896 MHz to explicitly converted 88.2 kHz float PCM. Freeze the
versioned filter, coefficients/gain, delay/tail handling and independent oracle
before acceptance; use a new measurement schema when required. Integrate P02's
metadata shape and P05's native-DSD inapplicable ancestry state. All six
container/rate combinations are required; DST and other excluded encodings get
specific unsupported results, not PCM fallback.

## P05 — reference interpretation, verdict and rule trace

**Astra; after P02/P03/P04a/P04b/P04c.** Read Python `_score`, `_verdict`, `analyse`,
`_bit_depth_verdict`, source-profile decisions and comparison inputs.
Implement the separate versioned reference assessment specified in P01: scores,
rule contributions, vetoes, candidate interpretations, missing-input abstentions
and exact source intervals. Keep measurement ancestry INCONCLUSIVE/index null.
Reference scores/labels must be identified as uncalibrated method outputs;
do not invent replacement weights or import the MQA certainty override.
**Done:** frozen rule traces match Python on equivalent feature vectors and
generated audio; every intentional deviation is explained. Users receive a
useful final reference interpretation and detailed evidence, not just raw numbers.

P01 implementation constraints:

Implement all 34 PYTHON_PARITY rule rows and RELEASE_CONTRACT D01–D12, including
excluded header/alias/MQA effects, cassette hiss gate, Python rounding, source
veto interactions and missing-input propagation. Exact legacy labels/text are
audit fields; qualified candidate wording is the product default. Do not remove
source/depth interpretations merely because research validation is unfinished.

## P06 — spectrogram

**Sol; after P01.** Read Python `generate_spectrogram` and Rust STFT geometry.
Implement bounded spectrogram data/artifacts with time/frequency axes, units,
channel basis, prefix scope and defined reduction for long/high-rate files.
Rendering must work offline without shelling out to FFmpeg/SoX in the app.
**Done:** tones, silence and time-limited events appear at expected positions;
dimensions/memory stay bounded, cancellation works, and the artifact contract is
documented for A06. No full-file spectrogram allocation based on duration headers.

P01 implementation constraints:

Follow RELEASE_CONTRACT artifact dimensions/basis/reduction and collision-safe
output rules. SoX pixel/color parity is explicitly not required. Keep artifact
status separate from report success and preserve the shared prefix.

## P07 — report, batch and comparison workflows

**Sol; after P05/P06/P03a.** Read Python `print_report`, `_report_to_dict`, batch and
comparison functions; current CLI/evidence layer and P01 result contract.
Expose reference verdict, full metadata, measurements, candidate source/depth
interpretations, spectrogram links and diagnostics. Add the required comparison
workflow with deterministic ordering and stated reference criteria; ranking is
not proof of perceptual quality. Preserve existing machine JSON consumers.
**Done:** saved-report, mixed-success batch, absent-field and comparison fixtures
pass; native channels, units, unknowns and scopes remain visible. Documentation
shows ordinary user workflows and distinguishes reference from validated claims.

P01 implementation constraints:

Complete PYTHON_PARITY W01–W08/W10, including actual no-decode info, directory
batch, fast alias, saved product rendering and separately versioned product JSON.
Preserve all mapped fields/aliases with status/domain/unit/scope and raw tag
access; no legacy certainty text in an unqualified headline.

## P08 — differential acceptance and regression

**Sol; after P02–P07/F01/F02 and any required P01 child tasks.** Use PYTHON_PARITY.md
to assemble a finite generated differential suite; compare exact decoding and
documented DSP/rule tolerances. Include format applicability and failed inputs.
Freeze code/fixtures before the final appropriate Rust regression, Clippy,
format/schema/no-CLI checks; record toolchain and actual totals once.
**Done:** required parity rows have evidence, no unexplained failure is waived,
and `CORE_RELEASE_VALIDATION.md` records what passed/failed/not run. A failure
gets a targeted fix/recheck, not automatic oracle regeneration or a new audit.

P01 implementation constraints:

P03a and P04c are required P01 child packets. Run the P01 inventory checker
before assembling the differential suite; inventories do not replace numerical
checks. Cover every required group/rule/workflow and schema/version transition.
Do not use private recordings as public fixtures or re-open locked controls.

## P09 — core release gate

**Astra; after P08.** Review required parity rows, interpretation wording,
format boundaries and final evidence. Check new code risks, not every already
closed parser experiment. Fix concrete release blockers and rerun affected checks.
**Done:** written accept/reject decision in CORE_RELEASE_VALIDATION.md with
version, supported scope and explicit limitations. New research questions become
E tasks and cannot silently block a faithful, correctly described release.

## A01 — Android integration contract

**Astra; after P01.** Read CORE_INTEGRATION.md, JOB_VALIDATION.md, source/progress
API and result contract. Produce `ANDROID_CONTRACT.md`: choose bridge mechanism,
opaque handle ownership, JSON/data representation, thread/cancel/result lifecycle,
URI staging limits, history limits, ABI/build strategy and resource acceptance.
Plan Android 14–16 compatibility and Redmi 13 4G/Android 16 physical testing;
verify SDK/NDK/API choices from current official Android documentation at execution.
**Done:** implementation-ready interfaces and tests for A02–A07, including safe
close/poll/cancel races and process death. Keep unsafe FFI, if necessary, confined
to a separately reviewed boundary rather than weakening the safe analysis core.

P01 implementation constraints:

Design against RELEASE_CONTRACT v1 result layers and required formats, including
700 MiB import and DSD converted-domain descriptors. A01 owns concrete ABI,
staging/history/queue limits and resource budgets; this does not authorize
changing the frozen fidelity scope.

## A02 — Alfred scaffold and builds

**Sol; after A01.** Create the Android project, offline screens/navigation,
native build integration and pinned reproducible tool versions. Use **Alfred**;
development application ID until owner finalizes it. No accounts/network service.
GitHub Actions may be prepared, preserving the source-only publishing rules.
**Done:** local debug APK builds, native library is packaged for the declared
ABI, build instructions are repeatable, and missing SDK/NDK prerequisites are
explicit. Tool installation requiring approval is not silently bypassed.

## A03 — native bridge

**Astra; after A02.** Implement A01's ownership contract around AnalysisJob:
start, latest progress, cancel, consume result, close and structured bridge errors.
Preserve exact report integers and version dispatch. Keep Android UI threads free
of decoding and blocking waits; never allow panics across the native boundary.
**Done:** actual NDK link plus packaged native-load/generated-input smoke;
stale handles, repeated close/cancel/poll and resource release tests pass. State
whether evidence is emulator or physical device; compilation alone is insufficient.

## A04 — file import and staging

**Sol; after A03.** Implement Android document-picker/URI access, source ownership,
permission expiry handling and bounded staging for nonseekable providers using
A01 limits. Display actual codec support, including ALAC versus AAC in M4A.
**Done:** seekable/nonseekable, denied/revoked/missing input, insufficient disk,
cancel-during-copy and large-file paths behave explicitly; temporary files and
handles are released. No whole-audio memory buffering or unrestricted-storage
permission merely to read a user-selected document.

## A05 — analysis lifecycle

**Sol; after A04.** Implement the agreed background execution/notification model,
single active job, bounded queue if specified, progress and cancellation UI.
Handle rotation, backgrounding, screen-off and process recreation; never show
completion merely because a decode pass ended. Use fresh jobs/tokens on retry.
**Done:** automated lifecycle/state tests and emulator smoke pass; stale callbacks
cannot update a new job, teardown does not block UI, and interrupted work is
reported honestly. Physical behavior is checked again in A07.

## A06 — detailed results and local workflows

**Sol; after A05/P07.** Implement Alfred's verdict/reference assessment screen
plus all metadata, expandable measurements, detector scopes/caveats, spectrogram,
history, comparison and explicit share/export. Reuse Rust policy, never recode
scoring in Kotlin. Bound stored data and provide deletion; no implicit uploads.
**Done:** UI fixtures cover success and every failure/abstention/version state;
nulls are not zero, large integers survive storage/export, and the owner can
inspect essentially all supported information rather than a verdict-only screen.

## A07 — physical-device gate

**Astra + owner; after A06/P09/U01.** Test the APK on the Redmi 13 4G/Android 16
and cover Android 14/15 with available devices/emulators, labeling evidence.
Exercise import/all required formats, approximately 600–700 MB input handling,
cancel/retry, rotation, background/screen-off, process death, storage pressure,
thermal/runtime/memory behavior and result parity on generated controls.
**Done:** `ANDROID_VALIDATION.md` records actual device/build/results against
A01 budgets; concrete blockers fixed/retested. If no phone is available, save
the ready APK/checklist and mark the device portion BLOCKED_INPUT, not passed.

## A08 — GitHub beta handoff

**Sol + owner; after A07/U03.** Prepare versioned release build, hashes, notices,
supported formats/OS versions, known limitations, install/update instructions and
GitHub Release draft text. Finalize package identity and owner-held signing plan
before public distribution. Audit artifacts for private evidence/recordings.
**Done:** reproducible installable release candidate and owner acceptance/signing
instructions are ready. Actual public publication requires explicit authorization;
record prepared versus published separately. No store project is required.

## E01 — AAC phase robustness

**Astra; ready when explicitly requested.** Read AAC_MUSIC_VALIDATION.md,
GROUPED_CONTROLS_VALIDATION.md and AAC source/tests. Use exposed development
controls to implement/evaluate an exhaustive integer-phase candidate against
the coarse baseline, preserving the threshold. Test trims, false hits,
runtime/memory and cancellation. Do not consume unseen groups while tuning.
**Done:** reproducible comparison and justified accept/reject decision; document
whether trim failures recover and content misses persist. A broader phase search
does not automatically preserve a prior false-hit interpretation.

## E02 — Vorbis precision sensitivity

**Astra; ready when requested.** Inspect existing paired 16/24-bit controls,
TRANSFORM_VALIDATION.md and Vorbis arithmetic. Isolate quantization/dither effects
with a frozen small matrix; propose/test one mechanism-based candidate if supported.
**Done:** causal evidence or clearly bounded unresolved finding, PCM-exact receipts,
cross-codec/negative controls and resource checks. Do not simply lower thresholds
until known files pass. Retain the baseline alongside any candidate.

## E03 — broader codec experiment execution

**Sol; after E04 protocol.** Implement/run the specified finite matrix of
encoders, bitrates, channel bases, window/tool settings and edits. Record actual
tool versions/options, lineage and output geometry; retain originals/intermediates.
**Done:** independent PCM, grouping and schema checks pass or failures are
preserved; output stratified observations without inventing label truth. No
encoder installation/purchase or unlimited matrix expansion is assumed.

## E04 — real-corpus and evaluation protocol

**Astra + owner.** Read evaluation/intake code and owner's U02/U06 policy.
Specify scoped labels, review criteria, grouping, development design, acceptance
targets and uncertainty treatment. Prepare intake for available documented
sources; unknown upstream history stays unknown. Freeze each candidate before
selecting fresh final-evaluation groups on demand; no permanent reserved corpus
is required. Existing generated locked groups retain their recorded status
until explicitly assigned a use; opening them changes exposure, not history.
**Done:** `ENDGAME_EVALUATION_PROTOCOL.md`, ready manifests/intake receipts for
available material and a precise list of missing evidence. Protocol completion
does not mean sufficient ground truth exists for E05–E07.

## E05 — calibrated aggregation candidate

**Astra; after E03/E04 and adequate reviewed controls.** Develop on exposed
groups; model correlated evidence, abstention and coverage. Freeze candidate,
thresholds, labels and evaluation procedure before fresh groups are selected.
Run the defined evaluation once; report group counts, errors and uncertainty.
**Done:** justified acceptance/rejection with retained receipts, or explicit data
block. Do not manufacture calibration from synthetic variants or uncertain
commercial histories. Keep reference assessment and validated policy distinct.

## E06 — original-depth inference

**Astra + owner; after E04 and known chains.** Separate exercised bits from
recording-depth hypotheses. Freeze controls for gain, dither, truncation,
floating conversion, noise shaping and quiet-content masking.
**Done:** tested scoped candidate or negative research result with counterexamples,
limits and required future controls. Fully exercised bits or a low noise floor
alone must not become verified original depth.

## E07 — source-medium inference

**Astra + owner; after E04 and documented capture chains.** Study vinyl/cassette/
digital candidates with matched musical/mastering confounds, capture-chain groups
and known processing. Keep the shipped reference profiler available separately.
**Done:** frozen experiment and candidate evaluation or precise data block;
musical attacks, EQ, hiss or cutoff variation alone are not verified medium.

## E08 — frequency mirroring

**Astra; explicit research task.** Specify an actual frequency-order relationship,
then implement one bounded candidate with generated mirror/nonmirror, filtering,
tonal/noise and resampling counterexamples. Compare an independent numerical oracle.
**Done:** measured applicability, errors, resource/cancellation behavior and
accept/reject decision. Negated absolute time-domain correlation is not the method.

## E09 — MQA confirmation

**Astra + owner; READY for a named review, not a beta dependency.** The owner
supplied existing independent work during P01 at
`C:\Users\Bishal\Documents\antigravity-dev\mqa`. Read `task-results/P01-MQA.md`
and inspect the fingerprinted prototype, tests and saved receipts before doing
new work. Do not recreate the scanner or assume recordings are unavailable.
Freeze the candidate and review the five finite obligations in that intake:
confirmation-run/payload binding, exact packet representation, conditional
probability claims, bounded host/format integration and actual corpus evidence.
Reproduce concrete source findings with generated tests; distinguish skipped
corpus tests, saved Python observations, new Rust runs and independently checked
PCM. Record known/unknown provenance; private recordings remain local.
**Done:** accept a scoped signalling policy/candidate with precise caveats and
validation receipts, or reject it with targeted remaining fixes/evidence needs.
Do not treat IID random-bit bounds as general calibrated confidence, editable
tags as confirmation or packet fields as independently verified source history.
Preserve the excluded unconditional score 100. E10 owns accepted integration,
versioned schema/consumer compatibility and affected host tests. No blind copy
over `src/detectors/mqa.rs` based on the external integration instructions.

## E10 — ship an accepted improvement

**Sol; after an accepted E candidate and frozen interface.** Integrate the new
versioned method/policy, migrate stored-result presentation where necessary and
update release notes without silently reinterpreting old reports.
**Done:** relevant Rust/app regression and device smoke pass; old reference and
new validated/experimental results remain distinguishable. Prepare release
artifacts; publication still needs authorization.
