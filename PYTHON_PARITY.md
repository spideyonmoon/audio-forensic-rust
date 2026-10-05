# Python/Rust fidelity map — P01

Frozen 2026-10-05 against Python commit
`c6ecce2296256b516709d87088896d1be913908c` and the inspected local Rust 0.22.0
working tree (which includes earlier uncommitted work). The Python checkout was
clean. This map records **current coverage and required work**, not completed
implementation. [RELEASE_CONTRACT.md](RELEASE_CONTRACT.md) freezes release
scope, domains, deviations D01–D12, result shape and tolerances.

Status: **E** = exact/discrete or numerical parity on the cited existing
matching-scope tests; **D** = intentional difference, not end-to-end Python
parity; **M** = missing. A mixed row states which parts exist. Every field and
top-level/class method has an inventory entry below; internal nested helpers
inherit their enclosing method. Each inventory row resolves through its group
to current Rust, tests, disposition, finite task and acceptance oracle. Existing
tests are historical evidence and were not rerun by this documentation packet.

## Implementation and acceptance groups

| Group | Python behavior / current Rust and evidence | Current status and required disposition | Owner; finite acceptance oracle |
| --- | --- | --- | --- |
| G01 | Tags, technical metadata, encoder traces: `extract_mediainfo`, `detect_encoder_trace`; `src/metadata.rs`, `src/container.rs`; `tests/metadata.rs`, METADATA_VALIDATION.md | P02 implemented bounded FLAC/WAV text, raw duplicates/keys, declared technical fields, descriptors and unscored signatures; D12 native projection retains ReplayGain/IART that MediaInfo 24.01's pinned extractor loses. Opaque ancillary structures stay explicit; additional formats pending | P02 DONE, F01/F02 container hooks; generated named/unknown/duplicate/Unicode/oversized/artwork controls passed, equivalent pinned extractor/encoder fields and independent declarations checked; finite coverage in validation doc |
| G02 | ReplayGain `audit_replaygain`; `src/metadata.rs::{audit_replaygain,audit_metadata_replaygain}` | E finite legacy arithmetic/format/strict gates on 21 frozen Python vectors; D11 qualified summary, explicit invalid nonfinites and actual native-meter prefix/hash binding (D04); no standard/re-encoding claim | P02 DONE; positive/negative/malformed/missing gains and exact 1/3 dB boundaries passed against pinned pure function; integrated generated native prefix audit passed |
| G03 | `PcmStats`, `native_level_display`, `src/tool_statistics.rs`; native/byproduct/statistics tests | E native PCM/display units and fixed scope; D native channels/meters versus signed-maximum astats normalization. Correct linear/dB crest and distinct tool overall/native mappings; raw mislabeled crest retained only in legacy audit (D04) | P03/P03a DONE; native controls plus 28 pinned tool recipes, units/nulls/rounding and exact report/PCM checks passed |
| G04 | `src/tool_statistics.rs`, `tests/tool_statistics.rs`, `scripts/check_tool_statistics.py`, separate schema 1 | E pinned astats extrema/noise/range/flat/peaks/entropy/crossings and all 15 SoX stat keys, explicit lanes/overall/interleaving/window/tail/precision. D undefined sentinels null with reasons; D12 encoding guesses stay outside measurement keys. `sox_entropy` is astats entropy; pinned AST verifies legacy last-occurrence/absolute-peak regex mixing | P03a DONE; frozen FFmpeg 7.1.1/SoX 14.4.2 source/version/28 generated controls, exact counts/PCM and RELEASE_CONTRACT tolerances passed; see TOOL_STATISTICS_VALIDATION.md |
| G05 | `src/loudness.rs`, `src/loudness_range.rs`, `src/true_peak.rs`, `src/byproducts.rs::native_level_display`; LOUDNESS_VALIDATION.md, LISTENING_LEVELS_VALIDATION.md, BYPRODUCT_VALIDATION.md | D native-rate meters versus Python 48 kHz FFmpeg resample; original method/availability/gates/tails stay explicit (D04). Fixed −16/−14 deltas use unrounded native LUFS, signed one-place text and no current-service-policy claim | P03 DONE; native loudness/listening-level regression, delta/format/missing/short/rate controls and decoded display/schema checks passed; no certified meter claim |
| G06 | `src/tool_statistics.rs::DrLane`, `ToolCollector`, frozen tool fixture and statistics API/schema | E distinct f32 drmeter three-second histogram numeric DR/legacy truncated label; exact-multiple/zero-target/silence null reasons. D product overall mean/label explicitly separate from Python first positive channel label. LRA/crest are not substitutes | P03a DONE; frozen varying-level/unequal-stereo/short/exact/tail/bin-overshoot/prefix controls and integer labels passed; method/input contract in TOOL_STATISTICS_VALIDATION.md |
| G07 | `src/byproducts.rs::ByproductCollector`, separate decode APIs; `tests/byproducts.rs`, `tests/fixtures/byproduct_reference.json`; BYPRODUCT_VALIDATION.md | E pinned f32 mono/mid/side/reconstructed L/R counts, valid 100-ms block mean Pearson, strict −60 dBFS ≥floor(sr/2) quiet runs and positive-block RMS p5. D02 analyzed-duration denominator, bounded sections/scalars with exact totals/resource-limit null. Native whole-stream/precision/quiet measurements unchanged; legacy diagnoses audit-only | P03 DONE; 31 frozen pinned vectors/decoded exact-PCM cases, DC/mono/antiphase/plateau/fade/tail/0-9-10 counts/0.5s equality/prefix, limits/cancellation/serialization/schema passed; fixed-ceiling counter is not a plateau detector |
| G08 | Decode/STFT/basic spectral metrics; `src/source.rs`, `src/decode.rs`, `src/dsp.rs::{StreamingStft,SpectralStats}`; `tests/parity.rs`, `tests/core.rs`; VALIDATION.md | E native mono and new separate f32 reference basis/base inputs in `src/reference_inputs.rs`; D01 native lanes preserved; D03 four-active-frame/null gates | P04c DONE inputs; P04c; existing immutable oracle plus generated unequal stereo/antiphase, exact strict-frame/activity/percentile and active-gap checks; REFERENCE_INPUTS_VALIDATION.md records P04c checks |
| G09 | `_banding_score`, `_side_channel_anomaly`, `_lpf_scan`, `_dsd_scan`; no corresponding Rust outputs | M. Banding is global-region-relative log-magnitude temporal std; side is unmasked stride-4 HF mean ratio; LPF uses ≥90% Nyquist sum and reverse −40 dB scan; DSD scan is a spectral ratio, not native format | P04a; pin exact Python arrays, narrow/missing bands, low rates, dual mono/side-only, banding 0.92, side 0.02/0.08, LPF 0.00005 and DSD 1.5 boundary fixtures |
| G10 | `_check_header_integrity`; actual decoder/frame integrity in `src/container.rs`, `src/decode.rs`, `src/flac_frame.rs`; `tests/flac_integrity.rs`, `tests/wav_formats.rs` | D integrity is stricter/structured; M separate reference duration/bitrate observations. D07 removes ancestry penalties/forgery claims | P04a; >0.5s duration and 0.80/1.35 bitrate vectors, metadata padding and prefix/full mismatches; existing hard failures remain hard failures |
| G11 | `_segment_voting` and `_codec_fingerprint`; `src/detectors/segments.rs`, `tests/detectors.rs`, `tests/fixtures/detector_reference.json`; DETECTOR_VALIDATION.md | E reference duration/Random(42) plan, widened f32 clip inputs and nearest-wall adapter in `src/reference_inputs.rs`; D03 qualified vote plus legacy audit; P04a/P04b adaptive dependencies explicit | P04c DONE inputs; P04c inputs, P05 decisions; exact reference offsets/counts/even-half vote, first/last/random probe, short/silent clips, single/multiple splices, overlapping nearest-wall ties; REFERENCE_INPUTS_VALIDATION.md records P04c checks |
| G12 | `_resample_check`, `_is_fake_hires_bandwidth`; `src/detectors/resampling.rs`, `tests/transforms.rs`; TRANSFORM_VALIDATION.md | E applicable band/mirror arithmetic; D all candidates and quiet-band abstention versus first matching source rate. M reference first-hit and bandwidth-input adapter | P04a; same mid frames, ordered 44.1/48/88.2/96 hypotheses, notch/mirror/wall controls, empty bands and fake-hires gates; preserve D03 |
| G13 | `_aucdtect_features`; `src/detectors/structure.rs`, `tests/structure.rs`, `tests/fixtures/structure_reference.json`; STRUCTURE_VALIDATION.md | E source-stride/f32 moments/20-bin mode and compacted phase audit on shared mid; D03 flat/adjacent energy-qualified product fields retained separately | P04c DONE inputs; P04c; independent Python mean/mode/phase on matched mid, >2500 frames, active gaps and flat spectrum; retain applicability fix with named deviation, record both native and reference values; REFERENCE_INPUTS_VALIDATION.md records P04c checks |
| G14 | `_kbd_window`, `_mdct_batch`, `_mdct_quant_error`; `src/detectors/{mdct,aac}.rs`, `tests/aac.rs`, `tests/fixtures/aac_reference.json`; AAC_VALIDATION.md, AAC_MUSIC_VALIDATION.md | E f32-basis AAC adapter and first M/S available winner; D documented deterministic equal-energy anchor tie and four-probe/band gates preserved | P04c DONE inputs; P04c adapter, P05 gates; f32-basis and winning mono/mid/side mapping, exact counts, threshold 0.06/0.10; insufficient active probes remain null, never zero; REFERENCE_INPUTS_VALIDATION.md records P04c checks |
| G15 | `_vorbis_grid`; `src/detectors/{mdct,vorbis}.rs`, `tests/transforms.rs`, `tests/fixtures/transform_reference.json`; TRANSFORM_VALIDATION.md | E pinned f32 M/S widened then reconstructed L/R (mono M), available winner/tie/support/tested adapter; D03 unsupported/insufficient nulls preserved | P04c DONE inputs; P04c adapter, P05 floor; shared reference basis, winner tie order/support/tested, 0.03 boundary and short/unsupported-rate cases; REFERENCE_INPUTS_VALIDATION.md records P04c checks |
| G16 | `calculate_autocorrelation`, `calculate_temporal_variance`, `_fft_band_extract`, vinyl/cassette noise inputs; `src/detectors/{noise,noise_dynamics}.rs`; `tests/noise.rs`; NOISE_VALIDATION.md, NOISE_DYNAMICS_VALIDATION.md | D windowed band power/circular correlation/per-second STFT history are not capped whole-signal FFT filtering/Pearson. M exact reference-input adapter | P04b; independent filtered time series and Python lag/time variance, constant/DC/quiet bands, 60/180s caps, rate-scaled lags and musical/noise counterexamples; no substitution |
| G17 | `_smooth_envelope`, filter helpers, vinyl clicks, psychoacoustic preceding energy; `src/detectors/{transients,preceding_energy}.rs`; `tests/transients.rs`, `tests/preceding_energy.rs`; TRANSIENT_VALIDATION.md, PRECEDING_ENERGY_VALIDATION.md | D bounded causal measurements, relative thresholds/histogram median/eligible denominator versus reference Hilbert/absolute −3dB/edge denominator. M compatible inputs with edge correction traced | P04b; independent SciPy filters/Hilbert/peaks, isolated attacks/gain/edges; musical attacks prevent physical-click or codec-pre-echo certainty |
| G18 | `_cassette_source` rolloff; `src/detectors/rolloff.rs`, `tests/rolloff.rs`; ROLLOFF_VALIDATION.md | D actual gated endpoint geometry instead of Python −120 placeholders; source score/hiss veto missing | P04b inputs, P05 policy; endpoint same-bin comparison, noise hiss, gentle EQ and cutoff std 30/50/300 boundaries; not physical wow/flutter |
| G19 | `_spectral_sparsity`, `_ultrasonic_envelope_correlation`; `src/detectors/{sparsity,envelope}.rs`; `tests/sparsity.rs`, `tests/envelope.rs`; SPARSITY_VALIDATION.md, ENVELOPE_VALIDATION.md | D energy/bin geometry gates versus reference zero/one fallback; reference adapter missing | P04b; identical active-mid frame arrays, exact below-cutoff counts, coefficient controls and empty/tiny/constant bands; track D03 abstentions |
| G20 | `_psychoacoustic_artifacts` MP3 spectral-lag/comb path; `src/detectors/spectral_lags.rs`, `tests/spectral_lags.rs`; SPECTRAL_LAGS_VALIDATION.md | E supported arithmetic to independent oracle; D full band/variance gates and no outer cutoff gate; missing reference geometry/comb predicate | P04b inputs, P05 gate/+10; exact three lag/neighbour vectors, ≥2 qualifying peaks, 80-bin boundary, EQ/comb negative interpretation controls |
| G21 | `_effective_bits`, `_noise_floor_profile`, `check_bit_depth_authenticity`, `_bit_depth_verdict`; `src/dsp.rs`, `src/detectors/noise_floor.rs`; `tests/core.rs`, `tests/noise_floor.rs`; NOISE_FLOOR_VALIDATION.md | E bit algorithm on exact integers; D native first-300 blocks, corrected zero indices versus midpoint/downmix. M explicit first-30s reference profile/depth rules | P04b inputs, P05 candidate; pad/stray bits/dither/zero removal/antiphase, exact −86/−102/−89 dB and 7dB boundaries; unknown original depth retained |
| G22 | `_score`, `_verdict`, `analyse` rules and all `_interp_*`; `src/evidence.rs` only groups observations; `tests/evidence.rs` is NOT score parity | M. Implement ordered rule ledger below; versioned trace, raw legacy wording and qualified display; no invented weights; D05–D11 explicit | P05; pure feature vectors at every branch/equality/veto/rounding boundary, then finite generated integrated examples; assert measurement JSON unchanged |
| G23 | MQA fields/scanner/override; `src/detectors/mqa.rs`, `src/model.rs`, `src/metadata.rs`, `tests/{detectors,metadata}.rs`; DETECTOR_VALIDATION.md, METADATA_VALIDATION.md and Python `test_mqa.py` | D exact bit candidates/payload observations, no confirmation or 100 override. P02 metadata claims/token boundaries now available separately; production bit scanner unchanged. Do not make test_structural_hit_forces_known_lossy_100 a Rust acceptance expectation | P02 tags DONE, P05/P07 display/excluded trace; generated magic/partial payload/wrong planes retain prior checks, new tag-only/no-score controls passed; E09 existing-prototype review pending, not a release gate |
| G24 | `_psychoacoustic_artifacts` alleged mirror/aliasing subrule | M and excluded as a classifier: absolute correlation ignores negation and does not reverse frequency. D05 excludes both +10/+15; optional descriptive correlation only | P04b/P05; source-inspected algebra, same correlation under sign reversal and explicit excluded-rule trace; no new E08 research here |
| G25 | Spectrogram path/rendering; no Rust artifact; STFT available in `src/dsp.rs` | M, D bounded offline renderer replaces SoX/FFmpeg pixels and output side effects | P06; tone/silence/impulse/prefix axes, bucket reduction/cap/unknown duration, cancellation, collision/no phantom path |
| G26 | `build_report`, JSON/text/by-field status/color helpers, elapsed/file size; `src/main.rs`, `src/lib.rs`, `src/model.rs`; `tests/cli_contract.rs`, `tests/report_contract.rs` | D current measurement-only output; M deep product report and reference interpretation. All aliases retain provenance; no score inferred from observations | P07; generated saved-product report fixtures, missing/partial fields, exact u64, escaping, unchanged old JSON and independent product-schema checks |
| G27 | `_compare_key`, comparison JSON/text and batch summary; current CLI sequential batch only | M comparison/reference batch tables; D qualified ranking/missing winner (D12) | P07; sort tuple fixtures including ties, all unavailable, mixed failures, unequal methods/domains; generated same-track variants; no quality/provenance assertion |
| G28 | `main`, `build_info_report`, directory, fast, workers; `src/main.rs`, `src/job.rs`, `src/progress.rs`, `src/worker.rs`; `tests/{cli_contract,jobs,progress,worker_budget}.rs` | D single worker/structured continuation/progress, M fast alias/info/directory/product export; info actually no decode (Python calls SoX despite README) | P07; no-decode source spy, directory order/empty/missing files, mixed batch exits, fast interval all extractors, cancel; A01–A07 adapt same contracts |
| G29 | External command/tool/TTY infrastructure `_run`, `_pipe_ffmpeg_to_sox`, `_Status`, etc. | D replace subprocesses, dependency probing, thread fan-out and terminal painting with offline Rust lifecycle; numerical semantics belong to respective groups | P07; no external process runtime, progress stderr/JSON stdout, no blocked UI; existing job/progress tests plus new workflow tests |
| G30 | Native DSD special handling, codec/container matrix; current `src/container.rs`/`src/decode.rs` only WAV/FLAC; `tests/wav_formats.rs`, `tests/flac_integrity.rs` | M required ALAC and native DSD; actual scope in RELEASE_CONTRACT; no extension fallback | F01/F02; exact generated PCM for ALAC, DSD packing + independent conversion, malformed/truncation/seek/prefix/cancel; new-schema applicability |

