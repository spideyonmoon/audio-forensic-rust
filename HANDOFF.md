# Start here: Audio Forensic Rust

Last updated: **2026-10-03**. This file is the portable continuation point for
another session, coding tool or human. Keep it current using `AGENTS.md`.

## Current state and next action

- **FLAC continuity/rewind audit completed (2026-10-03).** Engine 0.18.3,
  schema/policy/dependencies unchanged; Android deferred. Read
  `FLAC_INTEGRITY_VALIDATION.md`. The initial generated 204-case matrix retained
  408 schema-valid reports and ten exact independent PCM controls, but exposed
  clean-source failures. A locked reader keeps sequence state after seeking,
  merging valid frames on pass two; FLAC seek also requires known byte length.
  The core now re-probes a bounded fresh reader, checks unchanged STREAMINFO and
  audio offset, contiguous packet timestamps/durations, and accounted logical
  byte positions. Five initial FLAC tests, two parity tests and ten source tests
  passed after this fix; the 204-case matrix also passed. Further generated
  zero-padding/hidden-frame controls exposed 48 false successes despite valid
  CRCs/MD5; a bounded single-frame layout check now rejects unconsumed bytes and
  nonzero padding. The expanded matrix passed 270 cases / 540 schema-valid
  reports with 34 independent exact PCM controls. Another 96 generated
  FFmpeg-compressed inputs passed 192 reports / 96 exact PCM comparisons.
  Seven FLAC tests and the long-unary cooperative-deadline unit passed.
  All-target MSRV Clippy passed after a test-expression parentheses repair;
  no-CLI check and formatting passed. An early regression was stopped after
  the Clippy result, preserving its logs and pre-repair fingerprint; it is not
  a completed regression. Final frozen hashes cover 121 files. Rust 1.85 debug
  regression completed with exit 0 in `full-msrv-tests-final.log` and
  `full-msrv-tests-final.exit`. Receipts are ignored in
  `flac-integrity-v18-3-20261003/` under
  `corpus/local/results`. Production snapshot `source-package-v18-3-20261003/`
  passed offline MSRV build, no-CLI check, Clippy and its own 732 reports / 130
  exact PCM cases; 35-file ZIP integrity passed. Stable release CLI build passed;
  four generated FLAC duration/rate controls passed exact PCM and schema with
  sampled peak memory, including 48 kHz stereo for 10/120/600 seconds and
  384 kHz stereo for 10 seconds. All 788 regenerated mutation inputs match the
  prior batch; one prior success now fails framing. Only regression remains
  completed. The final Rust 1.85 debug regression completed **161 tests with
  zero failures/ignored**, including 27 suites and doc-tests. Final preservation
  passed for all 121 frozen source/test/helper/schema files, 188 earlier files,
  the staged index/diff and the clean pinned reference. The production-only
  0.18.3 snapshot and 35-file ZIP passed; the release duration controls and
  mutation comparison passed. This milestone is ready to publish as the local
  `codex/flac-v18-3` branch once its commit is created. Remote `main` is kept
  untouched. No private recordings were analyzed or uploaded.
  No commit, push, upload or private-recording analysis.
- **WAV applicability hardening completed (2026-10-03).** Continuing the next
  core gate with Android deferred. Generated 64-case full/prefix characterization
  retained 128 schema-valid reports and found unknown-data-length inconsistency,
  unsupported channel-basis acceptance and nonzero precision-padding acceptance.
  Engine 0.18.2 now adds explicit guards; schema/policy/dependencies unchanged.
  `tests/wav_formats.rs` and `scripts/check_wav_formats.py` add exact generated PCM
  controls. Initial FFmpeg auto-detection disagrees on two 24-in-32-bit integer
  cases; explicit declared integer decoding agrees, with both outcomes retained.
  Read `WAV_FORMAT_VALIDATION.md`. Initial evidence/fingerprints are ignored in
  `corpus/local/results/wav-contract-v18-2-20261003/`. Focused Rust tests passed
  23 controls; the final matrix passed all 128 reports and 46 independent PCM
  cases (two need the explicit integer decoder). All 82 frozen source/test hashes
  still match. All **153 release tests passed**, zero failed/ignored, using
  Rust 1.98.1 and the existing LLVM linker. See `full-release-tests-final.log`
  and `full-release-test-summary.json`. The earlier debug regression exited
  1073807364 mid-suite; an earlier release build also ended without a result.
  Those remain incomplete attempts. A sandbox linker-permission failure and
  successful authorized retry are retained separately. MSRV all-target Clippy,
  stable formatting, Python syntax and schema drift passed. The snapshot in
  `source-package-v18-2-20261003/` passed offline Rust 1.85 build, no-CLI check,
  production Clippy and its own 128-report/46-PCM matrix; ZIP integrity passed.
  New `scripts/check_pcm_boundaries.py` passed 220 generated cases, 440 reports
  and 180 independent PCM checks in `precision-boundaries-final/`: every valid
  integer width, float signed zero/subnormals/normal minima/±16, and invalid
  float samples inside/outside the prefix. All 788 regenerated mutation inputs
  match the prior batch; the new run passed, with 64 status changes explained
  solely by stricter byte-rate geometry. No dependencies or policy changed.
  Final preservation passed for 82 frozen files, 185 earlier unrelated files
  (including all 48 recording/note files), the staged index/diff and the clean
  pinned reference. **No task jobs remain running.** Next: audit FLAC unknown
  totals/checksum absence and damaged-frame continuity with generated exact-PCM
  controls; inspect `tests/parity.rs` and the locked decoder before changing
  support. No commit, push, upload, private audio analysis or external backup.
- **Source API and batch resilience completed (2026-10-03).** The user requested continuing
  substantially further within the active session. Android remains deferred.
  Inspecting caller-provided short reads, unknown/incorrect byte lengths,
  injected read/seek failures, cancellation and source mutation between passes.
  Before-source/index/reference fingerprints are saved in ignored
  `corpus/local/results/source-contract-v18-20261003/before-state.json`.
  Initial source controls passed six of eight tests and exposed two actual
  defects: transient Interrupted reads failed decode, and probe masked source
  PermissionDenied as unsupported format. The guarded-source fix passed all
  eight initial controls. A further test demonstrated failed float metadata
  incorrectly claiming integer PCM; precision now derives from the selected
  native codec before decode. Repeated interrupts/cancellation and raw EOF-error
  controls were added. CLI directory/entry/metadata errors now remain per-input
  failures, preserving other inputs; new factory `AnalysisReport::input_failure`
  supports input discovery failures. Engine patch is 0.18.1; schema/policy stay
  0.18.0 / observations-only-v18. Cargo dependencies are unchanged.
  One initial CLI test compile error (moving a non-Copy status) was corrected;
  all initial failures are retained in the private batch. Focused final tests
  passed 15 tests (10 source, 3 CLI units, 2 CLI process integrations). All 60
  exported source reports and a 3-report mixed CLI smoke passed the unchanged
  schema; the valid smoke input's PCM hash is exact. A final interrupted-read
  deadline unit was added, then source/tests were frozen in `frozen-source.json`.
  All **147 Rust 1.85 debug tests passed**, zero failed/ignored, with frozen
  source/test hashes unchanged. `full-msrv-test-summary.json` records actual
  counts; initial failures stay retained. Do not modify Rust during a build.
  All-target MSRV Clippy, no-CLI library compilation and stable formatting
  passed; original/staged/pinned-reference preservation passed. See
  `SOURCE_API_VALIDATION.md`. The staged index, pinned reference and 258 earlier
  unrelated files passed preservation. Cargo changes are only the root version;
  dependencies/features are unchanged. Release tests, Android compilation/link/
  device behavior and remote CI were not rerun for 0.18.1. No private recording
  analysis, upload, commit, push or external backup. **No task jobs remain running.**
  Next: characterize generated unknown-length WAV/RF64 and remaining PCM header
  applicability against independent exact PCM before expanding support.
