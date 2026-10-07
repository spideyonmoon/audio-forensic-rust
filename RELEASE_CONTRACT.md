# Audio Forensics faithful-release contract v1

Frozen by P01 on 2026-10-05. This is a delivery specification, **not implemented
support or a passed release gate**. Engine at the P01 freeze was 0.22.0. The source
inventory, row owners and acceptance oracles are in [PYTHON_PARITY.md](PYTHON_PARITY.md).
Subsequent acceptance: [P09 accepted scoped engine 0.32.0 on 2026-10-06](docs/validation/CORE_RELEASE_VALIDATION.md#p09-decision--accept-2026-10-06),
including the launch-scope amendment below. That decision does not change the
frozen numerical contract or authorize publication.
The pinned reference is `c6ecce2296256b516709d87088896d1be913908c`.

## Product-boundary clarification — 2026-10-06

This frozen numerical/format/workflow contract belongs to standalone Audio
Forensics and its Alfred feature integration, not to Alfred as a whole. P09 ends
standalone delivery; A01 begins Alfred afterward. Spectrogram and Audio Compare
remain required existing-functionality workflows, independently reachable in
Alfred (A06a/A06b). These are initial integration slices, not the full Alfred
Spectrogram/Compare product requirements. Spectrogram's broader viewing and
exploration ambition is not limited to image generation. A01 defines long-term
ownership of reusable P06/P07 code/contracts at Alfred extraction; reuse existing
implementations now without duplication/refactoring. Expanded requirements are
future work and do not change this frozen initial contract or launch acceptance.
Tag Studio, Converter and Archival Tools remain future scope.
See ALFRED_ARCHITECTURE.md. Completed evidence and D01–D12 are unchanged.
P07 uses `audio-forensic-product-v1` (replacing the proposed `alfred-product-v1`
identifier); it names a forensic product envelope, never a universal
Alfred workspace/result schema. P07 documented that final host-neutral identifier
and consumer dispatch in docs/PRODUCT_REPORT.md.

## Initial-launch scope amendment — 2026-10-06

Owner approved deferring F02/DSD beyond the first standalone release and Alfred
launch to conserve Astra budget. Initial required analysis formats are FLAC,
WAV and ALAC/M4A. P07 → P08 → P09 closes that release; F02 is not an initial
core/app gate. P08/P09 must identify native DSD parity as deferred and verify
honest unsupported outcomes, never claim conversion/DSD support.

This is delivery scope revision 2 of the P01 contract. It does not change
`contract_version=1` in completed reference methods, numerical tolerances,
rule semantics, schemas or historical acceptance. The future DSD specification
below remains frozen for a later named F02 packet and its own release acceptance.
DSD-like observations in PCM remain implemented and are distinct from native DSD.

## Release scope

The first useful release provides offline import, full/prefix analysis, a detailed
reference-method verdict, all useful metadata and measurements, source/depth
candidate interpretations, spectrogram, history, comparison and export. Python,
FFmpeg, SoX, MediaInfo, network access and accounts are not runtime prerequisites.
Development oracles may use the pinned Python and independently versioned tools.
P09 accepts the core; A07 accepts actual Android/device behavior; A08 prepares a
release for owner-authorized publication. No E-task is a beta dependency.

| Input | Required first-release scope | Outside v1 / structured unsupported |
| --- | --- | --- |
| WAV | Existing validated RIFF little-endian PCM 8/16/24/32, float32/64, supported extensible precision/masks; mono/stereo, 8–384 kHz | RF64/BW64, RIFX, compressed WAV, surround and unsupported extensible layouts; retain WAV_FORMAT_VALIDATION.md conditions |
| FLAC | Native FLAC, mono/stereo, 8–384 kHz, required 16/24-bit integer precision; retain existing additional accepted precision plus framing/CRC/MD5 and unknown-total behavior | Ogg FLAC, multichannel and decoder-unsupported precision/block geometry; do not infer support from the extension |
| ALAC/M4A (F01) | Unencrypted nonfragmented ISO BMFF/M4A carrying actual ALAC; mono/stereo 16/24-bit; 8–384 kHz; seekable app source; moov before or after mdat | AAC/Opus in M4A, DRM, fragmented MP4, other precision/layouts, unrelated video/multiple-audio selection ambiguity |
| DSD (F02, deferred beyond launch) | DSF and uncompressed DSDIFF/DFF; mono/stereo; DSD64 2,822,400, DSD128 5,644,800 and DSD256 11,289,600 bits/s/channel; native bit order/packing and channel layout verified | DST-compressed DFF, SACD ISO, DoP, DSD512+, 48-kHz-family DSD rates, multichannel; distinct diagnostics |

The DSD scope is the P01 engineering selection implementing the owner's broad
DSD requirement; it is not a claim that the owner supplied examples at each rate.
For its later DSD release, F02 must deliver all six container/rate combinations
and both channel counts or record a blocker for that release. Other Python formats (MP3, AAC, Ogg/Vorbis, Opus, WMA,
APE, WavPack, AIFF, standalone ALAC and arbitrary FFmpeg containers) are later
decoder work, not implicit beta gates. Identification/metadata may succeed on an
unsupported codec, but no successful audio analysis or invented reference score
may follow. File-size support must include a 700 MiB generated input without
whole-file memory buffering; A01 sets staging/storage quotas, A07 tests them.

Future F02 DSD analysis uses explicit converted floating PCM at **88,200 Hz**, preserving
native mono/stereo. F02 freezes and versions the filter coefficients, gain,
decimation stages, delay compensation, startup/tail treatment and independent
oracle before implementation acceptance. This algorithm choice belongs to F02;
changing the frozen container/rate/output-domain scope requires a contract revision.
Native bitstream hashes/packing checks are exact. Converted PCM is not exact
integer PCM and cannot establish original recording precision. Integer-bit/MQA
tests and the PCM lossy-ancestry score are inapplicable. Display native DSD
identity and available descriptive converted-domain levels/spectrum/source
profile candidates with the conversion caveat. Never translate Python's native
DSD `GENUINE` branch into verified provenance. Wall/noise-shaping patterns in a
PCM file are separate from a natively parsed DSD stream.

## Three result layers

1. Existing `AnalysisReport` measurement JSON stays unchanged for existing
   WAV/FLAC clients (`schema_version=0.18.0`, `observations-only-v18`), with
   ancestry `INCONCLUSIVE` and evidence index `null`. Existing `assess_evidence`
   version 1 remains the unscored grouping layer, not the new assessment.
2. New optional product output is a separately versioned envelope. Implemented P07
   schema ID `audio-forensic-product-v1`; required members: `product_schema_version`,
   `engine_version`, `contract_version`, `measurement_report`, `metadata`,
   `reference_inputs`, `reference_assessment`, `artifacts`, `diagnostics`.
   Unknown versions are rejected explicitly. Existing `--json` semantics stay;
   P07 supplies a separate `--product-json` workflow. New format requirements
   that cannot fit the strict measurement schema use a **new measurement schema**
   and validator with explicit dispatch; never relabel widened DSD/ALAC reports
   as 0.18.0 or silently loosen its existing PCM/coverage constraints. Codec is
   currently a string, not a WAV/FLAC-only schema enum; F01 may reuse the schema
   if all its field semantics remain unchanged.
3. `reference_assessment` uses `assessment_version=1`,
   `method_id=python-reference-c6ecce2-v1`, `contract_version=1`,
   `calibration_status=uncalibrated`, and the exact reference commit. Its input
   binding includes measurement version/policy, source PCM hash and coverage,
   adapter versions and all feature IDs. No live recomputation from a different
   file is allowed when interpreting a saved result.

The assessment contains `status` (`available`, `partial`, `inconclusive`,
`unsupported`, `failed`, `cancelled`, `timed_out`), nullable `scores`, nullable
`reference_label`, qualified `display_summary`, `source_candidates`,
`depth_candidates`, `rules`, `deviations`, `missing_inputs`, and `caveats`.
`scores` has integer lossy/natural/net/max/heuristic/main values and optional
raw-lossy percent; legacy `net_confidence_pct` is retained only under explicitly
named `legacy_outputs`, never a confidence/probability gauge. All source/depth
results are reference-method candidates. Raw Python wording/labels belong to
the audit export with their qualification, not an unqualified app headline.

Each feature has `id`, `version`, `status`, nullable `value`, `unit`, `domain`
(`native_integer_pcm`, `native_float_pcm`, `reference_mid`, `reference_side`,
`dsd_converted_pcm`, `metadata`), channel indices, half-open frame intervals,
sample rate, `algorithm`, and deviations/caveats. Available false/zero is
different from absent. Each rule trace has a stable ID, Python symbol/source
span, input feature IDs, predicate operands and thresholds, state
(`applied`, `not_triggered`, `vetoed`, `inapplicable`, `missing_input`,
`excluded_by_contract`), effect (`add`, `subtract`, `floor`, `clamp`, `override`,
`label_only`), before/after scores and the causal veto-rule ID where relevant.
Scores and labels are exact deterministic rule outputs, not floating approximations.

Evaluate only applicable branches. Missing input on a branch that could change
the result makes the composite score/label null (`partial`); completed independent
measurements/candidates remain visible. A failed/unsupported/cancelled/timed-out
file cannot emit a successful assessment from stale fields. A known excluded
rule is traced and does not force permanent partial status: this is the specified
modified reference policy, not an undisclosed zero. Missing P01-required
implementation is a release failure, not permission to ship every file partial.
Unknown detector/adapter versions and unequal domains must not be substituted.

## Sampling and resource semantics

The selected stream supplies both samples and technical metadata. Default mode
uses the whole selected stream; fast means `[0, min(60 s, EOF))`. Every extractor,
artifact and adapter observes this interval. Current `--max-seconds` remains.
No midpoint-only bit-depth pass or full-file loudness/spectrogram escapes a
requested prefix. Prefix output never claims full-file checksum/header integrity.
Integer PCM remains exact and MSB-aligned for checks/hashes. Float nonfinites
fail explicitly; do not clip valid finite out-of-range float samples silently.

Native channel measurements always remain. For reference spectral/byproduct
inputs, stereo is `mid=(L+R)/2`, `side=(L-R)/2` after the pinned f32 conversion and
operation order; mono uses channel 0. Report mid cancellation explicitly: an
antiphase pair must not become a genuine verdict merely because mid is silent.
No multichannel downmix fallback. P04c supplies this separate adapter without
changing the native DSP. Base reference STFT uses symmetric f32 Hann 4096,
hop 2048, strict frame end `< length`, pinned activity mask and thresholds.
Units remain exact: magnitude ratio is not power ratio; raw FFT dB is not dBFS.

Reference transform/time-domain coverage is capped at the pinned 180 seconds;
cassette hiss uses 60 seconds, bit/noise-floor profile the first 30 seconds
(300 complete blocks), all clipped to the shared interval. Whole-stream effective
bits stay separately available. Profiles retain original block indices after
zero removal. Python's indexed-zero bug is never emulated. Segments use pinned
duration-adaptive 9–36 two-second probes and seeded offset selection in P04c;
store offsets/counts, not a full PCM array. Reference scatter subsampling/mode,
phase pairs, side stride, Vorbis best reconstructed L/R (mono M) and AAC
selection must each be explicit, not aliases for native measurement records.
P04c source inspection corrected the former “best mid/side” wording on
2026-10-05: the pinned code widens f32 M/S before reconstructing Vorbis L/R.

No duration-sized PCM/spectrogram retention. Fixed FFT/bin accumulators, bounded
captures and extra cancellable passes are permitted. P03a/P04b must publish
worst-case scratch bytes at 384 kHz and their bounded selection/histogram or
temporary-storage strategy **before** accepting new algorithms. Approximation
requires a named deviation, bound and independent oracle; no quiet switch from
whole-cap FFT filtering to circular STFT correlations. Source blocking-I/O and
cooperative deadline limits remain as documented in SOURCE_API_VALIDATION.md.
One active core analysis per process; no Python extractor fan-out or automatic
three-file concurrency. Progress is stage/pass/frames, not fabricated ETA.

## Metadata and artifacts

P02 preserves named fields and raw unknown text tags, original keys, duplicates,
ordering/source container and truncation status. Limits: 1,024 tag entries,
256 UTF-8 bytes/key, 16 KiB/value and 1 MiB total exposed text per file. Truncate
only on UTF-8 boundaries and report omitted counts/bytes where known; do not
pretend this is complete text when a limit fires. Artwork is a descriptor
(type, dimensions when safely parsed, byte length), not embedded base64.
Container technical fields distinguish declared versus measured/derived values,
codec versus extension, duration versus analyzed span, unknown versus zero.
Larger bounded preflight limits may reject a container before tag extraction;
retain those structured resource failures. Tags never change the lossy score.

P06 produces at most 1280 time columns × 513 frequency rows per displayed basis,
with rate, FFT/window/hop, actual frame bounds, axes, unit, reduction and color
scale in the descriptor. v1 display basis is mono or stereo mid, explicitly
labeled, with an antiphase warning and native measurements still available.
Use streaming linear-power averaging into time buckets (coarsen/merge buckets
when duration is unknown); display dB with a declared reference/floor, not a
claim of SoX pixel parity. Test tone frequency within one bin and event position
within one output bucket. Pixel colors/typography can differ. The owner's
2026-10-06 P06 follow-up requires a complete Rust-rendered lossless PNG canvas:
title/filename, codec/rate/depth/encoded audio bitrate/channels and FFT header,
kHz through exact Nyquist and M:SS through analyzed end, ticks/subtle guides,
ember-v1 0 to −140 dB legend and labeled native-channel p95 cutoff overlay.
Missing bitrate/cutoff remains unavailable, prefix/mid scope is visible, and
distinct native cutoffs are never averaged into a mid value. Presentation
version 1 is separate from unchanged spectral artifact v1/numerical geometry.
Use bounded 1600×900 / 2560×1440 default / 3840×2160 RGB8 canvases, antialiased
embedded font and 300-dpi metadata. Higher pixel count does not add measured
frequency/time detail. P07/A06 own path/storage/share workflows and consume the
Rust PNG; this revises the earlier app-owned composition/encoding decision.
Exports are collision-safe; do not overwrite a neighboring user's image.
No artifact failure may masquerade as an existing path or erase a valid report.

P07 provides ordinary text, metadata-only (actually no audio decode), mixed-success
batch/directory input, fast/full, progress/cancel, saved-product-report rendering,
product JSON, comparison and artifact export. Preserve current measurement JSON
and exit behavior. Fail one file without suppressing later files. Comparison is
explicitly for variants of one track and uses the pinned tuple: availability,
main score ascending, rate-history flag, cutoff descending, depth category,
DR descending, precision × rate descending; stable input order breaks ties.
Expose the tuple and missing fields. Do not rank unlike method/domain/coverage
versions together, or name a winner when all candidates are unavailable. The
headline is “reference-method ranking,” never “most authentic” or best sound.

## Fidelity and numerical acceptance

Existing per-method tolerances in the validation documents and checked-in tests
remain binding for their existing geometries. They are not evidence for new
channel bases. New matching-scope adapters use the pinned source as oracle:

| Quantity | Acceptance bound / oracle |
| --- | --- |
| PCM, counts, sample indices, gates on exact vectors | Exact independent integer PCM/hash, exact counts/intervals |
| Base cutoff / variance | 0.01 Hz / 0.01 Hz² on equivalent frames |
| Base dB / magnitude ratio / entropy | 0.02 dB / 1e-6 absolute / 1e-5 bits |
| Resampling bands / correlations | 0.05 dB (0.2 below −120 dB relative) / 0.003; applicability gates preserved |
| Segment clip levels/cutoff | Existing DETECTOR_VALIDATION tolerances; exact offsets/vote logic; new void/adaptive paths compared separately |
| AAC lattice / anchors | 1e-12 score; exact anchors except documented equal-energy tie rule; RMS 1e-10; MDCT 1e-8, KBD 2e-14, gamma 1e-13 |
| Vorbis | 1e-4 score, exact support/tested counts on equivalent basis/probes |
| Reference scatter / phase | One FFT bin for mean/mode bounds; 0.001 bits for phase on identical eligible pairs; exact selection/histogram counts |
| New banding / ratios / signed correlations | 1e-3 absolute; exact discrete side/LPF/DSD decisions on fixed feature vectors |
| New reference profile band levels / event percentages | 0.05 dB / 0.1 percentage point; exact event counts on isolated events away from thresholds |
| Quiet/profile percentiles | Existing NOISE_FLOOR_VALIDATION oracle for corrected indices; P04b exports equivalent mid-domain values separately |
| SoX/astats scalar statistics | 1e-5 absolute linear values, 0.05 dB levels, exact integer counts; pinned tool output precision respected |
| DR | Numeric DR within 0.1 dB of frozen drmeter, displayed integer exact away from integer boundaries; it is not LRA or crest |
| LUFS/LRA/true peak | Existing LOUDNESS/LISTENING_LEVELS validation bounds; named native-meter difference from Python's 48 kHz FFmpeg path, not asserted bit parity |
| Rules, labels, ranking and formatted thresholds | Exact feature-vector branch decisions, Python ties-to-even rounding and rule order |
| DSD conversion | F02 independent same-filter reference: max absolute 1e-6 normalized amplitude after documented alignment; independent alternative-filter comparison additionally reports passband/alias/delay differences, never exact PCM claim |

New profile tolerances are acceptance targets, not results. A measured discrepancy
must be explained with a reproducer; no widening/regeneration merely to pass.
At floating threshold boundaries test rule logic with supplied exact vectors on
both sides/equality; a feature error within tolerance does not silently excuse a
different verdict. Classify and version any necessary numerical deviation.
P08 freezes generated fixture/tool versions and runs the finite row suite plus
final regression once; old reports do not prove new adapters.

## Explicit policy deviations

| ID | Frozen disposition | Owner/check |
| --- | --- | --- |
| D01 | Preserve native measurements; add separate reference mid/side inputs and versioned adapters, never average detector outputs | P04c; antiphase/dual-mono/channel-isolation controls |
| D02 | Shared prefix, first-30-second depth profile, bounded output/storage, one worker; correct silence-percent denominator to analyzed duration | P03/P04b/P07; prefix/cap/resource tests |
| D03 | Preserve nonzero block indices and native numerical applicability; do not port fabricated clear zeros, empty-band fallbacks or quiet-bin correlation | P04b/P04c/P05; silence/zero/energy-gate controls and partial trace |
| D04 | Keep native loudness/true-peak estimates with explicit method instead of claiming FFmpeg resample/EBU equivalence; expose actual drmeter separately; correct Python’s mislabeled linear crest field with explicit units | P03/P03a; independent level/DR fixtures |
| D05 | Exclude negated absolute band-correlation aliasing score (+10/+15) and claimed frequency mirroring; optional descriptive correlation must not carry that name | P04b/P05; excluded rule trace; E08 remains separate |
| D06 | No MQA certainty/100 override, studio/OSR confirmation, or `detected` claim from one sync; preserve tags and candidate bit/payload locations with unknown state | P02/P05/P07; candidate controls; existing prototype acceptance E09 pending |
| D07 | Header duration mismatch stays integrity evidence, never adds +20; header bitrate ratio/extension rule never adds +25 or proves forgery | P04a/P05; padding/artwork/short-prefix fixtures; preserve raw hypothetical rule in trace |
| D08 | Known lossy identity requires parsed codec, not extension; categorical identification separate from ancestry; native DSD skips PCM ancestry | F01/F02/P05; renamed-file/mixed-codec tests |
| D09 | Preserve all other pinned weights/vetoes/floors on equivalent inputs, but attribute “fake hi-res,” vinyl/cassette, injected noise, depth and no-indicator labels to an uncalibrated reference method | P05; every rule branch and gate; no certainty or source proof |
| D10 | Cassette public candidate requires score ≥30 AND hiss, matching analyse veto; expose the old score-only build_report boolean only in legacy audit | P05; ≥30 without hiss fixture |
| D11 | ReplayGain uses the pinned −18 + stored comparison as a legacy audit, not a new standard-compliance check or evidence of re-encoding; normalization deltas are fixed −16/−14 method targets | P02/P03; ±gain, malformed, exact 1/3 dB boundary fixtures |
| D12 | Metadata bounded with lossless keys/duplicate retention within limits; actual no-decode info; spectrogram visual replacement; stable qualified comparison and null unavailable winner | P02/P06/P07; metadata/workflow fixtures |

The pinned `crest_factor_db` field reads astats “Crest factor” without a log
conversion. A one-second 1 kHz/48 kHz FFmpeg 7.1.1 probe returned 1.414182,
while peak minus RMS was 3.010106 dB. Preserve the raw linear legacy value in
audit and display `20*log10(crest)` in dB with a correctly named field (D04).
Do not port the mislabeled number or its “compressed” quality assertion.

D07 deliberately changes the legacy score because container size includes metadata
and decode failure is not ancestry. It is recorded here before P05 rather than
silently dropped later. D05–D08 excluded rule effects must remain inspectable.
All other pinned source-profile rules, including conditional clean-silence credit,
are implemented as attributed candidate rules, not deleted pending E06/E07.

## Independent MQA candidate available

The owner supplied an existing Rust multi-packet scanner and saved local evidence
at `C:\Users\Bishal\Documents\antigravity-dev\mqa` during P01. See
[the intake record](task-results/P01-MQA.md). E09 now reviews that candidate and
its evidence; no missing-recordings assumption or rewrite from scratch remains.
This v1 contract preserves current production candidate output until acceptance
and versioned integration. Structural signalling recognition, if accepted, must
stay separate from decoded payload provenance claims, calibrated probability
and the excluded Python main-score override. It is not a beta blocker.

## Proposed checkpoint policy — pending owner approval

Do not commit, push or publish under this proposal yet. At each DONE packet,
review the diff and exact checks, then create one local development checkpoint
including source/docs/tests/generators but no private recordings/reports/secrets.
Record its SHA in the task result. Keep `main` development history local: it
contains excluded fixtures and must not be pushed to the public remote.
For an explicitly authorized source publication, prepare/review the existing
`publish/source-only` tree with the established allowlist, compare remote state,
then push only that publication history. Do not overwrite/rebase user work.
If the owner wants generated tests publicly available, separately approve a
publication-policy revision first. Ignored evidence requires an owner-controlled
external research backup and restore check; a commit or same-drive copy is not
that backup. U05 remains outstanding. This packet requests no overnight decision.
