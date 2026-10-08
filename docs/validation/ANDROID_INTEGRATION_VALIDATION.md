# Android integration validation

## A06 accepted — 2026-10-08

Compiled 8094a01 passed both ABI builds/native host controls/lint/packaging and
entire API 31–34 in full run 37806954007. Same hash/source-bound APK passed entire
API 30/35/36 on harness f64d670 in retry 37809483892. All 18 feature controls and
five Compose history/restart/qualification/exact u64/null checks passed Android
11–16, along with all retained input/lifecycle controls. API-30 picker remains
A04 manual Hot 11S coverage. Acceptance spans these two runs. No jobs running.
Initial sandbox/compiler, unsupported API-30 wm command, below-fold picker wait
and missing retry manual marker are repaired and retained in
[A06](../../task-results/A06.md). Python/whitespace, actual-XML regression and
APK/hash/source reuse guard passed; real compiled-source changes remain rejected.
No core/native Rust, DSP/policy or physical/16-KiB/RSS/thermal acceptance change.
The earlier implementation progression below is historical.

Forensics now consumes shared acquisition/jobs/storage and unchanged Rust products.
Qualified summaries, paged read-only exact metadata/measurement/reference fields,
mixed outcomes, history/retry/delete and original-byte export/share are implemented.
Future payloads remain exportable; future assessment methods are not interpreted.
Explicit heap admission protects large saved-document inspection. The native
manifest preserves item/output binding and separate artifact diagnostics.

Corrected offline Kotlin compilation/lint passed after a debug nullable-map repair
(0 errors/12 warnings/3 hints). Python syntax/whitespace passed; generated Android
feature/UI acceptance is pending. Initial sandbox daemon
connection failed; permitted offline retry ran. The debug harness adds actual
feature-work success/mixed-failure and synthetic abstention/version/null/u64/export
controls; no runtime pass claimed yet. [A06 record](../../task-results/A06.md).
No core/DSP/scoring change, private-audio upload or physical/16-KiB/RSS acceptance.

## A05 acceptance — 2026-10-08

Shared service/jobs, durable attempts/manifest recovery, bounded queue/progress,
cancel/retry, ViewModel rotation, disk reservation/history/grant leases and explicit
SAF export/FileProvider sharing are implemented. Integration API and lifecycle
ownership: [shared jobs](../../apps/alfred/SHARED_JOBS.md).

