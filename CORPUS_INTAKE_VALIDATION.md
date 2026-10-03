# Local corpus intake — 2026-10-02

`scripts/ingest_corpus.py` prepares private engineering receipts for explicitly
declared source groups. It is optional Python standard-library development
tooling for Python 3.11 or later (tested on 3.13); the Rust library/CLI remains
entirely offline and requires no Python.
Engine/schema/policy remain `0.18.0` / `0.18.0` / `observations-only-v18`.

## Contract

The input manifest supplies IDs, group assignments, splits, provenance notes and
history status. See `TEST_CORPUS.md` for its format. The tool does not infer
history from filenames, tags, container, detector hits or the user's word
"documented". That field describes the supplied documentation; it is not verified
ancestry ground truth. Accuracy evaluation and verified ancestry labels remain
false in every receipt.

Before invoking the CLI, intake validates unique IDs/paths, contained relative
paths, one split per declared group, parent membership and acyclic lineage.
Derivatives require a declared parent and a recipe with tool/version/arguments.
A transformation cannot upgrade claimed/unknown parent history to documented.
Claimed/unknown entries belong in `challenge`. Identical encoded files cannot
be assigned to separate groups, even when their filenames differ.

Every original is read in 1 MiB chunks to save an encoded SHA-256 and byte count.
UTF-8 provenance notes are hashed and copied into the manifest snapshot, with a
1 MiB per-note limit. The manifest and engine executable are fingerprinted too.
Inputs are rechecked after analysis, including sources whose decoder failed.
Changed inputs fail the receipt; no source or notes file is rewritten/repaired.
Stable local files are required; hashing does not provide an adversarial file
locking or filesystem snapshot guarantee.

The CLI runs one full-file analysis per non-locked entry, without prefix options.
Receipts retain raw reports/logs, actual statuses, stream geometry, coverage,
versions and decoded PCM hashes. Success requires complete two-pass coverage,
consistent exit/status and observations-only policy. Identical decoded PCM in
different measured groups fails intake, including different containers with
different encoded file hashes. Failure/unsupported results remain explicit and
the batch continues. A cooperative core deadline plus a subprocess timeout
prevents a permanently blocked analysis from hanging the intake script forever.

`locked_test` entries are hashed and **reserved without invoking the analyzer**.
Their detector reports and decoded PCM are not exposed by intake. Consequently,
decoded-duplicate checking excludes locked groups. Their full validation belongs
to a later frozen evaluation, not detector tuning. Encoded duplicates, declared
parent/group consistency and split consistency are checked for every entry.

Each run requires a new directory beneath ignored `corpus/local`. Existing
receipts are never overwritten. Exit 0 means the intake checks passed (reserved
entries have not been decoded); 1 means at least one analysis or post-analysis
check failed; 2 means setup/manifest/output validation failed. A run interrupted
before `summary.json` is written is incomplete; preserve its partial directory
and choose a new output for the next attempt.

## Limits

Declared groups still need human provenance review: matching hashes catch exact
copies, not every crop, alternate master, re-encoding or related session. A passed
receipt does not establish independent source counts or authenticity. It neither
generates/executes transform recipes nor verifies the truth of their descriptions.
The stored Rust PCM hash checks the engine's two passes; it does not substitute
for independent decoding. Use `scripts/validate_local.py` for the separate
FFmpeg PCM/integrity comparison, recording actual failures and coverage.

During the initial tooling validation, no real source corpus was ingested:
`corpus/local/sources` and `challenges` had no
recordings or provenance notes during this continuation. Existing recordings in
`reference/test_files` remain unverified diagnostic inputs and were not used.

## Actual checks

- **16 stdlib unit controls** passed: schema/JSON, IDs/paths/file duplicates,
  declared splits, uncertainty, parent recipes/groups/cycles, copied notes,
  full-file policy and exit checks, reserved locked groups, batch continuation,
  decoded duplicates, source/notes/manifest/binary changes, malformed output,
  timeout and non-overwrite behavior. The subprocess is mocked for these
  failure controls; they do not claim real codec/DSP verification.
- **Six end-to-end generated controls** passed with the Rust 1.85 source-only
  desktop CLI: positive intake, decoded-PCM leakage, explicit unsupported input
  followed by successful analysis, declared split leakage, non-overwrite and
  private-output enforcement. Expected negative exits were 1/2 as appropriate.
- Two different generated 8 kHz mono s16 WAV containers (one with a JUNK chunk)
  matched an independently computed MSB-aligned s32le PCM SHA-256 exactly:
  `5ccf1eea1b68c6775542bc36e60c04d3b6fe58a404f7d4e9653ea1eb56270529`.
  Both contained 1600 frames and reached EOF in two passes. The reserved generated
  entry produced no report. All generated audio/note file hashes were unchanged.
- Python syntax checks passed. No user audio, independently characterized corpus,
  real encoder recipe execution, source classification, locked evaluation or
  private upload was performed.

Commands:

```text
python -m unittest discover -s tests -p test_corpus_intake.py -v
python -m py_compile scripts/ingest_corpus.py scripts/check_corpus_intake.py tests/test_corpus_intake.py
python scripts/check_corpus_intake.py --binary target/msrv-source-only-v18/debug/audio-forensic.exe --output corpus/local/results/corpus-intake-v1-smoke-final
```

