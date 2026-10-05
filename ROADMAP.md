# Audio Forensic: delivery and endgame roadmap

Updated: **2026-10-05**. Owner: Bishal. App: **Alfred**. Current core: **0.26.0**.

## Direction agreed with the owner

Ship a useful offline Android analyzer after finishing a faithful Rust rewrite
of the Python product. Continue improving detector reliability toward the
endgame in a separate research track. The app does **not** wait for a perfect
detector, calibrated probabilities, a representative corpus or MQA confirmation.

The Python product combines DSP, structural/sample observations, empirical
detectors and interpretation rules. Do not describe it as entirely heuristic.
Faithfulness covers its useful outputs and workflows as well as its numerical
methods. Preserve intentional correctness/resource fixes; make deviations
explicit instead of quietly substituting different inputs into its rules.

This decision supersedes older “Android deferred” and “next: exhaustive AAC
phase search” instructions. Android is now authorized as part of the delivery
sequence below. **MQA prototype acceptance/integration remains separate**, and improving AAC beyond
the pinned Python behavior belongs to the endgame track unless a port defect
prevents reference parity. Historical validation records remain unchanged.

The endgame is retained: increasingly reliable detection, measured false alarms
and misses, broad real-world coverage, and honest abstention where samples do
not distinguish histories. It has no promised perfect-accuracy date.

## How to use this roadmap in a new session

Open this repository and select the model shown for the task. Send:

> Read AGENTS.md, HANDOFF.md, ROADMAP.md and the P04a card in ROADMAP_TASKS.md.
> Complete task P04a, including its acceptance checks and durable updates.
> Preserve unrelated work. Do not start endgame work or expand the release scope.

Replace **P04a** with any eligible task ID. The task card and shared execution
contract supply the rest; no old conversation is required.

For automatic continuation, send:

> Resume the delivery roadmap. Finish the active task, or the next eligible
> delivery task in ROADMAP.md. Follow its card and update the roadmap/handoff.

For research, say **“Complete E01”** or **“Resume the endgame track”** explicitly.
An ordinary “resume”, “continue” or “start cooking” prioritizes delivery.
One requested task is one work packet; an agent must not quietly turn it into
the entire roadmap. If a task spans sessions, resume the same ID and saved state.

## Current dispatch

- **Active task:** none; P04c completed 2026-10-05.
- **Next delivery task:** **P04a — missing spectral/reference inputs (Astra)**.
- **Owner inputs received:** Redmi 13 4G, Android 16, APK/ADB available;
  FLAC/WAV/ALAC-M4A/DSD; detailed verdict, complete metadata and deep results;
  approximately 600–700 MB upper typical file size; Alfred via GitHub Releases.
  Consider Android 14/15 compatibility. U02/U05 continue alongside implementation.
- **Next Sol task:** F01 or P06 when separately requested; A02 can follow A01 while core work continues.
- **Next endgame task when explicitly requested:** E01, the AAC trim work.
- **Independent MQA work:** existing prototype/corpus at
  `C:\Users\Bishal\Documents\antigravity-dev\mqa`; E09 now READY for a
  named review, not a beta gate. See `task-results/P01-MQA.md`.
- **Running jobs:** none.

P01 is a completed specification packet, not a completed port. Its frozen
[parity map](PYTHON_PARITY.md) and [release contract](RELEASE_CONTRACT.md) own
required rows, deviations, format scope and acceptance oracles. P03a/P04c are
new required child packets discovered by P01; other implementation packets remain uncompleted.

P02 is complete for the documented FLAC/WAV metadata adapters, with bounded raw
text/duplicates, editable encoder/MQA observations, native-scope ReplayGain audit
and separate schemas. See [P02 results](task-results/P02.md) and
[container coverage/validation](METADATA_VALIDATION.md). Opaque ancillary
structures stay explicit; this does not claim all binary tag formats, P07's
product workflow, or ALAC/DSD support.

P03 is complete: separate version-1 reference phase/ceiling/silence/RMS-p5
byproducts and native level/crest/fixed-delta mapping, preserving the two-pass
native report and policy. See [P03 results](task-results/P03.md) and
[byproduct validation](BYPRODUCT_VALIDATION.md). P03a is complete: bounded
native-lane/overall astats, all 15 SoX stat keys and distinct drmeter values,
correct linear/dB crest and legacy audit mappings. See
[P03a results](task-results/P03a.md) and
[tool-statistics validation](TOOL_STATISTICS_VALIDATION.md). P04c is complete:
separate f32 basis/base/scatter/phase/segment/transform inputs, a third verified
pass and bounded segment ring. See [P04c results](task-results/P04c.md) and
[reference-input validation](REFERENCE_INPUTS_VALIDATION.md). P04a/P04b own the
remaining spectral/source inputs and adaptive-wall dependencies. No P03b/P03c cards exist.

## Release boundaries