Every group's future oracle is an acceptance obligation, **not an existing test**.
Where a field is a display alias, its numerical dependency uses the input-group
oracle and P07 additionally verifies presentation. Resource/cancellation checks
apply to each new collector. The source inventory below guards against silent
omission; it does not prove behavioral completeness on its own.

## Ordered scoring and veto ledger

All rules are owned by **P05**, with input owners above. The oracle is the pinned
function on equivalent supplied feature vectors; every predicate gets false,
true and equality cases. Integrated controls then verify input bindings. `N`
means Nyquist. Exact source remains authoritative for compound predicate order;
line references in the method inventory are pinned to this commit.

| ID | Source path and effect | Gate/interaction and disposition |
| --- | --- | --- |
| R01 | `_score`: low cutoff +2 | cutoff <0.85N and <18500 |
| R02 | hard cliff +3, else soft +1 | sharpness >15 or cliff >35 below .93N; else sharpness >8 or cliff >20 below .93N |
| R03 | HF depletion +1 | ratio <.005 |
| R04 | void +3 else quiet +1 | above-cutoff raw FFT dB <−70 else <−40 |
| R05 | rigid cutoff +1 | variance <1000 and cutoff <.85N |
| R06 | banding +1; side +2 | banding >.92 and cutoff <.80N; side >.60 |
| R07 | natural rich HF/noise/entropy +1 each | disabled under spectral DSD flag; HF >.05 with >.85N, floor >−50, entropy >8.5 with >.85N |
| R08 | natural slope/variance/side credits | sharpness <5 +1; variance >100000 non-DSD and >.85N +1, else >10000 and >.85N +1; side <.2 +1 (mono reference side default is explicit label-only input) |
| R09 | `analyse`: base net and scaling | net=max(0,lossy−natural), max=14; main=Python round(net×45/14), ties-to-even |
| R10 | resample +45 | first applicable `_resample_check` result; no cassette/vinyl veto; candidate interpretation only |
| R11 | header +20 duration, +25 bitrate | **excluded D07**; retain raw observations/legacy effect in trace, never applied |
| R12 | psychoacoustic pre-event +15 or +10 | outer cutoff <21000 OR MP3 profile; percentage >10 else ≥5; denominator edge correction is a named adapter deviation |
| R13 | aliasing +15 or +10 | >.5 else ≥.3: **excluded D05**, not a frequency-mirror detector |
| R14 | MP3 comb +10 | outer R12 gate, geometry and ≥2 qualifying lag peaks; ac >.25 and >2× neighbours |
| R15 | `_cassette_source`: internal score | cutoff <19000; hiss >−55dB AND lag corr <.2 gives +30/hiss; −6<slope<−3 +20 else slope<−10 −20; no comb +15; 50<std<300 +15 else std<30 −10; clamp ≥0 |
| R16 | cassette veto −40 | internal score ≥30 AND hiss; suppress segment penalty/splice, silence/vinyl branch, fingerprint/scatter penalties and AAC; do not suppress R10, fake-hires, phase, sparsity, envelope or Vorbis |
| R17 | fake-hires bandwidth +20 | no resample, not native DSD; `_is_fake_hires_bandwidth` rate≥88200, 0<cutoff<.6N, cliff>25, measured void<−80dB; source flag retained as candidate |
| R18 | adaptive segment majority +55 | cliff>30 and (void<−85 or fingerprint), not resample wall, cutoff<22500 arms cutoff+400 above base16500; density 9–36; suppress score under cassette; even half vote qualifies when positive |
| R19 | single partial-region +25 | no majority, no cassette; per-probe low cutoff/cliff OR full-band-global void predicate; one region requires cliff>35 and wall fingerprint |
| R20 | multi-region bonus | ≥2 anomalies +30, ≥4 +40; any void-backed +25; median cutoff fingerprint +15; preserve offsets/conditions and candidate language |
| R21 | `_silence_and_vinyl`: dirty silence +50 early return | quiet runs <−40dB ≥.5s, total≥2s; concatenated first30s versus 10–40s music when available; music HF>1.2e−8; ratio>.3; exact band/normalization inputs required |
| R22 | noise void +20 or vinyl −40, clicks −10 | >cutoff band level <−70 plus cutoff<22500/cliff>25 gives +20; otherwise corr<.3 and temporal variance<5 gives vinyl/−40; 5≤clicks/min≤50 adds −10 |
| R23 | clean silence −30 | 0≤ratio<.15, no segment majority, wall≤16500, cutoff>.85N, no resample; skipped with whole silence branch under cassette |
| R24 | wall fingerprint +10 | fingerprint and (void_verified OR cliff>30), no cassette/vinyl; select pinned nearest candidate, keep all native candidates separately |
| R25 | scatter collapse +25 | sample rate≥40000 and 0<average bound<16500, no cassette/vinyl |
| R26 | phase disruption +10 | entropy>4.5, cutoff<.85N, cutoff<22500, cliff>25; not suppressed by source veto |
| R27 | sparsity +10 | fraction>.30 and cutoff<.95N |
| R28 | independent HF envelope +15 | corr<.15 and 0<average bound<cutoff−2000 and cutoff>16500; candidate, not proven injected noise |
| R29 | AAC +55 else +15 | score≥.10 else ≥.06; not native DSD/vinyl/cassette; 44.1/48k only; retain 8-sample phase grid and applicability |
| R30 | Vorbis suspicion floor | score≥.03 sets main=max(main,55), not additive; not native DSD; no source veto |
| R31 | final clamp and `_verdict` | clamp [0,100]; known codec branch first (D08); native DSD inapplicable; ≥86 LIKELY_LOSSY precedes resample/fake-hires SUSPICIOUS; then ≥55, ≥31, ≥11, else GENUINE. All labels attributed/qualified |
| R32 | `build_report` / `_apply_mqa_override` | Python MQA sets known codec/main=100 but preserves heuristic: **excluded D06**; candidate metadata/structural observations only |
| R33 | `_bit_depth_verdict` independent candidates | pad≤claimed−8; reduced<claimed; profile absent/masked; floor<−102; ≥24/flat/floor≤−89; colored/limited and consistent branches. Exact Python rounding, corrected-profile/first30s deviation; never confirmed original depth |
| R34 | `_interp_*`, report aliases and caveats | keep every threshold/category in audit; no “impossible naturally,” “proven,” verified vinyl, physical wow/flutter or quality promise in display. Cassette score-only alias corrected D10 |

