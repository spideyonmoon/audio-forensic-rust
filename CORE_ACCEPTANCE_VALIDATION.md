# Version 0.18 core acceptance work — 2026-10-02

Schema `0.18.0`, policy `observations-only-v18`. This audits engineering gates;
it does not establish detector accuracy, stable API or Android phone readiness.

## Process budget and cooperative control

`src/worker.rs` permits one active core analysis per process. Acquisition happens
after option/seekability checks and before container probing, decoder construction
or DSP allocation. Waiting callers check cancellation and the same elapsed
deadline every 5 ms; waiting time counts toward that deadline. RAII releases the
permit on success, structured failure or ordinary panic unwinding. There is no
internal task queue, thread pool, fairness guarantee or operating-system-wide
limit. Caller-owned sources, waiting threads and completed reports remain outside
the budget. A reentrant source must not synchronously start a second analysis.

The integration control holds the first analysis inside a source read, cancels
and expires queued calls with **zero source reads**, then releases the owner and
checks a later successful call. An isolated budget unit checks permit release
and repeated waiting checks. These passed before the container guard addition.
Blocking caller-provided reads/seeks are still cooperative, not interruptible;
neither cancellation nor the deadline promises to interrupt such I/O.

## Container and decoder limits

Inspection of the locked Symphonia 0.5.5 source showed that FLAC picture/comment
fields and RIFF INFO allocate before reading their payload. FLAC's format reader
does not receive `MetadataOptions`. Merely setting those options would therefore
not establish these limits. WAV data without a usable format/block alignment can
also reach a division by zero before decoder construction.

`src/container.rs` now checks native `fLaC` or `RIFF/WAVE` before the probe:

- At most 1024 leading FLAC metadata blocks or WAV chunks before the first data
  chunk, and at most 16 MiB cumulative encoded ancillary bytes including headers.
  Padding/unknown ancillary data counts too. WAV audio bytes are exempt.
- At most 4096 aggregate comment, seek-table, cue/index or WAV INFO records.
- Individual comment/vendor/MIME/description/INFO fields at most 1 MiB; native
  artwork data at most 8 MiB. Every embedded length must fit its enclosing block.
  Nested base64 artwork is checked too, matching the dependency's Unicode
  lowercase key recognition. Oversized malformed lengths fail before allocation.
- Exactly one leading 34-byte FLAC STREAMINFO. Seek-table lengths, application
  minimum size, cuesheet fields and nested artwork geometry are checked.
- WAV requires format before data, mono/stereo integer/float PCM, nonzero block
  alignment matching channel count and encoded width, and complete PCM frames.
  Standard unknown-length RIFF/data sentinels remain accepted by preflight;
  decoding still decides whether actual coverage matches the declared length.

Preflight does not retain or rewrite audio. It restores source position zero.
It uses a bounded temporary block buffer for comments/pictures/cues (at most
16 MiB) and a bounded nested base64 buffer; both drop before decoder probing.
It checks control between blocks and INFO records. Stable seekable input is
required across preflight and both analysis passes; changing source contents are
not an allocation sandbox. The two-pass PCM hash check detects decoded changes.

Unsupported limits return `unsupported`; malformed lengths/geometry and I/O
errors return `failed` with structured diagnostics. Leading ID3 and other
non-native wrappers, compressed WAV, duplicate format chunks, oversized artwork/
metadata and malformed auxiliary fields are outside this support matrix. They
are not silently skipped as trustworthy input. Trailing ancillary WAV chunks
after the first data chunk are not inspected or used by the current reader.

Both decoded block frame count **and capacity** must be <=1,048,576 before the
core's integer/f64 sample-copy allocation. The source stream's default ring is
64 KiB. Decoder/container internal allocations occur before that decoded-block
check. These bounds, and duration-independent DSP storage, are not a hard process
RSS cap or proof against every defect in a third-party parser. No waveform or
duration-derived allocation was added.

