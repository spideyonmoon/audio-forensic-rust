# Start here: Audio Forensic Rust

Last updated: **2026-10-01**. This file is the portable continuation point for
another session, coding tool or human. Keep it current using `AGENTS.md`.

## Current state and next action

- **Version 0.5.0 spectral structure is implemented and locally validated.**
  Schema: `0.5.0`; policy: `observations-only-v5`.
  The full engine is unfinished.
- Goal: a Rust core that runs entirely on the phone. Build the Android app after
  the core is validated. Continue engineering with generated controls while the
  user collects recordings with independently known processing histories.
- All overall ancestry reports remain `INCONCLUSIVE`; evidence index is `null`.
  Individual observations do not prove codec ancestry, authenticity or MQA.
- **Next: design bounded silence/noise-band measurements** before analog-source
  profiles, with explicit applicability and generated controls.
- AAC v0.4 is committed as `c66d705` (`feat: add bounded AAC lattice observations`).
  The v0.5 source checkpoint is `feat: add streaming spectral structure measurements`
  on local `main`, including the new oracle, scripts, tests and documentation.
  All 53 tests passed in one full release run; the release build and duration
  checks passed. No background jobs or partial detector work remain.
  No private audio was changed. Use `git log` / `git status` to verify current state.

## Read in this order

| File | Purpose |
| --- | --- |
| `AGENTS.md` | Persistent project and documentation instructions |
| `README.md` | Current API/CLI coverage, usage, numerical conventions and limits |
| `PORTING_PLAN.md` | Agreed architecture, policy, Python assessment decisions and acceptance gates |
| `STRUCTURE_VALIDATION.md` | Latest v0.5 scatter/phase measurements, applicability and reference differences |
| `AAC_VALIDATION.md` | v0.4 AAC numerical tests, intentional differences and known trim miss |
| `TRANSFORM_VALIDATION.md` | Latest v0.3 tests, numerical tolerances, encoder experiments and resource results |
| `DETECTOR_VALIDATION.md` | v0.2 segment/MQA validation and important false-hit/miss controls |
| `VALIDATION.md` | Initial core validation, reference baseline and corrupt-input findings |
| `TEST_CORPUS.md` | Recording shopping list, provenance requirements and grouped evaluation |

## Reference and implemented components

Python repository: https://github.com/spideyonmoon/audio-forensic

Local clone: `reference/audio-forensic`, pinned and clean at baseline SHA
`c6ecce2296256b516709d87088896d1be913908c`.
Keep the clone unchanged. No Python refactor is needed before continuing the port.
Python parity checks validate the port's arithmetic, not the forensic accuracy of
the original algorithm. Baseline results: 79/79 DSP checks and 22 accuracy/Vorbis/
MQA unittest methods passed, including the synthetic encoder cases.

| Location | Implemented behavior |
| --- | --- |
| `src/lib.rs` | Seekable-source and path APIs, public types and DSP module |
| `src/model.rs` | Versioned typed reports, coverage, applicability and outcomes |
| `src/decode.rs` | Native Symphonia WAV/FLAC, two-pass decoding, PCM hashes, stream selection, integrity checks, cancellation and deadlines |
| `src/dsp.rs` | Streaming per-channel statistics and STFT, activity/cutoff/entropy/wall measurements, exercised/effective integer precision |
| `src/detectors/segments.rs` | Up to 36 non-overlapping two-second probes per channel, wall patterns and overlapping codec-wall candidates |
| `src/detectors/mqa.rs` | Native integer stereo sync/payload candidates in the first three seconds; no confirmation claim |
| `src/detectors/resampling.rs` | Streaming lower-Nyquist notch/wall/mirror observations; all eligible rate hypotheses retained |
| `src/detectors/mdct.rs` | Windowed MDCT kernel, checked against the direct cosine definition |
| `src/detectors/vorbis.rs` | Per-channel phase/grid search at 44.1/48 kHz, at most twelve captured spans from the first 180 analyzed seconds |
| `src/detectors/aac.rs` | Explicit mono/mid/side AAC lattice, bounded first-pass energy selection and at most sixteen second-pass captured spans per basis |
| `src/detectors/structure.rs` | Native-channel streaming scatter bounds and energy-gated phase entropy; measurements only |
| `src/detectors/mod.rs` | Detector reports, families, thresholds and caveats |
| `src/main.rs` | Desktop text/JSON CLI, directory batches, limits and per-file failures |
| `tests/` | Generated fixtures, reference outputs and Rust regression tests |
| `scripts/` | Toolchain wrapper, reference generators and optional validation experiments |