Native codec identification, ReplayGain mismatch, encoder tags, clipping, DR and
normalization deltas do not introduce extra lossy points. `raw_lossy_pct` uses
base lossy/max (capped100); `heuristic_score` is the post-clamp pre-override score;
legacy `net_confidence_pct` is main cast to float, not calibrated confidence.
`scipy_available` becomes an audited reference capability flag; Rust absence of
SciPy is not missing DSP or an excuse to skip required rules.

## Tool-stat and workflow completeness

The SoX dictionary is open-ended in Python. P03a must map the canonical `stat`
keys: samplesRead, lengthSeconds, scaledBy, maximumAmplitude, minimumAmplitude,
midlineAmplitude, meanNorm, meanAmplitude, rmsAmplitude, maximumDelta,
minimumDelta, meanDelta, rmsDelta, roughFrequency, volumeAdjustment. Retain
original tool key and units in the oracle; CamelCase spelling is checked against
the pinned `_camel_case`. `stat` normalization/display scaling and channel
interleave are distinct from per-native-channel `PcmStats`. Tool warnings are
diagnostics, not numeric fields. The observed SoX `Try: -t raw -e mu-law -b 8`
hint is parsed as `try` by Python; D12 retains it only as a tool diagnostic,
not a required audio statistic. Unknown tool-version keys are retained in the
oracle record and explicitly reviewed, not silently dropped. FFmpeg astats
peak count/flat factor/entropy are separate from these SoX keys.

