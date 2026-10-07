# Start here: Audio Forensic Rust

Updated **2026-10-08**. This is the current continuation point. Historical milestone
entries and exact older check totals are preserved in
[the handoff archive](docs/history/HANDOFF_20261006.md); newer decisions here win.

## A03 handoff — 2026-10-08

**Implementation finished; packaged generated-input runtime acceptance PENDING.**
Owner wants to move to a cheaper/new A04 thread and conserve usage. Do not repeat
completed core audits/build matrices. A04 depends on A02 and can proceed while
this last A03 CI gate completes. A03 must not be marked DONE without runtime success.

Current branch: `codex/a03-native-adapter`. Source through **ce037d3** is committed
and pushed with explicit owner authorization. Final CI is running:
https://github.com/spideyonmoon/audio-forensic-rust/actions/runs/37673381546.
Next action: inspect that run, specifically packaged JNI generated-input results.
If green, mark A03 DONE and advance normal dispatch to A04; otherwise inspect the
failed job/artifact and fix only the reproducer. No local build remains running.

Implemented: serial native worker/handles, cancellation/source-release admission,
JNI byte arrays, exact Kotlin JSON/control executor, complete metadata/product/
Spectrogram payloads, saved compare/render, atomic manifests and memory limits.
**8 focused Rust tests, all-target Clippy, Kotlin compilation and Android lint
passed.** Prior run 37671795459 passed both NDK ABI links, APK build/lint and
ELF/ZIP/signature checks. API 30/31 loaded JNI but failed the debug harness because
its generated fixtures were absent from the APK. That run was cancelled; the
fix explicitly schedules asset/lint consumers and adds exact packaged-fixture
verification. Local merge/lint-model generation passed and all three fixtures
are present (`target/a03/assets-final.log`). Runtime after this fix is pending.

Earlier failures: Windows default linker (existing gcc/lld shim fixed), PNG
flush access (fixed, report retained on PNG failure), one Clippy finding (fixed),
Gradle asset producer/lint dependency (fixed), API-31-only debug handle conversion
(replaced with checked API-30 conversion). No core or oracle changes. Details:
[task record](task-results/A03.md), [adapter guide](apps/alfred/NATIVE_ADAPTER.md),
[validation](docs/validation/ANDROID_INTEGRATION_VALIDATION.md).

Prior-run APKs/receipts are local `target/a03/ci-artifacts/`; they predate the final
fixture fix and are NOT final acceptance APKs. Original logs and failed API-30
receipt are under `target/a03/`. These ignored files are not a source backup.
Core source/Cargo/schema/fixtures, pinned reference and private recordings remain
unchanged. Original cleanup edits below and in task-results/README.md remain local
and outside A03 commits. No physical/16 KiB runtime, SAF/jobs/history/UI or release
acceptance is claimed. Native storage reservations and orphan/lease cleanup are
explicit A04/A05 integration responsibilities.

## Main synchronization — 2026-10-07

Owner requested getting the completed work up to date. A01/A02 are DONE; A03 is
next and has not started. Local main fast-forwarded to the tested Android branch
after staging and verifying an exact matching source tree, without resetting or
stashing owner files. Three additional local planning edits were preserved;
AGENTS/architecture/validation index and stale current handoff pointers now agree.
Remaining synchronization changes are documentation only. Normal source, core,
private recordings and ignored tool/build/evidence files are preserved. Source
Git history is distinct from local target/.tools evidence and external backups.

Checks: exact pre-merge source tree equality passed; no merge conflict. Final
current-dispatch and documentation-only checks passed, with 115 local Markdown
file targets verified. Staged whitespace and core-preservation checks passed.
No rebuild, analysis/device test or new jobs are needed for these doc changes.
Existing green Android run 37635443625 remains the build/runtime scaffold evidence.
Recovery snapshots are ignored target/sync-main-20261007/ local files only.

## Android 11 and GitHub Actions amendment — 2026-10-07