Runtime support is mono/stereo WAV/FLAC, 8-384 kHz. No runtime Python, FFmpeg or
network service is required. Inputs must be seekable. The default `cli` feature
can be disabled for library consumers.

## Build and verify

Run from the project root. This Windows workspace has a local Rust/Cargo 1.98.1
GNU toolchain in `.tools/`; `scripts/cargo.ps1` configures it for the process and
selects an installed GCC driver when available. GCC on this host is at
`C:\msys64\ucrt64\bin\gcc.exe`. The bundled linker failed previously; the wrapper
avoids that host issue. The manifest minimum is Rust 1.85, not the tested version.

```powershell
./scripts/cargo.ps1 test --locked --offline
& ./scripts/cargo.ps1 @('fmt', '--all', '--', '--check')
& ./scripts/cargo.ps1 @('clippy', '--locked', '--offline', '--all-targets', '--', '-D', 'warnings')
./scripts/cargo.ps1 build --release --locked --offline
./scripts/cargo.ps1 check --lib --no-default-features --target aarch64-linux-android --locked --offline
./target/release/audio-forensic.exe --json --fast reference/test_files
```

Use array arguments for PowerShell commands containing the `--` separator.
Debug transform tests can take several minutes. The last CLI command is a
60-second-prefix smoke check, not full-file integrity validation.

On another machine, use normal `cargo` in place of the wrapper, omit `--offline`
until dependencies are cached, and install the Android target with
`rustup target add aarch64-linux-android` before that optional target check.
Preserve `Cargo.lock`. Ordinary tests need neither Python/FFmpeg nor private music.

## Verification actually completed

### Current v0.5 checks — 2026-10-01

- **53 tests passed in one full release-suite run:** 21 unit, 4 AAC,
  13 core/CLI, 5 segment/MQA, 2 original parity/integrity, 3 structure and
  5 transform tests. No failures/ignored tests. Full debug suite not rerun.
- Formatting, Clippy (all targets, warnings denied), Android ARM64 library
  compilation without CLI and Python script syntax checks passed. The
  release executable built and reports `0.5.0`. Android linking, phone execution,
  remote CI and independent grouped accuracy evaluation remain untested.
- Six generated public fixtures / nine native channels match independent
  FFmpeg PCM exactly and match the pinned/NumPy numerical oracles within the
  documented tolerances. No historical oracle or input audio was changed.
- Nine additional generated noise/filter/tone/gap/stereo controls passed, all
  with exact FFmpeg PCM. High phase entropy occurs in unencoded noise, and
  lowpass filtering reduces scatter bounds without a codec. These measurements
  do not produce authenticity or analog-source labels.
- Generated 10/120/600-second controls matched FFmpeg PCM and used about
  19.59/19.79/19.77 MiB peak Windows working set. A release build ran concurrently;
  times are not controlled comparisons or Android measurements.
- The private collection was not rerun for v0.5. Its latest full-file results
  remain the historical v0.3 record below.

Current local, ignored evidence:

```text
corpus/local/results/structure-v5/manifest.json
corpus/local/results/structure-v5/summary.json
corpus/local/results/structure-v5/observations.json
corpus/local/results/resources-v5/summary.json
```

`STRUCTURE_VALIDATION.md` records differences from Python, numerical tolerances,
generated controls, resource results and actual checks.

### Historical v0.4 checks — 2026-10-01

- **46 tests passed in one final full debug-suite run:** 17 unit, 4 AAC,
  13 core/CLI, 5 segment/MQA, 2 original parity/integrity and 5 transform tests.
  No failures or ignored tests. Four AAC integration tests also passed in release
  mode. These include exact Python score/anchor and FFmpeg PCM agreement,
  mono/mid/side behavior, quiet/tonal/transient abstention, prefix/rate coverage,
  JSON reports, KBD/MDCT/gamma, bounded storage and search cancellation.