- **Production-only source verification completed (2026-10-03).** A new local
  snapshot under `corpus/local/results/source-package-v18-1-20261003/` includes
  34 files: frozen production Rust/Cargo/license, JSON Schema and standalone
  build notes. Offline Rust 1.85 CLI build, no-CLI library check, production
  Clippy, exact-PCM and schema smoke passed with no private tests/fixtures.
  Copied files and archive integrity passed. `audio-forensic-0.18.1-source.zip`
  is a local artifact with hashes/logs in that batch, not Git history or an
  external backup. No recording, private report, toolchain or Git history is
  inside; validation evidence remains outside. No commit/push/publication.
- **Container mutation campaign completed (2026-10-03).** Optional stdlib plus
  schema-validation helper `scripts/check_container_mutations.py` generated
  788 WAV/FLAC header-bit-flip/truncation inputs, using deterministic PCM and a
  private FFmpeg FLAC baseline. The copied 0.18.1 CLI retained all 788 outcomes:
  102 analyzed, 161 unsupported, 525 failed, with valid schema/consistency and
  no successful measurements on unsuccessful cases. Both baselines have exact
  independent PCM hashes. Process exit 1 was expected; no crash/timeout/lost
  report occurred. Inputs and binary fingerprints stayed unchanged. Evidence
  is local/ignored in `corpus/local/results/container-mutations-v18-20261003/`.
  This is a bounded deterministic sweep, not complete fuzzing or accuracy proof;
  it required no further production change. No campaign job remains running.
- **Report contract validation completed (2026-10-03).** The user requested
  moving beyond the completed AAC audit and explicitly deferred Android work.
  New job: export a versioned JSON Schema from the current serialized model,
  validate all file outcomes and reject contract/policy/interval/channel drift.
  `scripts/generate_report_schema.py` generated 47 definitions in
  `schemas/analysis-report-0.18.0.schema.json`; a new generated-only Rust contract
  test and optional offline validator are saved. A project-local Python validator
  environment is installed under ignored `.tools/report-schema-venv`.
  All 125 reports passed the final offline validator: 117 analyzed, 5 unsupported,
  one each failed/cancelled/timed_out. The 117 saved reports remained byte-identical;
  eight fresh generated reports cover all five statuses. All 53 controls passed:
  51 invalid cases rejected and two valid numeric boundaries accepted. The Rust
  1.85 focused integration passed (one test/eight cases), with exact integer/float
  PCM hashes and maximum one-ULP float JSON roundtrip difference (four-ULP bound).
  Focused MSRV Clippy, stable formatting, exporter drift and Python syntax passed.
  Initial wrapper, exact-float equality and ineffective mutation harness failures
  are retained/explained in the private batch; no oracle/core output was replaced.
  Read `REPORT_SCHEMA_VALIDATION.md` and `schemas/README.md`. Receipts/logs are
  ignored local files under `corpus/local/results/report-contract-v18-20261003/`.
  No production Rust, manifest/dependency, version or detector policy changed;
  the prior 130-test regression was not rerun. No Android package was downloaded
  or installed, remote CI run, upload, commit, push or external backup. Staged
  index, pinned reference and earlier input/source fingerprints are preserved.
  **No task jobs remain running.** Next: audit app-provided seekable sources with
  generated read/seek failures, inconsistent lengths and changes between passes;
  verify structured failure, worker release and exact coverage. Inspect existing
  `tests/core.rs`, `tests/parity.rs` and `tests/worker_budget.rs` before adding cases.
- **AAC excerpt/tool and integer-phase follow-ups completed (2026-10-02).**
  Generated 27 new private FLACs: intensity-stereo/PNS ablations at AAC256 for
  the three existing first-60-second source groups, plus deterministic piano
  excerpts at 180, 600 and 1200 seconds. Each AAC intermediate has paired 16/24-bit
  exports. No threshold, denominator or Rust implementation changed.
  Inputs, implementation and staged index are fingerprinted in
  `corpus/local/results/aac-coverage-controls-20261002/`. All 33 lineage intake
  entries and 72 matched-basis comparisons passed; 54 comparisons are new and
  18 repeat existing defaults/baselines. Exact PCM, original/excerpt lineage,
  all twelve intermediate decoded lengths and 264 unchanged files passed.
  Initial final harness incorrectly assumed exact 60-second AAC exports; the
  corrected check records the independently verified sixteen extra tail frames,
  preserving the failed assertion in `INITIAL_ATTEMPT.md`. All nine tool-setting
  ablation streams differ from defaults, but piano still misses at all settings
  and later excerpts; other source groups retain hits. See `AAC_MUSIC_VALIDATION.md`.
  `phase_diagnostic.py` passed twenty M/S diagnostics on four piano AAC excerpts
  and six unchanged generated AAC/trim/clean/cross-codec controls. All original
  coarse subsets and independent PCM matched. All integer phases leave every
  piano maximum unchanged at phase 512, but recover the generated 137-sample trim
  at phase 375 (scores 0.184949/0.197704). The four related generated clean/cross-
  codec controls stay below 0.10; this is not wider-search calibration. Output is
  `fine-phase/` within the batch. Python syntax, whitespace and final preservation
  checks passed; Rust tests were not rerun because Rust is unchanged. No upload,
  commit, push, policy/version or Android support change. **No task jobs remain
  running.** Next: isolate coefficient-population/source-basis and controlled
  bitrate effects with generated sparse/multiband and cross-codec controls. A
  production finer grid needs separate resource/cancellation and broader
  false-hit validation; do not force the piano cases to hit.
- **AAC music audit completed (2026-10-02).** Added optional development
  comparator `scripts/audit_aac_receipts.py`. Corrected run `audit-final/` under
  `corpus/local/results/aac-music-audit-20261002/` passed 18 basis comparisons
  on nine supplied/generated clips: fixed-anchor SciPy score/phase/scale/votes,
  RMS, exact FFmpeg PCM, and unchanged inputs. The original pinned Python method
  on its normal f32 input path reproduces the piano AAC miss; its scores agree
  with Rust within numerical tolerance. An initial harness run passed contiguous
  f64 into a reference function that expects f32 and aliases f64 scratch; its
  direct-reference scores are invalid and preserved in `audit/`, explained in
  `INITIAL_ATTEMPTS.md`. `audit-final/` is authoritative; reference unchanged.
  Five same-source AAC256 TNS/M/S configurations at two export depths produced
  ten new descendants; their 14-entry lineage intake passed, including four
  existing parents/baselines. All twenty new basis comparisons passed, but all
  setting descendants stayed below 0.10, including TNS off and forced M/S.
  Four unchanged generated AAC/clean oracles and silence/tone abstention controls
  added six passing mono comparisons. Total: **44 basis comparisons passed**,
  zero matched score error, maximum original f32-path score error 2.776e-17,
  maximum anchor RMS error 2.609e-15. Two negative controls rejected conflicting
  receipts and non-private outputs. Python syntax passed; Rust source/manifest,
  staged index, reference, audio and notes were verified preserved.
  A limited private header inspection parsed first SCE/CPE window descriptors in
  20,680 packets from eight controlled AAC-LC streams. The piano uses mostly long
  KBD windows; this does not establish a TNS payload or explain the whole miss.
  Read `AAC_MUSIC_VALIDATION.md`. No Rust code, gate, version, accuracy label or
  Android support changed; no upload/commit/push. **No task jobs remain running.**
  Next: compare later piano excerpts and intensity-stereo/PNS encoder controls
  against the same matched oracle, keeping source groups together. A threshold
  or denominator change requires broader controls; do not force this case to hit.
