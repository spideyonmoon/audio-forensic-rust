# P05 oracle versions

`reference_assessment_v2.json` and `reference_assessment_traces_v2.json` are
the accepted policy harness version. Assessment API version remains **1**.
`reference_verdict_boundaries.json` adds verdict precedence and ties-to-even cases.

The files without `_v2` are intentionally retained **rejected harness v1**
artifacts. A vinyl equality check exposed incorrect selection of an AST branch
in the oracle harness. They are not used as acceptance expectations. See
`REFERENCE_ASSESSMENT_VALIDATION.md` for the diagnosis and independent correction.

Generators use exclusive creation; `--check` is read-only. No fixture contains
private recordings or data derived from private audio.
