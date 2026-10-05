# Frozen codec-pattern evaluation — 2026-10-04

Engine 0.22.0 adds `evaluation::FrozenEvaluation` and `EvaluationSession`, plus
the CLI-free `evaluate_reports` example. This is offline research infrastructure
for the current provisional AAC/Vorbis methods. It does not implement ancestry,
medium, original-depth, MQA or calibrated codec classification. Measurement
schema 0.18.0 and policy `observations-only-v18` remain unchanged.

## Scoped labels and review

Each case declares a target `codec` (`aac` or `vorbis`) and a stage label:

- `present`: a retained recipe or reviewed chain establishes a target codec
  stage. A recorded added stage on an unknown-history parent supports this
  positive label, without establishing anything about the upstream history.
- `absent`: a generated control or independently reviewed complete chain
  establishes no target stage. An original commercial CD/rip/download, an
  unencoded immediate parent, or a detector miss does not establish absence.
  Other codecs can be negative controls for the target only when their chain
  satisfies this requirement; use separate processing strata for them.
- `unknown`: the target stage is unestablished. These cases must be in
  `challenge`; they retain outcomes but never contribute labeled rates.

`provenance` distinguishes `generated_control`, `reviewed_history`,
`recorded_added_stage` and `unknown_history`. The latter cannot receive a known
label, and recorded added stages cannot receive an absent label. Every case
references retained evidence with a SHA-256. Labeled cases also require an
independently decoded PCM SHA-256 in the core's documented sample representation.
The evaluator validates declaration consistency and compares PCM hashes; it does
not read/authenticate evidence notes, execute recipes or establish label truth.
Keep notes, independent decoding receipts and recipe/tool versions together.
The existing intake `documented` field does not automatically become
`reviewed_history`; these contracts are deliberately separate.

One source group contains all related masters, trims, codec variants, gain/bit
depth variants and recordings from the same dependent source/session. A group
must stay within one split: `development`, `validation`, `locked_test` or
`challenge`. Expected identical PCM cannot be assigned to different groups.
Hash checks cannot discover all related excerpts or authenticate independence.
IDs/group names/processing strata contain only ASCII alphanumerics, `_` and `-`,
with a maximum of 128 bytes. Evidence references are at most 4096 bytes. Hashes
are lowercase 64-character hex. Manifest size is limited to 10,000 cases.

## Frozen candidate behavior

Policy `codec-pattern-presence-v1` uses the existing version-1 transform records:
AAC native mono or both mid/side bases, and Vorbis every native channel.
One valid target hit gives one file hit, even if another basis abstains. No hit
requires every expected target basis/channel to say `not_detected`. Otherwise
the outcome is `abstain`. Missing/duplicate/unknown-version/family records or
missing hit/no-hit intervals abstain. Other codec and spectral hits do not vote.
There are no new score weights, thresholds or detector arithmetic changes.

Only full-file, two-pass, nonempty, supported reports with valid detector scope
can produce hits/no-hits. Prefixes, invalid scope and unavailable files abstain.
An independent PCM mismatch or engine/schema/policy mismatch fails evaluation;
it is not silently excluded. Failed, unsupported, cancelled and timed-out files
retain separate file statuses. Report JSON is assumed to come from the current
producer; this is not a replacement for strict external JSON schema validation.

The complete manifest is frozen before reading evaluation reports. Its canonical
typed JSON hash binds labels, group assignments, strata, hashes and engine version.
The library pins evaluator and policy versions. The runner additionally pins its
executable fingerprint. Freezing is a reproducibility guard, not proof of prior
non-exposure: retain the frozen artifact externally before opening locked groups.
Never tune on a locked result and call another run of that group held out.

## Output and denominators

Each case retains its file status, hit/no-hit/abstain, reason, analyzed frames,
target detector indices and normalized typed report SHA-256. Keep the exact
original reports alongside the summary for detector interval/basis inspection.
Full-file decoding does not enlarge the detectors' own sampled coverage.

Labeled results are separated by target codec, processing, label and provenance.
Every stratum includes file counts, every source group's counts, and equal-group
mean fractions for hit, no hit and abstention. For each group, divide an outcome's
count by all of that group's cases in that stratum; then average across groups.
Thus three hit variants of one group and one missed variant of another produce
a group mean hit fraction of 0.5, with file counts 3 hit / 1 no hit. Abstentions
stay in every denominator. Within a present stratum, no hit is a miss; within an
absent stratum, hit is a false alarm. Unknown cases have no labeled denominator.
Rates from separate strata are not a pooled population accuracy estimate.

Declared group counts and their distribution remain visible. No confidence
interval is supplied: group independence, representativeness and an uncertainty
estimator need review against the eventual corpus design. This engineering
runner does not establish statistically validated classifier performance.

## Use

Build with no CLI dependencies, then retain the resulting executable unchanged:

```text
cargo build --locked --offline --no-default-features --example evaluate_reports
evaluate_reports freeze manifest.json new-plan.json
evaluate_reports run new-plan.json development saved-reports new-summary.json
```

Use the actual path of the built example executable. `saved-reports/<id>.json`
holds one original AnalysisReport object per selected case. Report arrays are
not accepted. Only the selected split's reports are read; freezing reads no
reports, and missing unselected locked reports do not block development work.
The reader resolves report paths beneath the supplied directory and accepts at
most 64 MiB per JSON document, with one report retained at a time. No audio is
opened or decoded by this runner. Library sessions accept one typed report at
a time and retain only bounded case outcomes.

Both plan and summary outputs require new filenames. An existing file is never
overwritten. Missing/duplicate/wrong-split reports prevent a completed summary.
Setup, parse, read, validation or write failure exits nonzero; successful output
does not mean that every case was analyzable. Inspect abstentions and file statuses.
An interrupted write can leave a partial file; retain it and choose a new output.

Manifest shape (replace descriptive placeholders with actual retained hashes):

```json
{
  "manifest_version": 1,
  "report_engine_version": "0.22.0",
  "cases": [{
    "id": "source01_aac256",
    "source_group": "source01",
    "split": "development",
    "codec": "aac",
    "label": "present",
    "provenance": "recorded_added_stage",
    "processing": "aac256_s16",
    "evidence_ref": "private-receipts/source01-recipe.json",
    "evidence_sha256": "REPLACE_WITH_RECEIPT_SHA256",
    "expected_pcm_sha256": "REPLACE_WITH_INDEPENDENT_PCM_SHA256"
  }]
}
```

## Validation and remaining work

Actual checks:

- Rust 1.85 offline no-CLI focused checks passed **10 tests**: nine library
  integration controls and one runner freeze/run control. They cover group/split
  and PCM leakage, label eligibility, manifest changes, native mono/stereo and
  codec separation, duplicate/missing/unknown detector records, invalid coverage,
  all unavailable file statuses, exact engine/PCM matching, unequal variant counts,
  abstention denominators, provenance strata, unknown exclusions, required report
  completeness, non-overwrite and executable fingerprint rejection.
- Rust 1.85 all-target Clippy passed with warnings denied. The CLI-free example
  built successfully. Stable formatting, whitespace, helper Python syntax and
  schema drift checks passed; the measurement schema still has 47 definitions.
- The built runner passed **seven process smoke checks** using five unchanged
  generated-only 0.18.3 consumer reports: freeze, selected-split run, refusal to
  overwrite plan/summary, missing locked report, changed manifest and changed
  executable fingerprint. The two full WAV/FLAC reports match the retained
  independent PCM oracle. They produce two no-hit outcomes; cancellation is an
  abstention in their one source group. Failed/unsupported unknown cases remain
  visible but outside labeled rates. The unselected locked case has no report.
  Executable, original report and oracle hashes were preserved.
- Preservation checks matched **209 prior unrelated files**, including the user
  recordings. The pinned Python reference remains clean at
  `c6ecce2296256b516709d87088896d1be913908c`. Existing uncommitted work was retained.

The first test build failed at the sandbox-denied linker before tests ran; its
authorized retry passed the initial seven tests. The expanded final ten-test run
then passed. The first process smoke failed an incorrect harness expectation of
three abstentions: source report inspection showed valid AAC `not_detected`
records in both full noise controls. The corrected helper verifies those source
statuses before running, and expects two no hits plus one cancellation abstention.
No Rust policy, report, oracle or input was changed to make that check pass.
All failed and successful attempts are retained in ignored
`corpus/local/results/evaluation-v22-20261004/`; final smoke receipts are in
`generated-smoke-final/`. Synthetic report controls test accounting and rejection
rules; they are not encoder experiments or evidence of codec accuracy.

Reproduce the process check with a new output directory:

```text
python scripts/check_evaluation.py --binary target/msrv-desktop-v18/debug/examples/evaluate_reports.exe --reports corpus/local/results/core-integration-20261003/consumer-final --output corpus/local/results/evaluation-v22-new-smoke
```

The full regression, new audio/encoder experiments, private corpus evaluation,
Android compilation/link/device checks and population confidence intervals were
not run. The reader's 64 MiB limit and filesystem-link containment rejection
were not separately exercised. No jobs remain running, and no commit, push or
private upload occurred. The version change only changes engine metadata in
measurement reports; detector arithmetic, schema, policy and dependencies are
unchanged.

The next control-assembly step is now recorded in
`GROUPED_CONTROLS_VALIDATION.md`: six procedural families, 48 characterized
audio variants, and a frozen 96-case manifest with separate development,
validation and reserved locked groups. This supplies generated engineering
controls, not an independently reviewed recording corpus or locked evaluation.
Existing private challenge
music is not a negative ancestry corpus. AAC piano/trim and Vorbis quantization
limitations remain open; no threshold was changed by this implementation.
