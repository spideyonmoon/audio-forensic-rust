# Caller-provided source and batch resilience — 2026-10-03

Engine **0.18.1**, schema `0.18.0`, policy `observations-only-v18`. This continuation
fixes source I/O handling, failed-report precision and CLI input discovery. It
adds no decoder dependency or detector/accuracy policy. Ancestry stays
`INCONCLUSIVE`, evidence index stays null, and the existing JSON Schema is unchanged.

## Observed defects and fixes

The first generated source run passed six of eight integration tests and exposed
two production defects. A transient `Read` error of kind `Interrupted` could fail
an otherwise valid input. An I/O permission error during probing could be replaced
by Symphonia's generic unsupported-format result. A later float failure control
showed `stream.integer_pcm` incorrectly true before a successful scan finalized.

`src/source.rs` wraps each caller-provided source after worker acquisition. It
retries interrupted reads while checking the same cancellation token, elapsed
start time and deadline. It records the first terminal source I/O/control error;
probe/scan/rewind boundaries return that error even if the decoder replaces or
swallows it. Locks are taken only for storing/checking a fault, not while calling
source I/O. The wrapper retains one shared fault record and no audio buffer.
Underlying `Ok(0)` still represents normal EOF; an underlying `UnexpectedEof`
error is a source failure, not permission to treat interrupted input as complete.

Native float PCM codecs now initialize `stream.integer_pcm` correctly before
decoding, so failed reports preserve known precision. Successful reports still
derive/check actual decoded representation and exact interleaved PCM hashes.
Neither fix changes detector arithmetic or the wire field types.

CLI inspection also found that directory enumeration/entry errors propagated
out of `run` before JSON output, discarding later valid inputs. Directory
enumeration, entry and audio-file metadata errors now become per-input failures.
An empty/non-audio directory has an explicit failed report. Other available
inputs still run in input order, with each directory's immediate WAV/FLAC entries
sorted. Scanning stays nonrecursive; explicit duplicate arguments remain separate
results. Broken/disappearing audio entries produce diagnostics. Non-audio children
are skipped without metadata inspection.

`AnalysisReport::input_failure(source, message)` creates a current-version failed
report with `invalid_input` diagnostics and null/empty measurement fields. It is
used by path-open failures and CLI discovery, and is available to library callers
whose own input enumeration/opening fails. A known discovery error remains a
failure; a later batch cancellation still determines the CLI's exit code 130.
Output/setup errors retain exit code 2; any per-input unsuccessful result gives
exit 1 while keeping a complete JSON batch.

## Checks and limits

Private receipts/logs are in
`corpus/local/results/source-contract-v18-20261003/`. All source adapters and
audio controls are generated. No user recording or pinned Python reference was
changed or analyzed in this continuation. Source/test fingerprints were frozen
before full regression; no Rust file is edited during that build.

| Check | Actual result |
| --- | --- |
| Focused caller-source integration | 10 tests passed, including exact generated PCM and worker-release checks |
| CLI discovery controls | All 3 units passed: enumeration, entry/metadata, empty directory |
| CLI process controls | Both integrations passed: mixed ordering/results and complete batches for deadlines/invalid intervals/tracks |
| Fresh source reports | All 60 passed the existing offline schema and interval/channel checks: 37 analyzed, 17 failed, 5 cancelled, 1 unsupported |
| Copied CLI mixed-input smoke | Missing path, empty directory and valid WAV retained; exit 1; exact PCM and all 3 report contracts passed |
| Deadline during interrupted read | Passed in the frozen full-regression library units |
| Full Rust 1.85 regression | All 147 debug tests passed, zero failed/ignored, with source/test fingerprints unchanged |
| All-target MSRV Clippy / stable formatting / no-CLI library compilation | All passed on the frozen patch |

The ten source controls cover twelve short-read/length combinations, source
errors in preflight/probe/first/second scan, one-time and repeated interruptions,
seek errors, underreported length, full and prefix mutations, truncation between
passes, cancellation, no-read unsupported/invalid/pre-cancelled inputs and three
failed float cases. Each failure/recovery control explicitly runs another
analysis with an exact expected hash to check permit release. The normal integer
control has 4,096 generated mono s16 frames at 8 kHz. Prefix comparisons analyze
800 frames: mutations inside that prefix fail; changes solely in the unmeasured
tail are outside its contract. No whole-file integrity is claimed for a prefix.

Unknown `byte_len` and an overestimate can succeed when container metadata and
decoded PCM are valid. An underestimate enclosing required metadata fails.
`byte_len` is an ancillary bound, not the authoritative PCM duration; decoded
coverage and declared audio frames are compared separately. This is not a promise
to validate all bytes in a container, detect arbitrary metadata-only changes or
contain a malicious caller violating the Read/Seek contract. Native WAV/FLAC must
remain stable through preflight and both passes.