Workflow acceptance W01 single full report; W02 first60s/explicit prefix across
all products; W03 info with no decoding; W04 stable directory/nonrecursive batch
with per-file failures; W05 unchanged old JSON plus separately versioned product
JSON; W06 progress/cancel/deadline; W07 saved product re-render/export; W08
same-track comparison/ties/all-unavailable; W09 bounded spectrogram; W10 no
external tools/runtime internet. P07 owns W01–W08/W10, P06 W09, A01–A07 Android
adaptation. Existing Python terminal colors/box drawing, file-size `stat()` race,
ETA estimation, parallel extractor count, adjacent spectrogram filename and
dependency probing are not byte-for-byte compatibility requirements.

## Independent MQA work received during P01

The owner supplied `C:\Users\Bishal\Documents\antigravity-dev\mqa` after the
initial mapping. A Rust multi-packet prototype and saved corpus survey already
exist. [Read-only intake](task-results/P01-MQA.md) records source/evidence hashes,
actual saved counts and finite review obligations. E09 is READY for review of
that work; “no MQA implementation/material available” is no longer current.
G23 still accurately describes production candidate-only behavior. This does not
reinstate Python's MQA score override or make E09 a beta prerequisite.

## Source inventory

The tables below enumerate all fields of the seven dataclasses and every top-level or
direct class method in the pinned module. Qualified names are stable coverage
keys. Each group above supplies the status, Rust mapping, tests, owner and oracle.
Private formatting helpers are included to make presentation coverage auditable.
Nested helpers (e.g. `hf_energy`, `_last`, `_field`, `payload`, `_clipped`) are
covered by their enclosing methods; constants/codec tables are covered by their
consumers. This inventory deliberately contains no copied reference implementation.


### Fields

