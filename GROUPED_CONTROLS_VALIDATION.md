# Characterized generated controls — 2026-10-04

This batch implements the control-assembly step after the v0.22 evaluation
runner. `scripts/build_grouped_controls.py` creates procedural sources, records
known transformations, characterizes decoded PCM independently, freezes the
manifest/executables and evaluates selected non-locked splits. No production
Rust code, detector thresholds, schema, policy or dependencies are changed.

## Scope and frozen design

These are six **procedural source families**, not six independent commercial
recordings. Their histories are known because they begin as generated integer
PCM, with no imported audio. Each family uses a fixed seed and different signal
construction. All descendants and both codec-specific labels remain in the same
group and split. Related encodings are never counted as independent sources.
No separate seeds of the same generator family are split across train/test.

| Group | Split | Rate | Generated content |
| --- | --- | --- | --- |
| broadband | development | 44.1 kHz | Shaped noise with changing envelope |
| harmonic | development | 48 kHz | Harmonic stack, envelope and low-level noise |
| attacks | validation | 44.1 kHz | Noise attacks and decays over quiet background |
| chirps | validation | 48 kHz | Swept tones, modulation and broadband noise |
| multiband | locked_test | 44.1 kHz | Modulated disjoint noise bands |
| plucked | locked_test | 48 kHz | Decaying harmonic notes over a quiet background |

Every source is four seconds of native stereo signed 24-bit PCM. A common
scaling factor preserves channel differences and gives a 0.72 full-scale sample
peak. These sources do not represent the diversity, duration, encoder population
or production histories of real music. The distinct held-out families also
introduce content distribution differences, so comparison between splits is
descriptive rather than a statistically representative accuracy estimate.

Each group has eight variants:

| Processing | Target AAC stage | Target Vorbis stage |
| --- | --- | --- |
| native_s24 | absent | absent |
| lowpass_s24, FFmpeg 14 kHz low-pass | absent | absent |
| aac256_s24, FFmpeg AAC 256 kbps, TNS enabled | present | absent |
| aac256_trim137_s24, exact 137-frame trim of decoded AAC | present | absent |
| aac256_s16, 16-bit export of the same AAC intermediate | present | absent |
| vorbisq5_s24, libvorbis q5 | absent | present |
| vorbisq5_s16, 16-bit export of the same Vorbis intermediate | absent | present |
| mp3192_s24, libmp3lame 192 kbps | absent | absent |

`absent` refers only to the selected codec stage, not to all lossy processing.
The MP3 controls are negative for these two target methods, with a known MP3
stage. Labels derive from the retained generation/encoding chain, never from
detector output. Every case has `generated_control` provenance. There are 48
audio files, 96 target-specific cases, and 16 files / 32 cases in each split.

## Characterization and reproducibility

Preparation uses NumPy and FFmpeg as optional local research tools. The Rust
application remains offline and requires neither. The batch retains:

- A generator source snapshot, Python/NumPy versions, seeds and frozen design.
- Native sources, all 18 lossy intermediates, all 48 analysis WAV files, and
  successful encoder/export commands with stderr and exit codes.
- FFmpeg/FFprobe executable hashes and FFmpeg version; actual decoded sample
  rate, channel count, precision and frame counts for every analysis input.
- Independent FFmpeg s32le PCM SHA-256 for all 48 inputs. All six native exports
  must match the directly constructed MSB-aligned integer PCM hashes. All six
  trimmed AAC exports must equal their decoded 24-bit parents from frame 137
  onward, byte for byte. Codec padding and output lengths are recorded as measured.
- One recipe receipt per source group, referenced and hashed in all its cases;
  the complete case manifest, frozen evaluator plan, and copied analyzer/runner.
- A file inventory covering the above artifacts, verified before and after
  every evaluation. The runner additionally checks its own binary fingerprint.

The Rust evaluation plan is frozen **after independent characterization and
before any Rust detector report**. Preparation does no detector analysis.
The optional evaluation stage reads only the selected split. The helper accepts
development and validation; it intentionally offers no locked-test evaluation
command. Locked audio may be independently decoded for characterization without
exposing detector outcomes. No external timestamp/signature or external backup
is claimed for this local freeze.

Each analyzed file must match its independent PCM/frame count, and its report
must pass the unchanged JSON Schema plus interval/channel checks. The same
report is associated with the two codec cases, without a second audio analysis.
All methods keep the frozen `codec-pattern-presence-v1` behavior. No hit/miss
expectation is used as a test-pass criterion; misses and false alarms are results.
An input, integrity or process failure is preserved and stops this engineering
run for investigation; it is not silently discarded from a completed evaluation.

## Commands and local artifacts

The current bundle is ignored/local at
`corpus/local/results/grouped-controls-v1-20261004/bundle/`. It is not in Git
history or an external backup. Original user recordings are not part of it.
Preparation and evaluation outputs are non-overwriting; an interrupted run must
be retained and explicitly handled before retrying into a new location.

