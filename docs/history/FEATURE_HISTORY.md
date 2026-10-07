# Feature history through 2026-10-06

Archived README introduction. Current dispatch and launch scope are in the root
ROADMAP.md and HANDOFF.md; DSD was subsequently deferred.

# Audio Forensic Rust

**Current delivery plan:** finish the independent Audio Forensics Rust library/CLI
through P09, then build **Alfred**, a file/folder-first offline Android workspace
that consumes it. Audio Forensics is one feature; Spectrogram and Audio Compare
are independently reachable workflows. Shared app input/jobs/storage will live
outside this core, initially here and later in Alfred's own repository. Tag Studio,
Converter and Archival Tools remain future placeholders. See
[product architecture](../../ALFRED_ARCHITECTURE.md); endgame research remains separate. Start with
[ROADMAP.md](../../ROADMAP.md), [task cards](../../ROADMAP_TASKS.md) and the
[owner checklist](../../OWNER_CHECKLIST.md). Native ALAC/M4A joins WAV/FLAC in 0.28.0;
required DSD support remains F02. P01 is complete:
[Python fidelity map](../../PYTHON_PARITY.md) and [release contract v1](../../RELEASE_CONTRACT.md)
freeze required outputs, explicit deviations and the DSF/uncompressed-DFF
DSD64/128/256 scope. P02 implements bounded metadata and ReplayGain;
P03 adds reference byproduct measurements and native level display; P03a adds
bounded astats/SoX statistics and distinct DR. P04c adds separate reference input adapters;
P04a/P04b add the remaining spectral/source inputs. P05 adds the versioned
reference assessment; DSD format support remains F02.

P04a/P04b add version-2 reference spectral and source-profile inputs: banding,
side/LPF/DSD-like observations, ordered resampling, integrity observations,
capped FFT-filtered noise, transient candidates, quiet depth profiles and
adaptive-wall dependencies. See [profile validation](../validation/REFERENCE_PROFILES_VALIDATION.md)
and [v2 schema](../../schemas/reference-inputs-2.schema.json). These remain uncalibrated
inputs; P05 owns interpretation. The worst-case 384kHz profile is memory-heavy;
its explicit bounds are documented, and phone resource validation remains A07.

Version **0.31.0** adds the separate [reference assessment](../validation/REFERENCE_ASSESSMENT_VALIDATION.md).
`reference_assessment::assess_reference(&ReferenceAnalysis)` interprets bound
P04 v2 inputs without decoding again. It exposes scores, all 34 rule traces,
qualified source/depth candidates and exact legacy labels in audit fields.
Scores are **uncalibrated method outputs**, not probabilities or proof of
recording history. Missing applicable inputs produce a partial assessment with
null composite scores. Measurement ancestry stays INCONCLUSIVE/index null.
The saved-result example is `read_reference_assessment SAVED_REFERENCE_JSON`;
P07 still owns product CLI/report/comparison workflows.

An offline Rust library and CLI for audio forensic analysis. Version **0.30.0**
adds a complete [calibrated Rust PNG spectrogram](../validation/SPECTROGRAM_VALIDATION.md)
over P06's unchanged version-1 data.
`analyze_path_with_spectrogram` / `analyze_source_with_spectrogram` return
unchanged native measurement plus a separate version-1 mono/stereo-mid artifact:
at most 1280 × 513, explicit time/frequency axes and power units, shared prefix,
PCM hash and cancellation. Streaming power buckets coarsen without trusting
duration headers. `render_canvas` / `write_png_new` draw the filename or supplied
title, codec/rate/depth/audio bitrate/channels, Hann/hop, kHz and M:SS axes/grid,
ember-v1 0 to −140 dB legend and labeled native-channel p95 cutoffs entirely in
Rust. PNG is lossless RGB8 with embedded antialiased Noto Sans and 300-dpi metadata:
1600×900 standard, **2560×1440 default**, or **3840×2160 large**. Higher pixel
resolution improves presentation; FFT resolution remains rate/1024 Hz.
Bitrate is the encoded audio average over complete packets inside the analyzed
interval; old saved wrappers show unavailable bitrate. Prefix and stereo-mid
scope remain visible. Caller-selected create-new paths prevent overwriting.
Raw RGB/PPM remain available. Artifact failure preserves valid native measurements.
P07/A06 consume the engine canvas and own workflow, storage and sharing.