Generated regression controls check hostile length/count fields, nested and
Unicode-key pictures, ordinary artwork/tag PCM identity, aggregate byte/block/
record limits, WAV INFO, missing format, extreme channels, zero alignment and
partial frames. No giant allocation is required to represent the hostile headers.

## Acceptance matrix

| Gate | Current evidence / outstanding work |
| --- | --- |
| Exact decode / DSP arithmetic | v0.18: 130 release tests and 69 independent cases (137 native analyses); v0.18.1: all 147 final MSRV debug tests passed |
| Channels / precision / intervals / nulls | Explicit reports and full generated regression passed |
| Machine-readable report contract | 2026-10-03 schema covers 47 types; all 125 fresh/saved reports and 53 contract controls passed; see `REPORT_SCHEMA_VALIDATION.md` |
| Caller source / batch resilience | Engine 0.18.1 fixes interrupted reads, probe-masked I/O failures, failed float precision and directory batch failures; 15 focused tests, 60 report contracts, 788 mutation outcomes and the full 147-test regression passed |
| Worker budget / waiting control | Unit and queued-worker integration passed in the final release suite |
| Corrupt input / allocation limits | All seven hostile/valid-metadata integration controls and prior core/parity checks passed |
| Runtime / bounded memory | All nine v0.18 generated duration/rate controls passed exact PCM; resource figures below; no on-device bound established |
| Native Android portability | v0.18 no-CLI ARM64 target checks passed on Rust 1.98.1 and 1.85.0; NDK link and device tests not run |
| Minimum compiler | v0.18.1 Rust 1.85.0: all 147 debug tests, all-target Clippy and source-only/no-CLI desktop checks passed; prior v0.18 no-CLI ARM64 compilation passed |
| Classification / source profiles | Requires independently characterized recording groups, development/validation split and frozen policy; no calibrated score |
| Stable API / complete support | Schema remains under development; source profiles, structural MQA confirmation, validated frequency mirroring and aggregation remain absent |
| Publication / remote CI | No new commit, push, external backup or remote CI run; local tests/evidence are excluded from source-only publication |

No Android SDK was found at the standard local path during this audit, and
ANDROID_HOME/ANDROID_NDK_HOME/ANDROID_NDK_ROOT were unset. This is not an exhaustive
search of every disk. Rust target compilation cannot establish linking, APK
behavior, device memory, lifecycle or thermal performance.

## Execution record

The preflight implementation and seven integration controls are saved. First
Clippy found unused Read/Seek imports, which were removed. A focused LLVM-linked
run passed 30 units, six initial container controls, 13 core tests, two parity
tests and the worker integration. The subsequent final code adds missing-format,
zero-alignment, partial-frame and extensible companded-subtype guards; the full
regression subsequently passed as recorded below. Final formatting, all-target Clippy, Python syntax and
no-CLI ARM64 compilation passed after those changes.

All 26 PCM relationship / 16 preceding-energy / 27 spectral-lag generated cases
passed in separate `*-regression-v18` result directories. Original immutable
expectations (including separate f32 FFT supplements) were copied before the
checks; generators verified existing audio without changing it. Exact PCM,
counts/status/nulls and numerical errors remained identical to the previous
records. These runs preceded the final extensible subtype restriction, which
changes applicability for compressed WAV and no measurement arithmetic.
All nine resource controls passed exact PCM, including 10/120/600 seconds at
48 kHz, 10 seconds at 8/44.1/96/192/384 kHz and 120 seconds at 384 kHz. Results
are in `corpus/local/results/core-resources-v18/`; generator is
`scripts/check_core_resources.py`. A copied release executable avoids Windows
file locking during concurrent builds; its version/hash/time and scope are in
`corpus/local/results/core-acceptance-v18/binary-evidence.json`. Compilation and
private-prefix validation ran concurrently, so timings are uncontrolled host
observations, not comparative benchmarks. This snapshot predates the final
extensible companded-subtype restriction; that does not change these PCM controls.