| Coverage key | Python source line | Mapping group |
| --- | --- | --- |
| `field:AudioTags.title` | `audio_forensic.py:227` | G01 |
| `field:AudioTags.album` | `audio_forensic.py:227` | G01 |
| `field:AudioTags.date` | `audio_forensic.py:227` | G01 |
| `field:AudioTags.album_artist` | `audio_forensic.py:227` | G01 |
| `field:AudioTags.artist` | `audio_forensic.py:228` | G01 |
| `field:AudioTags.bpm` | `audio_forensic.py:228` | G01 |
| `field:AudioTags.comment_quality` | `audio_forensic.py:228` | G01 |
| `field:AudioTags.comments` | `audio_forensic.py:228` | G01 |
| `field:AudioTags.replaygain_track_gain` | `audio_forensic.py:229` | G01 |
| `field:AudioTags.replaygain_album_gain` | `audio_forensic.py:229` | G01 |
| `field:AudioTags.other` | `audio_forensic.py:230` | G01 |
| `field:AudioTechnical.bit_rate` | `audio_forensic.py:234` | G01 |
| `field:AudioTechnical.channels` | `audio_forensic.py:234` | G01 |
| `field:AudioTechnical.precision` | `audio_forensic.py:234` | G01 |
| `field:AudioTechnical.sample_rate` | `audio_forensic.py:234` | G01 |
| `field:AudioTechnical.sample_encoding` | `audio_forensic.py:235` | G01 |
| `field:AudioTechnical.duration` | `audio_forensic.py:235` | G01 |
| `field:AudioTechnical.duration_sec` | `audio_forensic.py:235` | G01 |
| `field:AudioTechnical.writing_library` | `audio_forensic.py:236` | G01 |
| `field:AudioTechnical.format_profile` | `audio_forensic.py:236` | G01 |
| `field:AudioTechnical.compression_mode` | `audio_forensic.py:236` | G01 |
| `field:AudioTechnical.codec` | `audio_forensic.py:237` | G01 |
| `field:LoudnessProfile.peak_db` | `audio_forensic.py:241` | G03 |
| `field:LoudnessProfile.rms_db` | `audio_forensic.py:241` | G03 |
| `field:LoudnessProfile.rms_peak_db` | `audio_forensic.py:241` | G04 |
| `field:LoudnessProfile.rms_trough_db` | `audio_forensic.py:241` | G04 |
| `field:LoudnessProfile.noise_floor_db` | `audio_forensic.py:242` | G04 |
| `field:LoudnessProfile.dynamic_range_db` | `audio_forensic.py:242` | G04 |
| `field:LoudnessProfile.crest_factor_db` | `audio_forensic.py:242` | G03 |
| `field:LoudnessProfile.flat_factor` | `audio_forensic.py:243` | G04 |
| `field:LoudnessProfile.peak_count` | `audio_forensic.py:243` | G04 |
| `field:LoudnessProfile.sox_entropy` | `audio_forensic.py:243` | G04 |
| `field:LoudnessProfile.dc_offset` | `audio_forensic.py:243` | G03 |
| `field:LoudnessProfile.zero_crossings_rate` | `audio_forensic.py:244` | G04 |
| `field:LoudnessProfile.lufs_integrated` | `audio_forensic.py:244` | G05 |
| `field:LoudnessProfile.lufs_range` | `audio_forensic.py:244` | G05 |
| `field:LoudnessProfile.true_peak_dbtp` | `audio_forensic.py:245` | G05 |
| `field:LoudnessProfile.lufs_momentary_max` | `audio_forensic.py:245` | G05 |
| `field:LoudnessProfile.lufs_shortterm_max` | `audio_forensic.py:245` | G05 |
| `field:LoudnessProfile.apple_music_delta` | `audio_forensic.py:246` | G05 |
| `field:LoudnessProfile.spotify_delta` | `audio_forensic.py:246` | G05 |
| `field:SpectralAnalysis.cutoff_hz` | `audio_forensic.py:250` | G08 |
| `field:SpectralAnalysis.cutoff_hz_str` | `audio_forensic.py:250` | G08 |
| `field:SpectralAnalysis.cutoff_variance` | `audio_forensic.py:251` | G08 |
| `field:SpectralAnalysis.cutoff_variance_interp` | `audio_forensic.py:251` | G22 |
| `field:SpectralAnalysis.cutoff_sharpness_db` | `audio_forensic.py:252` | G08 |
| `field:SpectralAnalysis.cutoff_sharpness_interp` | `audio_forensic.py:252` | G22 |
| `field:SpectralAnalysis.cliff_depth_db` | `audio_forensic.py:253` | G08 |
| `field:SpectralAnalysis.hf_energy_ratio` | `audio_forensic.py:254` | G08 |
| `field:SpectralAnalysis.hf_energy_interp` | `audio_forensic.py:254` | G22 |
| `field:SpectralAnalysis.banding_score` | `audio_forensic.py:255` | G09 |
| `field:SpectralAnalysis.banding_interp` | `audio_forensic.py:255` | G22 |
| `field:SpectralAnalysis.nf_above_cutoff_db` | `audio_forensic.py:256` | G08 |
| `field:SpectralAnalysis.nf_interp` | `audio_forensic.py:256` | G22 |
| `field:SpectralAnalysis.side_anomaly_score` | `audio_forensic.py:257` | G09 |
| `field:SpectralAnalysis.side_interp` | `audio_forensic.py:257` | G22 |
| `field:SpectralAnalysis.entropy` | `audio_forensic.py:258` | G08 |
| `field:SpectralAnalysis.entropy_interp` | `audio_forensic.py:258` | G22 |
| `field:SpectralAnalysis.lpf_detected` | `audio_forensic.py:259` | G09 |
| `field:SpectralAnalysis.lpf_cutoff_str` | `audio_forensic.py:259` | G09 |
| `field:SpectralAnalysis.dsd_detected` | `audio_forensic.py:260` | G09 |
| `field:SpectralAnalysis.lossy_score` | `audio_forensic.py:260` | G22 |
| `field:SpectralAnalysis.natural_score` | `audio_forensic.py:261` | G22 |
| `field:SpectralAnalysis.net_score` | `audio_forensic.py:261` | G22 |
| `field:SpectralAnalysis.max_score` | `audio_forensic.py:261` | G22 |
| `field:SpectralAnalysis.raw_lossy_pct` | `audio_forensic.py:262` | G22 |
| `field:SpectralAnalysis.net_confidence_pct` | `audio_forensic.py:262` | G22 |
| `field:SpectralAnalysis.heuristic_score` | `audio_forensic.py:263` | G22 |
| `field:SpectralAnalysis.known_lossy_codec` | `audio_forensic.py:263` | G23 |
| `field:SpectralAnalysis.verdict_label` | `audio_forensic.py:264` | G22 |
| `field:SpectralAnalysis.primary_verdict` | `audio_forensic.py:264` | G22 |
| `field:SpectralAnalysis.evidence` | `audio_forensic.py:265` | G22 |
| `field:SpectralAnalysis.natural_evidence` | `audio_forensic.py:266` | G22 |
| `field:SpectralAnalysis.caveats` | `audio_forensic.py:267` | G22 |
| `field:SpectralAnalysis.main_score` | `audio_forensic.py:269` | G22 |
| `field:SpectralAnalysis.spectral_sparsity` | `audio_forensic.py:270` | G19 |
| `field:SpectralAnalysis.sparsity_interp` | `audio_forensic.py:270` | G22 |
| `field:SpectralAnalysis.hf_envelope_correlation` | `audio_forensic.py:271` | G19 |
| `field:SpectralAnalysis.hf_env_corr_interp` | `audio_forensic.py:271` | G22 |
| `field:SpectralAnalysis.preecho_pct` | `audio_forensic.py:272` | G17 |
| `field:SpectralAnalysis.aliasing_corr` | `audio_forensic.py:273` | G24 |
| `field:SpectralAnalysis.mp3_noise_pattern_detected` | `audio_forensic.py:274` | G20 |
| `field:SpectralAnalysis.cassette_score` | `audio_forensic.py:275` | G22 |
| `field:SpectralAnalysis.silence_ratio` | `audio_forensic.py:276` | G16 |
| `field:SpectralAnalysis.vinyl_noise_detected` | `audio_forensic.py:277` | G22 |
| `field:SpectralAnalysis.vinyl_clicks_per_min` | `audio_forensic.py:278` | G17 |
| `field:SpectralAnalysis.header_duration_mismatch` | `audio_forensic.py:279` | G10 |
| `field:SpectralAnalysis.header_bitrate_mismatch` | `audio_forensic.py:280` | G10 |
| `field:SpectralAnalysis.segment_walled` | `audio_forensic.py:281` | G11 |
| `field:SpectralAnalysis.segment_total` | `audio_forensic.py:281` | G11 |
| `field:SpectralAnalysis.segment_wall_hz` | `audio_forensic.py:281` | G11 |
| `field:SpectralAnalysis.segment_map` | `audio_forensic.py:282` | G11 |
| `field:SpectralAnalysis.codec_fingerprint` | `audio_forensic.py:283` | G11 |
| `field:SpectralAnalysis.resample_detected` | `audio_forensic.py:284` | G12 |
| `field:SpectralAnalysis.resample_src_rate` | `audio_forensic.py:284` | G12 |
| `field:SpectralAnalysis.fake_hires` | `audio_forensic.py:285` | G12 |
| `field:SpectralAnalysis.auc_avg_bound_freq` | `audio_forensic.py:286` | G13 |
| `field:SpectralAnalysis.auc_bound_interp` | `audio_forensic.py:286` | G22 |
| `field:SpectralAnalysis.auc_prob_bound_freq` | `audio_forensic.py:287` | G13 |
| `field:SpectralAnalysis.auc_phase_entropy` | `audio_forensic.py:288` | G13 |
| `field:SpectralAnalysis.auc_phase_interp` | `audio_forensic.py:288` | G22 |
| `field:SpectralAnalysis.mdct_quant_score` | `audio_forensic.py:289` | G14 |
| `field:SpectralAnalysis.mdct_quant_interp` | `audio_forensic.py:289` | G22 |
| `field:SpectralAnalysis.vorbis_grid_score` | `audio_forensic.py:290` | G15 |
| `field:SpectralAnalysis.vorbis_grid_support` | `audio_forensic.py:290` | G15 |
| `field:SpectralAnalysis.vorbis_grid_tested` | `audio_forensic.py:291` | G15 |
| `field:SpectralAnalysis.vorbis_grid_channel` | `audio_forensic.py:291` | G15 |
| `field:SpectralAnalysis.vorbis_grid_interp` | `audio_forensic.py:292` | G22 |
| `field:SpectralAnalysis.scipy_available` | `audio_forensic.py:293` | G22 |
| `field:AuthenticityReport.spectral` | `audio_forensic.py:297` | G26 |
| `field:AuthenticityReport.spectral_cutoff_hz` | `audio_forensic.py:298` | G26 |
| `field:AuthenticityReport.spectral_cutoff_verdict` | `audio_forensic.py:298` | G26 |
| `field:AuthenticityReport.lpf_detected` | `audio_forensic.py:298` | G09 |
| `field:AuthenticityReport.lpf_cutoff_hz` | `audio_forensic.py:299` | G09 |
| `field:AuthenticityReport.bit_depth_authentic` | `audio_forensic.py:299` | G21 |
| `field:AuthenticityReport.phase_correlation` | `audio_forensic.py:299` | G07 |
| `field:AuthenticityReport.phase_verdict` | `audio_forensic.py:300` | G07 |
| `field:AuthenticityReport.clipped_samples` | `audio_forensic.py:300` | G07 |
| `field:AuthenticityReport.clipping_verdict` | `audio_forensic.py:300` | G07 |
| `field:AuthenticityReport.silence_total_pct` | `audio_forensic.py:301` | G07 |
| `field:AuthenticityReport.silence_sections` | `audio_forensic.py:301` | G07 |
| `field:AuthenticityReport.rg_stored` | `audio_forensic.py:302` | G02 |
| `field:AuthenticityReport.rg_measured_lufs` | `audio_forensic.py:302` | G02 |
| `field:AuthenticityReport.rg_delta` | `audio_forensic.py:302` | G02 |
| `field:AuthenticityReport.rg_verdict` | `audio_forensic.py:302` | G02 |
| `field:AuthenticityReport.cassette_rip_detected` | `audio_forensic.py:303` | G22 |
| `field:AuthenticityReport.vinyl_rip_detected` | `audio_forensic.py:304` | G22 |
| `field:AuthenticityReport.mqa_detected` | `audio_forensic.py:305` | G23 |
| `field:AuthenticityReport.mqa_metadata_claimed` | `audio_forensic.py:306` | G23 |
| `field:AuthenticityReport.mqa_studio` | `audio_forensic.py:307` | G23 |
| `field:AuthenticityReport.mqa_original_sample_rate` | `audio_forensic.py:308` | G23 |
| `field:AuthenticityReport.mqa_bit_plane` | `audio_forensic.py:309` | G23 |
| `field:AuthenticityReport.mqa_sync_sample` | `audio_forensic.py:310` | G23 |
| `field:AuthenticityReport.mqa_evidence` | `audio_forensic.py:311` | G23 |
| `field:AuthenticityReport.mqa_scan_status` | `audio_forensic.py:312` | G23 |
| `field:AuthenticityReport.mqa_scan_error` | `audio_forensic.py:313` | G23 |
| `field:AuthenticityReport.side_channel_analysis` | `audio_forensic.py:314` | G09 |
| `field:AuthenticityReport.header_integrity` | `audio_forensic.py:315` | G10 |
| `field:AuthenticityReport.encoder_trace` | `audio_forensic.py:316` | G01 |
| `field:ForensicReport.filepath` | `audio_forensic.py:320` | G26 |
| `field:ForensicReport.tags` | `audio_forensic.py:320` | G01 |
| `field:ForensicReport.technical` | `audio_forensic.py:321` | G01 |
| `field:ForensicReport.sox_stats` | `audio_forensic.py:322` | G04 |
| `field:ForensicReport.loudness` | `audio_forensic.py:323` | G26 |
| `field:ForensicReport.authenticity` | `audio_forensic.py:324` | G26 |
| `field:ForensicReport.dr_score` | `audio_forensic.py:325` | G06 |
| `field:ForensicReport.spectrogram_path` | `audio_forensic.py:325` | G25 |
| `field:ForensicReport.analysis_seconds` | `audio_forensic.py:326` | G26 |
| `field:MQADetection.detected` | `audio_forensic.py:430` | G23 |
| `field:MQADetection.metadata_claimed` | `audio_forensic.py:431` | G23 |
| `field:MQADetection.studio` | `audio_forensic.py:432` | G23 |
| `field:MQADetection.original_sample_rate` | `audio_forensic.py:433` | G23 |
| `field:MQADetection.bit_plane` | `audio_forensic.py:434` | G23 |
| `field:MQADetection.sync_sample` | `audio_forensic.py:435` | G23 |
| `field:MQADetection.error` | `audio_forensic.py:436` | G23 |