- Formatting and Clippy (all targets, warnings denied) passed. ARM64 Android
  library compilation passed without the CLI feature; linking and phone
  execution were not tested. Release executable built successfully and reports
  version `0.4.0`. Remote CI has not run. Python validation scripts compile.
- All 24 generated processing cases matched FFmpeg PCM exactly. AAC 128/256/320
  with TNS disabled hit at both rates; dual mono and anti-phase encoded controls
  hit only on the active basis. Clean, quiet and cross-codec controls did not hit.
- **Known miss:** a 137-sample trim loses the AAC observation because phase
  search is stepped by eight. An 8-sample trim and the tested gain retain it.
  Enabling TNS produced identical PCM on this source, so TNS robustness is untested.
- Generated 10/120/600-second controls matched PCM and used about
  19.48/19.72/19.80 MiB peak Windows working set. The debug suite ran concurrently;
  these are not controlled timing comparisons or Android measurements.
- The private collection was not rerun for v0.4. Its latest results are below.

Historical v0.4 local, ignored evidence:

```text
corpus/local/results/aac-v4/manifest.json
corpus/local/results/aac-v4/summary.json
corpus/local/results/aac-v4/observations.json
corpus/local/results/resources-v4/summary.json
```

`AAC_VALIDATION.md` records tolerances, deliberate reference differences,
applicability and experimental limits. Public fixtures are generated signals;
ordinary Rust tests require no Python, FFmpeg or private recordings.

### Historical v0.3 checks — 2026-09-30

- 39 unique Rust tests passed: 14 unit, 13 core/CLI, 5 segment/MQA, 2 original
  parity/integrity and 5 transform integration tests. The first full run exposed
  a deep-noise-floor parity issue; after the documented applicability/tolerance
  fix, the 19 unit/transform tests were rerun successfully. The other 20 tests
  had passed and were unchanged. Do not misreport this as one final full-suite run.
- Formatting and Clippy with warnings denied passed. Release executable built
  and reports version `0.3.0`.
- ARM64 Android library target check passed. NDK linking, APK creation and phone
  execution have **not** been validated. The Windows/Linux CI workflow is present
  at `.github/workflows/ci.yml`; remote CI has **not** run.
- Ten v0.3 generated processing cases matched independent FFmpeg PCM hashes and
  expected observations: clean controls, Vorbis q4/q6/q8/q10, transient content,
  trim/gain and SWR/SoXR resampling. Eight earlier codec controls also matched PCM.
  These are narrow engineering checks, not measured real-world accuracy.
- Of 34 supplied FLACs, 31 completed full analysis and matched FFmpeg PCM exactly.
  All source file hashes were unchanged. Three corrupt inputs failed in both
  implementations; see below. All 31 ancestry results stayed inconclusive.
- Generated 10/120/600-second controls used about 19.87/20.00/20.01 MiB peak
  working set on this Windows host. These are not Android performance results
  or a hard process-memory guarantee.

Historical v0.3 raw evidence is local and ignored by `.gitignore`:

```text
corpus/local/results/transforms-v3/summary.json
corpus/local/results/synthetic-transforms-v3/manifest.json
corpus/local/results/synthetic-transforms-v3/summary.json
corpus/local/results/other-codecs-v3/summary.json
corpus/local/results/resources-v3/summary.json
```

Neighboring numbered JSON files hold per-file reports. Detailed v0.3 findings and
tolerances are in `TRANSFORM_VALIDATION.md`; earlier documents remain historical.
Optional experiments use `scripts/validate_local.py`, `check_aac.py`,
`check_structure.py`, `check_transforms.py`, `check_detectors.py` and `check_resources.py`. Inspect their output-directory
defaults and choose a new results location before rerunning historical experiments.
Reference generation needs Python/NumPy/SciPy and FFmpeg as documented in the
generators; never regenerate saved oracles just to hide a mismatch.

## Known pitfalls and input issues

- Do not silently downmix: anti-phase stereo can cancel. Preserve exact integers
  for bit/MQA checks and declare any future detector-specific channel transform.
- The global activity mask requires two passes. Use a fresh decoder for each:
  resetting a FLAC decoder does not reset its verification checksum state.
- The reference STFT uses strict `end < length`, implemented with one-sample
  lookahead. Magnitude ratios are not power ratios; above-cutoff FFT level is not
  dBFS. Do not casually replace these conventions.
