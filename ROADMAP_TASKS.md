# Executable roadmap task cards

Updated **2026-10-06**. Read [ROADMAP.md](ROADMAP.md) for order/status and
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
[tool-statistics validation](docs/validation/TOOL_STATISTICS_VALIDATION.md).

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

**DONE 2026-10-05, engine 0.27.0.** See `task-results/P04a.md` and `REFERENCE_PROFILES_VALIDATION.md`.

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

**DONE 2026-10-05, engine 0.27.0.** See `task-results/P04b.md` and `REFERENCE_PROFILES_VALIDATION.md`.

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

**Sol; DONE 2026-10-06, engine 0.28.0.** See [F01 results](task-results/F01.md)
and [ALAC validation](docs/validation/ALAC_VALIDATION.md). After P01. Add offline ALAC/M4A decoding, bounded container metadata,
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

**DEFERRED 2026-10-06 by owner beyond initial release and Alfred launch.**
Resume only on a named later request. The scope below remains the future F02
acceptance contract; inability to deliver it blocks that later DSD release, not
P08/P09 or initial Alfred gates.

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

**DONE 2026-10-06, engine 0.31.0.** See [P05 results](task-results/P05.md) and
[assessment validation](docs/validation/REFERENCE_ASSESSMENT_VALIDATION.md).

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

**Sol; DONE 2026-10-06**, engine 0.30.0 including the owner-requested calibrated
PNG follow-up. See `task-results/P06.md`, `task-results/P06-PNG.md` and
`SPECTROGRAM_VALIDATION.md` for implementation, actual checks and the A06
rendering contract. Original task: after P01, read Python `generate_spectrogram` and Rust STFT geometry.
Implement bounded spectrogram data/artifacts with time/frequency axes, units,
channel basis, prefix scope and defined reduction for long/high-rate files.
Rendering must work offline without shelling out to FFmpeg/SoX in the app.
**Done:** tones, silence and time-limited events appear at expected positions;
dimensions/memory stay bounded, cancellation works, and the artifact contract is
documented for A06. No full-file spectrogram allocation based on duration headers.

The follow-up makes composition/PNG engine-owned: filename or title; codec,
rate, depth, complete-packet audio bitrate, channels and FFT header; kHz/Nyquist
and M:SS ticks/grid; ember-v1 0 to −140 dB bar; labeled native-channel p95
markers. Default 2560×1440, bounded maximum 3840×2160; offline antialiased font,
300-dpi lossless RGB8 PNG, explicit prefix/mid warning and collision-safe path.
P07/A06 reuse this canvas rather than recreating calibration in a UI wrapper.

P01 implementation constraints:

Follow RELEASE_CONTRACT artifact dimensions/basis/reduction and collision-safe
output rules. SoX pixel/color parity is explicitly not required. Keep artifact
status separate from report success and preserve the shared prefix.

## P07 — report, batch and comparison workflows

**DONE 2026-10-06, engine 0.32.0.** [Packet results](task-results/P07.md),
[workflow validation](docs/validation/PRODUCT_WORKFLOW_VALIDATION.md) and
[user/library guide](docs/PRODUCT_REPORT.md) record the implemented scope,
exact checks and bounded saved/comparison/export limitations.

**Sol; after P05/P06/P03a.** Read Python `print_report`, `_report_to_dict`, batch and
comparison functions; current CLI/evidence layer and P01 result contract.
Expose reference verdict, full metadata, measurements, candidate source/depth
interpretations, spectrogram links and diagnostics. Add the required comparison
workflow with deterministic ordering and stated reference criteria; ranking is
not proof of perceptual quality. Preserve existing machine JSON consumers. This is the independent library/CLI
product workflow, not Alfred navigation or a general Audio Compare redesign.
**Done:** saved-report, mixed-success batch, absent-field and comparison fixtures
pass; native channels, units, unknowns and scopes remain visible. Documentation
shows ordinary user workflows and distinguishes reference from validated claims.

P01 implementation constraints:

