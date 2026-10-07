# Alfred product boundary — 2026-10-06

DONE: owner-requested planning correction only. No implementation packet started.

The standalone Audio Forensics delivery ends at P09 after remaining F02, P07,
P08. Alfred development begins at A01 afterward; the earlier overlapping Android
start permission is superseded. F02 remains default; P07 is independently eligible.

Alfred owns workspace/selection, applicable operations, input/staging,
jobs/lifecycle, retention and export. A03/A06 integrate Audio Forensics as a
consumer of this independent Rust library. New A06a/A06b cards separate existing
Spectrogram/Compare integration from forensic results; A07 depends on all three.
Future Tag Studio, Converter and Archival Tools have no executable tasks or launch
gates. Existing P02 read-only metadata and F02 analysis conversion do not implement
those tools. General Compare requirements beyond the pinned P07 workflow are open.

ALFRED_ARCHITECTURE.md records responsibility-level interfaces, temporary separate
app subtree, one-way dependency, future app repository extraction and independent
library/CLI builds. A01 chooses concrete bridge/layout/packaging/resource contracts;
no exact repository move date or new beta dependency was invented. No consequential
product ambiguity blocks this plan.

Changed planning files: AGENTS.md, ROADMAP.md, ROADMAP_TASKS.md, PORTING_PLAN.md,
CORE_INTEGRATION.md, RELEASE_CONTRACT.md, README.md, OWNER_CHECKLIST.md,
HANDOFF.md, plus new ALFRED_ARCHITECTURE.md and this record. Owner requirements
were supplemented from the explicit correction; existing answers are retained.
The release contract title/scope now describe the forensic deliverable; frozen
formats, numerical tolerances, deviations and completed evidence are unchanged.
The unimplemented proposed envelope ID is retained with an explicit forensic-only
scope and P07 final naming obligation, avoiding a universal Alfred report schema.

Checks:

- `python scripts/check_parity_contract.py`: PASS, 155 fields, 139 methods,
  30 groups, 34 rules, task owners, acyclic board and local links; pinned Python
  reference clean. This is inventory validation, not numerical acceptance.
- Targeted Python document check: PASS, all 34 board IDs have task cards; local
  Markdown links in nine revised planning documents resolve.
- `git diff --check`: PASS; Git emitted CRLF normalization notices only.
- Rust/DSP/Android builds/tests: not run; no implementation changed this turn.

Existing dirty and untracked implementation files were inspected and preserved.
Historical validation/task records, schemas, fixtures and source files were not
edited. No private audio analysis, repository move, commit, push, upload or
publication. No jobs remain running. Next concrete action: requested F02 using
its existing frozen format/conversion contract.
