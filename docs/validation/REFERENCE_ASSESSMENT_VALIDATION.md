# P05 reference assessment validation

Engine 0.31.0; assessment version 1, method `python-reference-c6ecce2-v1`,
contract version 1, pinned commit `c6ecce2296256b516709d87088896d1be913908c`.
Assessment outputs are uncalibrated deterministic reference-method results.
Measurement schema 0.18.0 / observations-only-v18 is unchanged, including
INCONCLUSIVE ancestry and null evidence index.

## API and policy

`reference_assessment::assess_reference(&ReferenceAnalysis)` interprets a saved
P04 v2 result without I/O, decoding, recomputation or mutation. The input binding
records measurement schema/policy, PCM hash/encoding, exact coverage, parsed
stream and adapter version/method. Unknown versions, mismatched hashes/three
passes, domains or out-of-scope intervals fail explicitly. Failed, unsupported,
cancelled and timed-out measurements cannot yield successful stale assessments.
`read_assessment_json` dispatches the saved assessment version/method explicitly;
it is not a replacement for validating/re-assessing its original bound inputs.
`examples/read_reference_assessment.rs` demonstrates saved-input interpretation.
P07 still owns the production product envelope, CLI and comparison workflows.

R01–R34 preserve pinned weights, order, cassette/vinyl vetoes, segment offset
deduplication for splice interpretation, adaptive majority, Vorbis floor and
Python ties-to-even rounding. The trace exposes predicates/thresholds, operands,
feature IDs and source spans, rule results and before/after ledgers. On partial
results those ledgers contain known contributions only; composite scores/labels
and legacy confidence aliases are null. Optional audit label/depth availability
does not itself make a determinable ancestry result partial. A missing applicable
score/veto input does. Completed independent candidates remain visible.

Default display qualifies source, depth, bandwidth and lossy interpretations.
Raw Python verdict/depth/interpretation wording is retained only in legacy audit.
Scores do not establish probabilities, original depth, physical medium or
recording history. Parsed codec identity is separate from ancestry. Native DSD
explicitly skips PCM ancestry; this policy guard does not implement F02 decoding.

## Frozen deviations

- D01: use P04 f32 mono/mid/side adapters; never substitute native lookalikes.
  Mono side=0 is an explicit source policy default, not a measured side channel.
- D02: retain prefix and first-30/60/180-second scopes. Feature intervals describe
  analysis scopes/support envelopes; qualified silence may be disjoint inside
  its full-prefix scope and only its first 30 seconds enter the HF comparison.
- D03: preserve applicability/nulls, qualified segment probes, physical adjacent
  phase and corrected quiet-profile indices. Out-of-range DSD spectral/comb
  geometry is explicitly inapplicable; foreign-Nyquist resampling geometry without any eligible source rate is
  likewise explicitly inapplicable. Absent energy on applicable branches is
  not replaced by a successful clear result.
- D04: no quality claim or ancestry points from native loudness/linear crest/DR.
  P03/P03a remain their authoritative measurement/audit providers.
- D05: R13 aliasing/mirroring points excluded and visible in the trace.
- D06: R32 MQA certainty/main=100 override excluded; candidate observations do
  not prove a codec. E09 confirmation research is unchanged.
- D07: R11 header +20/+25 excluded; supplied raw header booleans remain visible.
- D08: extension cannot set codec identity; native DSD ancestry is inapplicable.
- D09: exact reference thresholds, candidate-only default wording, audit labels.
- D10: cassette candidate needs score>=30 AND hiss; score-only legacy alias is
  visible separately. Source vetoes retain their original selective scope.
- D11: ReplayGain and fixed normalization targets add no ancestry points;
  metadata/byproduct audits remain separate, with their existing methods.
- D12: bounded metadata and spectrogram remain independent. P07 owns comparison
  presentation and null unavailable winners; P05 invents no replacement weights.

## Oracles and repaired attempts

