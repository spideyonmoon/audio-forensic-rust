# Non-MQA evidence interpretation — 2026-10-04

Engine 0.21.0 adds `assess_evidence(&AnalysisReport)` and CLI `--summary`.
This implements the missing grouping/presentation layer separately from decoder
and detector arithmetic. The measurement report remains schema 0.18.0, policy
`observations-only-v18`, with ancestry INCONCLUSIVE and evidence index null.
The derived assessment has its own integer `assessment_version: 1`. It is not
a calibrated classifier or a replacement for the original measurements.

## Implemented behavior

Related observations appear in deterministic domains: spectral patterns, codec
transforms, sample-rate history, integer precision, source-profile measurements,
listening levels and channel relationships. Unknown families or detector versions
remain in an uninterpreted group. Existing MQA records are referenced separately
as deferred; their scanner and measurements are unchanged.

Each group retains original detector indices and all five record-status counts.
Those indices preserve access to the exact native channels, AAC bases, intervals,
measurements, thresholds and caveats. Counts describe records, not independent
votes. Repeated AAC hits across mid/side bases or repeated native-channel hits
produce one named pattern per method. Spectral-wall matches and related spectral
measurements share a domain; they do not multiply a score. Multiple AAC/Vorbis
patterns remain separate candidates, without selecting a supposed winning codec
or claiming multiple encoding stages. Sample-rate/bit-depth hits are not relabeled
lossy-ancestry evidence. Cross-domain dependence remains explicit.

Measured, not-detected, inconclusive and unsupported records remain distinct.
Failure/cancellation/timeout/unsupported file reports produce no interpreted
groups, even if externally supplied reports contain stale hit records. Unknown
schema/policy versions abstain. Missing stream/coverage, out-of-range channel
indices and invalid detector intervals abstain. This is a current-producer API,
not complete validation of arbitrary externally supplied JSON.

The shared analyzed frame count and end-of-stream flag stay explicit, alongside
the warning that each detector can cover narrower spans. The interpretation
never enlarges sampled detector spans to whole-track findings.

## Use

```text
audio-forensic --summary input.flac
audio-forensic --summary --progress --max-seconds 60 input.wav
cargo run --locked --offline --no-default-features --example explain_report -- saved-report.json
```

`--summary` produces human-readable grouped evidence instead of the detailed
text listing. It conflicts with `--json`; the existing measurement JSON contract
stays unchanged. Structured consumers call `assess_evidence` and can serialize
the returned assessment with serde. Keep it with the exact source report, since
detector indices refer to the original record order.

The CLI-free saved-report example reads a single report or report array (maximum
64 MiB input), emits an assessment array in input order, and does no audio decode.
It exits successfully if interpretation completes; callers must inspect each
assessment's availability. Malformed/empty/oversized input returns an error.

## Completion boundary and remaining work

Android and MQA confirmation are explicitly deferred by the user. The following
non-MQA distinctions must not be collapsed into a claim that only one coding
task remains:

| Area | Current implementation | Still unsupported |
| --- | --- | --- |
| Evidence grouping | Deterministic domains, deduplicated pattern names, preserved scope/statuses | Calibrated aggregation and ancestry classification |
| Source profiles | Noise, quiet blocks, temporal variation, transients, rolloff and related measurements | Validated vinyl/cassette/digital labels |
| Bit depth | Exact exercised integer bits and effective-bit statistic | Original recording-depth inference |
| Codec observations | AAC/Vorbis methods and related spectral observations | General codec identification; known AAC piano/trim and Vorbis quantization misses persist |
| Frequency mirroring | Existing scoped resampling measurements | The reference's negated absolute correlation is not a valid general frequency-mirroring detector |

Supplied commercial recordings remain claimed-history challenge material.
Recorded added lossy stages support scoped sensitivity observations, but their
native parents are not verified negative ancestry controls. Numerical parity
and synthetic controls alone cannot close these inference gaps. No new weights,
threshold tuning or confident labels were introduced to manufacture completion.
Version 0.22.0 now supplies the scoped codec-stage label contract and frozen
grouped evaluation runner described in `EVALUATION_VALIDATION.md`. Independently
reviewed source histories, grouped controls and actual evaluation of the frozen
candidate remain outstanding. More reruns of the same challenge files do
not substitute for that missing evidence.

## Validation

Actual checks:

- Rust 1.85 focused tests passed **10 tests** (six interpretation, four CLI),
  zero failures/ignored. Controls cover complete record preservation, unchanged
  report serialization, duplicate channels/bases, simultaneous AAC/Vorbis patterns,
  preserved abstentions/unsupported counts, prefix scope, failure/stale-hit
  handling, unknown contracts/versions/families, invalid intervals/channels,
  MQA deferral, CLI failure diagnostics and summary/JSON option conflicts.
- Rust 1.85 all-target Clippy passed with warnings denied. The saved-report
  example built with no CLI features. Stable formatting, whitespace and schema
  drift checks passed; the measurement schema still has 47 definitions.
- Applied the new Rust interpreter to **44 existing private report files** from
  the prior full-track intake. All passed: **37 available**, **7 input-unavailable**.
  Each successful report's detector indices are partitioned exactly once between
  interpreted groups and deferred MQA. Every status count and coverage value
  matches its source; pattern names are unique within a group. No current
  detector records fell into the unknown-family/version group. Source report
  and executable hashes stayed unchanged. All conclusions remain inconclusive.

These are saved engine-0.18.0 observations, not newly decoded recordings or
verified ancestry labels. There was no original-audio access, new corpus accuracy
evaluation, threshold tuning or full regression rerun. The saved-report example's
malformed/oversized-file error branches were not separately exercised. Evidence
and the local receipt script are ignored in
`corpus/local/results/evidence-v21-20261004/`. No jobs remain running; no private
upload, commit or push occurred.
