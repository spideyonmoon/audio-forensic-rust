# Report contract validation — 2026-10-03

Engine/schema `0.18.0`, policy `observations-only-v18`. The user requested the next
core job after the AAC audit and deferred Android. This milestone adds a versioned
machine-readable report contract and development checks. Production Rust, Cargo
dependencies, wire output, engine/schema versions and detector policy are unchanged.

## Contract and scope

`schemas/analysis-report-0.18.0.schema.json` contains all **47** serialized model
definitions from `src/model.rs`. Every field is required; `Option` permits null
rather than omission. Structs reject extra properties, enums use exact current
snake-case variants, fixed arrays retain their lengths, unsigned widths are
bounded and f64 values must be finite. Measurement/threshold maps accept numeric
values with arbitrary string keys. PCM hashes require 64 lowercase hex digits.

The root fixes the current schema/policy and observations-only ancestry/index.
Engine version remains a nonempty string. Analyzed reports require non-null
stream/coverage, 8–384 kHz mono/stereo, two passes and native channel results.
Other outcomes may retain partial metadata outside the analyzed support matrix.
The optional validator additionally checks unsuccessful diagnostics, native
channel order/sample counts, interval bounds, referenced channels and precision
versus PCM hash encoding. It does not compare DSP values to external oracles,
verify a hash against audio, establish every detector invariant, or calibrate
source history. The fresh Rust test independently verifies its integer/float
PCM hashes against generated source samples.

