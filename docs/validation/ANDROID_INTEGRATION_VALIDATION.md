# Android integration validation

## Android 11 compatibility amendment â€” 2026-10-07

Owner expanded planned support to Android 11â€“16/API 30â€“36 and requested GitHub
Actions builds/emulator checks. Gradle/native minimums now use API 30, with
compile/target 36; this supersedes A01/A02 API-34 baseline records below without
rewriting their historical evidence. Shared resource budgets are unchanged.
ARM64 remains primary; a separate x86_64 CI artifact supports emulator smoke.
Physical Hot 11S/Android 11 checks are explicitly distinct from emulator evidence.
Results: [compatibility packet](../../task-results/ANDROID-11-CI.md).

Both API-30 ABI builds/lint and native packaging/signature verification passed
in GitHub Actions run 37635443625. API 30â€“35 x86_64 emulator install/workspace/JNI
smoke passed; API 36 remains pending at this checkpoint. Initial SDK setup,
missing Linux metadata/tool checksum pins and a smoke-test scrolling defect are
recorded in the packet; strict checksum validation was retained. No physical
Hot 11S/Redmi, 16 KiB load, analysis, persistent SAF or background-job acceptance.

## A01 contract review â€” 2026-10-07

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

## A01 review corrections â€” 2026-10-07

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
bindings, and the renderer defaults to Publication 2560Ã—1440. These are existing
core contracts; no implementation changed. Targeted documentation checks and
manual lifecycle/scope review validate the correction, not JNI/device behavior.
No full regression, corpus, platform build or phone checks were repeated.

## A02 scaffold and reproducible builds â€” 2026-10-07

A02 is complete. The five-module Kotlin/Compose workspace is under
`apps/alfred/`, with independent `native/` Cargo workspace and explicit path
dependency to the unchanged core using `default-features = false`. The app opens
to session-only document/folder selection and independently routes Forensics,
Spectrogram and Compare scaffold screens. No SAF persistence, analysis, jobs or
result storage is claimed; those are A03â€“A06.

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