Owner authorized Android 11–16 support for Hot 11S/Helio G88 and offloading heavy
checks to GitHub Actions. All five Gradle minimums/native linker now use API 30;
compile/target 36, ARM64 primary ABI and common memory admission are unchanged.
No phone-specific chunking/throttling or core/DSP changes. Separate x86_64 CI
APK enables emulator smoke; lifecycle API guards and planned A07 coverage are
amended in ANDROID_CONTRACT.md. A03 remains the next delivery task.

The tested `codex/android11-ci` branch has been fast-forwarded into local main.
A01/A02 source and Android compatibility work are now tracked in Git history.
The source tree was verified identical to the tested branch before the merge;
remaining local planning edits were preserved and updated for the A03 handoff. Owner's large-diff concern was audited: root core, Cargo files,
schemas/examples/tests unchanged; bulk additions were existing dependency pins.
No Kotlin application screen or Rust source changed in this compatibility packet.

Run 37635443625 passed both API-30 native/Gradle ABI builds, lint and ELF/ZIP/
signature checks. All seven API 30–36 emulator install/workspace/JNI smoke jobs passed;
the complete run finished successfully. Successful API-30 saved UI evidence was
independently verified. Existing A02 APK verifier regression and Python/Bash/XML
syntax/staged whitespace passed. Local YAML parser was absent; GitHub parsed and
executed the workflow. No heavy laptop build or private-audio upload ran.

Initial failures retained in [the packet](task-results/ANDROID-11-CI.md): missing
OAuth workflow permission (owner authorized), retired SDK tools package, four
missing upstream-reviewed metadata/Linux-tool pins, and a test that failed to
scroll to the native status. No checksum bypass or oracle change. Core CI
37632592799 passed; app-only changes now avoid redundant core builds.

Completed run: https://github.com/spideyonmoon/audio-forensic-rust/actions/runs/37635443625.
Next concrete action: A03 under the amended contract. No local/remote jobs running. Phone RSS/background, 16 KiB
runtime, analysis/SAF/jobs/results and release acceptance remain A03–A07 work.
Ignored evidence under target/android11-* is local, not source history or backup.

## A02 completion — 2026-10-07

Owner requested complete A02. Five-module Compose scaffold and separate native
Cargo workspace are created under apps/alfred; pinned provisioning/build scripts
and build guide are present. Existing A01 dirty work and core/private inputs are
preserved. Native transport is only the caught-unwind load/version bootstrap;
analysis/SAF acquisition/jobs/results remain A03–A06.

Tool provisioning completed, log target/a02/provision-resume.log. Initial
Invoke-WebRequest JDK download stalled and was interrupted; authorized curl retry
verified JDK/Gradle/platform archives. Prerequisites are local .tools/alfred,
with explicitly approved SDK licence acceptance. Root CLI and no-CLI builds passed with Rust 1.85, locked/offline and the
existing local gcc/lld shim (CLI required an authorized linker retry);
logs target/a02/core-cli.log and core-no-cli.log. Native NDK link passed on authorized retry, and the checksum-pinned wrapper
passed after a fresh-process retry for Windows immutable-cache rename failure.
Complete online Gradle build/lint, dependency locks, clean offline repeat and
ELF/ZIP/signature inspection passed.
No publication/device acceptance is implied. Task record: task-results/A02.md.

Completion evidence: the complete online Gradle build/lint and clean offline
repeat passed. The debug APK is `apps/alfred/app/build/outputs/apk/debug/app-debug.apk`;
native/ELF/ZIP/signature evidence is `apps/alfred/app/build/outputs/apk/debug/alignment.json`.
The build receipt is `apps/alfred/build/receipts/build.json`. A03 is next; no
device/runtime, SAF, JNI handles, jobs or result-payload acceptance was run.

## A01 completion — 2026-10-07