Complete PYTHON_PARITY W01–W08/W10, including actual no-decode info, directory
batch, fast alias, saved product rendering and separately versioned product JSON.
Preserve all mapped fields/aliases with status/domain/unit/scope and raw tag
access; no legacy certainty text in an unqualified headline.
For spectrogram export, call P06 `write_png_new` with caller-selected new paths,
retain optional saved `presentation`/title and expose preset selection. Do not
redraw axes/legend or infer unavailable bitrate/cutoff in the CLI.

## P08 — differential acceptance and regression

**DONE 2026-10-06, engine 0.32.0.** [Packet results](task-results/P08.md),
[acceptance ledger](docs/validation/CORE_RELEASE_VALIDATION.md). 271 Rust tests,
zero failed/ignored; frozen generated differential/schema/workflow checks passed.
F02 native DSD deferred, structured unsupported controls passed. P09 subsequently accepted 0.32.0.

**Sol; after P02–P07/F01 and any required P01 child tasks.** Use PYTHON_PARITY.md
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
checks. Cover every initial-release group/rule/workflow and schema/version transition.
Record native DSD rows as explicitly deferred F02, not passed or unexplained
missing parity. Check structured unsupported DSD behavior; defer conversion
oracles/new DSD schema acceptance until F02.
Do not use private recordings as public fixtures or re-open locked controls.

## P09 — standalone Audio Forensics release gate

