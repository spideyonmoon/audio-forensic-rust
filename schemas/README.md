# Report contracts

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
