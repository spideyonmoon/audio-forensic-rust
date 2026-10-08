# Bishal's part of the roadmap

Updated 2026-10-06. Fill this file in directly, or tell the agent your answers
and ask it to update the file. Unknowns do not stop unrelated implementation.

## U01 — first app and test device

- Phone model: **Redmi 13 4G**.
- Android version: **Android 16**.
- Additional manual tester confirmed 2026-10-08: **Infinix Hot 11S / Android 11**;
  single/multiple/folder-subset picker checks passed. Owner approved A04 closure
  without waiting for replacement-APK physical ALAC retest; not run, collect later.
- Can install a test APK: **yes**.
- USB debugging/ADB available: **yes; available if needed, but not required for
  ordinary owner testing**.
- Initial formats: **FLAC, WAV and ALAC in M4A**.
- DSD remains wanted; owner deferred F02 **beyond first release and Alfred launch**
  on 2026-10-06 to conserve Astra budget. The frozen future scope is retained.
- Your three most important Python outputs/workflows:
  1. **The forensic verdict / final interpretation.**
  2. **Complete metadata exposure.**
  3. **Everything else the analyzer can extract, measure or infer.**
- Typical longest/largest file: **usually a single song; generally no more than
  roughly 600–700 MB even for unusually high-resolution material**.

Owner clarification 2026-10-06: Alfred opens to a file/folder workspace and offers
applicable operations for selected tracks. Audio Forensics is one feature;
Spectrogram and Audio Compare are first-class features. Spectrogram is intended
to rival existing viewing/exploration apps, not stop at image generation. Reuse
P06 and existing comparison functionality now; A01 owns their long-term code,
contract and test ownership at repository extraction. Detailed competitive and
interactive requirements remain to be defined later; initial integration does
not claim that full scope is complete. Tag Studio, Converter
and Archival Tools remain future placeholders. Finish standalone delivery through
P09 before Alfred A01; co-locate initially, then extract Alfred to its own repo
while preserving this independent library. Existing device/format/signing and
forensic information requirements below remain unchanged.

The forensic feature philosophy is to expose essentially every useful piece of information
that can be obtained from the audio file rather than reduce analysis to a simple
verdict screen. The app should preserve the depth of the Python analyzer and make
the underlying measurements, metadata, detector observations and interpretations
available to users who want them.

The first app must work entirely offline. It must not require a cloud account or
upload users' audio for analysis.

Initial compatibility includes FLAC, WAV and ALAC/M4A. DSD is deferred, but important
because source-medium analysis, including vinyl/cassette-rip detection, makes
high-resolution and DSD material relevant. Other formats can be evaluated through
P01 rather than silently becoming release blockers.

The owner currently runs Android 16, but compatibility should not assume Android 16
is the minimum supported version. Owner expanded the support contract to
Android 11–16 on 2026-10-07 for an Infinix Hot 11S (Helio G88). Use existing
shared memory admission; no phone-specific chunking/throttling. Heavy build and
emulator checks belong in GitHub Actions; physical acceptance stays separate.

## U02 — collect useful evidence without creating false labels

Continue keeping original files and existing notes. For each useful new source,
record when practical:

- filename;
- where/how it was acquired;
- known recording, mastering, capture, export or encoding steps;
- what is unknown;
- which other files came from the same recording/session/master;
- any existing rip, extraction or capture logs.

The owner has access to a very large pool of additional recordings, so obtaining
more audio files is not expected to be a major bottleneck. The important limitation
is trustworthy provenance and processing history, not raw file availability.

Put ordinary private additions under:

`corpus/local/sources`

or:

`corpus/local/challenges`

Leave the existing `reference/test_files` collection untouched unless a roadmap
task explicitly requires otherwise.

The agent handles manifests, hashes, independent PCM checks and reproducible
codec/processing variants. The owner does not need to manually create dozens of
conversions.

Commercial CD, web, vinyl, cassette or other stated origins do not automatically
establish the complete production history. Those files remain useful challenge
material without being falsely labeled as verified positives or negatives.

For depth research, retain the actual starting PCM precision and every known dither,
gain, conversion and export step when available. For medium/source research,
documented physical playback/capture chains are more valuable than filenames or
subjective sound.

When a specific missing control is required, the agent should ask for that control
and explain why it is needed rather than producing an open-ended shopping list.

## U03 — test the alpha on the phone