**Owner-requested review corrections completed:** [A01 review](task-results/A01-review.md)
now records all three findings resolved in the contract: cancellable probe handles
with complete metadata payloads, one negotiated comparison scope and same-track
assertion, and Publication 2560×1440 as the default with all existing PNG presets.
Affected A03/A04/A06a/A06b acceptance cases are explicit. Correction checks are
recorded in task-results/A01.md and the Android validation record; the original
completion/check record below is preserved. At that earlier A01 handoff the
owner requested stopping before A02; A02 has since completed as recorded above.

**DONE**, documentation-only packet. [ANDROID_CONTRACT.md](ANDROID_CONTRACT.md)
freezes initial shared selection/capabilities, immutable bounded SAF staging,
app-owned JNI handles/exact JSON, serial jobs/lifecycle/interruption, quotas and
feature payloads, build/ABI/unwind strategy and repository extraction. P06/P07
computation/rendering/contracts/tests remain in this independent core; Alfred
owns feature UI and platform infrastructure. No duplication/refactor/new crate.

Product memory admission caps reference capture at 8,640,000 native-rate frames,
with explicit prefix offers for larger scopes. Reservations are unmeasured
engineering defaults for A07, not phone guarantees. 700 MiB staged import stays
required. Android 14–16, API 34/36 and NDK 30.0.16248370 choices were checked
against linked official documentation; actual tool installation/link/device
behavior remains A02–A07 work. Final package identity/signing stays U04/A08.

Checks passed: parity inventory (155/139/30/34, pinned clean reference), contract
and public-API/path/frame-budget checks, current dispatch/local Markdown targets,
287 unchanged frozen computational hashes and whitespace. Exact commands and
not-run scope: [A01 results](task-results/A01.md),
[validation](docs/validation/ANDROID_INTEGRATION_VALIDATION.md). Receipts are
ignored target/a01 local evidence only. No Rust/Android builds, private-audio
analysis, commit/push/upload or extraction; no unresolved failure or running job.
At that earlier A01 handoff A02 was next. A02 is now complete; A03 is next.

## Current task and delivery boundary

- Active task: **A03 runtime acceptance pending**; see current entry and CI run above.
- Next default delivery task: **A03 — Audio Forensics native adapter (Astra)**.
- **F02/DSD DEFERRED beyond the first standalone release and Alfred launch**,
  owner approved 2026-10-06 to conserve Astra budget. Its frozen DSF/DFF,
  DSD64/128/256, mono/stereo and 88.2 kHz conversion contract remains for later.
- Initial delivery: **P09 ACCEPT, engine 0.32.0**, covering FLAC/WAV/ALAC-M4A. P08/P09
  explicitly record deferred native DSD rows and honest unsupported behavior.
- Alfred begins at **A01 after P09**, initially in a separate app subtree here,
  later extracted to its own repository. The Rust library/CLI remain independent.
- Shared Alfred infrastructure: A01/A02/A04/A05. Forensics adapter/UI: A03/A06.
  Independent Spectrogram/Compare features: A06a/A06b are initial slices reusing
  P06/P07. Spectrogram's broader ambition is competitive viewing/exploration,
  not only image generation. A01 must define long-term ownership of reusable
  computation/rendering/contracts/tests for both areas before Alfred extraction;
  reuse existing implementations now without duplication/refactoring. Expanded
  requirements and competitive targets remain future named work, not launch gates.
  Tag Studio, Converter and Archival Tools are future placeholders only.
- Endgame E tasks, calibration and MQA confirmation remain separate, not launch gates.