| Rate / duration / stereo s16 WAV | Peak Windows working set | Elapsed |
| --- | ---: | ---: |
| 8 kHz / 10 s | 11.94 MiB | 0.27 s |
| 44.1 kHz / 10 s | 20.39 MiB | 0.80 s |
| 48 kHz / 10 s | 21.04 MiB | 0.75 s |
| 48 kHz / 120 s | 24.65 MiB | 4.00 s |
| 48 kHz / 600 s | 26.56 MiB | 15.02 s |
| 96 kHz / 10 s | 27.75 MiB | 0.90 s |
| 192 kHz / 10 s | 43.96 MiB | 1.55 s |
| 384 kHz / 10 s | 76.11 MiB | 3.03 s |
| 384 kHz / 120 s | 79.25 MiB | 30.97 s |

Rate-dependent clip FFT buffers dominate the high-rate increase; segment analysis
retains one reusable two-second sample buffer per channel, not all 36 probe
waveforms. Duration increases the capped band-dynamics history. This sample of
generated s16 WAVs does not establish worst-case memory for FLAC, float blocks,
metadata, all signal patterns or Android. Source-level bounds and the one-active
worker budget are separate from these measured controls.

Private **five-second prefix compatibility** ran on all 39 local FLAC files:
35 analyzed with exact FFmpeg PCM; four unsupported. The validator therefore
exited 1. Two have non-native leading signatures. Two Beatles files have multiple
artwork blocks including 8,590,760-byte blocks, beyond the 8 MiB picture limit,
and aggregate metadata beyond 16 MiB. They now abstain under explicit resource
limits; this is not a corruption or source-history verdict. Original files were
read only. Results remain ignored/private in
`corpus/local/results/private-prefix-compatibility-v18/`; no audio or report was
uploaded. Prefix success does not check full-file length/checksum: the previously
corrupt `13 - Where Is The Love.flac` prefix passes and remains a known full-file
failure. The two M4A files were outside the WAV/FLAC validator's support matrix.

The first full release build finished in 13m20s but failed one of seven container
tests after passing all 30 units and four AAC tests. A newly edited subtype
assertion was compiled against the library snapshot from before that guard.
Changing source/tests during the build made this run inconsistent; do not count
it as final regression. The final source is now frozen and a queued full rebuild
passed: **130 tests, zero failed or ignored**, including all preceding regressions,
the seven container controls and queued-worker test. Cargo reported 20m02s
including a preceding build-lock wait. Original failing log is retained as
`release-tests.txt`; the successful run uses `release-tests-final.txt`, with
aggregated counts in `release-test-summary.json`. No reference output or assertion
was changed to conceal the failure. Explicit release CLI build passed (Cargo
reported 1m16s). Final version/text/JSON/shared-prefix and companded extensible
subtype abstention smokes passed; `cli-smoke-summary.json` records the final
binary hash. No task jobs remain running.

A report-policy audit checked all 117 saved v0.18 actual reports from numerical,
resource and private-prefix runs: 113 analyzed and four unsupported. Every report
had schema/engine `0.18.0`, policy `observations-only-v18`, ancestry `INCONCLUSIVE`,
evidence index null and finite serialized JSON numbers. This checks the saved
representation, not every internal floating-point intermediate or source history.
The aggregate record is `core-acceptance-v18/report-policy-check.json`.

Rust/Cargo 1.98.1 and the process-local LLVM linker override were used; the
manifest's Rust 1.85 minimum was not compiler-tested. The earlier GNU ld exit-204
failure remains unresolved; no global linker setting or production release
profile was changed. No new remote CI, NDK linking, APK/device run, full private
collection integrity run, corpus accuracy evaluation, commit/push or external
backup occurred. The source is still an evolving measurement core. The next
classification gate requires independently characterized source groups and a
frozen evaluation policy, not interpretations of the existing collection's tags.
Source and evidence are local files; existing staged work and private recordings
remain preserved. The Python reference was checked clean at its recorded SHA.

## Rust 1.85 portability continuation — 2026-10-02