- **Private collection intake completed (2026-10-02).** User added the three
  named tracks, three EAC album logs and `sources.txt` to `reference/test_files`.
  There are now 44 audio files (42 FLAC, two M4A). The user says many existing
  files are vinyl rips, without mapping individual filenames, and prefers to
  skip voice recordings. Do not request voice or detailed hardware provenance
  as a prerequisite; proceed with claimed/unknown challenge history.
  Three new full tracks passed intake and independent FFmpeg exact s32 PCM
  comparison, with original hashes unchanged. Private evidence is under
  `corpus/local/results/private-additions-20261002/`; originals remain in place.
  A copied release v0.18 executable is fingerprinted in the receipts.
  Across all 44 originals, full intake analyzed 37 with exact independent
  FFmpeg PCM; six were unsupported and one failed decoding. Strict FFmpeg also
  failed on three originals (two rejected by native signature preflight and the
  truncated file); four Rust-unsupported files decoded in FFmpeg. The collection
  batch correctly exited 1; it is not a clean collection pass. Fifteen 60-second
  comparison FLACs were generated from the additions: three trim baselines and
  twelve added-codec descendants (MP3 128/320, AAC 256 with TNS, Vorbis q5).
  Commands/FFmpeg version are saved privately; related variants share source
  groups and retain claimed ancestry. All fifteen passed full-file analysis and
  exact PCM comparison. Six additional 24-bit exports from the SAME AAC/Vorbis
  intermediates also passed. The 18-entry and 12-entry lineage receipts include
  repeated original/trim parents; there are 21 unique generated FLACs, not 30.
  AAC hit on two of three 256 kbps descendants at both export depths. Vorbis q5
  did not hit on any 16-bit descendant; 24-bit export produced hits for two
  source groups, with the third still undetected. Pinned Python agreed exactly
  on all twelve native-channel Vorbis score/support/active/hit checks. This is a
  shared sensitivity limit, not evidence for declaring unmarked files clean.
  Read `CORPUS_INTAKE_VALIDATION.md` and the ignored batch's `EXPERIMENT.md`.
  No source labels, detector thresholds or Rust implementation were changed;
  no files uploaded, committed or pushed. **No task jobs remain running.**
  The subsequent AAC audit above closed its measured port-parity question;
  broader paired excerpt/encoder-tool controls remain before policy changes.
- **Collection request clarified (2026-10-02).** The user requested exact track
  names and CD/vinyl/web requirements rather than genre targets. `TEST_CORPUS.md`
  now names five existing files needing provenance notes, three additions (Miles
  Davis's So What, Keith Jarrett's The Köln Concert Part I, Daft Punk's Giorgio
  by Moroder), and two optional direct PCM microphone captures. CD rips or
  original official WAV/FLAC downloads are suitable; vinyl is optional. Confirm
  edition/URL and capture/conversion history before assigning labels. Public
  artist/label catalogues confirmed track identities. Free sample links tested
  during research were unavailable or inaccessible, so none are promised as
  working downloads. Changes are documentation only; no new audio was acquired
  or ingested, and no Rust tests were rerun for this clarification. Existing
  mixed staged/unstaged implementation remains untouched; no jobs are running.
  Next concrete action: receive `sources.txt`/local paths, review provenance,
  prepare the local manifest and run full-file intake plus independent PCM checks.
- **Completed continuation (2026-10-02): Rust 1.85 portability and corpus intake.**
  Installed project-local Rust/Cargo 1.85.0 with Android ARM64 target. Desktop
  all-target compilation, no-CLI Android compilation and a source-only desktop
  CLI build passed with a local GCC/LLVM linker shim. Initial GNU links failed
  with the known exit 204; logs are preserved in `corpus/local/results/msrv-v18/`.
  All **130 MSRV debug tests passed**, zero failed/ignored, in a frozen-source run.
  All-target and source-only production MSRV Clippy, stable formatting, no-CLI
  desktop compilation, Python syntax and CI YAML/job checks also passed.
  CI now declares MSRV desktop and Android jobs; remote CI has not run.
  Corpus intake passed 16 stdlib controls and six generated end-to-end controls,
  including exact PCM, split/decoded leakage, explicit batch failure and immutable
  receipts. `scripts/ingest_corpus.py` snapshots declared provenance/group splits
  and full-file reports without ancestry labels; locked groups are reserved and
  never analyzed. Read `CORPUS_INTAKE_VALIDATION.md` and `TEST_CORPUS.md`.
  Original Rust/manifest files match the frozen source-only copy, and the staged
  index and clean pinned reference were verified preserved. Evidence is local in
  `msrv-v18/` and `corpus-intake-v1-smoke-final/` under `corpus/local/results`.
  No task jobs remain running. No independently characterized source files/notes
  are currently present in `corpus/local/sources` or `challenges`.
- **Version 0.18.0 engineering validation is complete for the documented support matrix.**
  Schema: `0.18.0`; policy: `observations-only-v18`. One process-wide active
  analysis, cooperative queued cancellation/deadlines, container preflight
  limits and decoded-capacity checks are saved. Read `CORE_ACCEPTANCE_VALIDATION.md`.
- Version 0.17.0 preceding-event energy passed six new integration tests and
  16 independent stereo controls / 32 native analyses with exact PCM. Causal 10–20 kHz power
  uses bounded first-pass median bounds and second-pass 250 ms history around
  existing peaks, with eligible-only counts and explicit expired context.
  Maximum independent power error was 4.164e-17; final v0.18 regression passed.
  Read `PRECEDING_ENERGY_VALIDATION.md`.
- Version 0.16.0 PCM crest/stereo relationships passed focused validation. Crest uses scaled RMS;
  signed stereo Pearson statistics use bounded centered/scaled updates over
  simultaneous native-channel pairs. All 28 units and six new integration tests,
  26 independent generated cases, exact hashes, Clippy and Python syntax passed.
  See `PCM_RELATIONSHIPS_VALIDATION.md`. Its additions now passed the final
  v0.18 full regression; the initial v0.16 checks alone were focused.
- The user explicitly asked why work stopped after a milestone. Continue active
  core engineering across milestones; do not treat each milestone as a requested
  handoff. Scheduled continuation remains paused. Preserve actual progress here.
- All **106 release tests** passed with LLVM linking, plus **27 generated stereo
  numerical/PCM cases** and 10/120/600-second resource controls. Maximum lag
  coefficient error was 1.289e-7; peak working set stayed below 27 MiB.
  Formatting, all-target Clippy, release CLI build, no-CLI Android compilation, Python syntax and
  CLI text/JSON/version smokes passed. See `SPECTRAL_LAGS_VALIDATION.md`.