Read [ROADMAP.md](ROADMAP.md), [A01 card](ROADMAP_TASKS.md#a01--alfred-shared-architecture-and-integration-contract),
[release scope amendment](RELEASE_CONTRACT.md),
[Alfred boundary](ALFRED_ARCHITECTURE.md) and [OWNER_CHECKLIST.md](OWNER_CHECKLIST.md).
P09/A01/A02 are complete. Read [ANDROID_CONTRACT.md](ANDROID_CONTRACT.md) for
A03–A07 implementation decisions. The app/native bootstrap scaffold exists and
passes Android 11–16 CI; analysis handles, acquisition and jobs are not implemented.

## Implemented state and evidence

Actual manifest version is **0.32.0**, Rust 1.85 minimum. Native report schema is
**0.18.0**, policy `observations-only-v18`, ancestry `INCONCLUSIVE`, index null.
P09 is accepted; completed P/F statuses remain intact. F02 is deferred;
A01's contract and A02 scaffold/builds are done; A03–A08 implementation/device
gates remain open.

- **P07**: independent full product/saved/info/batch/comparison and P06 export
  workflows, all 155 Python field locations. Scoped suite 103 passed; final
  domain/progress recheck 9 passed, independent schema/PCM/60s/FLAC/ALAC controls,
  MSRV Clippy/no-CLI compilation passed. [Results](task-results/P07.md),
  [validation](docs/validation/PRODUCT_WORKFLOW_VALIDATION.md),
  [user/API guide](docs/PRODUCT_REPORT.md). Native JSON/policy unchanged.
- **P05**: saved/bound P04 v2 reference assessment, all 34 rules, qualified
  candidates and missing-input propagation. Final optimized MSRV/no-CLI suite
  61 passed; Clippy/no-CLI/Android target and pinned inventory checks passed.
  [Results](task-results/P05.md), [validation](docs/validation/REFERENCE_ASSESSMENT_VALIDATION.md).
- **P06 + PNG follow-up**: bounded spectrogram data and Rust-rendered PNG canvas,
  embedded licensed font, axes/legend, scoped bitrate and collision-safe export.
  Final follow-up focused tests 80 passed plus independent generated PNG/data
  checks. [Results](task-results/P06-PNG.md), [validation](docs/validation/SPECTROGRAM_VALIDATION.md).
- **F01**: offline native ALAC/M4A, 16/24-bit mono/stereo 8–384 kHz, bounded
  metadata/seekable source. Final optimized MSRV/no-CLI suite 96 passed;
  72 generated matrix controls and 700 MiB seek/padding control passed.
  [Results](task-results/F01.md), [validation](docs/validation/ALAC_VALIDATION.md).
- **P04a/P04b/P04c** and P01–P03a remain complete; original task/validation
  records retain exact commands, bounds, initial failures and final evidence.
  [Packet index](task-results/README.md), [validation index](docs/validation/README.md).

Those packet totals are historical scoped runs; the new P08 full-core
regression and finite acceptance are recorded below. Android
checks above are target compilation only; NDK linking/APK/device behavior remain
unverified. The P04 maximum-rate reference profiles are memory-heavy; A01/A07
must stage/budget them using documented bounds. Preserve ALAC unwind/panic
containment before any Android panic=abort decision. P07 implements product envelope
naming/version dispatch as `audio-forensic-product-v1` (the proposed
`alfred-product-v1` is not emitted); it contains no shared Alfred state. No Kotlin scoring or app-owned state belongs in the safe core.

## Repository cleanup and checks — 2026-10-06

Documentation is organized under docs/validation, docs/history and docs/REFERENCE.md;
root retains agent entry/planning files. README is now a concise front page.
Historical record contents are preserved; relative Markdown targets were rebased.
Bare old filenames are searchable in the documentation index. Source code,
schemas, fixtures and private audio were not reorganized or modified.

Checks passed: parity inventory (155 fields/139 methods/30 groups/34 rules,
acyclic updated gates and pinned clean reference), 231 local Markdown links,
inventory-helper Python syntax and whitespace. Preservation accounts for all
350 prior nonignored files: 333 byte-identical at original/relocated paths,
17 intended edits, zero missing/unexpected non-document edits. Root files fell
from 53 to 15; 35 validation records and archived README/handoff retain content
apart from Markdown targets. The only non-Markdown edit removes F02 from the
inventory helper's initial P08 dependency expansion.
See [cleanup record](task-results/REPO-POLISH.md) for receipts. No Rust/DSP/Android builds were run for these
planning/documentation changes. No commit, push, upload or repository extraction.

## Feature-ownership clarification — 2026-10-06

[Planning record](task-results/SPECTROGRAM-OWNERSHIP.md): A01 now owns explicit
long-term spectrogram/comparison ownership decisions and standalone compatibility;
Spectrogram has broader viewing/exploration ambitions beyond P06/A06a. Parity
inventory, local Markdown targets and whitespace passed. Historical validation
and implementation files were not edited; no builds/tests rerun or jobs started.

## Preservation and next action

Existing dirty/untracked implementation work is local and remains intact. Inspect
`git status` before edits; do not reset, clean, regenerate fixtures or overwrite
owner work. The pinned Python clone remains at
`c6ecce2296256b516709d87088896d1be913908c`. Preserve reference/test_files and all
private recording notes; stated CD/vinyl origins are not verified truth.

Ignored evidence/build receipts are under target/, corpus/local/ and .tools/;
these are local files, not Git history or an external backup. Prior milestone
receipts remain where originally recorded (including target/p06-msrv and
accepted P05/P06/F01 receipts). U05 external backup remains owner work.

Next concrete action: **A03**, implementing the app-owned native adapter under
ANDROID_CONTRACT.md. Consume existing product APIs without changing core/DSP
semantics; preserve shared memory admission and use GitHub Actions for heavy checks.
Keep P06/P07 reuse unchanged. P09 details
are in task-results/P09.md and docs/validation/CORE_RELEASE_VALIDATION.md;
no publication is authorized by the gate.

## P07 completion — 2026-10-06

Engine 0.32.0 adds `audio-forensic-product-v1`/comparison v1, combined source
collectors, actual no-decode info, saved rendering, all 155 field locations and
collision-safe P06 export. Native JSON/schema/policy unchanged. Product/info
batches retain at most 32 inputs/results, saved files 64 MiB, export history
32 attempts; method/domain/duration-EOF comparison compatibility is conservative.
Scores remain uncalibrated; unknowns stay null and unlike/all-unavailable
comparisons have no winner. No app-owned state was added to the core.

Checks: scoped Rust suite 103 passed, zero failed/ignored; final domain/progress
recheck 9 passed. Independent schema/runtime 9 rejection controls, exact generated
PCM hash, gain variants, saved rendering after generated-audio deletion, u64 max,
actual 60s/480000-frame fast and FLAC/ALAC product controls passed. Final binary
smoke, Rust 1.85 all-target Clippy warnings denied, no-CLI library/example
compilation, formatting/helper syntax/schema drift/parity inventory/local links/
whitespace passed. Initial fractional-frame assertion and Clippy style failure
were corrected without changing DSP/oracles; attempts retained in target/p07.

Exact commands/bounds and not-run scope: task-results/P07.md and validation above.
Receipts are ignored `target/p07/`, local evidence only. No full-core/P08,
private-audio/calibration/DSD, Android target/link/APK/device, remote CI,
commit/push/publication or external backup ran. Existing dirty/untracked work and
private recordings/reference were preserved. **No unresolved P07 failures or
running jobs. At that handoff P08 was next; its completion is recorded below.
Alfred A01 begins after standalone acceptance.**

## P08 completion — 2026-10-06

P08 DONE in engine 0.32.0; production source/dependencies/schemas/accepted fixtures
unchanged. Permanent finite freeze/acceptance helper: scripts/check_core_release.py.
All 30 P01 groups and R01–R34 have mapped differential/deviation evidence in
docs/validation/CORE_RELEASE_VALIDATION.md; exact commands in task-results/P08.md.

Final Rust/Cargo 1.85.0 optimized core regression: **271 passed, zero failed or
ignored** (40 executables, empty doctest block). MSRV all-target Clippy warnings
denied, no-CLI library/examples check, CLI/all-example build, stable formatting,
16 read-only contract/helper checks and final inventory/local links/whitespace
passed. Fresh generated product/ALAC/spectrogram/PNG and equivalent-input Python
policy checks passed. Twelve DSF/DFF native/product/fast signature controls
returned structured unsupported, exit 1, null coverage/scores and unchanged
measurement ancestry/index. F02 conversion remains deferred, not passed.

Initial sandbox linker denial stopped compilation before tests; authorized
offline retry passed with the existing process-local gcc/lld shim. Helper map/
Windows log normalization refinements affected no Rust source/fixture/schema.
Initial documentation encoding was repaired; subsequent edits use UTF-8. No
unresolved acceptance failure or job remains. The final frozen 287-file manifest,
binary hash, exact command/exits and receipts are ignored target/p08 local files,
not Git history or an external backup. Existing dirty/untracked work, pinned
Python checkout and private recordings are preserved.

Not run: default thin-LTO release build, fresh 700 MiB/RSS benchmark, private
audio/calibration/MQA confirmation, DSD conversion/new schema, Android target/
link/APK/device, remote CI, commit/push/publication or external backup.
**Next P09 finite standalone accept/reject review; Alfred A01 follows P09.**

## P09 completion — 2026-10-06

**ACCEPT standalone engine 0.32.0**, Rust minimum 1.85, for documented offline
FLAC/WAV/ALAC-M4A scope. F02/native DSD remains deferred beyond initial release
and Alfred launch. The written decision and finite review are in
[CORE_RELEASE_VALIDATION.md](docs/validation/CORE_RELEASE_VALIDATION.md#p09-decision--accept-2026-10-06)
and [P09 results](task-results/P09.md). No production blocker or change; native
schema 0.18.0 / observations-only-v18 / INCONCLUSIVE / null index unchanged.
Reference outputs remain uncalibrated; the existing 30-group/34-rule P08 ledger
and D01–D12 cover required rows and named deviations.

Fresh checks passed: unchanged 287-file P08 computational tree and binary hash,
actual P08 log/receipt verification (271 Rust passes, zero failed/ignored;
16 contract + six independent successful commands; 12 saved DSD rejections),
parity inventory, CLI version, saved rendering/comparison, documentation file
targets/current dispatch and whitespace. These verify prior acceptance plus
fresh smoke; no full regression or build was repeated. Receipts/scripts are
ignored target/p09 local evidence. No unresolved failure or running job remains.

Accepted limits: maximum-rate P04 payload 2,346,057,728 bytes plus plans/other
state/overhead; no phone RSS guarantee. Desktop CLI directory collection precedes
its 32-input admission limit and is outside analysis deadlines; library callers
own batch/serialized-size admission. The product guide now states those limits.
Keep ALAC's unwind boundary and cooperative/blocking-I/O cancellation caveats.
A01/A04/A07 must explicitly handle app admission, staging and folder enumeration.

No fresh private-audio/calibration/MQA/DSD/700 MiB/RSS, default thin-LTO release,
Android target/link/APK/device, remote CI, commit/push/upload/publication or
external backup. Existing dirty/untracked work and recordings remain intact.
**Next A01; not started.** P09 acceptance does not authorize publication.

Final P09 documentation verification completed 2026-10-07: 271 local Markdown
file targets and whitespace passed; frozen 287-file tree unchanged. The local
link checker needed a Windows-1252 read fallback for the preserved historical
NOISE_FLOOR_VALIDATION.md; no historical evidence was edited.

## Source publication - 2026-10-07

Owner explicitly authorized committing and pushing all non-ignored local project
changes to origin/main (github.com/spideyonmoon/audio-forensic-rust). Includes
engine 0.32.0, generated fixtures and documentation reorganization. Private
reference recordings, corpus/local and target evidence remain ignored and local.
Publication checks: staged path inventory reviewed; generated ALAC provenance
checked; staged whitespace checked and one trailing blank line repaired. Prior
P08/P09 validation remains as recorded above; no build or tests rerun for this
publication. Next delivery task remains A01; no background jobs started.
