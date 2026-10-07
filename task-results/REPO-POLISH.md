# Repository polish and DSD deferral — 2026-10-06

Owner explicitly approved deferring F02/DSD beyond the first standalone release
and Alfred launch to conserve Astra budget. Initial formats are FLAC/WAV/ALAC.
P07 is next, then P08/P09; Alfred still begins at A01 after P09. F02 stays mapped
and DEFERRED, with its complete frozen future format/conversion contract retained.
Launch-scope amendment changes delivery gates, not completed reference method
contract_version, report schema, scoring or numerical tolerances. P08/P09 must
record deferred native DSD rows and honest unsupported outcomes.

Repository polish groups 35 validation records under docs/validation and moves
integration, interpretation and corpus guides under docs/. README is a concise
front page; detailed reference and feature history remain under docs/. The full
previous handoff is archived, with a shorter self-contained current handoff.
Documentation and task-result indexes make records discoverable. Root instruction,
roadmap, owner and frozen-contract filenames remain stable for continuation tools.
Actual Markdown links are rebased; bare historical filenames use the index.

Historical acceptance content is preserved apart from relative link targets.
A pre-existing non-UTF-8 byte in NOISE_FLOOR_VALIDATION.md was preserved rather
than rewritten as part of relocation. An initial relocation script stopped on
that byte before moving files; a byte-preserving retry completed the move.
Source/fixture/schema/private-audio layout is unchanged. The only Python helper
edit updates the parity inventory checker's explicit P08 dependency expansion to
exclude deferred F02; the future F02 ownership mapping stays checked.

Actual checks:

- `python scripts/check_parity_contract.py`: PASS, 155 fields, 139 methods,
  30 groups, 34 rules, owners, revised acyclic dependencies, local links and
  clean pinned Python reference. This inventory check does not establish DSP parity.
- Repository Markdown link check: PASS, 231 local targets resolve across root,
  docs, task-results and schemas (before this final record expansion).
- Python AST parse of the updated inventory helper: PASS.
- `git -c core.safecrlf=false diff --check`: PASS.
- Preservation manifest: 350 pre-existing nonignored files accounted for,
  333 byte-identical at their original/relocated path, 17 intended existing edits,
  zero missing and zero unexpected non-document edits. The only non-Markdown edit
  is the inventory helper's delivery dependency expansion. Rust source, manifests,
  binary fixtures, schemas, assets and examples remain byte-identical.
- All 35 relocated validation records were checked for content equivalence
  apart from Markdown targets. The complete old handoff and both original README
  portions are preserved in archive/reference documents apart from link rebasing.
- Root files reduced from 53 to 15; 38 documents relocated. No files deleted.
- Rust/DSP/Android tests/builds were not run for documentation/scope changes.

Receipts include before.json, moves.json and preservation.json.
Receipts under ignored target/repo-polish-20261006 are local evidence, not Git
history or an external backup. No implementation, DSD work, Rust/Android build,
commit, push, upload or app repository extraction was performed. No jobs running.
Next action: requested P07, with the revised initial-launch scope.