- Installed GNU ld failed new release links with exit 204, including a serial
  retry. LLVM linking passed with the same GCC and release profile; reproduce
  using the process-local flags in Build and verify below. Its root cause remains
  undetermined. A below-floor tone diagnostic initially differed from the dense
  f64 oracle; independent f32 FFT agreed with Rust. Original expectations remain
  preserved, with separate precision supplements. Focused v0.18 units, six
  initial container controls, original core/parity and worker tests passed.
  The first full build mixed a newly edited subtype test with an earlier library
  snapshot and failed that assertion; source was frozen and rerun. The final
  full LLVM-linked suite passed **130 tests, zero failed/ignored**, including
  all seven container controls and the last extensible-subtype guard.
  Explicit release CLI build and final text/JSON/version/subtype smokes passed.
  **No task jobs remain running.**
  Final Clippy, formatting and no-CLI Android
  compilation passed. All 69 independent generated cases / 137 native analyses
  passed against copied immutable expectations. All nine duration/rate resource
  controls passed exact PCM with a copied v0.18 binary. A private five-second
  prefix run analyzed 35/39 with exact PCM and returned four explicit unsupported
  cases (two signature, two oversized artwork); it exited 1 and does not establish
  full-file integrity. Details are in the acceptance document. Implementation is still local.
- Resource observations: 48 kHz stereo controls at 10/120/600 seconds used
  21.04/24.65/26.56 MiB peak Windows working set. The 384 kHz 10/120-second
  controls used 76.11/79.25 MiB. Compiles/private-prefix checks ran concurrently;
  timings are uncontrolled desktop observations, not phone benchmarks or worst-case caps.
- v0.14 validation was closed on resume with all 99 preceding release tests
  passing. Its saved numerical/resource results were inspected; details remain
  in `LISTENING_LEVELS_VALIDATION.md`.
- Goal: a Rust core that runs entirely on the phone. Build the Android app after
  core validation. Engineering can continue while independently characterized
  recordings are collected. The full engine is unfinished.
- All ancestry reports remain `INCONCLUSIVE`; evidence index is `null`.
  Quiet-block levels/color do not identify isolated noise, source bit depth,
  dither, medium or authenticity. No score or source verdict was added.
- **Next: generated WAV/PCM applicability audit.** Android work is explicitly
  deferred. The source/batch, schema, mutation and source-package milestones
  above are complete. Characterize unknown-length WAV/RF64 and remaining PCM
  header cases against independent exact PCM before adding support. AAC
  coefficient-population/source-basis/bitrate research remains an optional later
  job; existing misses must stay recorded. The current 44-file collection remains
  challenge history, not verified ancestry labels. Voice recordings and extra
  hardware provenance are deferred. Android NDK linking/device behavior and
  remote CI remain untested. Keep eligible event counts,
  causal-filter timing, baseline scope, gain/attack controls and limitations explicit.
  Loudness at rates not divisible by ten remains explicitly unsupported; other
  applicable core measurements still run at those rates.
  Do not infer a source medium or relabel cutoff variation as wow/flutter.
- The user requested that hourly continuation stop: automation
  `continue-audio-forensic-core` stays PAUSED. No scheduled run was restarted.
  Continue only the active session unless the user requests scheduling again.
- Local development `main` remains the AAC commit `c66d705`. The staged index,
  earlier changes and local tests/scripts were preserved. Do not push this
  development history: it contains files excluded from publication.
- Local `publish/source-only` was inspected at v0.11 commit
  `08c54762d8dceb3d9c91c03e32acfac89247e41a`, subject
  `feat: add bounded below-cutoff sparsity measurements`.
  Remote state was not queried this session. No v0.12/v0.13/v0.14/v0.15 commit, push or external
  backup was made, including v0.16/v0.17/v0.18. Current source, tests and evidence remain local files.
  Source-only publication excludes tests/scripts/fixtures and private evidence;
  it is not a complete research backup. No private recording was changed.

## Read in this order

| File | Purpose |
| --- | --- |
| `AGENTS.md` | Persistent project and documentation instructions |
| `README.md` | Current API/CLI coverage, usage, numerical conventions and limits |
| `PORTING_PLAN.md` | Agreed architecture, policy, Python assessment decisions and acceptance gates |
| `CORE_ACCEPTANCE_VALIDATION.md` | v0.18 process worker budget, preflight allocation/geometry limits, MSRV portability and remaining engineering gates |
| `REPORT_SCHEMA_VALIDATION.md`, `schemas/README.md` | Versioned 47-type JSON report contract, offline validation, all outcomes, null/width/interval/channel controls and actual checks |
| `SOURCE_API_VALIDATION.md` | v0.18.1 source I/O/precision and CLI failure fixes, exact generated PCM, worker recovery and 788 container mutations |
| `WAV_FORMAT_VALIDATION.md` | v0.18.2 WAV support matrix, exact widths/valid bits, layout/GUID/header checks and unknown-length applicability |
| `CORPUS_INTAKE_VALIDATION.md` | Local provenance snapshots, group/lineage/split checks, immutable receipts, reserved locked groups and generated controls |
| `AAC_MUSIC_VALIDATION.md` | Matched AAC music/reference arithmetic, TNS/M/S controls, generated abstentions and retained detector misses |
| `PRECEDING_ENERGY_VALIDATION.md` | v0.17 causal band energy before envelope peaks, median bounds, eligible counts and independent controls |
| `PCM_RELATIONSHIPS_VALIDATION.md` | v0.16 scaled PCM RMS/crest and signed native stereo covariance, coverage and independent controls |
| `SPECTRAL_LAGS_VALIDATION.md` | v0.15 centered log-spectrum products, actual lag geometry, eligibility and independent controls |
| `LISTENING_LEVELS_VALIDATION.md` | v0.14 FIR true-peak estimates, range gates/histogram bounds and independent controls |
| `LOUDNESS_VALIDATION.md` | v0.13 K weighting, programme power, two-pass gates, window coverage and independent checks |
| `NOISE_FLOOR_VALIDATION.md` | Latest v0.12 bounded RMS/color, original indices, exact bin edges, underflow and independent controls |
| `SPARSITY_VALIDATION.md` | v0.11 below-cutoff sparse counts, geometry, numerical floors and independent controls |
| `ENVELOPE_VALIDATION.md` | Latest v0.10 paired band RMS correlation, energy/variation gates, independent inverse-FFT checks and limitations |
| `ROLLOFF_VALIDATION.md` | Latest v0.9 two-band slope, geometry, energy gates, independent EQ/filter controls and limits |
| `TRANSIENT_VALIDATION.md` | Latest v0.8 filter/envelope, baseline bounds, peak coverage, SciPy controls and interpretation limits |
| `NOISE_DYNAMICS_VALIDATION.md` | Latest v0.7 circular band correlation, temporal variation, applicability and independent checks |
| `NOISE_VALIDATION.md` | Latest v0.6 quiet runs, normalized band powers, coverage and numerical controls |
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
| `src/loudness.rs` | Fixed-size programme power ring, K filters, exact two-pass gates and grid-sampled maxima |
| `src/true_peak.rs`, `src/loudness_range.rs` | Fixed FIR peak interpolation and bounded gated range histogram |
| `src/decode.rs` | Native Symphonia WAV/FLAC, two-pass decoding, PCM hashes, stream selection, integrity checks, cancellation and deadlines |
| `src/worker.rs`, `src/container.rs` | Process-wide permit and bounded native-container preflight before third-party parsing |
| `src/stereo.rs` | Scaled centered native-channel PCM correlation over simultaneous pairs |
| `src/dsp.rs` | Streaming per-channel statistics and STFT, activity/cutoff/entropy/wall measurements, exercised/effective integer precision |
| `src/detectors/segments.rs` | Up to 36 non-overlapping two-second probes per channel, wall patterns and overlapping codec-wall candidates |
| `src/detectors/mqa.rs` | Native integer stereo sync/payload candidates in the first three seconds; no confirmation claim |
| `src/detectors/resampling.rs` | Streaming lower-Nyquist notch/wall/mirror observations; all eligible rate hypotheses retained |
| `src/detectors/mdct.rs` | Windowed MDCT kernel, checked against the direct cosine definition |
| `src/detectors/vorbis.rs` | Per-channel phase/grid search at 44.1/48 kHz, at most twelve captured spans from the first 180 analyzed seconds |
| `src/detectors/aac.rs` | Explicit mono/mid/side AAC lattice, bounded first-pass energy selection and at most sixteen second-pass captured spans per basis |
| `src/detectors/structure.rs` | Native-channel streaming scatter bounds and energy-gated phase entropy; measurements only |
| `src/detectors/noise.rs` | Bounded native-channel quiet runs and all/quiet Hann-normalized high/above-cutoff band powers |
| `src/detectors/noise_dynamics.rs` | Signed circular band correlation at two lags; at most 180 complete seconds of band-power variation |
| `src/detectors/transients.rs` | Native-channel high-pass envelope maxima, bounded median survey and 180-second/128-event caps |
| `src/detectors/rolloff.rs` | Active mean-spectrum endpoint slope with actual band geometry and per-bin energy gates |
| `src/detectors/envelope.rs` | Constant-size running Pearson statistics for paired native-channel band RMS envelopes |
| `src/detectors/noise_floor.rs` | Bounded 100 ms native-channel RMS percentiles and selected-block spectral color, using original indices |
| `src/detectors/sparsity.rs` | Fixed per-bin relative-magnitude counts, deferred global cutoff and explicit eligibility |
| `src/detectors/spectral_lags.rs` | Bounded finalization of the existing mean spectrum into signed high-band lag/neighbor products |
| `src/detectors/preceding_energy.rs` | Bounded causal band-power baseline and eligible context before selected envelope peaks |
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
avoids that host issue. Rust 1.85.0 is now also installed and compiler-tested;
stable remains the default toolchain.

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