Offline app Kotlin/shared lint passed with 0 lint errors/8 warnings (pinned
dependency, application-context singleton, conservative usable-space and KTX
style suggestions). Final generated-harness Kotlin compilation and Python syntax
passed. Initial share-Intent return-type and misplaced harness-block compile
errors were repaired. Exact logs/state: [A05 packet](../../task-results/A05.md).
Both ABI build/packaging/lint passed on source 774eb74 in CI 37773573203.
Entire emulator jobs passed API 30–32 there and API 33/35/36 in hash/source-bound
retry 37775318484. Final API-34 retry
[37777694067](https://github.com/spideyonmoon/audio-forensic-rust/actions/runs/37777694067)
passed all suites on harness 851bdb1, running picker before lifecycle rotation
with every check retained. A05 is DONE; no jobs running; next A06. Acceptance
spans these runs. API-30 picker evidence remains A04 manual Hot 11S coverage.
Rotation/background/screen-off, delayed cancel/queue/retry, real copy/native/
finalization kill and manifest recovery passed; API 35/36 platform timeouts passed.
Earlier picker harness failures and the root-Markdown reuse-guard correction are
recorded in the A05 packet. Guard regression checks excluded documentation while
rejecting compiled-source changes. The initial API-33 missing control receipt did
not reproduce; original Activity diagnostics were not collected and cause remains
unknown. A07 physical/16-KiB/resource gates remain unrun.

No core/native Rust, scoring, fixtures or private audio changes. Owner cleanup
work remains separate. Local ignored logs are not Git history or backups.

## A04 accepted — 2026-10-08

Replacement source `cf542ab` passed [Android CI 37765269431](https://github.com/spideyonmoon/audio-forensic-rust/actions/runs/37765269431)
(both ABI builds and all seven API 30–36 runtime jobs) and
[core CI 37765269643](https://github.com/spideyonmoon/audio-forensic-rust/actions/runs/37765269643).
API-30 real-picker coverage is the manual Hot 11S test; its automated pointer
interaction was not run in the successful replacement CI. Owner approved A04
closure without waiting for the corrected APK's two-file physical ALAC retest;
that retest was **not run**, and is not counted as a pass. Generated/local exact
ALAC validation and CI passed. No jobs remain. Next A05; A07 retains full physical
memory/background/16-KiB acceptance. Earlier pending records below are historical.

Owner-confirmed **Infinix Hot 11S / Android 11** screenshots and manual results
pass single/multiple/folder-subset selection and persisted access for APK source
`a1b3c42`. This closes the platform picker-interaction gap below. Full-build CI
now explicitly records API-30 manual picker coverage instead of repeating the
broken pointer helper; native/provider/staging assertions still run on API 30.
Owner-supplied ALAC originals exposed a desktop-core preflight compatibility
limit. Narrow all-zero `sdtp` and cookie-precision corrections pass eight ALAC
tests, focused Clippy and exact native/FFmpeg one-second PCM comparison on both
unchanged originals. See [ALAC validation](ALAC_VALIDATION.md) and
[A04 packet](../../task-results/A04.md). No private recordings or screenshots are
in source history. Replacement APK CI passed; owner deferred its physical ALAC
retest from A04 closure.
This manual input check does not replace A07 memory/background/16-KiB acceptance.

Shared SAF workspace/staging/probing is implemented. Source `a1b3c42` passed
both ABI links/APK/lint/packaging checks and generated native/input-provider
controls on API 30–36 in CI 37740958937. Real DocumentsUI single/multiple/folder
persisted-grant checks passed API 31–36. That run concluded failure because the
API-30 automation did not activate its file; API-30 picker acceptance remains
unverified. Local Kotlin/shared lint passed, 0 errors and 3 warnings. Exact
700 MiB/overflow, cancellation/disk pressure, encoded hashes, shared leases,
codec routing and selection-change cleanup passed on every supported API.
[A04 record](../../task-results/A04.md) retains exact cases, repaired initial
failures and the isolated API-30 retries. Runtime APK reuse validates compiled
source and the verified APK hash, with changed compiled source correctly refused.
No core/private input changes. A05 jobs/history and A07 physical acceptance remain.


## A03 accepted — 2026-10-08

Final source ce037d3 passed [CI 37673381546](https://github.com/spideyonmoon/audio-forensic-rust/actions/runs/37673381546):
both ABI NDK/APK/lint/alignment/signature checks and all seven API 30–36 emulator
jobs, including packaged generated WAV/FLAC/ALAC exact PCM/product/PNG, complete
metadata, exact integer/null and cancellation checks. A03 is DONE; A04 is next.
Eight focused host tests and Clippy passed. No core/private-audio changes or
repeated audits were required for closure. Earlier pending statements below are
historical. Physical ARM64/16 KiB runtime, SAF/jobs/UI and release gates remain.


## A03 adapter — 2026-10-08, Android acceptance pending

Implemented only in `apps/alfred/`: one serial worker and nonreused handle
registry, JNI byte-array controls, exact Kotlin parser/control executor, complete
probe/product/Spectrogram payload persistence, saved comparison/rendering and
checked resource admission. The core source, Cargo files, schemas and generated
fixtures are unchanged. Source: [adapter guide](../../apps/alfred/NATIVE_ADAPTER.md).

Windows Rust 1.85 final focused suite: **8 passed**, zero failed/ignored. The
two registry tests deterministically exercise source Drop blocked after close,
busy admission until release, stale tokens, repeated poll/cancel/close, injected
spawn failure, panic recovery and token exhaustion. Six integration tests cover
WAV/FLAC/ALAC exact existing PCM hashes, product and all PNG preset paths,
saved comparison/rendering, AAC-in-M4A marker/DSD unsupported behavior, complete
120,040-byte retained metadata text via a descriptor, u64-max/null, malformed
and future versions, reservation limits, changed source, cleanup and PNG failure
preserving an analyzed report. No oracle was regenerated. Kotlin offline
compilation, all-target Clippy with warnings denied, formatting, Python smoke
syntax and frozen-tree/whitespace checks passed.

Product full-scope admission conservatively assumes the 180-second capture at
the probed native rate even when a header claims a shorter duration. A declaration
does not enforce an allocation bound if actual PCM is longer. High-rate full
requests therefore require an explicit prefix; no silent shortening occurs.
The rate/FFT/frame bound and 512 MiB additional margin are engineering limits,
not measured physical RSS. Metadata-only probe remains available. A07 measures
the admitted cases. Shared storage reservation, snapshot/reader lease deletion,
orphan cleanup and retained input-item mapping are explicit A04/A05 seams.

Initial CI 37643823844 passed Linux host tests, then was intentionally cancelled
for the final probe-reason/future-version corrections. Subsequent run 37671795459 passed both NDK ABI links, APK build/lint and native
alignment/signature checks. API 30/31 loaded JNI but the generated-input harness
failed because debug fixtures were missing. Explicit producer dependencies and
an APK fixture-byte check now guard this; local asset merge/lint-model generation
passed with all three files. Final source ce037d3 is under CI 37673381546;
packaged generated-input runtime acceptance remains pending. A04 is independently
eligible after A02.
Exact commands, initial failures and local evidence: [A03](../../task-results/A03.md).
Physical phone, 16 KiB runtime, SAF/background/history and release acceptance
remain later gates; no private audio was uploaded.

## Android 11 compatibility amendment — 2026-10-07

Owner expanded planned support to Android 11–16/API 30–36 and requested GitHub
Actions builds/emulator checks. Gradle/native minimums now use API 30, with
compile/target 36; this supersedes A01/A02 API-34 baseline records below without
rewriting their historical evidence. Shared resource budgets are unchanged.
ARM64 remains primary; a separate x86_64 CI artifact supports emulator smoke.
Physical Hot 11S/Android 11 checks are explicitly distinct from emulator evidence.
Results: [compatibility packet](../../task-results/ANDROID-11-CI.md).

Both API-30 ABI builds/lint and native packaging/signature verification passed
in GitHub Actions run 37635443625. All seven API 30–36 x86_64 emulator install/workspace/JNI
smoke jobs passed; the complete run succeeded. Initial SDK setup,
missing Linux metadata/tool checksum pins and a smoke-test scrolling defect are
recorded in the packet; strict checksum validation was retained. No physical
Hot 11S/Redmi, 16 KiB load, analysis, persistent SAF or background-job acceptance.

## A01 contract review — 2026-10-07

[ANDROID_CONTRACT.md](../../ANDROID_CONTRACT.md) is an implementation contract,
not evidence of working Android code. [A01 results](../../task-results/A01.md)
record the exact finite documentation checks. Existing P09 acceptance and P06/P07
numerical evidence remain unchanged.

Reviewed actual `AnalysisJob` admission/drop/poll semantics; product and
spectrogram public source APIs; metadata source probing; ALAC catch_unwind;
reference capture/FFT/HF scratch bounds; native exact-integer JSON and separate
product/artifact/comparison versions. Product collection needs an app-owned
worker because AnalysisJob returns only a measurement report. Root unsafe-code
prohibition remains intact. No source/dependency/schema/fixture changes.

The contract's acceptance ledger assigns linking/race/PCM/integer checks to A03,
SAF/700 MiB/leases to A04, lifecycle/storage to A05, feature cases to A06/a/b,
and physical RSS/thermal/screen-off/API/page-size checks to A07. Numerical
reference capture limits and memory reservations are conservative design choices,
not measured phone guarantees; neither staging nor target compilation proves
that the largest P04 profiles fit the device. Record actual downstream outcomes
here, retaining initial failures and the distinction between emulator/physical
evidence. A01 introduces no new corpus/calibration experiment or launch gate.

## A01 review corrections — 2026-10-07

The three findings in [the focused review](../../task-results/A01-review.md)
are corrected at contract level. Probes use cancellable handles and store complete
metadata as a separate payload; Compare negotiates a single scope and requires
same-track assertion; PNG retains the accepted Publication default and presets.
The contract's A03/A04/A06a/A06b rows now contain the corresponding runtime cases.

Affected checks passed: three contract/source/release consistency checks, the
90-second 48/96 kHz example, all three existing PNG presets, 60 local file targets
in six affected documents and whitespace. A02 remains TODO, no app subtree.
Commands/receipt are in task-results/A01.md. Source inspection confirmed MetadataReport
can retain 1 MiB of tag text, P07 compares actual duration/EOF and method/domain
bindings, and the renderer defaults to Publication 2560×1440. These are existing
core contracts; no implementation changed. Targeted documentation checks and
manual lifecycle/scope review validate the correction, not JNI/device behavior.
No full regression, corpus, platform build or phone checks were repeated.

## A02 scaffold and reproducible builds — 2026-10-07

A02 is complete. The five-module Kotlin/Compose workspace is under
`apps/alfred/`, with independent `native/` Cargo workspace and explicit path
dependency to the unchanged core using `default-features = false`. The app opens
to session-only document/folder selection and independently routes Forensics,
Spectrogram and Compare scaffold screens. No SAF persistence, analysis, jobs or
result storage is claimed; those are A03–A06.

Pinned toolchain: Rust 1.85.0, Temurin 17.0.16+8, Gradle 8.13, AGP 8.11.1,
Kotlin/Compose compiler 2.2.21, API 36/build-tools 36.0.0, NDK
30.0.16248370 and arm64-v8a. The native library links against API 34 with
panic-unwind and explicit 16 KiB page-size flags. The complete online build and
lint passed; a clean offline repeat also passed. The debug APK is at
`apps/alfred/app/build/outputs/apk/debug/app-debug.apk`.

Verifier evidence passed for every packaged arm64 native library, ELF LOAD
alignment, declared native dependencies, APK ZIP alignment and debug signature.
The APK contains the app bridge and Compose's `libandroidx.graphics.path.so`,
both inspected. Root CLI and no-CLI builds passed independently. Receipts and
exact commands are in [A02 results](../../task-results/A02.md),
`apps/alfred/BUILDING.md` and ignored `target/a02/` logs.

Not run: physical or emulator runtime, 16 KiB load test, SAF/grants, native
handles, analysis, jobs/lifecycle, storage, feature payloads or release signing.
Android build success is not device acceptance.