```text
cargo run --locked --offline --no-default-features --example read_spectrogram -- input.flac full new-spectrum.png
cargo run --locked --offline --no-default-features --example read_spectrogram -- input.flac full new-spectrum-large.png large
cargo run --locked --offline --no-default-features --example read_spectrogram -- input.wav 60
```

Version **0.28.0**
adds [native ALAC/M4A](../validation/ALAC_VALIDATION.md): 16/24-bit mono/stereo at 8–384 kHz,
nonfragmented unencrypted input, moov before/after mdat, bounded metadata and
exact integer PCM through the existing path/source APIs. AAC in M4A stays
unsupported. Decoder/container limits and actual checks are in the validation
record; Android linking and device behavior remain later gates. F01 acceptance
completed **2026-10-06**; see [F01 results](../../task-results/F01.md).

Version **0.27.0**
adds [reference spectral/source-profile inputs](../validation/REFERENCE_PROFILES_VALIDATION.md).
`analyze_path_with_reference_inputs` / `analyze_source_with_reference_inputs`
return native measurements plus separate version-2 inputs on the pinned f32
mono/mid/side basis: base STFT features, sampled scatter, qualified phase,
seeded segment probes and AAC/Vorbis winners. Vorbis searches f64 L/R
reconstructed from f32 M/S, as the pinned source does. A third PCM-verified
pass supplies the final-count-dependent scatter stride; ordinary APIs stay
at two passes. Segment PCM storage is a fixed two-second ring. Missing adaptive
wall dependencies stay null until P04a/P04b supply them. Native applicability,
measurement schema, ancestry and policy stay unchanged; P05 owns interpretation.
`examples/read_reference_inputs.rs` runs without CLI features or runtime tools.

Version **0.25.0** added [pinned tool statistics and DR](../validation/TOOL_STATISTICS_VALIDATION.md).
`analyze_path_with_tool_statistics` / `analyze_source_with_tool_statistics`
return native measurements, P03 byproducts and a separate version-1 statistics
report from the same two passes. Native-lane astats includes 50 ms RMS extrema,
noise floor, amplitude range, flat factor, exact peak/crossing counts and entropy;
SoX stat includes all 15 canonical keys and explicit signed32 conversion;
DR has its own f32 three-second histogram method, per-channel/overall numeric
values and distinct overall versus Python legacy labels. Correct linear/dB
crest units, scope, finite/null reasons and bounded buffers remain explicit.
`examples/read_tool_statistics.rs` needs no CLI features or runtime tools.
The native measurement schema/policy and ancestry remain unchanged. P07 owns
CLI/report exposure; these statistics do not establish calibrated authenticity.

Version **0.24.0** added [separate reference byproducts](../validation/BYPRODUCT_VALIDATION.md): mean valid 100 ms
phase correlation, fixed 16-bit-ceiling sample counts, mid silence runs and
positive-block RMS p5 fallback, alongside native level/crest display and fixed
−16/−14 LUFS deltas. `analyze_path_with_byproducts` /
`analyze_source_with_byproducts` collect them inside the existing two passes,
worker, progress and cancellation contract. `examples/read_byproducts.rs` runs
without CLI features and returns both the original measurement report and a
separate version-1 byproduct report, with exact PCM/scope binding and bounded
section/RMS storage. Legacy phase/clipping diagnoses are audit wording;
ordinary summaries stay descriptive. Native channels/meters and measurement
JSON/policy remain unchanged; P03a statistics have their own versioned result.