On 2026-10-02, newly linked release binaries failed in installed GNU ld with
exit 204 (including a serial build). LLVM linking through the same GCC driver
worked, without changing the release profile. For this host, set these
process-local flags before `scripts/cargo.ps1` release commands:

```powershell
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = '-C link-arg=-fuse-ld=lld -C link-arg=-BC:/Users/Bishal/code/audio-forensic-rust/.tools/rustup/toolchains/stable-x86_64-pc-windows-gnu/lib/rustlib/x86_64-pc-windows-gnu/bin/gcc-ld/'
./scripts/cargo.ps1 test --release --locked --offline -j 2
./scripts/cargo.ps1 build --release --locked --offline -j 2
```

This selects the bundled `ld.lld.exe`; it is a local host workaround, not a
project dependency or a solved GNU linker diagnosis. The wrapper was not
modified. On another checkout, resolve the installed toolchain's corresponding
directory instead of copying this absolute path. Preserve any intentional
existing Rust flags when adding these arguments. Android checks need none of
these Windows-target flags.

For fresh Rust 1.85 host builds (including host tools in Android checks), use the
ignored GCC/LLVM shim saved in this workspace. It selects LLVM before Cargo
invokes the host linker, so it also covers proc macros/build scripts:

```powershell
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $PWD '.tools/msrv-gcc-lld.cmd'
./scripts/cargo.ps1 +1.85.0 test --locked --offline --target-dir target/msrv-desktop-v18 -j 2
./scripts/cargo.ps1 +1.85.0 check --lib --no-default-features --target aarch64-linux-android --locked --offline --target-dir target/msrv-android-v18 -j 2
```

The shim uses this checkout's GCC and bundled LLVM paths; inspect/resolve them on
another host rather than assuming this ignored local file is in source history.
Details and initial failures are in `CORE_ACCEPTANCE_VALIDATION.md`.

On another machine, use normal `cargo` in place of the wrapper, omit `--offline`
until dependencies are cached, and install the Android target with
`rustup target add aarch64-linux-android` before that optional target check.
Preserve `Cargo.lock`. Ordinary tests need neither Python/FFmpeg nor private music.

## Verification actually completed

### MSRV / corpus intake continuation — 2026-10-02

- Rust/Cargo 1.85.0: **130 ordinary debug tests passed**, zero failed/ignored;
  all-target compilation/Clippy, source-only CLI build/production Clippy, no-CLI
  desktop compilation and no-CLI Android ARM64 compilation passed. The earlier
  Rust 1.98.1 release suite is separate; no new release suite/build was run.
- Stable formatting, Python syntax and CI YAML/job assertions passed. CI now
  declares MSRV desktop and stable/MSRV ARM64 compilation; remote CI was not run.
- Corpus intake: **16 stdlib unit controls and six generated end-to-end controls
  passed**, including exact independent integer PCM, split/decoded leakage,
  explicit unsupported continuation, immutable results and reserved locked groups.
  No provenance group, private recording or locked source was analyzed.
- Manifest/lockfile/production Rust match the frozen validation copy; the staged
  index and clean pinned Python clone are preserved. The source-only debug CLI
  SHA-256 is `37674db0dbb24d882927f5bfdfea3b851eb7dc1868387af70365fd96935ae216`.
- Initial GNU host links failed with exit 204; the local GCC/LLVM shim passed.
  First Clippy argument forwarding failed; the PowerShell array retry passed.
  Those original logs remain preserved. GNU root cause, NDK linking, Linux/device
  execution, new corpus accuracy and remote CI remain outstanding.
- All work/evidence remains local; no commit, push, external backup, private upload
  or restarted automation. No task jobs remain running. Next action: reviewed
  provenance manifest and full-file ingestion when sources/notes arrive.

### Current v0.18 checks — 2026-10-02

- Final frozen-source release suite: **130 passed, zero failed/ignored** with
  process-local LLVM linking. The earlier inconsistent full-run failure is
  retained and explained in `CORE_ACCEPTANCE_VALIDATION.md`.
- Release CLI build, final version/text/JSON/shared-prefix and extensible
  companded-WAV abstention smokes passed. Final executable SHA-256:
  `37d41a8ad001c6d186cf038661fd93c409d0ace376cefd5e23a8034f09cde756`.
- Formatting, all-target Clippy, Python syntax and no-CLI Android ARM64
  compilation passed. This preceding release run used Rust/Cargo 1.98.1; MSRV
  checks are recorded separately above. NDK linking, APK/device behavior and
  remote CI were not run.
- **69 independent generated cases / 137 native analyses** passed copied
  immutable expectations and exact PCM. Earlier v0.15/v0.16/v0.17 evidence
  remains preserved; separate v0.18 regression result directories were used.
- All **nine generated resource controls** passed exact PCM. 48 kHz 600-second
  peak working set was 26.56 MiB; 384 kHz 120-second peak was 79.25 MiB.
  Concurrent compilation means elapsed times are uncontrolled host observations.
- Private five-second compatibility: 35/39 exact PCM analyses; four unsupported
  (two signature, two artwork limits), validator exit 1. The known full-file
  corrupt inputs remain preserved; prefix success does not clear them.
- All 117 saved actual reports passed schema/policy/ancestry/null/finite-JSON
  checks. Classification accuracy and independently characterized source groups
  remain outstanding; ancestry stays `INCONCLUSIVE`, evidence index null.
- Evidence and copied validation binary are ignored/local under `corpus/local`.
  No new commit, push, external backup, private upload or restarted automation.
  No task jobs remain running; source/tests/scripts are local changes.

### Previous v0.15 checks — 2026-10-02

- Full `test --release --locked --offline -j 2`: **106 passed**, zero failed or
  ignored. This includes all preceding tests, two exact-product/abstention units
  and five new generated integration tests. LLVM linker flags above were used;
  optimization/LTO settings were unchanged.