### Methods and workflows

| Coverage key | Python source line | Mapping group |
| --- | --- | --- |
| `method:_c` | `audio_forensic.py:56` | G26 |
| `method:_visible` | `audio_forensic.py:58` | G26 |
| `method:_kv` | `audio_forensic.py:59` | G26 |
| `method:_rule` | `audio_forensic.py:60` | G26 |
| `method:_section` | `audio_forensic.py:61` | G26 |
| `method:_subsection` | `audio_forensic.py:62` | G26 |
| `method:_camel_case` | `audio_forensic.py:63` | G04 |
| `method:_stat` | `audio_forensic.py:78` | G26 |
| `method:_interp` | `audio_forensic.py:79` | G26 |
| `method:_degl` | `audio_forensic.py:80` | G26 |
| `method:_mrow` | `audio_forensic.py:82` | G26 |
| `method:_run` | `audio_forensic.py:96` | G29 |
| `method:_extractor_workers` | `audio_forensic.py:102` | G29 |
| `method:_pipe_ffmpeg_to_sox` | `audio_forensic.py:114` | G29 |
| `method:_Status.begin` | `audio_forensic.py:164` | G29 |
| `method:_Status.update` | `audio_forensic.py:169` | G29 |
| `method:_Status.done` | `audio_forensic.py:177` | G29 |
| `method:_Status.clear` | `audio_forensic.py:187` | G29 |
| `method:_Status._render` | `audio_forensic.py:193` | G29 |
| `method:_tool_available` | `audio_forensic.py:217` | G29 |
| `method:ForensicReport.file_size_mb` | `audio_forensic.py:328` | G26 |
| `method:_prettify_mi_key` | `audio_forensic.py:349` | G01 |
| `method:extract_mediainfo` | `audio_forensic.py:352` | G01 |
| `method:detect_encoder_trace` | `audio_forensic.py:407` | G01 |
| `method:_mqa_original_sample_rate` | `audio_forensic.py:438` | G23 |
| `method:_scan_mqa_pcm` | `audio_forensic.py:449` | G23 |
| `method:inspect_mqa` | `audio_forensic.py:495` | G23 |
| `method:_mqa_note` | `audio_forensic.py:532` | G23 |
| `method:_apply_mqa_override` | `audio_forensic.py:548` | G23 |
| `method:detect_mqa` | `audio_forensic.py:570` | G23 |
| `method:extract_sox_stats` | `audio_forensic.py:580` | G04 |
| `method:extract_loudness` | `audio_forensic.py:597` | G05 |
| `method:_effective_bits` | `audio_forensic.py:675` | G21 |
| `method:_noise_floor_profile` | `audio_forensic.py:702` | G21 |
| `method:check_bit_depth_authenticity` | `audio_forensic.py:749` | G21 |
| `method:_bit_depth_verdict` | `audio_forensic.py:806` | G21 |
| `method:measure_phase_correlation` | `audio_forensic.py:854` | G07 |
| `method:detect_clipping` | `audio_forensic.py:885` | G07 |
| `method:_noise_floor_from_audio` | `audio_forensic.py:914` | G07 |
| `method:map_silence` | `audio_forensic.py:933` | G07 |
| `method:audit_replaygain` | `audio_forensic.py:956` | G02 |
| `method:generate_spectrogram` | `audio_forensic.py:971` | G25 |
| `method:bandpass_filter` | `audio_forensic.py:1018` | G17 |
| `method:highpass_filter` | `audio_forensic.py:1023` | G17 |
| `method:calculate_autocorrelation` | `audio_forensic.py:1028` | G16 |
| `method:calculate_temporal_variance` | `audio_forensic.py:1038` | G16 |
| `method:SpectralEngine.__init__` | `audio_forensic.py:1106` | G08 |
| `method:SpectralEngine._frame_hint` | `audio_forensic.py:1123` | G08 |
| `method:SpectralEngine._pcm_cmd` | `audio_forensic.py:1135` | G08 |
| `method:SpectralEngine._decode_audio` | `audio_forensic.py:1141` | G08 |
| `method:SpectralEngine._decode_stereo` | `audio_forensic.py:1178` | G08 |
| `method:SpectralEngine._compute_frames` | `audio_forensic.py:1217` | G08 |
| `method:SpectralEngine._compute_stft` | `audio_forensic.py:1220` | G08 |
| `method:SpectralEngine._freq_bins` | `audio_forensic.py:1247` | G08 |
| `method:SpectralEngine._interp_variance` | `audio_forensic.py:1250` | G22 |
| `method:SpectralEngine._interp_sharpness` | `audio_forensic.py:1258` | G22 |
| `method:SpectralEngine._interp_hf_ratio` | `audio_forensic.py:1265` | G22 |
| `method:SpectralEngine._interp_banding` | `audio_forensic.py:1272` | G22 |
| `method:SpectralEngine._interp_nf` | `audio_forensic.py:1279` | G22 |
| `method:SpectralEngine._interp_side` | `audio_forensic.py:1286` | G22 |
| `method:SpectralEngine._interp_entropy` | `audio_forensic.py:1294` | G22 |
| `method:SpectralEngine._interp_bound` | `audio_forensic.py:1300` | G22 |
| `method:SpectralEngine._interp_phase_entropy` | `audio_forensic.py:1307` | G22 |
| `method:SpectralEngine._interp_sparsity` | `audio_forensic.py:1315` | G22 |
| `method:SpectralEngine._interp_ultra_corr` | `audio_forensic.py:1322` | G22 |
| `method:SpectralEngine._active_frame_mask` | `audio_forensic.py:1329` | G08 |
| `method:SpectralEngine._compact_rows` | `audio_forensic.py:1337` | G08 |
| `method:SpectralEngine._cutoff_per_frame` | `audio_forensic.py:1357` | G08 |
| `method:SpectralEngine._sharpness` | `audio_forensic.py:1380` | G08 |
| `method:SpectralEngine._cliff_depth` | `audio_forensic.py:1388` | G08 |
| `method:SpectralEngine._hf_energy_ratio` | `audio_forensic.py:1401` | G08 |
| `method:SpectralEngine._banding_score` | `audio_forensic.py:1404` | G09 |
| `method:SpectralEngine._noise_floor_above_cutoff` | `audio_forensic.py:1417` | G08 |
| `method:SpectralEngine._side_channel_anomaly` | `audio_forensic.py:1427` | G09 |
| `method:SpectralEngine._aucdtect_features` | `audio_forensic.py:1452` | G13 |
| `method:SpectralEngine._check_header_integrity` | `audio_forensic.py:1522` | G10 |
| `method:SpectralEngine._segment_voting` | `audio_forensic.py:1543` | G11 |
| `method:SpectralEngine._smooth_envelope` | `audio_forensic.py:1608` | G17 |
| `method:SpectralEngine._fft_band_extract` | `audio_forensic.py:1616` | G16 |
| `method:SpectralEngine._silence_and_vinyl` | `audio_forensic.py:1648` | G16 |
| `method:SpectralEngine._psychoacoustic_artifacts` | `audio_forensic.py:1755` | G20 |
| `method:SpectralEngine._kbd_window` | `audio_forensic.py:1849` | G14 |
| `method:SpectralEngine._mdct_batch` | `audio_forensic.py:1865` | G14 |
| `method:SpectralEngine._vorbis_grid` | `audio_forensic.py:1875` | G15 |
| `method:SpectralEngine._mdct_quant_error` | `audio_forensic.py:1957` | G14 |
| `method:SpectralEngine._interp_mdct` | `audio_forensic.py:2084` | G22 |
| `method:SpectralEngine._cassette_source` | `audio_forensic.py:2093` | G18 |
| `method:SpectralEngine._spectral_sparsity` | `audio_forensic.py:2151` | G19 |
| `method:SpectralEngine._ultrasonic_envelope_correlation` | `audio_forensic.py:2167` | G19 |
| `method:SpectralEngine._lpf_scan` | `audio_forensic.py:2187` | G09 |
| `method:SpectralEngine._dsd_scan` | `audio_forensic.py:2202` | G09 |
| `method:SpectralEngine._spectral_entropy` | `audio_forensic.py:2211` | G08 |
| `method:SpectralEngine._score` | `audio_forensic.py:2216` | G22 |
| `method:SpectralEngine._resample_check` | `audio_forensic.py:2279` | G12 |
| `method:SpectralEngine._codec_fingerprint` | `audio_forensic.py:2344` | G11 |
| `method:SpectralEngine._is_fake_hires_bandwidth` | `audio_forensic.py:2357` | G12 |
| `method:SpectralEngine._verdict` | `audio_forensic.py:2371` | G22 |
| `method:SpectralEngine.analyse` | `audio_forensic.py:2410` | G22 |
| `method:build_report` | `audio_forensic.py:2759` | G26 |
| `method:build_info_report` | `audio_forensic.py:2860` | G28 |
| `method:_fv` | `audio_forensic.py:2867` | G26 |
| `method:_db_val` | `audio_forensic.py:2871` | G26 |
| `method:_dr_assessment` | `audio_forensic.py:2873` | G26 |
| `method:_peak_colour` | `audio_forensic.py:2883` | G26 |
| `method:_noise_colour` | `audio_forensic.py:2890` | G26 |
| `method:_rms_colour` | `audio_forensic.py:2897` | G26 |
| `method:_lufs_colour` | `audio_forensic.py:2904` | G26 |
| `method:_crest_colour` | `audio_forensic.py:2911` | G26 |
| `method:_flat_colour` | `audio_forensic.py:2920` | G26 |
| `method:_main_score_colour` | `audio_forensic.py:2924` | G26 |
| `method:_bound_colour` | `audio_forensic.py:2930` | G26 |
| `method:_phase_ent_colour` | `audio_forensic.py:2936` | G26 |
| `method:_sparsity_colour` | `audio_forensic.py:2940` | G26 |
| `method:_mdct_colour` | `audio_forensic.py:2945` | G26 |
| `method:_ultra_corr_colour` | `audio_forensic.py:2951` | G26 |
| `method:_preecho_colour` | `audio_forensic.py:2956` | G26 |
| `method:_aliasing_colour` | `audio_forensic.py:2961` | G26 |
| `method:_silence_ratio_colour` | `audio_forensic.py:2966` | G26 |
| `method:_sox_entropy_colour` | `audio_forensic.py:2972` | G26 |
| `method:_sox_entropy_interp` | `audio_forensic.py:2980` | G26 |
| `method:_delta_colour` | `audio_forensic.py:2990` | G26 |
| `method:_db` | `audio_forensic.py:2998` | G26 |
| `method:_channel_label` | `audio_forensic.py:3002` | G26 |
| `method:_hz_label` | `audio_forensic.py:3003` | G26 |
| `method:_fmt_stat_key` | `audio_forensic.py:3006` | G26 |
| `method:_headroom_bar` | `audio_forensic.py:3008` | G26 |
| `method:_sox_amplitude_colour` | `audio_forensic.py:3032` | G26 |
| `method:_print_banner` | `audio_forensic.py:3046` | G26 |
| `method:print_report` | `audio_forensic.py:3087` | G26 |
| `method:_report_to_dict` | `audio_forensic.py:3315` | G26 |
| `method:print_batch_summary` | `audio_forensic.py:3322` | G27 |
| `method:_bitdepth_confidence` | `audio_forensic.py:3349` | G27 |
| `method:_dr_int` | `audio_forensic.py:3355` | G27 |
| `method:_container_quality` | `audio_forensic.py:3359` | G27 |
| `method:_container_str` | `audio_forensic.py:3366` | G27 |
| `method:_compare_key` | `audio_forensic.py:3372` | G27 |
| `method:_comparison_to_dict` | `audio_forensic.py:3385` | G27 |
| `method:print_comparison` | `audio_forensic.py:3402` | G27 |
| `method:main` | `audio_forensic.py:3444` | G28 |

Inventory: **155 fields**, **139 methods**. Check with
`python -X utf8 scripts/check_parity_contract.py`.
