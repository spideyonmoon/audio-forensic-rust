# WAV applicability and exact PCM — 2026-10-03

Engine **0.18.2**, schema `0.18.0`, policy `observations-only-v18`. This closes
the WAV characterization follow-up from `SOURCE_API_VALIDATION.md`. It changes
input applicability and validation, with no new dependency or detector policy.
Ancestry remains `INCONCLUSIVE` and evidence index remains null.

## Characterization and decisions

`scripts/check_wav_formats.py` builds deterministic samples before packing WAV
headers. Integer expectations use MSB-aligned signed s32le; float expectations
use f64le. The matrix covers mono/stereo integer 8/16/24/32, float 32/64, ordinary
and extensible headers, reduced valid-bit precision, masks, GUIDs, size sentinels,
RF64/RIFX, malformed fields, odd ancillary chunks, fact chunks and truncation.
All inputs are generated; no private recording or reference output is changed.

The original 0.18.1 run retained **128 schema-valid reports for 64 cases**:
full: 50 analyzed / 9 failed / 5 unsupported; prefix: 52 / 7 / 5. The receipt
`initial/summary.json` contains every mismatch against the proposed stricter
contract. Several are diagnostic/status refinements, not separate decoder bugs.
The meaningful findings were:

- `0xffffffff` data length is read as a literal huge frame count. Full analysis
  fails a header-length comparison; a short prefix can succeed with a fabricated
  declared duration. Data-length sentinels now return explicit `unsupported`
  before probing, in both modes. An unknown RIFF size with a concrete data size
  remains supported. An unknown `MediaSource::byte_len` is also supported when
  the actual WAV data length is concrete; these are different conditions.
- Extensible LFE/other masks, inconsistent masks and ambisonic GUIDs can reach
  ordinary mono/stereo measurements. The locked decoder repairs mask counts.
  The preflight now checks the complete standard integer/float subtype GUID and
  allows only mask 0/1/4 for mono and 0/3 for stereo. Other masks/layouts are
  unsupported, without declaring them universally malformed. Mask zero retains
  positional mono/stereo analysis under this limited support contract.
- Extensible samples can contain nonzero bits below their declared precision.
  Both scans now reject such samples within the measured interval; they do not
  mask, round or discard those bits. Tail violations outside a requested prefix
  are outside its contract. The check applies to native integer decoded PCM.
- Byte rate must match sample rate times frame alignment without overflow.
  Extensible valid-bit count must be positive and fit the container; float valid
  width must equal its container. Extension lengths must fit the fmt chunk, and
  the supported extension is 22 bytes. Subtype-specific encoded widths are checked
  before data parsing. RF64/BW64 and big-endian RIFX have explicit unsupported
  diagnostics; no claim of decoding those formats is introduced.