- Independent NumPy controls: **27 stereo cases / 54 native channels** passed
  exact FFmpeg s32le hashes, including prefixes, with exact counts, geometry,
  applicability and nullability. Maximum errors: 1.726097e-8 coherent amplitude,
  2.877911e-7 dB standard deviation, 1.288664e-7 coefficient. Original dense
  expectations are immutable; separate f32 FFT supplements validate below-floor
  minima after the documented rejected-tone precision finding.
- Final formatting, all-target Clippy with warnings denied, Android ARM64 library
  compilation without CLI, ordinary release CLI build with LLVM, Python syntax,
  CLI version/text/JSON smokes and Git
  whitespace passed. Android linking/device behavior remains untested.
- Generated 10/120/600-second duration controls matched exact PCM and used
  **20.79/24.48/26.42 MiB** peak working set in **0.8299/3.2695/15.3972 s**.
  Compilation overlapped; these are not controlled speed comparisons or phone
  measurements.
- GNU ld release-link attempts failed with exit 204, even with one job. A simple
  C link worked. Selecting bundled LLVM through the same GCC succeeded; the
  GNU root cause remains unresolved. See the validation document and build flags.
- No full debug suite, publication-only copy build, new codec/resampler matrix,
  private collection pass, grouped source evaluation, remote CI or phone tests.
  Python reference is still clean at the pinned SHA. No commit, push or external
  backup was made; the existing staged index and unrelated work were preserved.

Details: `SPECTRAL_LAGS_VALIDATION.md`. Source: `src/detectors/spectral_lags.rs`.
Local-only test/script/evidence: `tests/spectral_lags.rs`,
`scripts/check_spectral_lags.py`, `corpus/local/generated/spectral-lags-v15/`,
`corpus/local/results/spectral-lags-v15/` and `corpus/local/results/resources-v15/`.
The new source is untracked; this work is local files, not a Git/source backup.
No task jobs remain running.

### Historical v0.14 validation closure — 2026-10-02

- On resume, no preceding jobs remained running. Full release tests were rerun
  before changing production source: **99 passed**, zero failed/ignored.
- Inspected the saved 33 generated cases / 53 peak analyses, all 27 loudness
  regression cases and 10/120/600-second resource results. The preceding session
  completed the independent numerical, exact PCM and non-suite checks; the full
  release result was its only unrecorded check. `LISTENING_LEVELS_VALIDATION.md`
  now records completion. No v0.14 commit, publication or external backup exists.

### Historical v0.13 checks — 2026-10-02

- Full `test --release --locked --offline`: **92 passed**, zero failed/ignored.
  This includes all 85 existing tests and seven new loudness tests (one unit,
  six integration). Published 48 kHz coefficients, gates, native channel power,
  prefixes, exact window boundaries, silence and unsupported rates are covered.
- `scripts/check_loudness.py`: **27 generated cases**, exact FFmpeg s32le hashes,
  exact coverage/count/status agreement and maximum dense-oracle error
  **9.024e-13 LU**. FFmpeg loudness comparisons passed on 22 applicable cases;
  maximum integrated/M/S errors were **0.008846/0.000465/0.000481 LU**.
- Formatting, all-target Clippy with warnings denied, release CLI build/version,
  generated 0.8-second text smoke, Python syntax and Git whitespace checks passed.
  No-CLI Android ARM64 library compilation passed; linking/device behavior was
  not tested. An initial smoke command used a nonexistent fixture and correctly
  returned a structured failure; the corrected generated-file command passed.
- Generated 10/120/600-second resource controls matched PCM and used
  **20.62/23.61/26.14 MiB** peak Windows working set in
  **2.3692/9.7606/35.3437 s**. Compilation and independent controls overlapped;
  these are not controlled timing or Android benchmarks.
- No full debug suite, publication-only copy build, new encoder/resampler matrix,
  private collection pass, grouped source evaluation, certified meter suite or
  remote CI. No commit/push/external backup was made. The Python reference is
  clean at its pinned baseline; existing staged index and unrelated work remain.

Details: `LOUDNESS_VALIDATION.md`. Local-only source/evidence:
`src/loudness.rs`, `tests/loudness.rs`, `scripts/check_loudness.py`,
`corpus/local/generated/loudness-v13/`, `corpus/local/results/loudness-v13/` and
`corpus/local/results/resources-v13/`. The new source is an untracked local file;
tests/scripts/evidence are excluded from source-only publication. No v0.13 source
backup or external research backup has been created. No jobs remain running.

### Documentation cleanup - 2026-10-02

- Removed the session phase guide at the user's request and removed its startup
  references. Restored the next action to assessing loudness/listening quality.
- Confirmed the guide is absent and documentation whitespace checks passed.
  No code tests were rerun; no code, recordings, staged index or running jobs
  were changed. The v0.12 implementation checks below remain the latest results.

### Historical v0.12 checks - 2026-10-02

- All **85 release tests** passed in one final run: the existing 78 plus seven
  new quiet-block integration tests. No failures or ignored tests remain.
- All **17 generated analysis cases / 34 channel analyses** passed independent
  NumPy comparisons and exact FFmpeg s32le PCM hashes. Original block indices,
  statuses, counts, intervals and eligibility matched. Maximum absolute errors:
  2.950e-17 RMS, 1.655e-23 mean bin power and 4.264e-14 dB.
- Fixed and documented a real floating-point band-edge ambiguity discovered by
  the independent 8001-Hz control. Band membership now uses exact integer
  ratios. Original expected files are preserved; corrected ones have distinct
  names. The initial prefix-test rounding error and intermediate compile failure
  are documented in `NOISE_FLOOR_VALIDATION.md`; no outstanding failure remains.
- Formatting, all-target Clippy (warnings denied), no-CLI Android ARM64 library
  compilation, release CLI/version/text smoke checks, Python syntax and Git
  whitespace checks passed. Android linking and phone behavior are untested.
- Generated 10/120/600-second duration inputs matched PCM and used
  **20.60/23.84/26.09 MiB** peak Windows working set in **1.166/3.6962/14.177 s**.
  Compilation overlapped; these are not controlled speed or Android benchmarks.
- No full debug suite, separate publication-only build, new encoder/resampler
  matrix, private collection pass, grouped source evaluation or remote CI run.
  The Python reference remains clean at its pinned baseline. No commit/push or
  external backup was made; existing development index/work was preserved.

Details: `NOISE_FLOOR_VALIDATION.md`. Local-only source/evidence:
`tests/noise_floor.rs`, `scripts/check_noise_floor.py`,
`corpus/local/generated/noise-floor-v12/`,
`corpus/local/results/noise-floor-v12/` and
`corpus/local/results/resources-v12-final/`. The pre-boundary-correction resource
run remains separately under `resources-v12/`. These are not in a source backup.

### Historical v0.11 checks — 2026-10-01

- All **78 release tests** passed, including five new sparsity integration tests.
- Independent NumPy dense frame/bin arithmetic passed on **19 generated stereo
  files / 38 channels**; all FFmpeg decoded s32le hashes matched exactly. A notch
  control differed by one sparse observation out of 141,243, within a predeclared
  allowance of 28 near-threshold observations. Other counts agreed exactly.
- Tones produced about 99% sparsity without codec processing. Filter and changing
  bandwidth controls confirm that this measurement cannot establish ancestry.
  Silence, tiny signals, inadequate bandwidth and short prefixes abstain.
- Formatting, all-target Clippy with warnings denied, Android ARM64 library
  compilation without CLI, and publication-only production Clippy passed.
  The final caveat-text expansion was included in the release CLI rebuild and
  publication-only check; production arithmetic is the fully tested version.
