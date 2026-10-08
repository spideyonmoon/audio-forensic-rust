# Android integration validation

## A04 shared input — 2026-10-08, CI pending

Shared SAF workspace/staging/probing and generated provider/UI controls are implemented. Offline Kotlin compilation, existing pinned dependency configuration locks, Python syntax and whitespace passed. Required ABI APK/lint and API 30–36 runtime tests have not yet run; do not claim runtime acceptance. [A04 record](../../task-results/A04.md) includes exact cases, commands and initial failures. Core/private inputs are unchanged. A05 jobs/history and A07 physical acceptance remain later gates.


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