| Milestone | Required outcome | Does not require |
| --- | --- | --- |
| Faithful Rust core | Frozen Python feature/behavior contract, implemented required rows, traceable provisional reference assessment, documented deviations and passing scoped parity/regression | Better-than-Python detection, population calibration, MQA confirmation |
| Internal Android alpha | Installable app; import, analysis, progress/cancel and readable results on a real device | Store launch or polished research claims |
| Useful public beta | Core gate P09, device gate A07, owner acceptance U03, bounded storage/export, honest limitations and reproducible release artifacts | Completing any E task |
| Stronger detector releases | Separately versioned methods/policies evaluated against appropriate fresh controls and reviewed histories | Rewriting the app each time |

Required initial formats: **FLAC, WAV, ALAC in M4A, and DSD**, per the updated
owner file. Current implementation remains WAV/FLAC; F01/F02 close the additional
format gates. P01 froze mono/stereo DSF and uncompressed DFF at DSD64/128/256, with explicit
88.2 kHz converted PCM analysis; see RELEASE_CONTRACT.md for exclusions. Other
Python formats are later support, not additional release blockers.
Do not conflate M4A with ALAC or native DSD with PCM derived from DSD. Mono/stereo
is the existing core basis; unsupported modes remain explicit. Any unresolved
required format scope must be recorded rather than quietly removed.

### Faithful results without changing facts into probabilities

The current measurement report keeps ancestry `INCONCLUSIVE` and evidence index
`null`. It remains available unchanged to existing consumers.

P01 defines, and P05 will implement, a separate versioned **reference assessment**: the pinned
Python's decision rules, scores, contributing/veto rules and interpretations,
with their exact input scope and explicit uncalibrated status. A legacy score is
not a probability or calibrated evidence index. User-facing text attributes
conclusions to that reference method and qualifies unsupported source claims;
raw reference wording can be retained in the comparison/audit record.

This is the owner's requested faithful-product track, not a new calibrated
classifier. Preserve source-profile and bit-depth *candidate* interpretations
where their inputs can be reproduced; do not omit useful behavior merely because
endgame validation is unfinished. Never fabricate a missing input or claim a
modified measurement is mathematically identical. Explicit exclusions include
the MQA score-of-100 override and the claimed frequency-mirroring interpretation
of negated absolute correlation; keep their underlying observations where useful.
P01 records every other exception and its reason in RELEASE_CONTRACT.md D01–D12,
including exclusion of header-based ancestry penalties. The proposed checkpoint
policy awaits owner approval; no commit/push authorization is inferred.

## Delivery task board

Task details, allowed scope, inputs, outputs and finish checks:
[ROADMAP_TASKS.md](ROADMAP_TASKS.md). “After” means those tasks must be DONE,
unless the card explicitly permits work against the frozen contract.

| ID | Work packet | Model | After | Status |
| --- | --- | --- | --- | --- |
| P01 | Exhaustive Python/Rust map; freeze fidelity and release contract | Astra | — | DONE |
| P02 | Bounded tags, encoder traces and ReplayGain audit | Sol | P01 | DONE |
| P03 | Python byproduct measurements: phase, clipping, silence, level mapping | Sol | P01 | DONE |
| P03a | Bounded astats/SoX statistics and distinct DR output | Sol | P03 | DONE |
| P04c | Reference basis, segment, scatter and transform adapters | Astra | P01 | DONE |
| P04a | Missing spectral/reference inputs and declared geometry | Astra | P04c | READY |
| P04b | Source-profile input parity and adapters | Astra | P03, P04a | TODO |
| F01 | Offline ALAC/M4A decoding and exact PCM validation | Sol | P01 | TODO |
| F02 | Native DSD input and explicit conversion/applicability | Astra | P01 | TODO |
| P05 | Versioned Python-reference interpretation and rule trace | Astra | P02, P03, P04a, P04b, P04c | TODO |
| P06 | Bounded spectrogram artifact and rendering contract | Sol | P01 | TODO |
| P07 | CLI/report/comparison workflows using reference assessment | Sol | P05, P06, P03a | TODO |
| P08 | Frozen differential suite and final core regression | Sol | P02–P07, F01, F02; required child tasks | TODO |
| P09 | Core release review and finite acceptance decision | Astra | P08 | TODO |
| A01 | Android boundary, lifecycle and build contract | Astra | P01 | TODO |
| A02 | Android scaffold and reproducible native build pipeline | Sol | A01 | TODO |
| A03 | Native bridge and linked generated-input smoke | Astra | A02 | TODO |
| A04 | SAF import, seekable inputs and bounded staging | Sol | A03 | TODO |
| A05 | Background lifecycle, progress, cancellation and queue UI | Sol | A04 | TODO |
| A06 | Findings, spectrogram, history, comparison and export UI | Sol | A05, P07 | TODO |
| A07 | Real-device lifecycle/resource correctness gate | Astra + owner | A06, P09, U01 | TODO |
| A08 | Beta packaging, release notes and distribution handoff | Sol + owner | A07, U03 | TODO |

Default sequence: P02 → P03 → P03a → P04c → P04a → P04b → F01 → F02 → P05 → P06 → P07 → P08 → P09,
then A01 → A08. To reduce waiting, A01/A02/A03 may start after their listed
dependencies while core packets progress. A real-device smoke in A03 is useful
early; the complete device gate remains A07. A01 may design against P01's frozen
result contract without waiting for all policy implementation.

