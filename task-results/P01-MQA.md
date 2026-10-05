# P01 addendum: existing independent MQA work

Inspected read-only on 2026-10-05 after the owner supplied its location during
P01. Resolved path: `C:\Users\Bishal\Documents\antigravity-dev\mqa` (plural
Documents). No files in that tree were changed, copied into production, decoded,
uploaded or rebuilt. This is intake for the fidelity contract, **not completed
E09 validation**. The owner reported independent work, not a new requirement to
make MQA integration block the Android release.

## Existing assets

- `mqa-engine` 0.1.0: Rust 2024, MSRV 1.85, serde, optional Symphonia corpus
  feature, unsafe forbidden. `src/mqa_detector.rs` supplies configurable
  three-second/full-stream and early-exit modes, target/off-plane scanning,
  payload/cadence runs, retained packets and a legacy-observation conversion.
- Eight source unit tests, five optional corpus tests, `corpus_scan` and
  `test_converted` examples. Tests were inspected, **not run** in P01.
- `ARCHITECTURE.md`, `INTEGRATION.md`, `INSTRUCTIONS_FOR_OPUS.md`, Python
  exploration/survey scripts and `tools/out` evidence. Treat the integration
  recipe as a proposal: the prototype uses its own result/status types, and
  extending the current strict schema requires an explicit version decision.
- 57 local FLAC files and a 57-record saved full survey. The saved JSON sums to
  **55 records with sync matches, 2 without, 23,552 sync matches, zero off-target
  matches**; depth counts 44×24-bit and 13×16-bit, rates 39×44.1k and 18×48k.
  These are counts from saved observations, not newly validated provenance or
  independent negatives. The architecture's “57 MQA / 23,497 packets” wording
  must be reconciled with these counts (difference 55); do not silently rewrite
  original evidence. `check3s.json` has three records, not a 57-file Rust receipt.

Fingerprints of the inspected local artifacts:

| File | SHA-256 |
| --- | --- |
| `mqa-engine/src/mqa_detector.rs` | `cc7bdbee28ab84f951afba1c3aa9dfed6f052e7ec94779a76a47be6b924409b4` |
| `tools/out/survey_full.json` | `c1e17093795d47be997be0ad04aa80736dbc3c51fffb2e0842934896191882c1` |
| `tools/out/check3s.json` | `ca5c7fe0fee15016d9e77d05a2b803b92403095eaa114bb0e1bb1413f8fa21c2` |

## Finite E09 review obligations discovered by inspection

1. **Confirmation/payload binding:** `validate_completed_packet` resets current
   consensus after a mutation/cadence break but retains `max_consecutive_valid`;
   `finish` decides confirmation from that maximum and reports the latest
   consensus. Define whether confirmation means any historical qualifying run
   or a consistent entire interval. Bind output payload to the qualifying run
   and report later anomalies. Reproduce three matching packets followed by a
   changed payload and by a cadence break before choosing the fix.
2. **Packet representation:** `raw_preamble` claims 40 bits, but completion and
   removal occur at offset 33. Off-target completed packets do not pass through
   the retained-packet updater. Test exact raw bit count, complete/partial
   payloads on every monitored plane, cap boundaries and retained truncation.
3. **Probability language:** the IID Bernoulli derivation is an assumed random-bit
   model, not a calibrated false-positive probability for arbitrary music,
   correlated/repeated signals or intentionally inserted markers. Do not expose
   `false_positive_p_bound` as general confidence, legal/scientific certainty,
   or “mathematically impossible.” Document scan/plane/search factors, trial
   scope and assumptions if retaining it as a theoretical model quantity.
   Marker consistency is separate from authentic mastering/rendering/history.
4. **Host integration:** preserve native integer applicability and exact plane
   mapping, reject invalid configs, bound retention/in-flight storage, check
   cancellation at host boundaries. Scanner early-exit must never stop the
   host's full decode/hash/integrity or other measurements. The converted-file
   example's fallback `Some(24)` is not a production precision oracle.
5. **Evidence acceptance:** corpus tests return success when a file is absent
   and the runner can skip decode errors. Retain a manifest/hash/actual-run
   receipt with explicit skips/failures and independently checked PCM. Reuse
   the supplied recordings locally; generate public tests for malformed,
   repeated, inserted, truncated, off-plane and late-anomaly streams. Do not
   treat two zero-hit music files as proof of 100% population specificity.

These findings are source inspection, not executed defect reproducers. E09
starts from this prototype and receipts, not from a new speculative scanner.
No claimed test/build result is inferred from the presence of a target folder.

## Roadmap disposition

E09 becomes **READY** for a named existing-prototype review/acceptance packet;
the old “waiting for any MQA recordings” premise is superseded. Integration
follows acceptance via E10 (and schema/host tests), not a blind file replacement.
P05 continues to exclude the pinned Python unconditional MQA score-of-100.
Until accepted integration, production still exposes candidate observations.
The first Android release does not wait for E09. Preserve this external tree and
its private evidence separately; it is not part of this repository's source
history or an established external backup.
