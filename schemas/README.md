# Report contracts

P04c adds [`reference-inputs-1.schema.json`](reference-inputs-1.schema.json).
`ReferenceAnalysis` wraps unchanged `measurement` and separate `reference_inputs`.
Dispatch on inputs version **1** and exact method `python-c6ecce2-p04c-f32-v1`.
Success requires reference inputs; other statuses require null. Values require
finite numbers with null reason, or null with a nonempty unavailable reason.
Three verified PCM hashes, explicit domains/intervals/deviation IDs, capped
probe arrays and separate source audit/qualified values prevent native inputs
from masquerading as reference inputs. The streaming helper additionally checks
hash identity, exact counts/intervals and frozen Python numerics. P05 must use
qualified fields and supply the missing adaptive-wall dependencies. Read
[REFERENCE_INPUTS_VALIDATION.md](../REFERENCE_INPUTS_VALIDATION.md).
`python scripts/generate_reference_inputs_schema.py --check` detects model drift.

P03a adds separate [`tool-statistics-1.schema.json`](tool-statistics-1.schema.json).
Dispatch on statistics version **1**, contract **1** and exact method
`ffmpeg-7.1.1-sox-14.4.2-v1`. `ToolAnalysis` wraps unchanged `measurement`, P03
`byproducts` and `tool_statistics`; validate all three separately. Successful
statistics require stream/coverage/measurements; other statuses suppress coverage
and measurements. Scalar applicability/value/display/reason is distinct from
legacy tool text and the mixed last-occurrence LoudnessProfile audit. Canonical
SoX stat has exactly 15 required keys; astats lane/overall and legacy audit maps
require all named fields. Astats channels/overall and drmeter
channels/overall remain separate. Schema validates shape; the helper also checks
PCM identity, exact counts, domains/nulls, formatting and frozen tool numerics.
Read [TOOL_STATISTICS_VALIDATION.md](../TOOL_STATISTICS_VALIDATION.md).
`python scripts/generate_tool_statistics_schema.py --check` detects model drift;
`scripts/check_tool_statistics.py --check-saved --output RECEIPTS` rechecks saved
controls without decoding or changing the frozen oracle. P07 owns CLI exposure.

P03 adds separate [`byproducts-1.schema.json`](byproducts-1.schema.json).
Dispatch on byproduct version **1**, contract **1** and the exact reference
method ID. `ByproductAnalysis` wraps the unchanged `measurement` report and
`byproducts` report; validate each against its own schema. Successful byproducts
require stream/coverage/reference/native-level records. Other statuses require
null coverage/reference/levels. Available display values require finite numeric
values/text with null reason; unavailable values require null/text reason.
Silence output is capped at 1024 sections, with all-run totals and omission
counts; the exact RMS percentile has a declared scalar cap and ResourceLimit
state. Legacy verdict/EOF strings are audit fields, not ordinary product claims.
`scripts/check_byproducts.py` additionally checks PCM identity, intervals, counts,
native mappings and pinned function parity. Read
[BYPRODUCT_VALIDATION.md](../BYPRODUCT_VALIDATION.md).
`python scripts/generate_byproduct_schema.py --check` detects model drift without
rewriting this artifact or the existing measurement/metadata/audit schemas.

P02 adds separate [`metadata-1.schema.json`](metadata-1.schema.json) and
[`replaygain-audit-1.schema.json`](replaygain-audit-1.schema.json). Dispatch by
metadata/audit version **1** and release contract **1**; unknown versions are
rejected by these producer schemas and the integrated audit's contract check.
Metadata entries retain original keys/order/duplicates and bounded UTF-8 text;
named fields reference entry indices. Artwork contains descriptors. A negative
text search is incomplete when text/opaque metadata is unavailable. Schema
validation checks shape; `scripts/check_metadata.py` also checks byte budgets,
omission accounting and index references. Read
[METADATA_VALIDATION.md](../METADATA_VALIDATION.md) for finite container coverage,
source binding, native-meter method and known Python projection differences.
`python scripts/generate_metadata_schema.py --check` detects separate model
drift without rewriting either schema. The measurement schema below is unchanged.

[`analysis-report-0.18.0.schema.json`](analysis-report-0.18.0.schema.json) describes
one serialized `AnalysisReport` using JSON Schema Draft 2020-12. Select a schema
by the report's `schema_version`. The CLI emits an array; validate each element
against the object schema. An optional local tool accepts either representation.

Every declared field is required, including fields whose value can be `null`.
Null preserves unknown or inapplicable measurements. Unknown object fields are
rejected by this producer-contract check; dynamic measurement/threshold maps
allow arbitrary names with finite numeric values. This strict validation is
separate from a consumer's choice to tolerate future extensions.

For this version, ancestry is always `INCONCLUSIVE`, evidence index is `null`,
and policy is `observations-only-v18`. Engine version is a nonempty string so a
producer can retain this schema across engine revisions. Successful reports
require supported stream metadata, native channels and two-pass coverage.
Unsuccessful reports may retain partial metadata. Detector hits are provisional
observations; accepting a report does not validate detector accuracy.

Preserve integer frame counts exactly, including u64 values above 2^53. The wire
domain for `usize` is bounded by u64; a 32-bit Rust consumer may impose a smaller
platform limit. JSON Schema defines integer mathematically, so a number such as
`1.0` can satisfy its integer type even though the producer emits integer tokens.
Required nullable fields and array lengths are part of this version's contract.

The optional validator adds checks that compare fields: native channel order and
sample counts, interval bounds, referenced channels, PCM hash encoding versus
sample precision, and diagnostics for unsuccessful outcomes. These relationships
are not all expressible in the standalone JSON Schema.

For local development:

```powershell
python scripts/generate_report_schema.py --check
python -m venv .tools/report-schema-venv
& ./.tools/report-schema-venv/Scripts/python.exe -m pip install -r scripts/report_schema_requirements.txt
& ./.tools/report-schema-venv/Scripts/python.exe scripts/validate_report_schema.py path/to/report.json
```

Dependency setup is optional development work. Validation uses only local schema
references and explicitly disables external retrieval. No Python package is
required by the offline Rust core. Private validation receipts, when requested
with `--output`, must be new files under ignored `corpus/local`.

The generator accepts only the current plain serde model syntax. Unsupported
types, custom serialization, serde attributes or ambiguous enum names fail for
review. `--check` detects drift without rewriting the artifact; generation refuses
to overwrite an existing version. Review a wire/policy change and its version
before creating a new schema. This exporter is not a general Rust parser.

See [`REPORT_SCHEMA_VALIDATION.md`](../REPORT_SCHEMA_VALIDATION.md) for the actual
checks and retained initial failures. This artifact does not freeze the evolving
Rust API or declare the detector suite complete. Include `schemas/` in a future
source-only publication so consumers can use the versioned contract. Local
scripts/tests and private receipts remain separate from published source history.