```text
python scripts/build_grouped_controls.py prepare --binary target/release/audio-forensic.exe --runner target/msrv-desktop-v18/debug/examples/evaluate_reports.exe --output corpus/local/results/NEW-BATCH/bundle
python scripts/build_grouped_controls.py evaluate --bundle corpus/local/results/NEW-BATCH/bundle --split development --schema-python .tools/report-schema-venv/Scripts/python.exe
python scripts/build_grouped_controls.py evaluate --bundle corpus/local/results/NEW-BATCH/bundle --split validation --schema-python .tools/report-schema-venv/Scripts/python.exe
```

The frozen bundle includes its own generator snapshot for reproducing the exact
procedure. Paths in command receipts describe the actual host used; copying the
bundle elsewhere retains evidence but does not recreate the installed encoder
or Python environment. Preserve those separately if bit-identical regeneration
is required. Library/runner label semantics are in `EVALUATION_VALIDATION.md`.

## Actual results

Preparation passed all 48 independent PCM decodes, six direct generated-PCM
checks and six exact trims. The generator's four unit controls passed: declared
split/label mapping, signed 24-bit packing against integer definitions, repeatable
distinct source families, and mutation/non-overwrite rejection. A fresh stable
release v0.22.0 CLI build passed after an initial sandbox linker denial and an
authorized retry. Separate FFprobe checks confirmed all 18 encoded intermediate
codec identities, stereo geometry and expected sample rates. These results are
retained in the batch's `encoded-characterization.json`.

Both development and validation completed: **32 distinct audio analyses**, each
matching independent PCM and exact frame count, producing **64 codec cases**.
All 64 associated report objects passed schema/policy/interval/channel checks;
each pair of codec cases refers to one audio analysis. There were no file
failures or detector-policy abstentions in this cohort. Frozen bundle inventories
passed before and after both runs. All 16 locked audio files / 32 codec cases
remain without Rust reports or detector outcomes.

Positive-stage hit counts (each cell has two distinct procedural groups, one
variant per group; these are not independent real-recording sample sizes):

| Target / positive processing | Development hits | Validation hits |
| --- | --- | --- |
| AAC / aac256_s24 | 1 / 2 | 1 / 2 |
| AAC / aac256_s16 | 1 / 2 | 1 / 2 |
| AAC / aac256_trim137_s24 | 0 / 2 | 0 / 2 |
| Vorbis / vorbisq5_s24 | 2 / 2 | 2 / 2 |
| Vorbis / vorbisq5_s16 | 2 / 2 | 1 / 2 |

AAC's untrimmed hits are broadband and chirps; harmonic and attacks miss at
both final precisions. Every tested AAC trim misses, including the two parents
that hit before trimming. Vorbis hits every 24-bit positive; the 16-bit attacks
export misses while its 24-bit counterpart hits. This establishes a paired
processing sensitivity in this control, not its internal algorithmic cause.
No threshold, denominator, recipe, label or source was changed after results.

Target-negative false-alarm counts are **0 / 10 for AAC** and **0 / 12 for
Vorbis** in each split, spread over the two groups. The five AAC-negative and
six Vorbis-negative processing strata each have zero hits in both groups.
There are no uncertainty intervals or pooled accuracy claims: two procedural
families per split cannot establish a population false-positive bound.

Full per-file outcomes, original measurement reports, timings, normalized report
fingerprints and group-weighted per-processing summaries are in
`bundle/analysis/development/` and `bundle/analysis/validation/`. `receipt.json`
confirms exact PCM and artifact preservation; `schema-receipt.json` records the
shape checks; `evaluation.json` holds the frozen evaluator's results.

Final preservation matched **217 prior unrelated files**, including all Rust
source/tests and user recordings, plus the unchanged Git index. The Python
reference remains clean at the pinned baseline. Python syntax and whitespace
checks passed. No Rust tests, full regression, private recording analysis,
Android checks, locked evaluation, commit, push or upload were performed in
this control-assembly continuation. No jobs remain running. Evidence is local
and ignored; it is neither committed history nor an external backup.

## Next concrete work

Control assembly and the first frozen development/validation evaluation are
complete. The AAC trim miss is now reproducible on two new positive parents.
A focused next implementation candidate is the already-researched exhaustive
integer-phase AAC search, using development controls for any method work, with
explicit runtime/cancellation and false-hit checks. Preserve the current
threshold and frozen reports. Both current development and validation groups
are now exposed: if their results influence a new policy, do not call a rerun
an unseen validation. Keep the reserved families untouched until a candidate
and evaluation procedure are frozen. A later representative recording corpus,
additional encoders/bitrates, mono and longer/edited material remain separate
coverage needs; this batch does not close those gates.

No source-medium/original-depth classifier, general codec identifier, calibrated
probability or real-music accuracy estimate follows from this synthetic cohort.
The reserved groups must remain unexamined during any follow-up method work.