Separate sessions should run **sequentially by default**. Concurrent sessions
need explicit ownership of disjoint files or suitable worktrees; two sessions
must not edit shared source/manifests/handoff blindly. This roadmap does not
authorize automatic subagents, new chats, commits, pushes or publication.

## Owner checklist

Instructions and a fill-in log are in [OWNER_CHECKLIST.md](OWNER_CHECKLIST.md).

| ID | Your deliverable | When it matters | Status |
| --- | --- | --- | --- |
| U01 | Test phone/Android version, must-have formats, first-release priorities | P01 decisions; A01/A07 device selection | PROVIDED |
| U02 | Original recordings and truthful known/unknown provenance notes | Parallel research; never blocks the beta by itself | ONGOING |
| U03 | Run the agent's phone acceptance checklist and return observations | A07/A08 | TODO |
| U04 | Alfred via GitHub Releases; finalize package identity/signing | A02 placeholders allowed; final identity before A08 | PARTIAL |
| U05 | Preserve a full external research backup under your control | Now and before major machine/repository changes | TODO |
| U06 | Select fresh groups on demand after freezing each evaluation candidate | E04/E05; no permanent reserved set required | POLICY PROVIDED |

Do not purchase hardware or re-rip an entire library to unblock development.
Existing uncertain-history recordings stay useful challenge data. “I don't know”
is a valid history entry. Collection can proceed while the app is being built.

## Endgame task board — separate from beta gates

| ID | Work packet / bounded milestone | Model | After / data | Status |
| --- | --- | --- | --- | --- |
| E01 | Exhaustive AAC phase candidate for documented trim failures | Astra | Existing exposed development controls | READY, not default |
| E02 | Vorbis quantization-sensitivity diagnosis and candidate | Astra | Existing exposed paired controls | READY, not default |
| E03 | Reproducible broader codec/window/edit control campaign | Sol | E04 experiment contract; available encoders | TODO |
| E04 | Grouped real-corpus label/split/uncertainty protocol | Astra + owner | Existing evaluator; U02/U06 inputs | TODO |
| E05 | Frozen aggregation candidate and one defined unseen evaluation | Astra | E03, E04; reviewed controls and U06 | TODO |
| E06 | Original-depth inference research milestone | Astra + owner | E04; known-depth processing chains | TODO |
| E07 | Source-medium inference research milestone | Astra + owner | E04; documented capture chains | TODO |
| E08 | Genuine frequency-mirroring method and counterexamples | Astra | Exposed generated controls | TODO |
| E09 | Review existing MQA prototype, evidence and confirmation contract | Astra + owner | Existing independent work; named review | READY, not default |
| E10 | Integrate an accepted improved method into the shipped app | Sol | Accepted E candidate and versioned contract | TODO |

Research packets finish with a reproducible supported result, a rejected
candidate with reasons, or a precise data block. A negative research result is
not “detector solved”; it does prevent repeating the same experiment. No agent
may mark a data-dependent inference complete merely because its budget ended.

## Model allocation and spending discipline

Use **GPT-6.1 Sol** for implementation with clear interfaces/oracles, UI, metadata,
packaging, test harnesses, routine regression and corpus bookkeeping. Start with
medium reasoning; use high for nontrivial integration. Use **GPT-6 Astra** for
DSP semantics, conflicting evidence, compatibility policy, native ownership,
experimental design and release-risk review. Start with high reasoning; increase
only for a specific unresolved problem. These effort settings are suggestions.

This division follows the official descriptions of
[GPT-6.1 Sol](https://developers.openai.com/api/docs/models/gpt-6.1-sol) as a
cost-conscious option for complex work and
[GPT-6 Astra](https://developers.openai.com/api/docs/models/gpt-6-astra) for the
most demanding work (checked 2026-10-04). Task assignments are engineering
judgment, not measured project-specific model benchmarks or guarantees about
subscription consumption. API prices do not determine your app-plan quota.

Models are recommendations, not permission barriers. An agent must not claim it
switched models itself. If a Sol task encounters ambiguous DSP or a policy change,
save the smallest reproducer and the exact unresolved choice for an Astra pass;
continue unaffected work. Avoid repeatedly sending the entire repository to a
larger model, re-running completed matrices, or asking Astra to format documents.

## Session completion record

Agent statuses: TODO, READY, IN_PROGRESS, BLOCKED_INPUT, DONE, DEFERRED. Owner
items may be PROVIDED or PARTIAL. The table is
the dispatch index; the detailed evidence belongs in `task-results/<ID>.md`.
When a task starts, create that record and set its status here. When it ends,
record files changed, exact commands/results, unresolved issues, human actions,
running job IDs/locations and the next concrete step. Update HANDOFF.md too.
Do not call a task DONE until every acceptance item is met; separate device/data
requirements from completed code. Preserve interrupted or failed attempts.

Every task follows the shared execution contract in ROADMAP_TASKS.md. The
earliest eligible delivery task is the default next action; an explicitly named
task takes priority if its dependencies are satisfied.