- Mirror correlation is unavailable outside its energy gates. Deep-floor f32
  FFT differences and explicit tolerances are documented in the latest validation.
- AAC uses an explicit f64 mid/side basis only for its own analysis. Quiet bases
  and sparse bands abstain. Its 8-sample phase step misses the tested 137-sample
  trim; hits and misses do not establish AAC ancestry. See `AAC_VALIDATION.md`.
- Spectral scatter and high-band phase are measurements only. Noise can have
  high entropy and mastering filters can lower a scatter bound. Phase compares
  adjacent active frames with shared energy; flat scatter and empty bands abstain.
- Wall/profile observations share evidence. A generated mastering low-pass can
  match a codec-wall profile, while tested AAC/Opus transcodes can miss the wall
  gates. Neither a hit nor absence of a hit establishes provenance.
- `reference/test_files` contains user-described CD/vinyl rips with unverified
  histories. Never use them as labeled authenticity or false-positive data.
- Known corrupt files: `01 - Hands Up.flac`, `11 - The APL Song.flac`, and
  `13 - Where Is The Love.flac`. Do not repair/re-encode them or count corruption
  as lossy ancestry. A full collection validator exits 1 because of these inputs.
- `7 - Bend the Clock - Dream Theater.flac` has no available embedded FLAC
  checksum-verification result but still matches independent FFmpeg PCM.
- The Manifesto track has mixed wall probes with no matching codec profile,
  documented in `DETECTOR_VALIDATION.md`. Its cause remains unknown.

## Next milestone and remaining work

Inspect `_silence_and_vinyl`, `_cassette_source`, `_fft_band_extract` and their
autocorrelation/temporal-variance helpers in the pinned reference. Define bounded
noise-band and silence measurements, their interval/gating rules and independent
generated controls before implementation. Do not copy the full-prefix FFT buffers,
vinyl/cassette confirmation language or score bypasses. Source labels require
independently characterized recordings and later grouped evaluation.

AAC's current port covers the reference long KBD window only. Exhaustive phase
search, other window geometries and explicit TNS/SBR support would require
separate resource and false-hit validation. Do not retune the provisional
threshold simply to hide the recorded trim miss. Neither a successful Python
port nor synthetic encoder controls calibrates statistical thresholds.

Other remaining areas: analog profiles, noise-floor bit-depth
analysis, loudness, defensible MQA confirmation, additional format support,
aggregation and independent grouped corpus evaluation. Android bindings, NDK
linking, device resource/lifecycle checks and UI follow the core acceptance gates.

## Portability and backup status

This Rust project root is now a **local Git repository on `main`**, initialized
on 2026-09-30. Its first commit is `chore: checkpoint validated Rust core v0.3.0`.
Use `git log --oneline` to find the checkpoint and `git status` to inspect later
changes. The source-only publication was pushed to
`git@github.com:spideyonmoon/audio-forensic-rust.git` on `main` at commit
`2a9095a8c3d64acbaa39d48bece5a012863a80af`. That remote tree contains only
production Rust, manifests, CI and Markdown documents; it intentionally omits
tests, validation scripts, generated fixtures, `reference/`, `corpus/`, build
artifacts and other work files. The local checkout still retains those files
and its full development history. The nested Python reference clone remains
separate and ignored.

To carry the project to another tool/machine, retain all root documents,
`Cargo.toml`, `Cargo.lock`, `LICENSE`, `.gitignore`, `.gitattributes`, `.github/`, `src/`, `scripts/`
and **all of `tests/`, including binary generated fixtures**. The code and these
Markdown files are ordinary local files; reading them needs no chat history.
Preserve the root `.git/` directory as well to retain local commit history.

For a complete local research backup, separately preserve `corpus/local/` and
`reference/` (private recordings, local reports and the pinned Python clone).
They are excluded from Git tracking by default. `target/` and `.tools/`
are rebuildable host artifacts. A second folder or archive on the same drive is
not an external backup; copy important work to another location under the user's
control. Do not upload private recordings when creating a source-only backup.

Suggested prompt for any coding assistant:

> Read AGENTS.md and HANDOFF.md in this project, inspect the current files, then
> continue the next Rust core milestone. Preserve the analysis and validation
> constraints, and update the handoff with actual progress before stopping.