The schema uses [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12/json-schema-validation).
The development validator uses the standard library's strict JSON loader plus
`jsonschema==4.26.0` and its Draft 2020-12 validator. A configured
[reference registry](https://python-jsonschema.readthedocs.io/en/stable/referencing/)
rejects external retrieval; both `$ref` and `$dynamicRef` must be local pointers.
The schema URI identifies the dialect and does not cause a network request.
Installation happened once in ignored `.tools/report-schema-venv`; neither
global Python nor core runtime dependencies changed. Actual installed versions:
attrs 26.1.0, jsonschema 4.26.0, jsonschema-specifications 2025.9.1, referencing
0.37.0 and rpds-py 2026.6.3, with Python 3.13.

The restricted source exporter fails on unsupported serde syntax/types, manual
serialization or ambiguous acronym enum spellings. Drift comparison is
deterministic and never updates a reference to pass. It is scoped to this model
file, not a general Rust parser or proof that arbitrary new implementations are
equivalent. Wire changes still require review and version decisions. Integer
consumers must preserve u64 above 2^53; JSON Schema integers are mathematical,
not a restriction to lexically integer tokens. The `usize` wire bound is u64;
32-bit platform deserialization can impose a smaller limit.

## Actual checks

Private evidence is under
`corpus/local/results/report-contract-v18-20261003/`. No audio was uploaded or
reanalyzed from the private collection for this milestone. Existing reports were
read only and byte-checked. These local receipts are not included in Git history
or an external backup.

| Check | Result |
| --- | --- |
| Restricted schema exporter / artifact comparison | Passed, 47 definitions |
| Fresh Rust 1.85.0 contract integration | 1 test passed, 8 generated reports covering all five file statuses |
| Independent generated PCM hashes | Exact s16 stereo MSB-aligned s32le and float mono prefix f64le hashes passed |
| Rust serialization/deserialization | Structure and integers exact; maximum float difference 1 ULP, allowed maximum 4 ULP |
| Current saved report validation | All 117 passed: 113 analyzed, 4 unsupported |
| Fresh report validation | All 8 passed: 4 analyzed, one each failed/unsupported/cancelled/timed_out |
| Final validator on both sets | All 125 reports passed across 118 input documents |
| Negative contract/parser/exporter controls | All 51 invalid cases rejected |
| Positive numeric boundary controls | Both passed: exact u64/partial metadata and finite f64 endpoints |
| Rust 1.85 focused Clippy with warnings denied | Passed for `--test report_contract` and its core dependency |
| Stable `cargo fmt --all -- --check` | Passed |
| Python syntax for three new scripts | Passed |
| Source/manifest/index/reference preservation | Passed; details in `preservation.json` |

The Rust control uses generated s16 stereo, float mono with a 0.125-second
prefix at 11,025 Hz, 64-frame 8 kHz silence, the existing generated AAC-hit FLAC,
a truncated WAV, an unsupported leading wrapper, pre-cancellation and an immediate
deadline. It checks required nullable fields, provisional hit policy, diagnostic
presence and serializable reports. No private audio or new public audio fixture
is required. GNU host builds used the existing process-local GCC/LLVM shim and
cached `target/msrv-desktop-v18`; no system linker setting changed.

Saved reports came from the v0.18 PCM-relationships (26), preceding-energy (16),
spectral-lag (27), resource (9) and private-prefix (39) directories. Expected DSP
oracles and summary files were excluded. This validates the saved representation;
the four unsupported outcomes remain unsupported. Counts describe reports,
not independent recordings or accuracy samples.

Negative controls cover missing required nullable fields, unknown root/nested
fields, incorrect enums/versions/policy, confident ancestry, numeric evidence
index, boolean/fractional/negative/overflowing integer values, bad nullable/map
values, fixed-array lengths and invalid hashes. Consistency mutations cover
missing/native duplicate channels, wrong counts/precision/pass count, unavailable
channels, reversed/out-of-prefix intervals and failure without diagnostics.
Parser controls reject duplicate keys, NaN, infinities, f64 overflow and truncated
JSON; empty/non-object report batches also fail. External refs and unsupported
exporter changes are rejected. Positive limits preserve `18446744073709551615`
exactly and accept finite f64 endpoints. Null/abstention cases pass unchanged.

Initial failures were harness issues and remain recorded in `INITIAL_ATTEMPTS.md`:

- The first PowerShell wrapper invocation lost Cargo's `--` separator and exited
  before compilation. Array arguments fixed invocation; its log is `rust-test.log`.
- The first compiled test asserted bit-exact float JSON round-trip equality.
  Default decimal parsing shifted some values by one adjacent f64. The replacement
  recursively checks structure/integers exactly and floats within four ULP,
  producing a concise path on failure. The original failure is retained in
  `rust-test-final.log`; corrected success is `rust-test-corrected.log`. No core
  implementation or PCM expectation changed.
- An initial exporter mutation targeted nonexistent `path` rather than `source`.
  The control asserted that it had actually changed the source and failed before
  writing its receipt. Correcting that control produced 53 passing controls.

The prior 130-test debug/release regression was **not rerun**: production Rust and
dependencies are unchanged. The focused new integration and lints are the actual
Rust checks for this milestone. No Android install/link/device check, remote CI,
corpus accuracy evaluation, commit, push or external backup ran.

## Reproduce and continue

Contract usage/evolution guidance is in `schemas/README.md`. To create fresh
generated evidence in a new private directory, set `REPORT_CONTRACT_OUTPUT` to a
new file in that directory, then run `cargo +1.85.0 test --locked --test
report_contract -- --nocapture`. On this workspace use the process-local LLVM
linker and pass the wrapper arguments as an array, as recorded in `HANDOFF.md`.
Unset the output variable before ordinary reruns; create-new output intentionally
refuses to overwrite an earlier receipt. Run:

```powershell
python scripts/generate_report_schema.py --check
& ./.tools/report-schema-venv/Scripts/python.exe scripts/validate_report_schema.py path/to/generated-reports.json
& ./.tools/report-schema-venv/Scripts/python.exe scripts/check_report_contract.py path/to/generated-reports.json --output corpus/local/results/new-batch/controls.json
```

This contract milestone is complete. The next non-Android engineering job is an
app-provided source API audit: generated seek/read failures, source-length
inconsistency and mutation between passes, checking structured failures, worker
permit release and exact two-pass PCM coverage. Inspect existing parity/worker
tests before adding controls; fix only an observed contract defect. Stable API
review, complete support and grouped detector accuracy remain open gates.
