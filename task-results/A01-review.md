# A01 focused review — 2026-10-07

Requested after the owner noticed A01 had been assigned to Sol. Reviewed the
deliverable rather than treating the model recommendation as a correctness gate.

**Review decision: retain the architecture; make three bounded contract corrections.**
The findings below describe the original contract. The owner's subsequent request
authorized those corrections only; they are recorded under Resolution below.
No redo, new research packet or core change is needed. The original A01 checks
remain historical evidence, not tests of Android behavior.

## Findings

1. **P2 — Probe lacks a cancellable lifetime and a complete payload route.**
   ANDROID_CONTRACT.md's bridge table makes `probe(path, generation)` a separate
   metadata response, while only `start` returns the handle required by
   cancel/close. `metadata::read_metadata_source` accepts a cancellation token,
   performs I/O and acquires the core permit; the proposed probe has no way for
   the host to address that token or release its lease after selection changes.
   Generation filtering prevents stale UI updates but does not cancel work.
   Also, the 64 KiB control-response cap cannot carry all valid metadata: the
   core permits 1 MiB of retained tag text before JSON overhead. Specify probing
   through the same job/handle cancellation lifecycle (or an equivalent explicit
   probe lifecycle), a small capability summary, and a file descriptor/manifest
   route for complete metadata. Do not truncate valid metadata or reject an
   otherwise supported input merely to fit control JSON. Add probe cancellation,
   changed-selection/lease-release and >64 KiB metadata acceptance cases.

2. **P2 — Compare needs operation-wide scope negotiation.**
   The contract offers different per-input prefix lengths but never defines how
   a live multi-input Compare request selects one interval. For example, a
   five-minute 48 kHz track can run full while its 96 kHz variant is offered a
   90-second prefix. P07 correctly rejects those results as incompatible, so
   independently following the admission rules defeats a routine mixed-rate
   comparison. Specify a shared user-selected interval that satisfies every
   input's budget, and recheck actual duration/EOF compatibility at completion.
   Preserve explicit unavailable/incompatible outcomes rather than auto-trimming
   saved reports. Carry the existing prerequisite that the user identifies these
   as variants of one track into operation discovery/request validation; the
   engine does not identify songs. Add a 48/96 kHz comparison scenario to A06b.

3. **P2 — The PNG default changes the frozen release behavior.**
   ANDROID_CONTRACT.md selects Standard 1600×900 as the default and withholds
   larger presets pending device checks. RELEASE_CONTRACT.md and the accepted
   P06/P07 guide specify Publication 2560×1440 as the default, with all three
   bounded presets. The contract cites no device result requiring a change.
   Preserve the accepted default, or explicitly record an Android-specific
   product decision and its acceptance conditions. A lower-resolution display
   preview need not change the exported PNG default. This is a small fidelity
   discrepancy, not a renderer defect.

## What is sound

- Separate app/native subtree, one-way dependency and independent root builds;
  unsafe JNI outside the safe core, pinning and extraction direction.
- P06/P07 computation, rendering, contracts and generated tests remain reusable
  in the core; Alfred owns independent feature navigation/UI/platform tests.
- App-owned product worker correctly accounts for AnalysisJob's measurement-only
  return type. Serial admission, token lifetime after close, unwind containment,
  exact u64/null handling and original saved bytes are appropriate.
- SAF/private disk staging is an allowed conservative engineering choice.
  700 MiB import, no whole-audio RAM buffer, quotas and process-death interruption
  remain explicit. Native DSD and broader future tools were not pulled into scope.
- Android 14–16/min 34/target 36, arm64 and 16 KiB verification are coherent.
  Official documentation rechecked NDK 30.0.16248370, mediaProcessing availability
  from API 35, specialUse requirements and stopSelf-on-timeout obligations.
- The phone memory figures are explicitly provisional. Their lack of device
  measurements is correctly assigned to A07, not a fabricated A01 test pass.
  High-rate whole-file rejection/prefix offers are a material resource policy
  to retain visibly in the UI and validate, not an established phone limit.

## Evidence and limits

Read the A01 card, current contract/result/handoff, release/product/integration
guides, relevant resource/job validation and source symbols in metadata.rs,
decode.rs, job.rs, reference_source.rs and product.rs. Inspected the existing
A01 checker and its receipt: its 17 marker checks, API-name checks, link checks
and unchanged-tree hashes are useful consistency evidence, not semantic tests
of the new Android design. No full regression or DSP experiment was justified.

Authoritative platform references rechecked:

- [NDK downloads](https://developer.android.com/ndk/downloads)
- [Foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types)
- [Foreground service timeouts](https://developer.android.com/develop/background-work/services/fgs/timeout)

The initial review changed only the review record/index and handoff annotation;
it did not fix the findings. The subsequent correction is recorded below.
No build, SDK installation, device test, source/fixture change, private-audio
analysis or publication was part of that review.

## Resolution — 2026-10-07

All three findings are corrected in ANDROID_CONTRACT.md:

1. Probing is `start(kind=probe)` on the same serial worker/handle lifecycle,
   with cancellation on an independent control executor, selection-generation
   invalidation and lease/output cleanup after worker exit. Complete MetadataReport
   v1 is a saved payload; poll returns a small summary and descriptor. The 64 KiB
   control limit no longer implies a metadata truncation limit. A03/A04 acceptance
   includes cancellation, selection replacement and metadata larger than 64 KiB.
2. Compare requires the user's same-track assertion and one negotiated full or
   common prefix scope for all live inputs. The 48/96 kHz example uses 90 seconds
   for both. Whole-second prefix choices avoid unequal frame rounding. Serial
   probe/re-acquisition preserves the single-snapshot quota and checks encoded
   hashes before analysis. Actual EOF/method/domain/version compatibility still
   belongs to P07; saved reports are never trimmed to force a match. A06b cases
   now cover these conditions.
3. Publication 2560×1440 is again the saved-render/export default. Standard and
   Large remain selectable, preview sizing cannot silently change export, and
   explicit render failure preserves the requested preset. A06a covers all three.

Affected checks passed: contract/source/release consistency for all three fixes,
the shared 48/96 kHz scope calculation, 60 local file targets across six affected
documents and whitespace. See task-results/A01.md for the exact commands and
local receipt. Manual review covered cancel/close/selection races, snapshot
ownership, actual-EOF compatibility and preview/export separation.
No implementation/runtime test is claimed.
This is a correction of the unshipped A01 contract, not an Android implementation.
No open design finding from this review remains. A02 is still unstarted and is
the next delivery packet for a later request. No jobs are running.