**DONE — ACCEPT 2026-10-06, engine 0.32.0.** [Packet results](task-results/P09.md),
[decision and limitations](docs/validation/CORE_RELEASE_VALIDATION.md#p09-decision--accept-2026-10-06).
Frozen P08 code/evidence verified; no production blocker or change. Next A01.

**Astra; after P08.** This ends initial standalone delivery, before Alfred A01.
Accept the independent FLAC/WAV/ALAC library/CLI without Android linking, APK
or device gates. Document the owner-approved F02 deferral; DSD is unavailable
until its later gate, not an initial rejection reason.
Review required parity rows, interpretation wording,
format boundaries and final evidence. Check new code risks, not every already
closed parser experiment. Fix concrete release blockers and rerun affected checks.
**Done:** written accept/reject decision in CORE_RELEASE_VALIDATION.md with
version, supported scope and explicit limitations. New research questions become
E tasks and cannot silently block a faithful, correctly described release.

## A01 — Alfred shared architecture and integration contract

**Astra; DONE 2026-10-07.** See [ANDROID_CONTRACT.md](ANDROID_CONTRACT.md) and
[A01 results](task-results/A01.md). Alfred development begins here, after the standalone core
acceptance. Read ALFRED_ARCHITECTURE.md, CORE_INTEGRATION.md, JOB_VALIDATION.md
and the release/result contract. Produce `ANDROID_CONTRACT.md` separating shared
workspace/selection, input ownership/staging, capability routing, jobs/lifecycle,
retention/export and feature adapters. Entry is a file/folder workspace, not a
forensic report. Support one/many tracks and bounded folder enumeration.
Freeze only initial integration interfaces: identity/source leases, capability
outcomes, job progress/cancel/completion, versioned feature payload/artifact
ownership and host failures. Choose bridge mechanism, opaque native ownership,
exact JSON integers, ABI/build strategy, safe close/poll/cancel races, process-death
behavior, staging/history/queue limits and resource budgets. Preserve core
worker admission, ALAC unwind/panic boundary and P04 memory constraints.

Use a separate app subtree/native adapter now, with one-way dependency on the
independent Rust library. Document build independence and eventual extraction of
Alfred to its own repository; choose dependency packaging without source copying.
A01 must define the long-term ownership boundary for reusable P06 spectrogram
and P07 comparison functionality when Alfred moves repositories. Produce a
matrix for computation, rendering, versioned contracts, tests and consumers;
decide core-retained versus Alfred-owned responsibilities and any justified
future host-neutral component. Document extraction steps and standalone CLI/host
compatibility, with one-way dependencies. Current reuse does not make these
features conceptually part of Forensics. Do not duplicate/refactor them now or
preselect a new crate simply to satisfy this planning requirement.
Record Spectrogram as a broader competitive viewing/exploration feature, with
A06a only its initial slice. Fuller requirements and comparison expansion remain
future packets, not detailed APIs or implicit launch requirements.
Do not design detailed APIs for Tag Studio, Converter or Archival Tools.
Plan Android 11–16/Redmi 13 4G and Hot 11S testing; verify SDK/NDK/API choices against current
official documentation during execution. Required formats, 700 MiB import and
fidelity remain the release contract's. DSD is deferred beyond launch; reserve
feature payload version dispatch without designing/implementing F02 now.
**Done:** implementation-ready shared seams and initial feature contracts for
A02–A07, extraction plan with explicit spectrogram/comparison ownership decisions,
standalone-consumer compatibility, resource/lifecycle tests and unresolved product
choices recorded. Unsafe FFI stays outside the safe analysis core.

## A02 — Alfred workspace scaffold and builds

**Status: DONE 2026-10-07.** [A02 results](task-results/A02.md); later
[Android 11–16 CI acceptance](task-results/ANDROID-11-CI.md) passed. A03 is next.

**Sol; after A01.** Create the separate Android application subtree, offline
workspace/navigation and pinned reproducible native builds. Open to selection;
route applicable operations without requiring a forensic report. Add feature
boundaries for Forensics, Spectrogram and Compare. Future tools remain documented
placeholders, not working buttons or implemented features. Use Alfred and a
development application ID until owner finalizes it. No accounts/network service.
**Done:** debug APK builds, native library packaging/build instructions repeat,
app consumes the core explicitly, and the core library/CLI still builds independently.
Missing SDK/NDK prerequisites remain explicit; no silent approval bypass.

## A03 — Audio Forensics native adapter

**Status: DONE 2026-10-08.** Both ABI builds and all API 30–36 packaged JNI
smoke jobs passed in CI 37673381546. See [A03 results](task-results/A03.md).

**Astra; after A02.** Implement A01's app-owned bridge to the existing Rust
source/job/result APIs and P07 product layers. Keep feature semantics in the
Forensics adapter and scheduling/lifecycle transport in shared Alfred services.
Start, progress, cancel, consume, close and structured errors preserve ownership,
exact integers and version dispatch. Preserve optional reference/spectrogram
paths; if the existing AnalysisJob result shape needs an adapter, keep that
adapter app-owned rather than imposing Alfred state on the portable core.
UI threads never decode or block; no panic crosses the boundary.
**Done:** NDK linking, packaged native-load/generated-input smoke, stale handles,
repeated close/cancel/poll and release tests pass. Label emulator/physical evidence;
Android target compilation alone is insufficient.

## A04 — shared workspace selection, input and staging

**Status: IN PROGRESS 2026-10-08 — implementation, both ABI builds and input controls passed API 30–36; real picker passed API 31–36, API-30 activation unresolved.** [A04 results](task-results/A04.md).

**Sol; after A02.** Implement shared SAF file/folder picker/workspace, one/many
track selection, bounded folder enumeration, URI grants, source acquisition and
staging for nonseekable providers under A01 limits. Features receive owned input
access, not independent import systems. Route only applicable installed operations;
selection count and actual codec checks include ALAC versus AAC in M4A. Capability
checks do not require a forensic report and unsupported content stays explicit.
**Done:** single/multiple/folder selection and routing, seekable/nonseekable,
denied/revoked/missing inputs, disk pressure, copy cancellation and large files
behave explicitly; handles/temporary files release correctly. No whole-audio
buffering or unrestricted-storage permission merely to read selected documents.

## A05 — shared jobs, lifecycle and local retention

**Sol; after A03/A04.** Implement A01 background execution/notifications,
job identities, bounded conservative admission/queue, progress/cancellation,
rotation, background/screen-off and process recreation. Feature adapters supply
work and versioned results; shared services do not assume every operation is an
analysis or ends in AnalysisReport. Preserve core worker limits; no new concurrent
DSP scheduling. Retry uses fresh tokens/sources. Bound local result/artifact
storage, version dispatch, deletion and explicit export/share infrastructure.
**Done:** lifecycle/state tests and emulator smoke pass; stale callbacks cannot
update new jobs, teardown does not block UI, decode completion is not job success,
and interrupted work is honest. Storage ownership/cleanup and failure controls
pass; A07 checks physical behavior.

## A06 — Audio Forensics feature integration

**Sol; after A05/P07.** Integrate the Forensics operation from the Alfred workspace:
qualified reference verdict, complete read-only metadata, expandable measurements,
detector scopes/caveats, forensic history and report export/share. Reuse Rust policy;
no Kotlin scoring. Use shared input/jobs/storage. Link to separate Spectrogram and
Compare workflows; neither belongs exclusively inside this results screen.
Current metadata exposure is not Tag Studio editing.
**Done:** fixtures cover success/failure/abstention/version states, nulls are not
zero, large integers survive storage/export and all supported forensic information
remains inspectable. Artifact failure does not erase successful measurements.

## A06a — initial Spectrogram feature integration using P06

**Sol; after A05/P06.** This is the initial delivery slice of Alfred's broader
Spectrogram viewing/exploration feature, not its complete product scope. Follow
A01's ownership/extraction boundary. Add a first-class workspace operation using existing Rust
P06 source collection/canvas/PNG. It opens without first viewing a forensic report,
even if its backend reuses the existing two-pass analysis. Use shared source/jobs,
retention and explicit export/share. Preserve title/preset, hash/interval, native
channel/mid warnings, axes/calibration, optional presentation and separate artifact
failure. Do not redraw the Rust canvas or require a new spectral-only core API.
**Done:** direct workspace reachability, generated artifact/display/export, old
presentation handling and failure/cancellation controls pass. P06 numerics stay
unchanged; no duplication/refactoring or new DSP scope in this packet. Completion
does not establish a full interactive viewer or competitive parity; later named
requirements/implementation packets own that expansion.

## A06b — independent Audio Compare workflow

**Sol; after A05/P07.** Add a first-class multi-selection workspace operation
using P07's versioned comparison of variants of one track, including saved results
where applicable. Acquire inputs/run needed jobs through shared infrastructure;
no prerequisite forensic results screen. Follow A01's long-term reusable
comparison ownership boundary; initial code residence does not make Audio Compare
a Forensics subfeature. Preserve explicit reference tuple,
compatible method/domain/coverage checks, ordering/nulls and ranking caveats.
Broader comparison requirements are unfrozen; do not invent perceptual metrics,
sample alignment or unlike-track rankings.
**Done:** independent routing, selection constraints, mixed/absent/incompatible
results, ties and all-unavailable cases match P07; export/storage use shared seams.

## A07 — Alfred physical-device gate

**Astra + owner; after A06/A06a/A06b/P09/U01.** Test Redmi 13 4G/Android 16;
label Android 11–16 emulator evidence and Hot 11S/Android 11 physical evidence
when available. Use shared memory admission, without device-specific chunking
or throttling; CI emulator success does not establish phone RSS/background behavior. Exercise shared file/multiple/folder
selection and operation routing, each initial workflow, required formats,
600–700 MB inputs, cancel/retry, rotation, background/screen-off, process death,
storage pressure and runtime/thermal/memory behavior. Compare generated forensic
results, spectrogram artifacts and reference comparisons with accepted core outputs.
**Done:** ANDROID_VALIDATION.md records actual build/device/results against A01
budgets and repaired blockers. No available phone means saved ready APK/checklist
and BLOCKED_INPUT device portion, never a passed device gate. Future tools and
endgame research are not acceptance dependencies.

## A08 — Alfred beta and repository-boundary handoff

**Sol + owner; after A07/U03.** Prepare reproducible release candidate, hashes,
notices, supported formats/OS, install/update instructions and GitHub Release
text describing Alfred's implemented features and future placeholders honestly.
Finalize package identity and owner-held signing before distribution; audit for
private evidence. Record separate library and app versions/dependency, independent
core build and extraction readiness/remaining move work. The actual repository
move occurs once Alfred has taken shape; no fixed date or beta gate is assumed.
**Done:** installable candidate, owner acceptance/signing instructions and clear
app/core handoff ready. Actual publication requires explicit authorization;
prepared and published are recorded separately. No store project required.

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