- Generated 10/120/600-second files matched PCM and used
  **20.07/23.70/25.57 MiB** peak Windows working set in **0.70/2.48/9.20 seconds**.
  CLI version `0.11.0` and a 0.8-second-prefix text smoke check passed.
- No full debug run, new transcode/resampler matrix, private collection pass,
  grouped source evaluation, remote CI completion, Android linking or phone
  testing. Reference clone remains clean at its pinned baseline.

Details and numerical definitions: `SPARSITY_VALIDATION.md`. Local-only evidence:
`corpus/local/generated/sparsity-v11/`, `corpus/local/results/sparsity-v11/`,
`corpus/local/results/resources-v11/`, `tests/sparsity.rs` and
`scripts/check_sparsity.py`. Publication contains no tests/scripts or local evidence.

### Historical v0.10 checks — 2026-10-01

- **73 tests passed in the full release suite:** 21 unit, 4 AAC, 13 core/CLI,
  5 segment/MQA, 4 envelope, 7 noise, 2 parity/integrity, 4 roll-off, 3 structure,
  5 transform and 5 transient. No failures or ignored tests in the final run.
- Formatting, all-target Clippy with warnings denied, no-CLI Android ARM64
  compilation and production Clippy in `target/source-only-v10` passed. The
  publication copy excludes tests/scripts/fixtures. Android linking and phone
  behavior remain untested.
- Eighteen generated stereo 24-bit controls / 36 native channels passed exact
  FFmpeg PCM hashes and independent inverse-FFT/time-domain band power and
  stored-envelope Pearson checks. Maximum coefficient error was 1.249e-8;
  statuses, counts, intervals and applicability flags matched exactly.
- Shared modulation measures about +1, opposite modulation about -1, and
  different 1/3 Hz modulation about -0.000949. Quiet/constant/missing bands and
  short input abstain. The 44,000/44,001 Hz complete-band boundary is explicit.
- The first focused run had a one-bit JSON float-roundtrip discrepancy; that
  assertion now uses 1e-14 tolerance and exact frame counts. No production
  arithmetic or historical oracle was changed to hide a failure.
- The release CLI reports `0.10.0` and its generated 0.8-second-prefix text smoke
  check passed. Unchanged 10/120/600-second duration controls matched PCM, used
  20.05/23.64/25.60 MiB peak working set and took 1.80/7.10/22.62 seconds. These
  overlapped compilation/other checks and are not controlled timing comparisons.
- Full debug suite, remote CI completion, new encoder/resampler experiments,
  private collection and independent grouped source evaluation were not run.
  The Python reference remains clean at the pinned baseline; scoring stays gated.

Local-only evidence: `corpus/local/generated/envelope-v10/`,
`corpus/local/results/envelope-v10/`, `corpus/local/results/resources-v10/`,
`tests/envelope.rs` and `scripts/check_envelope.py`. These are excluded from
publication and are not included in a source-only Git backup.

### Historical v0.9 checks — 2026-10-01

- **69 tests passed in the full release suite:** 21 unit, 4 AAC, 13 core/CLI,
  5 segment/MQA, 7 noise, 2 parity/integrity, 4 roll-off, 3 structure, 5 transform
  and 5 transient. Full debug suite not rerun.
- Formatting, all-target Clippy with warnings denied, Android ARM64 no-CLI
  compilation and publication-only production Clippy passed. The latter used
  `target/source-only-v9` with tests/scripts/fixtures absent. Android linking
  and phone behavior remain untested.
- The release CLI reports `0.9.0` and passed a 0.8-second-prefix text smoke
  check on generated EQ audio with explicit prefix coverage and measured slopes.
- Eighteen generated stereo 24-bit controls / 36 native channels passed exact
  FFmpeg PCM hashes and independent NumPy spectral checks. Statuses, counts,
  intervals and geometry matched; maximum slope error was 2.518e-8 dB/kHz.
  Controls include known EQ, filters, spectral gaps, antiphase/gain, inactive
  gaps, silence, tones and sample-rate coverage from 8–384 kHz.
- Initial roll-off test failures were traced to two generator mistakes: excessive
  float amplitude and a two-impulse comb incorrectly expected to be flat. The
  generator was corrected and the comb retained as an abstention control. A
  Clippy comparison-style finding was fixed; no production threshold or
  historical reference output was changed to make tests pass.
- Unchanged generated 10/120/600-second controls matched PCM and used
  20.16/23.79/25.71 MiB peak Windows working set, in 0.55/2.40/8.43 s.
  Checks overlapped compilation; these are not controlled speed comparisons.
- No new encoder/resampler matrix, private collection run, full debug suite,
  remote CI completion or independent source-accuracy evaluation. The Python
  reference remains clean at the pinned baseline. Overall scoring remains gated.

Local-only evidence: `corpus/local/generated/rolloff-v9/`,
`corpus/local/results/rolloff-v9/`, `corpus/local/results/resources-v9/`,
`scripts/check_rolloff.py` and `tests/rolloff.rs`. These are excluded from the
source-only Git backup.

### Historical v0.8 checks — 2026-10-01

- **65 tests passed in the full release suite:** 21 unit, 4 AAC, 13 core/CLI,
  5 segment/MQA, 7 noise, 2 original parity/integrity, 3 structure, 5 transform
  and 5 transient. No failures/ignored tests; full debug suite not rerun.
- Formatting, all-target Clippy with warnings denied and no-CLI Android ARM64
  compilation passed. Android linking and phone behavior remain untested.
- The release CLI reports `0.8.0` and its 0.8-second-prefix text smoke check
  passed. A publication-only copy in `target/source-only-v8` passed production
  Clippy with tests/scripts/fixtures absent.
- Fourteen generated stereo 24-bit controls / 28 native channels matched FFmpeg
  PCM exactly and passed independent SciPy filter/convolution/median/peak checks.
  Counts, retained positions, intervals, applicability and list caps matched;
  maximum retained envelope-peak error was 1.562e-15. Expected values were saved
  before Rust ran. Tests cover 8–384 kHz and a 181-second cap boundary.
- Three smooth tone bursts produce **60** envelope maxima. This is not a physical
  click counter or proof of vinyl. Near-pair controls demonstrate the deliberate
  chronological peak choice instead of Python's height-priority selection.
- Unchanged generated 10/120/600-second controls matched PCM and used
  20.11/23.76/25.64 MiB peak Windows working set, in 0.78/2.59/8.89 s. Checks
  overlapped other work; these are not controlled speed comparisons.
- Full debug suite, remote CI completion, Android device behavior, new encoding/
  resampling experiments and the private collection were not rerun. Independent
  grouped source evaluation and scoring remain outstanding. The reference clone
  remains clean at its pinned baseline.

Local-only evidence: `corpus/local/results/transients-v8/` contains independent
expected values, CLI reports and `summary.json`; generated WAVs are under
`corpus/local/generated/transients-v8/`. Duration evidence is in
`corpus/local/results/resources-v8/`. Local script/test:
`scripts/check_transients.py`, `tests/transients.rs`. These files are excluded
from publication and are not included in the source-only Git backup.

### Historical v0.7 checks — 2026-10-01

- **60 tests passed in the full release suite:** 21 unit, 4 AAC, 13 core/CLI,
  5 segment/MQA, 7 noise, 2 original parity/integrity, 3 structure and 5 transform.
  No failures/ignored tests; debug suite not rerun for v0.7.
- Formatting, all-target Clippy with warnings denied and no-CLI Android ARM64
  compilation passed. The release CLI reports `0.7.0`. Android linking and phone
  behavior remain untested.
  A publication-only copy in `target/source-only-v7` passed production Clippy
  without tests/scripts/fixtures, and the short-prefix text CLI smoke check passed.
