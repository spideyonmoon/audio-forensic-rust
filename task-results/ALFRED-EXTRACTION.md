# Alfred repository extraction — 2026-10-09

## Alfred extraction COMPLETE — 2026-10-09

Owner requested separate entity now. App/shared/features/native adapter/Android
workflows moved to C:\Users\Bishal\code\alfred, remote spideyonmoon/alfred,
branch codex/extract-android. Continue Android/UI/A07/A08 with that HANDOFF.md.
This repository retains independently usable core, schemas, generated DSP fixtures
and tests; source/Cargo files are unchanged from accepted engine pin 5c5ce00.
Alfred consumes that full Git revision with its own lockfile; no source vendoring.
Original history/dated Android records stay here. Old ignored app caches and
unrelated owner cleanup edits are preserved. Focused extraction is pushed in both
repositories; PR1 in each is open, neither merged. No release publication.

Compiled Alfred84355e7 passed both ABI links/APKs and all Android11–16 functional
suites across checkpoint 37892173286 (API 30/31) and source-bound reuse37893506220
(API 32–36). Each receipt has22 matching viewer/Compare controls plus native/input/
Forensics/UI and eight lifecycle/permission/recovery groups. Aggregate CI runs
remain red: first API 32 viewport-only harness failure was fixed/passed in reuse;
API 34 completed all tests then raw logcat UTF8 decoding failed. Collector fixed,
host boundary/syntax verified; no heavy rerun solely for diagnostics. This is
functional acceptance, not a claim of all-green CI or new physical behavior.

App-only idle foreground-service race and intermediate busy-admission regression
were corrected; details/earlier failures retained in Alfred task-results/EXTRACTION.md.
No numerical/schema/payload/adapter/private-audio changes. Windows Rust host link
failed ld204; Linux core/native controls passed. Kotlin/lint and final52s compile,
pin/source/hash/lock/syntax/link/whitespace guards passed. All7 local receipts and
APK bindings verified. Ignored local evidence is not Git history/external backup.

No jobs running. Next owner's UI scratchpad in Alfred; current visuals rejected
and unchanged. A07 physical resources/device and A08 signing/release remain there.