The earlier execution record above used Rust/Cargo 1.98.1. This continuation
installed project-local Rust 1.85.0 (`4d91de4e4`, LLVM 19.1.7) and Cargo 1.85.0
(`d73d2caf9`) with the native GNU and Android ARM64 standard libraries. The
default compiler was not changed. No dependency/lockfile, release profile,
engine/schema/policy or production Rust source was changed.

Actual completed checks:

- Desktop `check --locked --offline --all-targets` passed with Rust 1.85.0.
- No-CLI library `check --target aarch64-linux-android` passed with Rust 1.85.0.
- A fresh ignored source-only copy (manifests and `src/`, without local tests,
  fixtures or scripts) passed the ordinary desktop CLI build, production
  Clippy with warnings denied and no-CLI library compilation on Rust 1.85.0.
- All-target MSRV Clippy with warnings denied and stable formatting passed.
- The complete ordinary **debug** MSRV suite passed **130 tests, zero failed or
  ignored**, with source frozen across compilation/execution. The earlier
  130-test release result is separate and was not rerun with this compiler.
- The MSRV source-only CLI version and six generated corpus-intake smokes passed,
  including independently computed exact integer PCM. CI YAML parsing/job
  assertions and Python syntax/unit controls passed; remote CI was not run.
- Saved hash comparisons confirm the production tree and manifests match
  the frozen copy, the pre-existing staged index is unchanged, and the Python
  reference remains clean at its pinned SHA. No task jobs remain running.

Initial desktop/Android attempts failed while linking Windows-host build scripts
with the installed GNU linker (`ld returned 204`), before project compilation.
The Android target still needs host-native proc-macro/build-script executables.
The documented target Rust flags alone do not apply to those host executables
when `--target` is explicit. A project-local ignored linker shim
`.tools/msrv-gcc-lld.cmd` calls the same installed GCC with `-fuse-ld=lld` and a
`-B` path to the stable toolchain's bundled LLVM linker. Setting
`CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER` to that shim covers Windows host links
too. Both checks then passed. No Android linker was substituted, no global
configuration was changed, and the GNU failure's root cause remains undetermined.
A first source-only Clippy invocation also failed because PowerShell consumed
the `--` separator; the array-argument retry passed. Both logs are preserved.

Evidence is ignored/local in `corpus/local/results/msrv-v18/`. The production
copy is `target/source-only-v18-msrv-20261002/`; its ordinary debug CLI is
`target/msrv-source-only-v18/debug/audio-forensic.exe`, SHA-256
`37674db0dbb24d882927f5bfdfea3b851eb7dc1868387af70365fd96935ae216`.
The copy is for validation, not publication or a backup. No new release build,
NDK link, Linux execution, APK/device test, private recording analysis, commit,
push or external backup occurred. The CI extension uses the action's documented
[fixed toolchain revision and explicit matrix input](https://github.com/dtolnay/rust-toolchain).
This tests the manifest's [minimum compiler contract](https://doc.rust-lang.org/cargo/reference/rust-version.html),
not the forensic validity of measurements.

The next corpus step is ready in `scripts/ingest_corpus.py`; generated intake
checks are documented in `CORPUS_INTAKE_VALIDATION.md`. Independent provenance
groups are still absent; existing user-described CD/vinyl inputs were not relabeled.

## Engine 0.18.1 continuation — 2026-10-03

`SOURCE_API_VALIDATION.md` records three demonstrated source/precision defects
and CLI directory failure handling. The engine patch leaves schema/policy and
detector arithmetic unchanged. All-target Rust 1.85 Clippy, no-CLI library
compilation and stable formatting passed on frozen source. All 147 minimum-compiler
debug tests passed, zero failed/ignored, with frozen hashes unchanged. A 34-file
local production-only snapshot separately passed
offline CLI build, no-CLI check, production Clippy, exact generated PCM and schema
smoke without tests/private fixtures. Source archive and receipts remain ignored
local files; no publication, external backup or Android work occurred.
Release regression and Android compilation were not rerun for this patch;
prior v0.18 evidence remains separately recorded. The next generated applicability
audit covers unknown-length WAV/RF64 and remaining PCM header cases before any
support-matrix expansion.