Status: **ready for A07 physical acceptance; generated Android 11–16 CI passed**.
The source-bound development ARM64 APK is locally available at
`target/a06ab/accepted-build/arm64-v8a/app-debug.apk`, built from `5c5ce00` in
[CI 37820435450](https://github.com/spideyonmoon/audio-forensic-rust/actions/runs/37820435450).
Phone installation/use and physical/resource acceptance of this APK are not yet
recorded; earlier Hot 11S picker evidence used a different APK.

After A06/A07 supplies a usable build:

- install the APK on the Redmi 13 4G;
- test import;
- analyze representative files;
- cancel and retry analysis;
- background and foreground the app;
- rotate the device;
- turn the screen off during work;
- reopen analysis history;
- compare files;
- export reports/results;
- test at least one representative large/high-resolution file.

Report:

- app/build version;
- phone model;
- Android version;
- exact action performed;
- expected behavior;
- actual behavior;
- report/error text if applicable.

Screenshots or screen recordings can be supplied when useful. Uploading private
audio is not required.

The agent must supply:

- APK path;
- generated test controls where appropriate;
- exact test procedure;
- expected behavior.

Physical-device validation must not be marked passed merely because an emulator,
desktop test or Android cross-compile succeeded.

ADB is available if a roadmap task genuinely benefits from it, but routine user
acceptance should not depend on ADB.

Final owner acceptance question:

**Is the app useful enough, detailed enough and reliable enough for the intended
first users?**

## U04 — identity, signing and distribution

- Working app name: **Alfred**.
- Final app name: **Alfred** unless changed later by the owner.
- Final package/application ID: **undecided**.
- First distribution: **GitHub Releases**.
- Build/CI infrastructure: **GitHub Actions may be used where appropriate**.
- Signing key custody and secure backup: **owner-controlled; not created/finalized
  yet**.
- Monetization/store listing: **not a core, alpha or initial GitHub-release
  prerequisite**.

A development package/application ID may be used until the public identity needs
to be frozen.

Do not paste signing passwords or private keys into chat, logs or source control.

At A08, the agent should prepare the release candidate, hashes, notices, build
artifacts and publishing instructions. The owner controls signing credentials and
authorizes final publication.

Creating or following this roadmap does not authorize an agent to publish a release,
push private recordings, create public uploads, or expose private research data
without explicit owner approval.

## U05 — source control and external backup

The project directory already has a **GitHub remote configured**.

Tracked source work should therefore use the existing repository rather than treating
the project as local-only.

Agents should inspect actual repository state with commands such as:

`git status`

`git remote -v`

`git log --oneline`

before making claims about whether the project has a remote, commits or backups.

Repository work should not remain indefinitely as a large dirty working tree.
Roadmap tasks should follow the project's explicit commit policy once that policy
is defined. Agents must not silently assume that committing, pushing or publishing
is authorized unless project instructions permit it.

The GitHub repository is not necessarily a complete research backup.

Private/local material such as:

- `reference/`
- `corpus/local/`
- ignored validation outputs
- private recordings
- local research evidence

may not be included in the Git remote.

Therefore:

- GitHub remote: **configured**.
- Source repository backup: **available through the configured GitHub remote,
  subject to actual pushed commit state**.
- Private research-data backup: **not yet confirmed**.
- External backup location/type for private material: **not recorded**.
- Last verified full backup date: **not recorded**.
- Restore check: **not recorded**.

Before destructive machine/repository changes, preserve the source, root documents,
tests/fixtures, scripts and `.git`, plus important private `reference/` and
`corpus/local/` evidence, to storage under the owner's control.

Ignored local files are not backed up merely because tracked source code has been
committed or pushed.

## U06 — future evaluation material

A fixed reserved blind-test collection does **not** need to be created now.

The owner has access to a very large supply of additional recordings and can obtain
fresh material when a frozen evaluation is ready. There is therefore no reason to
hold back a scarce permanent set merely for future testing.

For a defined held-out or final evaluation:

1. Freeze the detector/method/policy being evaluated.
2. Select fresh source groups that were not used to develop that frozen version.
3. Keep derivatives of the same recording/session/master together as one source
   group rather than pretending they are independent examples.
4. Record whatever provenance is actually known.
5. Run and save the evaluation before using its results to change the detector.
6. If an evaluated source later guides a fix or threshold change, treat it as
   exposed development data for subsequent evaluations.
7. Obtain another fresh source group when another genuinely unseen evaluation is
   required.

No permanent reserved set is required.

Exact future evaluation material: **select on demand when needed**.

The existing generated development/validation controls may continue to be used
according to their recorded exposure status. Agents must not invent claims of
unseen evaluation merely because a file has not been discussed recently.

## Session prompts

Delivery:

> Resume the delivery roadmap.

One packet:

> Complete P02 from ROADMAP_TASKS.md and update the roadmap/handoff.

Research:

> Complete E01 from ROADMAP_TASKS.md; preserve the app release track.

Evidence intake:

> I added these originals and notes. Complete the appropriate intake work without
> inventing unknown provenance.

Android:

> Resume the Android delivery track from the earliest eligible A-task. Preserve
> the frozen core/result contract and update the roadmap/handoff when finished.

Choose the task's recommended model in the app before starting. The repository's
roadmap, handoff and task documents should carry enough state that the project does
not depend on old chat history.
