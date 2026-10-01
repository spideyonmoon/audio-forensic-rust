# Project instructions

## Start here

- Read `HANDOFF.md` before continuing work. Then read `PORTING_PLAN.md` and the
  validation document relevant to the change. Inspect the actual files before
  assuming that a recorded status is still current.
- The goal is an entirely offline Rust analysis core, followed by an Android app
  after core validation. Core development can continue while the user collects
  independently characterized recordings.
- Keep changes focused on the current milestone. Preserve unrelated user work.

## Analysis and validation contracts

- Preserve `reference/audio-forensic` at the baseline recorded in
  `PORTING_PLAN.md`. Preserve all user recordings under `reference/test_files`.
- Reports currently keep ancestry `INCONCLUSIVE` and evidence index `null`.
  Detector hits are provisional observations, not calibrated probabilities or
  proof of authenticity. Do not add confident scoring without corpus validation.
- Keep native channels, exact integer PCM, applicability, analysis intervals,
  bounded buffers, cancellation and structured failures explicit.
- Compare decoding with exact PCM hashes and DSP with documented tolerances.
  Investigate failures; do not regenerate reference outputs merely to pass tests.
- Public fixtures contain generated signals only. User-described CD/vinyl origin
  is not verified ground truth. Keep private audio and local reports out of public
  source history; do not upload them as part of routine development.
- Run checks appropriate to the change. Record exactly what passed, failed or was
  not run. Android target compilation does not establish linking or phone behavior.

## Durable progress

- After each meaningful milestone, and before handing off incomplete work, update
  `HANDOFF.md` with the date, current state, actual checks, outstanding failures,
  next concrete action and any running jobs or partial changes.
- Record detailed experiments in the applicable validation document. Update the
  README and porting plan when supported behavior or decisions change.
- Keep the handoff self-contained: another tool must be able to resume from files
  without conversation history, a particular AI subscription or hidden memory.
- Distinguish local files, Git history and external backups. Do not describe one
  as another or claim that ignored local evidence is included in a source backup.