Choose a new `--output` for any subsequent smoke run. Generated-only receipts and
logs are local under `corpus/local/results/corpus-intake-v1-smoke-final/`; the
earlier smoke directory is preserved separately. Unit logs are
under `corpus/local/results/msrv-v18/`. Source/tests/scripts are local files,
excluded from source-only publication; no commit, push or external backup was
made. Rust 1.85 portability evidence is in `CORE_ACCEPTANCE_VALIDATION.md`.

## Private collection follow-up — 2026-10-02

The user supplied three requested additions, their EAC album logs and
`sources.txt` under `reference/test_files`. The collection now contains 44 audio
files (42 FLAC and two M4A). The user's broad statement that many are vinyl rips
does not identify individual file media. Voice recordings and detailed capture
hardware notes are optional and deferred; they do not block diagnostic intake.

The three additions are stereo integer 16-bit/44.1 kHz FLAC. Full-track intake
passed all three with two-pass EOF coverage. Independent FFmpeg 7.1.1 strict
decoding (`-xerror`) matched all three decoded s32le hashes exactly. Track lengths
are 565.426667, 1562 and 544.626667 seconds, respectively. Originals and supplied
notes/logs are preserved. Album logs are supplied context, not authenticated
ancestry evidence; post-rip processing and acquisition remain unestablished.
All three are declared `claimed` / `challenge` with provisional album groups.

A copied v0.18 release CLI is SHA-256
`6595c60dd4147d20d71ad0252df44d8688277dec47b490ed71867d8629a529e4`.
All receipts, source notes/log snapshots, file inventory and optional private
experiment scripts remain under ignored
`corpus/local/results/private-additions-20261002/`.

The remaining 41-file full intake finished with 34 analyzed, six unsupported and
one failed result, and exited 1 as required. Together with the three additions,
the original collection has **37/44 fully analyzed with exact independent PCM**.
Six inputs remain unsupported (four native-signature cases and two oversized
metadata cases), and one has a decoded/header frame-count mismatch. Strict
FFmpeg also failed on three originals: two of those signature cases and the
frame-count/decode failure. It decoded the two M4A and two oversized-metadata
inputs, which remain outside the Rust support path. The independent collection
comparison exited 1, with zero mismatches across the 34 analyzed remaining files.
All source hashes were unchanged. Per-file names, logs and reasons remain in
the private receipts; originals were not repaired or overwritten.

Fifteen comparison FLACs were generated: one
native 60-second trim per addition plus MP3 128/320 kbps, FFmpeg AAC 256 kbps with
TNS enabled, and libvorbis q5 encode/decode descendants. Actual commands, FFmpeg
version, intermediate lossy files and lineage are saved privately. Descendants
retain their parent's claimed history and challenge group. A recorded added
lossy stage permits scoped observations about that stage, but these originals
cannot establish a negative-control false-positive rate or calibrated accuracy.

All fifteen generated files passed full-file Rust analysis and exact FFmpeg PCM
comparison. Their 18-entry intake/independent receipts also include three original
parents, so they are not eighteen independent generated sources. Six alternative
24-bit FLAC exports from the same AAC/Vorbis intermediates then passed analysis,
exact PCM and declared-lineage checks. This second 12-entry receipt includes
three originals and three trim parents; there are **21 unique generated FLACs**
across the experiments. Actual 24-bit/44.1 kHz stereo geometry was probed rather
than inferred from encoder arguments. These exports preserve more of the lossy
decoder's output; they do not make the 16-bit ancestor a native high-resolution
source.

## Scoped detector coverage and investigation

The AAC 256 kbps descendants produced a lattice hit for two of three source
groups at both final bit depths. The third source's piano excerpt remained
undetected. The Vorbis q5 descendants produced no grid hits at final 16-bit depth;
at 24-bit depth, two source groups produced a hit in at least one native channel,
while the third still produced no hit. All ancestors/baselines retained unknown
upstream history. No negative-control false-positive rate or calibrated accuracy
was calculated, and no threshold was adjusted to force these cases to pass.

The Vorbis misses were investigated against the clean pinned Python baseline on
the same six decoded 16/24-bit files, evaluated separately on each native channel.
All **12 channel comparisons passed**: exact PCM agreement; exact active/support
counts; matching hit status; and score error **0**, within the documented 0.0001
tolerance. Thus these Vorbis outcomes reproduce the reference heuristic's limits
for this batch rather than expose a measured Rust/Python port disagreement.
The paired exports show sensitivity to post-codec quantization for two source
groups. The third source's miss persists at both depths; its cause remains open.
AAC's missed piano case still needs a matched-basis/anchor reference investigation.

That subsequent investigation is now complete in
[AAC_MUSIC_VALIDATION.md](AAC_MUSIC_VALIDATION.md): 44 basis checks passed across
music, setting descendants and generated controls. The piano miss reproduces in
Python and persists with TNS off and forced M/S settings. This closes the measured
port-parity question for these cases; the detector coverage limitation remains.

Detailed experiments, full commands, intermediate files and per-channel values
are local in `EXPERIMENT.md`, `codec-observations.json`, `s24-generation.json`,
`vorbis-python-oracle.json` and `vorbis-parity-checks.json` under the ignored batch.
Receipts are `intake`, `remaining-intake`, `variants-intake`, `s24-intake`, and
their `pcm-*` independent comparisons. Original notes/log hashes were rechecked
unchanged and the pinned reference remained clean. Every ancestry verdict remains
`INCONCLUSIVE`, with evidence index null. No Rust implementation changed, so the
Rust regression suite was not rerun for these corpus experiments. No Android
link/device check, policy tuning, locked evaluation, commit/push or upload ran.
All task jobs completed.