Cancellation and deadlines remain cooperative. A blocking caller read/seek or
decoder call cannot be interrupted by the wrapper. The new deadline control
lets one interrupted read block past its deadline and verifies that no second
underlying read begins. Caller code can panic; ordinary unwinding releases the
worker, but the API does not convert arbitrary caller panics into reports.
No operating-system-wide worker limit or hard RSS sandbox is introduced.

Initial failures are retained: `initial-test.log` (two I/O defects),
`float-precision-initial.log` (wrong failed float precision), and
`cli-source-test.log` (a new test tried to move a non-Copy status). The test compile
error was corrected with a clone; the production failures required the fixes
above. No reference output or expected PCM hash was regenerated to hide them.

## Deterministic container mutation campaign

`scripts/check_container_mutations.py` generated **788** native WAV/FLAC inputs:
two baseline 64-frame 8 kHz mono controls, all single-bit flips in the 44-byte
WAV header and 42-byte FLAC marker/STREAMINFO region, plus header/payload
truncations. FFmpeg generated only the private baseline FLAC from deterministic
PCM; the Rust core requires no external process. The copied engine 0.18.1 CLI
processed the whole directory in one isolated process with a one-second
cooperative per-file deadline and a 120-second process timeout.

All 788 outputs passed the existing report schema and consistency checks:
**102 analyzed, 161 unsupported, 525 failed**. Both unmodified baselines had
exact independently computed MSB-aligned s32le PCM hashes. Unsuccessful cases
retained diagnostics and no successful coverage/channel/detector measurements.
The process exited 1 as expected; no crash, malformed JSON, timeout or lost
result occurred. Generated input and copied/original binary fingerprints stayed
unchanged. The batch took about four seconds on this host while regression tests
ran; this is an uncontrolled execution observation, not a performance benchmark.

Evidence is ignored under `corpus/local/results/container-mutations-v18-20261003/`.
Some header changes remain legal, so accepting a mutated input is not itself a
failure. This deterministic sweep is not complete fuzzing, worst-case resource
analysis or accuracy validation, and yielded no additional production change.

## Reproduce

Use normal Cargo on another host. On this Windows workspace, use the existing
process-local `.tools/msrv-gcc-lld.cmd` linker override and array wrapper arguments:

```powershell
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $PWD '.tools/msrv-gcc-lld.cmd'
& ./scripts/cargo.ps1 @('+1.85.0','test','--locked','--offline','--test','source_contract','--test','cli_contract','--bin','audio-forensic','--target-dir','target/msrv-desktop-v18','--','--test-threads=1')
```

Optional `SOURCE_CONTRACT_OUTPUT_DIR` must name an existing private directory
under `corpus/local`; each report uses create-new output. Unset it for ordinary
reruns to preserve receipts. The mutation campaign needs the optional schema
validator environment described in `schemas/README.md` and a local FFmpeg:

```powershell
& ./.tools/report-schema-venv/Scripts/python.exe scripts/check_container_mutations.py --binary path/to/audio-forensic.exe --output corpus/local/results/new-mutation-batch
```

Schema/policy stability, grouped detector accuracy and complete format support
remain separate gates. Android work stays deferred. No remote CI, commit, push
or external backup occurred; local source and ignored evidence are distinct.

## Production-only source snapshot

A local 34-file snapshot contains both Cargo manifests, the MIT license, all
production Rust modules, the versioned JSON Schema and standalone build notes.
It excludes recordings, reports, tests, fixtures, development scripts,
toolchains and Git history. Rust 1.85 offline CLI build, no-CLI library check and
production Clippy with warnings denied all passed from this snapshot. Its CLI
also passed the generated exact-PCM and report-schema smoke. Copied production
files match the frozen workspace byte for byte.

The archive is
`corpus/local/results/source-package-v18-1-20261003/audio-forensic-0.18.1-source.zip`
(103,714 bytes), SHA-256
`0f4c73d1ffac503d950c5537dc9c2c22baf7255d52611afb7ae14439d5b21ac9`.
Its `manifest.json` fingerprints all included files; ZIP integrity passed.
This is a local source snapshot, not a backup of excluded evidence, Git history
or an external publication. Another machine needs Rust and dependency setup
before offline compilation, as the packaged build notes explain. No snapshot
build, campaign or full-regression job remains running. Release tests were not
rerun for 0.18.1; the actual final regression is the ordinary Rust 1.85 debug run.
Earlier v0.18 release evidence remains historical. No new Android compilation,
NDK link/device run, Linux execution or private audio analysis was performed.

The full test record is `source-contract-v18-20261003/full-msrv-test-summary.json`:
147 tests, zero failed/ignored. `preservation.json` verifies the staged index,
clean pinned reference and 258 earlier unrelated fingerprints. Exactly the
intended six existing source/manifest files changed; Cargo manifest/lock changes
are only the root engine version, with all dependencies/features unchanged.
The new source module, tests and mutation helper are local files. The schema
artifact still matches its exporter exactly. Whitespace and Python syntax passed.

Next concrete core job: characterize unknown-length WAV/RF64 and remaining PCM
header applicability using generated controls and independent exact PCM before
extending the support matrix. Keep Android deferred and source-history claims
uncalibrated.