- Sixteen generated stereo cases / 32 native channels matched FFmpeg PCM exactly
  and passed an independent inverse-FFT/time-domain correlation and block-power
  oracle. Expected values are computed before Rust runs. This includes a 182-second
  cap boundary, silence/quiet blocks, impulses, tones, level changes and 8–384 kHz.
  Existing generated audio/oracles and the clean pinned reference were preserved.
- Duration controls at 10/120/600 s matched PCM and used 19.98/23.61/25.53 MiB peak
  Windows working set. Temporal spectra add up to 2.82 MiB/channel and stop growing
  after 180 seconds. Timing overlapped compilation/tests; not a speed benchmark.
- Full debug suite, remote CI completion, Android device behavior, the broad
  encoding experiment matrix and private collection were not rerun. Overall
  scoring and independent grouped source-accuracy evaluation remain outstanding.

Local-only evidence: `corpus/local/results/noise-dynamics-v7/` contains expected
measurements, CLI reports and `summary.json`; seven new generated WAVs are in
`corpus/local/generated/noise-dynamics-v7/`. The other nine controls reuse v0.6
generated WAVs unchanged. Duration records: `corpus/local/results/resources-v7/`.
Local tests/script: `tests/noise.rs`, `scripts/check_noise_dynamics.py`.
These files remain excluded from publication.

### Historical v0.6 checks — 2026-10-01

- **57 tests passed in one full release-suite run:** 21 unit, 4 AAC,
  13 core/CLI, 5 segment/MQA, 4 noise, 2 original parity/integrity,
  3 structure and 5 transform tests. No failures or ignored tests.
  The four new noise tests also passed in debug, exercising internal assertions.
- Formatting, Clippy (all targets, warnings denied) and Android ARM64 library
  compilation without CLI passed. The release executable reports `0.6.0`.
- Nine generated stereo 24-bit controls (eighteen native channels) passed exact
  FFmpeg PCM hash checks and independent NumPy quiet-run/band-power comparisons.
  No reference audio or historical oracle was changed. The pinned clone is clean.
- Generated 10/120/600-second controls matched FFmpeg PCM and used
  19.70/19.89/19.91 MiB peak Windows working set. Times were 2.09/5.76/18.88 s;
  compilation/tests overlapped, so these are not controlled speed comparisons.
- A source-only copy under ignored `target/source-only-v6` passed production
  Clippy (`--lib --bins -D warnings`) with tests/scripts/fixtures absent.
  Public CI now builds/lints production targets because an internal AAC test
  requires an excluded local fixture. Full validation needs this local workspace.
- Full debug suite, remote CI completion, Android linking/phone execution and
  independent grouped accuracy evaluation were not verified. The private audio
  collection was not rerun; its latest full-file results remain v0.3.

Local ignored evidence: `corpus/local/results/noise-v6/` contains independently
computed expectations, nine CLI reports and `summary.json`; corresponding audio
is under `corpus/local/generated/noise-v6/`. Duration results are in
`corpus/local/results/resources-v6/summary.json`. Local regression source is
`tests/noise.rs`, with controls in `scripts/check_noise.py`; these are excluded
from publication along with all other tests/scripts/work files.

### Historical v0.5 checks — 2026-10-01

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

Version 0.11 supplies bounded below-cutoff sparse counts using per-frame peaks
and the channel's final global p95 cutoff. See `SPARSITY_VALIDATION.md` for
numerical floors, actual geometry and natural tone/filter limitations.
Version 0.10 supplies bounded paired band RMS correlation with usable-energy and
temporal-variation gates; see `ENVELOPE_VALIDATION.md`. A signed coefficient does
not identify authentic or injected content. Version 0.9 supplies a two-band slope with actual frequency
geometry and per-bin amplitude gates. EQ alone reproduces a purported source
slope; this remains descriptive. See `ROLLOFF_VALIDATION.md`.
Version 0.8's envelope maxima are not physical click counts; see
`TRANSIENT_VALIDATION.md` for the musical-attack limitation.
Version 0.7's circular band correlation still does not claim full-stream Pearson
correlation. Do not reinterpret either measurement as a calibrated source test.

Version 0.12 now ports the useful `_noise_floor_profile` arithmetic with bounded
per-native-channel blocks. It fixes original-index selection after zero removal,
avoids antiphase downmix cancellation, honors the shared prefix and uses exact
rational FFT band membership. Source bit-depth/authenticity inference remains
unimplemented; no `_bit_depth_verdict` policy was imported.
Versions 0.13/0.14 supply loudness, range and FIR peak estimates; see their
validation documents. Version 0.15 adds high-band spectral-lag arithmetic with
explicit eligibility, without the reference's MP3 verdict. Versions 0.16/0.17 add
native crest/stereo relationships and bounded preceding-event band energy, with
independent gain/phase/DC and impulse/attack controls. Version 0.18 adds a
process worker permit and bounded container preflight. Its acceptance record is
complete, followed by Rust 1.85 portability and corpus-intake controls.
The current v0.18 report schema and offline checks also passed in the report
contract milestone. Engine v0.18.1 now supplies caller source/batch resilience,
with all 147 final MSRV debug tests and production-only snapshot checks passing.
Engine v0.18.2 now closes generated WAV/PCM applicability with 153 release tests,
23 focused MSRV tests, independent format/precision matrices, mutation regression
and a production-only source build. Read `WAV_FORMAT_VALIDATION.md`. The next
engineering job is FLAC unknown totals/checksum absence and damaged-frame
continuity; Android work remains deferred by the user.
Read the current state above before choosing further scope. This is core work;
Android UI, aggregation and source claims remain gated on validation.
Do not import the reference's source or injected-noise claims. Existing cutoff
standard deviation is not a demonstrated measurement of tape wow/flutter.
Source labels require independently characterized recordings and later grouped
evaluation. Continuous filtered-signal correlation would be separate future work.

AAC's current port covers the reference long KBD window only. Exhaustive phase
search, other window geometries and explicit TNS/SBR support would require
separate resource and false-hit validation. Do not retune the provisional
threshold simply to hide the recorded trim miss. Neither a successful Python
port nor synthetic encoder controls calibrates statistical thresholds.

Other remaining areas: analog profiles, source bit-depth
inference, validated frequency mirroring,
defensible MQA confirmation, additional format support,
aggregation and independent grouped corpus evaluation. Android bindings, NDK
linking, device resource/lifecycle checks and UI follow the core acceptance gates.

## Portability and backup status

This Rust project root is now a **local Git repository on `main`**, initialized
on 2026-09-30. Its first commit is `chore: checkpoint validated Rust core v0.3.0`.
Use `git log --oneline` to find the checkpoint and `git status` to inspect later
changes. The source-only publication target is
`git@github.com:spideyonmoon/audio-forensic-rust.git`, remote branch `main`.
Before v0.11, remote `main` was verified at `376dc6783df0791521cb8d821dacb0932c86a649`.
The separate local ref `publish/source-only` holds publication history; compare
`git rev-parse publish/source-only` with `git ls-remote` to verify the latest push.
Never push the development `main` history: it includes excluded test files.
The publication tree contains only
production Rust, manifests, CI and Markdown documents; it intentionally omits
tests, validation scripts, generated fixtures, `reference/`, `corpus/`, build
artifacts and other work files. The local checkout still retains those files
and earlier development history; uncommitted tests/scripts are local files, not
included in the source-only Git backup. The nested Python reference clone remains
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
