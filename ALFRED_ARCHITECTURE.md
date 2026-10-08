# Alfred product and repository boundary

Planning correction agreed 2026-10-06. This defines ownership and integration
seams, not implemented app behavior or detailed APIs for future features.

Alfred is an offline audio workspace. Audio Forensics is one feature. Open to
a file/folder picker or equivalent workspace, select one or more tracks or a
folder, then offer applicable operations. A forensic report is a feature result,
not the application's home, input model or universal result schema.

## Delivery boundary

The independent `audio-forensic-rust` library/CLI completed P07/P08/P09;
P09 accepted scoped engine 0.32.0 on 2026-10-06. A01 completed 2026-10-07;
[ANDROID_CONTRACT.md](ANDROID_CONTRACT.md) freezes the initial seams and retains
P06/P07 reusable backends/contracts/tests in this independent core. Alfred owns
their feature UI and the app/native host boundary. A02 scaffold/builds completed
2026-10-07; Android 11–16 scaffold CI passed. A03 native adapter and generated-input runtime
acceptance, A04 shared input, A05 jobs and A06 Forensics completed 2026-10-08;
A06a Spectrogram and A06b live/saved Compare completed 2026-10-09 with Android
11–16 generated runtime/UI acceptance; next A07 physical-device/resource gate.
Owner approved 2026-10-06 deferring F02/DSD
beyond that release and Alfred launch; its frozen future contract remains. Completed P/F packets and their validation remain intact.
P09 is the standalone release acceptance decision, not an Android gate or an
automatic public release. It covers required formats, faithful reference outputs,
metadata, spectrogram artifacts, CLI batch/comparison/export and scoped regression.
Endgame E tasks remain separate and do not block this delivery or Alfred launch.

Alfred development begins with A01 after P09. Build it here temporarily, in a
separate application subtree and separate adapter/build boundary. Once Alfred
has taken shape, move the application, shared Android infrastructure, feature
modules and app-specific native adapter to its own repository. The owner has
not set that extraction date; extraction is not an invented initial-beta gate.
A01 documents layout/extraction seams and explicitly decides long-term ownership
of reusable spectrogram and comparison functionality before extraction. A08
records remaining move work. Current residence in this crate is an implementation
reuse decision, not permanent product ownership by Audio Forensics.

The dependency points from Alfred to the Rust library. Keep the existing crate
name, library/CLI builds, safe core, host-neutral sources/jobs and public reports
usable without Kotlin, Android SDK, Alfred storage/navigation or app state.
Do not put Android IDs, URIs, notifications or feature routing into core reports.
During co-location use an explicit library dependency; after extraction pin a
version/revision by a packaging mechanism chosen at A01, without source copying
or making the core depend on Alfred. App-native FFI belongs outside the safe core.

## Shared Android ownership

| Seam | Shared Alfred responsibility | Feature responsibility |
| --- | --- | --- |
| Workspace/selection | Stable selection identity, one/many tracks or folder, bounded folder enumeration, URI grants, source acquisition/staging and release | Declare supported operation and selection shape; reject unsupported actual content explicitly |
| Operation discovery | Route independently to applicable installed workflows; show unavailable reasons; selection is not a forensic report | Supply capability checks; extensions alone do not establish codec support |
| Jobs | Identity, scheduling/admission, progress transport, cancellation, lifecycle and structured host failures | Perform feature work, declare resource needs and map feature progress/results; honor core worker limits |
| Results/artifacts | Bounded local retention, version dispatch, deletion, explicit export/share and artifact ownership | Own payload/method versions, units, scopes, caveats and rendering semantics |
| Native integration | Packaging and host boundary infrastructure | Audio Forensics adapter consumes the existing Rust APIs and preserves exact integers/unknowns |

These are responsibility-level interfaces. A01 freezes only concrete contracts
needed by the initial integrations. Do not invent universal feature result fields,
write transactions, conversion profiles, archive verification policies or APIs
for tools whose requirements are not frozen. Shared scheduling initially remains
bounded/conservative; this correction does not authorize concurrent DSP jobs or
replace `AnalysisJob`. Android lifecycle state must not be `AnalysisReport` state.

## Feature scope

| Alfred workflow | Current plan | Boundary |
| --- | --- | --- |
| Audio Forensics | A03 adapter; A06 detailed integration | Consume P07 result layers, P05 assessment and metadata; no Kotlin scoring or core rewrite |
| Spectrogram | First-class Alfred feature; A06a is its initial P06-backed slice | Broader viewing/exploration product, not limited to PNG generation; reuse P06 unchanged now, A01 defines long-term ownership |
| Audio Compare | First-class Alfred feature; A06b is its initial P07-backed slice | Reuse existing scoped comparison now; A01 defines reusable-code ownership, broader comparison requirements remain unfrozen |
| Metadata / Tag Studio | FUTURE placeholder, no implementation task | Deep editing, raw/unknown fields and custom tags require a later requirements packet; current P02/P07 metadata display remains read-only forensic coverage |
| Audio Converter | FUTURE placeholder, no implementation task | PCM/DSD export conversion needs later requirements; F02's DSD-to-PCM analysis path is not a converter feature |
| Archival Tools | FUTURE placeholder, no implementation task | CUE splitting, rip-log/integrity workflows need later requirements; existing integrity observations remain core behavior |

Spectrogram and Audio Compare may reuse inputs, jobs and Rust computation with
Audio Forensics, but each is reachable from the workspace without first opening
a forensic report. Forensic results may link to them. Existing computational
coupling is allowed; application navigation and ownership must remain separate.
Initial Alfred gates retain the already planned Spectrogram/Compare workflows.
The broader feature list does not add new launch requirements.

## Long-term Spectrogram and reusable-code ownership

The owner's ambition is a competitive, full spectrogram viewing and exploration
tool that can rival existing spectrogram applications. P06's bounded data/canvas
and A06a's initial workflow are a foundation, not the complete feature definition
or evidence of competitive parity. Spectrogram is conceptually independent of
Audio Forensics even while its first backend uses the existing analysis passes.
Audio Compare follows the same product-versus-code-residence distinction.

Reuse P06 and existing comparison functionality now. Do not duplicate them,
extract new crates or refactor the forensic core for this planning correction.
A01 must produce an ownership/consumer matrix covering computation, rendering,
result contracts and tests for both reusable areas. Decide what remains in the
independent library, what belongs in Alfred feature modules, and whether any
future host-neutral component boundary is justified. Record the extraction path,
version/dependency direction and compatibility obligations for standalone CLI and
other hosts. This is an A01 decision, not a preselected new package architecture;
no dependency from the core to Alfred is acceptable.

A later named Spectrogram requirements packet must turn the competitive ambition
into prioritized user workflows and measurable acceptance before implementation.
Viewing/navigation, spectral controls, channel presentation, inspection/readouts,
large-file responsiveness and export are candidate requirement areas to evaluate,
not a frozen feature list or detailed API. A focused comparison of relevant apps
belongs in that future packet; no survey or parity claim has been made here.
The full interactive roadmap, target competitors and first expansion milestone
are unfrozen. Completing A06a does not mean completing the full Spectrogram product.
Broader comparison semantics likewise require later requirements rather than
extending P07's forensic reference ranking by assumption. Neither expansion becomes
an initial-launch gate through this clarification.

No consequential ambiguity blocks the current delivery sequence. Exact bridge,
folder-provider handling, quotas, application layout and dependency packaging
are A01 engineering decisions. General Audio Compare semantics beyond P07 and
the three future tools' detailed requirements remain explicitly unfrozen; a
later named packet must settle them before implementation.
