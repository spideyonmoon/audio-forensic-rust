# Project instructions

## Start here

- Read `HANDOFF.md` before continuing work. Then read `PORTING_PLAN.md` and the
  validation document relevant to the change (see `docs/validation/README.md`).
  Older handoff details are in `docs/history/HANDOFF_20261006.md`.
  Inspect actual files before assuming that a recorded status is current.
- Read `ROADMAP.md`, the requested card in `ROADMAP_TASKS.md`, and current
  `OWNER_CHECKLIST.md`. A plain resume prioritizes the active/next eligible
  delivery task. Finish a named packet and update its status and handoff.
- Product boundary: finish standalone Audio Forensics through P09, then begin
  Alfred A01. Read ALFRED_ARCHITECTURE.md; Alfred is a file/folder workspace
  consuming this independent core, initially co-located and later extracted.
  Shared input/jobs/storage belong to Alfred; Forensics, Spectrogram and Compare
  are independent features. P06/A06a are only the initial Spectrogram slice;
  its broader competitive viewing scope needs later requirements. Reuse current
  spectrogram/comparison code without duplication/refactoring; A01 defines their
  long-term ownership/extraction boundary. Other tools remain future placeholders.
- Current priority: faithful Rust rewrite and the offline Alfred Android app,
  while endgame detector research proceeds separately. Android is authorized;
  endgame accuracy/calibration and MQA confirmation do not block the first app.
  Initial formats are FLAC, WAV and ALAC/M4A. Owner deferred F02/DSD beyond
  the first standalone release and Alfred launch on 2026-10-06. P09 is accepted;
  A01/A02 are complete; A03 implementation is finished, with final runtime CI
  acceptance pending in HANDOFF.md. A04 is independently eligible after A02.
  Android 11–16 scaffold CI passed; use GitHub Actions for heavy Android checks.
  Keep the frozen future DSD contract; follow current task cards.
- Conserve usage: targeted inspection/checks, concise updates, Sol for bounded
  implementation and Astra for difficult DSP/policy/native-boundary work. Do not
  rerun completed audits without a concrete reason or silently expand scope.
- Keep changes focused on the current milestone. Preserve unrelated user work.

## Analysis and validation contracts

- Preserve `reference/audio-forensic` at the baseline recorded in
  `PORTING_PLAN.md`. Preserve all user recordings under `reference/test_files`.
- Reports currently keep ancestry `INCONCLUSIVE` and evidence index `null`.
  Detector hits are provisional observations, not calibrated probabilities or
  proof of authenticity. Do not add confident scoring without corpus validation.
- The roadmap authorizes a separate versioned Python-reference assessment with
  traceable uncalibrated scores/verdicts and explicit deviations. Do not confuse
  that faithful product behavior with the unchanged measurement report or with
  validated endgame conclusions. Follow P01/P05 rather than inventing weights.
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