`scripts/generate_reference_assessment.py` executes pinned Python policy AST
with supplied DSP scalars and segment probes. Original `_score`, scalar cassette
and silence decisions, `analyse` order/vetoes and `_verdict` execute. Extraction
is supplied by the separately validated P04 adapters; this is policy differential
validation, not a new detector accuracy experiment. `--check` never rewrites.
`--traces` freezes main-score totals after original Python assignments. Header
and alias points are disabled explicitly for the D05/D07 comparison.

The initial `reference_assessment.json` and `reference_assessment_traces.json`
are retained as **rejected harness v1**, not acceptance oracles. Its silence AST
selector chose the preceding FFT guard instead of the energy decision. The
vinyl -70 equality control exposed the error. The corrected selector matches the
exact predicate and independently executes the complete original method on a
generated hiss control (expecting -40). Accepted candidate oracles are the
separate `reference_assessment_v2.json` and `reference_assessment_traces_v2.json`:
166 policy vectors and 224 depth cases. A separate frozen supplement has
30 verdict-precedence cases and 15 integer rounding values, including net=7
rounding to 22. No Python reference or prior P04 oracle
was changed to accommodate disagreement.

The first executable depth test passed; policy comparison exposed serde_json's
default parse rounding of 999.9999999999999 onto 1000. Enabling its
`float_roundtrip` feature preserves saved boundary values. Initial Rust syntax,
API signature and usize/u64 casts were corrected. Sandbox execution blocked the
existing LLVM linker; offline escalated checks use `.tools/msrv-gcc-lld.cmd`.
No network dependency fetch is needed.

The generated-audio checker initially expected all receipts to be available.
One is shorter than the required nine two-second probes and one has no qualified broad-band segment
probes: D03 correctly requires partial/null output for those. The complete
44.1kHz splice control additionally exposed a real P05 gate error: a sample rate
with no geometrically eligible foreign-Nyquist candidates was treated as missing
resampling data. P05 now uses the pinned/native geometry test to distinguish
inapplicable from missing. Its generated-noise API test requires `available`;
the short and unqualified controls continue to require partial/null results.
The first tightened API fixture was only 12 seconds (six repeats of a two-second
24-bit generated WAV), below the pinned 18-second minimum. It now uses nine
repeats with exact sample words retained. Full-band probes also correctly have
null cliff measurements when their upper cliff band would exceed Nyquist.
Validation now permits those nulls; adaptive-wall logic short-circuits definitively
false predicates, and missing inputs inside eligible anomaly branches still
propagate. This changes no weight or threshold and avoids permanent partial
results from irrelevant measurements.

`generate_reference_labels.py` transcribes all 12 pinned scalar `_interp_*`
functions into audit-only Rust labels. Its read-only check verifies source token
identity after Rust formatting. Existing private recordings and reference
checkout remain untouched; local build/receipt files are not Git history or an
external backup.

## Checks

P05 DONE, 2026-10-06. Final Rust 1.85 optimized/no-CLI suite: **61 passed**
(58 library + 2 assessment API + 1 serialization), zero failed/ignored.
All-target no-CLI Clippy with warnings denied, no-CLI example build, ARM64
Android target compilation, formatting and whitespace checks passed. No
Android linking/phone claim is made. Read-only Python policy/trace/verdict
reproduction, audit-label transcription, helper syntax and pinned parity
inventory passed. Four final generated-audio saved-result comparisons passed:
18s noise gives main=0, splice main=55; short/insufficient-probe controls remain
partial/null. Final output unit labels and unchanged source-receipt bytes passed.

Exact commands and repaired attempts are in `task-results/P05.md`. Final logs
are `target/p05/accepted-{tests,build,clippy,android}.log`; generated outputs,
source-receipt/PCM hashes and summary are `target/p05/accepted-generated/`.
No unresolved P05 failures or running jobs remain.
No Android linking/device result, full core/CLI regression, private audio,
calibration, commit, push or upload is claimed.