These contracts follow the field semantics in Microsoft's
[WAVEFORMATEXTENSIBLE documentation](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ksmedia/ns-ksmedia-waveformatextensible)
and [WAVEFORMATEX documentation](https://learn.microsoft.com/en-us/windows/win32/api/mmreg/ns-mmreg-waveformatex).
Valid integer bits occupy the high portion of their byte-aligned container,
with unused low bits zero. The narrow mask/extension support limits are project
decisions, not claims that all other combinations violate the format.

## Independent decoder disagreement

The generated expectations matched the original Rust PCM on every intended
supported case. Automatic FFmpeg decoding matched 44 of 46 independently checked
cases. For the two standard extensible **24 valid bits in 32-bit containers**,
this host's FFmpeg selects `pcm_f24le`, as `ffprobe` confirms, and produces different
samples. Explicitly selecting the header-declared integer decoder `pcm_s32le`
matches the originally computed PCM exactly. The final helper records automatic
and explicit-decoder commands, hashes and outcomes separately for these two cases.
It neither rewrites the initial disagreement nor substitutes expected Rust output.
The receipt records the exact FFmpeg version. Ordinary tests need no FFmpeg.

## Checks and evidence

Evidence is local and ignored under
`corpus/local/results/wav-contract-v18-2-20261003/`. `before-state.json` fingerprints
192 pre-existing source/test/script/schema/document/recording files, the staged
index/diff and the clean pinned reference. `initial/` preserves the original
matrix reports, commands, expectations and helper source. The new ordinary Rust
test `tests/wav_formats.rs` generates its inputs in memory and uses seven-byte
short reads with unknown source length. It covers full/prefix exact PCM, float
signed zero, precision padding, channel basis and explicit failures.

The focused Rust 1.85 retry passed **23 tests**: six WAV-format, ten source and
seven container controls. The initial focused link attempt was blocked by a
permission denial executing the local LLVM linker; its log remains retained.
The final 64-case matrix passed all **128 schema-valid reports**: full analysis
42 analyzed / 9 failed / 13 unsupported, prefix 43 / 8 / 13. All 46 independent
PCM cases matched with the declared decoder; the two automatic-detection
disagreements above remain recorded. Inputs and both binaries stayed unchanged.

The resumed session found the prior full MSRV debug run had exited 1073807364
mid-suite. Its log contains no completed full-suite result; the reason for that
termination is unconfirmed. An earlier release build likewise had no completed
result. The new sandboxed release retry failed while executing the local LLVM
linker (`Permission denied`); the subsequent authorized retry succeeded outside
the sandbox. No production Rust or ordinary test source was edited during these
builds. All 82 files in `frozen-source.json` matched before and after the run.
MSRV all-target Clippy with warnings denied, stable formatting, schema-exporter
drift and Python syntax checks passed. The complete **Rust 1.98.1 release
regression passed all 153 tests**, zero failed/ignored across 26 test invocations
including empty doc-tests, with the existing LLVM workaround and locked offline
dependencies. `full-release-test-summary.json` records the actual count. No
full-suite claim is made for the interrupted Rust 1.85 debug attempt.

`preservation.json` confirms the 82 frozen source/test/fixture/manifest files,
185 unrelated prior files (including 48 recording/note files), staged index/diff
and clean reference at `c6ecce2296256b516709d87088896d1be913908c`. The seven intended
existing changes are the two production modules, two Cargo files and three root
documents. New tests/helpers/validation notes are local files. Compared with the
0.18.1 production snapshot, Cargo files differ only in the root engine version;
dependencies and features are unchanged. Whitespace checks passed. No task jobs
remain running.

## Expanded precision and amplitude boundaries

`scripts/check_pcm_boundaries.py` adds generated mono/stereo extensible WAVs at
8 kHz for every valid precision from 1 through each supported 8/16/24/32-bit
integer container width. Float 32/64 controls cover signed zero, smallest positive
and negative subnormal/normal values, amplitudes above unity and the core's
inclusive absolute-amplitude limit of 16. NaN, infinities and the immediately
adjacent values outside ±16 are injected at the first frame and at frame 900.
The 400-frame prefix excludes the latter errors; full analysis includes them.
Invalid measured floats must fail without coverage or detector measurements.

Expectations are packed independently before Rust runs. The separate FFmpeg
comparison explicitly selects each header-declared integer/float codec, preserving
the known automatic-detection disagreement rather than silently treating it as
fixed. The first run passed 220 cases / 440 schema-valid reports and all 180
independent PCM comparisons. Its results are retained in `precision-boundaries/`.
The final run in `precision-boundaries-final/` improves the integer sample pattern
to visit both extrema in mono as well as stereo and snapshots the generator and
schema fingerprints. All **220 cases / 440 reports / 180 independent PCM checks
passed** again: full analysis 180 analyzed / 40 failed, prefix 200 / 20. Both
binary copies and all inputs remained unchanged. These are two runs of related
generated controls, not 440 independent sources. No production behavior changed.

## Mutation regression and production-only snapshot

The unchanged 788-case WAV/FLAC mutation helper passed against the freshly built
production-only 0.18.2 CLI: **57 analyzed, 142 unsupported, 589 failed**, with all
reports schema-valid and both generated baselines exact. The expected batch exit
was 1; no crash, lost report or timeout occurred. Compared with the 0.18.1 batch,
45 formerly analyzed and 19 formerly unsupported inputs now fail the byte-rate
geometry check. These are all 64 status changes; `status-changes.json` retains
their identities and diagnostics. The campaign is bounded deterministic
characterization, not exhaustive fuzzing, accuracy or a hard memory guarantee.

A new 34-file production-only snapshot under
`corpus/local/results/source-package-v18-2-20261003/` passed Rust 1.85 offline
CLI build, no-CLI library compilation and production Clippy. Its CLI independently
passed the same 128-report WAV matrix and 46 exact PCM cases. Copied source and
schema hashes match the frozen workspace. The ZIP's entries, hashes and integrity
passed. The archive `audio-forensic-0.18.2-source.zip` is 104,670 bytes; SHA-256
`9413059cb253bdc5f47b90233a0dff8434ef4d3b86b4d94861cbb9d9a65882bc`.
It contains production source, Cargo manifests, license, schema and standalone
build notes; it excludes recordings, reports, fixtures, tests, toolchains and
Git history. The local receipt helper initially resolved the repository root one
directory too high and failed before creating the snapshot; that path calculation
was corrected. This was a harness failure, not a failed Rust build.

No publication, upload, commit, push or external backup has been made. Android
remains deferred. No new Android compilation, NDK link, device test, remote CI
or private-recording analysis was performed in this continuation.

## Reproduce

Use ordinary Cargo elsewhere; on this Windows host the existing MSRV GCC/LLVM
shim is needed for linking:

```powershell
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $PWD '.tools/msrv-gcc-lld.cmd'
& ./scripts/cargo.ps1 @('+1.85.0','test','--locked','--offline','--test','wav_formats','--target-dir','target/msrv-desktop-v18')
& ./.tools/report-schema-venv/Scripts/python.exe scripts/check_wav_formats.py --binary path/to/audio-forensic.exe --output corpus/local/results/new-wav-matrix
& ./.tools/report-schema-venv/Scripts/python.exe scripts/check_pcm_boundaries.py --binary path/to/audio-forensic.exe --output corpus/local/results/new-pcm-boundaries
```

The optional helper requires the existing offline schema-validator environment
and local FFmpeg. Each invocation requires a new private output directory.
Full-file analysis covers the first declared data chunk; trailing ancillary chunks
and concatenated/multiple audio chunks are not claimed as validated whole-container
contents. `fact` sample counts do not override the concrete PCM data length.
No hard process-memory sandbox or noncooperative I/O interruption is promised.

Next core audit: generated FLAC controls for unknown total samples, absent or
wrong MD5, and damaged-frame continuity, with exact independent PCM and explicit
full/prefix coverage. Existing checksum/truncation tests in `tests/parity.rs`
and the locked decoder should be inspected before changing applicability.
