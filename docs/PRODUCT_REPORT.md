# Product reports and reference comparison

Engine 0.32.0 adds independent library/CLI workflows. The native `--json` array,
measurement schema 0.18.0, `observations-only-v18`, INCONCLUSIVE ancestry and null
index remain available. Product JSON is separately identified as
`audio-forensic-product-v1`; this replaces the proposed, unimplemented
`alfred-product-v1` name. It describes Audio Forensics results, not Alfred's
shared workspace/history/jobs. Comparison is `audio-forensic-comparison-v1`.

## Ordinary workflows

```text
audio-forensic track.flac
audio-forensic --info track.m4a
audio-forensic --info --json album-directory
audio-forensic --product-json --fast album-directory > album.json
audio-forensic --batch-summary track.flac missing.wav another.m4a
audio-forensic --product-json --progress --max-seconds 30 track.wav > prefix.json
audio-forensic --product-json --collect-spectrogram --title "Track title" track.flac > track.json
audio-forensic --saved track.json
audio-forensic --saved --product-json track.json > copied.json
audio-forensic --saved --spectrogram new-image.png --spectrogram-preset standard track.json
audio-forensic --compare original.flac gain-variant.wav
audio-forensic --saved --compare --comparison-json variant-a.json variant-b.json
```

Ordinary text leads with the qualified reference summary, scores and source/depth
candidates, followed by complete structured read-only metadata, measurements,
byproducts, tool statistics, rule/features/audit detail, spectrogram links and
diagnostics. Structured details retain escaping, units, nulls and intervals.
The field alias map covers all 155 pinned Python field names with locations,
availability, units/domains/scopes and qualification. Named tags resolve the
first retained entry; raw keys/duplicates/limits remain exposed. No terminal
color gauge, raw legacy certainty headline or fabricated probability is added.

`--info` calls only the bounded container/tag reader. It reads declarations and
can succeed even when the encoded audio is corrupt; it does not measure PCM,
verify checksums or issue a reference verdict. `--info --json` returns metadata
objects. Ordinary `--json` and `--summary` retain their established native APIs,
two-pass behavior and exit semantics. The full product API collects P03/P03a,
P04/P05 and optional P06 together on one open source under one worker/deadline;
its reference adapter verifies a third PCM pass. Progress counts passes, never
ETA; `--progress` stays on stderr. Metadata describes the selected stream and
all retained tags; it is not restricted to the analysis prefix.

Directory input is sorted by path, nonrecursive, with WAV/FLAC/M4A candidates.
The actual container/codec decides support, including ALAC versus AAC in M4A.
Direct unsupported inputs remain explicit. One failed input does not suppress
later analyzed inputs. Empty directories and enumeration errors have structured
failure rows. Product/info batches accept at most **32 inputs/results**; split
larger batches. Directory candidates are collected and sorted before that
admission check, so enumeration itself is not bounded to 32 entries or covered
by the analysis deadline. This desktop workflow is not Alfred's future bounded
folder enumeration. This finite result-count limit is separate from the core's
bounded DSP/profile buffers, and is not an Android memory/RSS guarantee.
The existing native JSON directory workflow remains unchanged.

Exit codes: 0 when all file analyses/export attempts succeed, 1 for a per-file or
artifact export failure, 2 for CLI/saved-version/binding/host errors, 130 for
requested cancellation. Partial reference assessment or an incompatible/
unavailable comparison remains a truthful successful workflow outcome, with
null scores/winner as applicable. `--compare` requires at least two results.

## Saved and artifact handling

`read_product_json` accepts one object or an array, checks versions and bindings,
and re-evaluates P05 only against the stored typed reference inputs. It does not
open the original audio or re-run measurements. Alias availability is rebuilt
from the stored fields for presentation, never used for scoring. It rejects
unknown product/measurement/assessment/adapter/statistics versions, mismatched
PCM/coverage/stream metadata, stale unsuccessful products, altered rule/score
records and invalid artifact binding. The development JSON Schema checker adds
strict nested shape/range checks; it is not a runtime requirement. Optional
`byproducts`/`tool_statistics` may be null; missing required keys fail explicitly.
Artifact `presentation`/`title` are optional for older saved shapes; absent
presentation gives unavailable bitrate through P06's existing renderer.
CLI saved documents are bounded to **64 MiB per file**, 32 expanded results.
Library callers must apply their own serialized-size and batch admission limits;
these CLI limits are not enforced by the in-memory saved/comparison functions.

Collect spectral data using `--collect-spectrogram`, or supply `--spectrogram
NEW_PATH` during live analysis. For multiple results, repeat `--spectrogram`
once per result in input order. All output paths must be new. Presets:
`standard` 1600×900, `publication` 2560×1440 default, `large` 3840×2160. The Rust
P06 renderer composes axes, native p95 overlays, legend and scoped encoded
bitrate; the CLI adds no spectral processing. Stored title/presentation survive
saving and re-rendering. Each product retains at most 32 export attempts.

An export failure returns no path, retains the successful measurements and adds
an artifact diagnostic. Saved successful paths are historical export receipts;
reading them does not verify current file existence. New exports use create-new
writes and cannot overwrite a user's neighboring image.

## Comparison contract

The caller asserts that selections are variants of **one track**. The library
does not identify music, align samples or compute a perceptual quality metric.
It ranks compatible available results using the frozen reference tuple:

1. Assessment availability (complete score/label).
2. Main uncalibrated score ascending.
3. Reference rate-history/bandwidth flag ascending.
4. Reference cutoff Hz descending.
5. Legacy first-channel depth category ascending: 0 consistent/fully exercised,
   1 abstain/analog-consistent, 2 flagged. These are audit categories, not truth.
6. Legacy first matching positive channel integer DR descending. Product overall
   DR is also retained separately; it is not silently substituted here.
7. Container precision bits × sample rate Hz descending.
8. Stable input order for exact ties.

Every tuple and missing field is exposed. Nullable measurements remain null;
sort-only legacy defaults are explicitly recorded in comparison caveats.
All-unavailable results have no winner. Available inputs with unequal versions,
reference basis domains or actual duration/EOF coverage return `incompatible`,
null ranks and no winner. Different native rates can be compared when their
reference methods/domains and coverage agree. Full versus prefix, mono versus
mid/side, and missing/unknown method bindings are not silently substituted.
The conservative coverage equality rule may require the caller to select an
explicit shared interval when variants have unequal durations; no alignment or
unlike-track ranking is implied. The headline is **reference-method ranking**,
never a claim of most authentic audio or best sound.

## Library entry points

`analyze_path_product` / `analyze_source_product` return `product::ProductReport`
with one measurement report, metadata, bound reference inputs/assessment,
byproducts, tool statistics, ReplayGain audit, optional spectral artifact/
presentation/title, export receipts, elapsed analysis time and diagnostics.
The source API retains app-owned I/O, cancellation and progress conventions.
`product::{read_product_json, render_product, render_batch, compare_products}`
operate on saved values. `ProductReport::export_png` calls P06 `write_png_new`.
The CLI-free core requires neither Python tools, fonts from the host, subprocesses,
network access nor Alfred app state. DSD remains unsupported until deferred F02.