Version **0.23.0** added the [bounded metadata API](../validation/METADATA_VALIDATION.md): raw FLAC/WAV text,
ordered duplicates, declared technical fields, artwork descriptors, editable
encoder/MQA claims and an explicitly attributed legacy ReplayGain comparison.
`metadata::read_metadata_path` / `read_metadata_source` decode no audio;
`examples/read_metadata.rs` runs without CLI features. Metadata/audit JSON uses
separate [versioned schemas](../../schemas/README.md); measurement JSON is unchanged.
P07 will add the product CLI/info workflow.

Version 0.22.0 added
**frozen grouped evaluation** of provisional AAC/Vorbis patterns through the
Rust `evaluation` API and the CLI-free `evaluate_reports` example. Declared
labels, source groups and splits stay explicit; abstentions stay in denominators
and unknown histories stay outside labeled rates. See the
[evaluation contract](../validation/EVALUATION_VALIDATION.md). Calibrated classification still
requires independently reviewed controls and evaluation.

The first [characterized generated cohort](../validation/GROUPED_CONTROLS_VALIDATION.md) now
contains six procedural source groups and 48 known-chain audio variants, with
independent PCM characterization and a frozen 96-case AAC/Vorbis manifest.
Development/validation results and reserved-test handling are recorded there.
These synthetic controls do not establish real-music accuracy.

Version 0.21.0 added **grouped evidence interpretation** through `assess_evidence` and CLI `--summary`.
Related observations stay together, matching methods are listed once across
channels/bases, and abstentions remain explicit. Android is authorized by the delivery roadmap; MQA has an independent prototype with
[acceptance/integration still pending](../../task-results/P01-MQA.md). Background jobs, live progress and FLAC/WAV integrity checks remain
available. Every report keeps
ancestry `INCONCLUSIVE` and the evidence index `null` until the detector suite and
aggregation policy are implemented and validated.

Use `audio-forensic --progress --json input.flac` for live progress on stderr
and the unchanged JSON report on stdout. Path-based library callers can use
`analyze_path_with_progress`; app-provided sources use `analyze_source_with_progress`.
Counts describe individual decode passes, not an overall time percentage.

Use `audio-forensic --summary input.flac` for grouped non-MQA findings and
inference limits. Existing `--json` output is unchanged. The
[interpretation contract](../EVIDENCE_INTERPRETATION.md) separates implemented
measurements/grouping from still-unvalidated source-history claims. The
[saved-report example](../../examples/explain_report.rs) interprets existing reports
without decoding the recordings again.

Continuing in a new session or tool? Read [HANDOFF.md](../../HANDOFF.md) for the current
state, checks, known issues and next task, and [AGENTS.md](../../AGENTS.md) for project
instructions. These files are maintained alongside the code.

For host applications, start with [CORE_INTEGRATION.md](../CORE_INTEGRATION.md)
and the runnable [background consumer](../../examples/background_analysis.rs), which
builds with `--no-default-features`. The current scope is an offline WAV/FLAC/ALAC-M4A
measurement component. Calibrated source-history classification is separate
research; it is not required to expose the existing measurements in an app.
Android binding/link and device validation remain outstanding.

The current [JSON report schema](../../schemas/analysis-report-0.18.0.schema.json)
preserves required nullable fields and the observations-only policy. Its
[usage guide](../../schemas/README.md) explains strict producer validation and exact
integer handling. [Report contract validation](../validation/REPORT_SCHEMA_VALIDATION.md)
records 125 passing reports across all file outcomes and 53 contract controls;
the Rust API and detector suite remain under development.
The [source/batch validation record](../validation/SOURCE_API_VALIDATION.md) documents the
0.18.1 fixes, generated failure controls and container mutation campaign.
The [WAV validation record](../validation/WAV_FORMAT_VALIDATION.md) defines the tested PCM
support matrix and explicit format limits in 0.18.2.
The [FLAC validation record](../validation/FLAC_INTEGRITY_VALIDATION.md) documents generated
continuity, framing, unknown-total/checksum and compression controls in 0.18.3.
