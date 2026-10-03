# Audio collection shopping list

Collect original recordings and provenance notes. We can generate the codec,
container and processing variations reproducibly from these sources. Do not
spend time manually making dozens of differently encoded copies.

## First delivery

### Named first batch requested on 2026-10-02

The broad coverage table below is a longer-term target. For the next batch, use
this concrete list instead. No hardware purchase or vinyl collection is needed.

**User follow-up (2026-10-02):** the three requested tracks, three album rip logs
and `sources.txt` are now in `reference/test_files`. The user states many existing
files are vinyl rips, without mapping that claim to individual filenames. Proceed
with the supplied music. Voice/microphone recordings are optional and deferred
at the user's preference; no additional hardware details or provenance form is
required before engineering intake. Minimum useful source information, when
known, is CD/web/vinyl/unknown plus any known conversion. Edition and equipment
details are optional context for later medium/capture comparisons. Preserve all
unknowns rather than assigning authenticity labels from tags or logs.

The earlier request below is retained for reference; the follow-up above removes
extra documentation/capture prerequisites. Source notes were originally requested
for these five existing files; unknown history is acceptable for challenge use.
Do not acquire or copy them again:

- Porcupine Tree — Anesthetize (`03 - Porcupine Tree - Anesthetize.flac`);
  confirm the album and whether this is the studio or a live recording.
- Tool — Pneuma (`02. Pneuma.flac`).
- ABBA — Dancing Queen (`01 - Dancing Queen.flac`).
- Sleep Token — Take Me Back To Eden (`Take Me Back To Eden - Sleep Token.flac`).
- Papon — Dinae Dinae (`Papon - Dinae Dinae.flac`).

The three requested additions, now received, are full tracks rather than albums:

| Artist / track | Album / version | Preferred acquisition |
| --- | --- | --- |
| Miles Davis — So What | Kind of Blue, original album performance | Physical stereo CD rip; an original official WAV/FLAC download is also suitable |
| Keith Jarrett — Köln, January 24, 1975, Part I | The Köln Concert, original concert recording | Physical CD rip; an original official WAV/FLAC download is also suitable |
| Daft Punk — Giorgio by Moroder | Random Access Memories, standard 2013 album version with drums | Physical CD rip; an original official WAV/FLAC download is also suitable |

Track identities were checked against the [Miles Davis catalogue](https://www.milesdavis.com/albums/kind-of-blue/),
[ECM catalogue](https://ecmrecords.com/product/the-koln-concert-keith-jarrett/)
and [Daft Punk catalogue](https://www.daftpunk.com/randomaccessmemories/).
These are catalogue references, not verified free download links. Prefer editions
already owned or available without buying equipment; identify unavailable titles
so named replacements can be chosen. Any stereo CD edition is acceptable if its
edition is recorded. This batch is chosen to add exposed instruments, piano
decays, speech and changing arrangements to the existing collection; the titles
do not establish lossless ancestry.

Rip CDs to unprocessed stereo 16-bit/44.1 kHz WAV or FLAC. Keep an existing rip
log if available. For official web downloads, keep the original WAV/FLAC and its
native bit depth and sample rate, with the download URL and date. Do not resample,
normalize, remove noise or convert a streaming/MP3 download into a proposed clean
control. Unknown-history files remain useful challenge material.

Optional future capture controls, deferred at the user's preference, would use
two raw microphone recordings if existing equipment supports PCM recording:

- `voice.wav`: about 60 seconds, with five seconds of room sound at each end and
  normal reading/speech between them.
- `taps-keys.wav`: about 60 seconds of separated handclaps, table taps and keys
  jingling, leaving quiet gaps and avoiding clipping.

Record directly to mono PCM WAV, preferably 16-bit/48 kHz; native 44.1 kHz is
also acceptable. Record and export at the same rate, and note the microphone,
application and any automatic gain/noise processing. Keep the original export.
These related captures stay in the same session group and are reviewed before
assigning an ancestry control label. They are not independent commercial music
sources. A phone recording is suitable only if it genuinely supports direct PCM
WAV capture; renaming or converting AAC/M4A does not make that history lossless.

Vinyl is optional for a later medium/capture challenge. If the existing
`A1 Immigrant Song (1).flac` is an actual capture from the user's record, request
its Led Zeppelin III pressing and turntable/cartridge/phono-stage/ADC/application
notes. A filename containing a side/track label does not establish vinyl origin.
For a new raw capture, native stereo 24-bit/96 kHz WAV is useful if supported;
native 48 kHz is acceptable. Keep it before EQ, click removal or normalization.
Do not buy a record or turntable for this first batch.

A single `sources.txt` can cover the batch. For each file record: filename;
CD/web/vinyl/self-recorded/unknown; edition or URL; rip/recording application and
hardware where relevant; known processing/conversions, or "unknown". Put new
originals and notes under `corpus/local/sources`; source-unknown additions belong
under `corpus/local/challenges`. Tooling will create hashes/manifests and codec
variants. This batch enables initial corpus work; it does not complete accuracy
validation or Android work.

Start with 10-15 diverse source recordings; aim for roughly 30-50 independent
source recordings as an initial engineering corpus. These are practical starting
sizes, not sufficient evidence for a particular accuracy claim. Several hundred
variants of one source still represent just one source group.

Prefer full tracks with quiet passages, attacks and fades intact. Keep the files
as acquired, without normalization, resampling, tag cleanup or re-exporting.

| Source material | Useful initial coverage | Why it helps |
| --- | --- | --- |
| Piano, orchestral or acoustic music | 4-6 recordings; quiet and loud passages | Tonality, natural rolloff, decays and wide dynamics |
| Dense rock, metal or loud pop | 4-6; include limited/clipped masters | Tests whether ordinary mastering is incorrectly flagged |
| Electronic music | 4-6; synth tones, bass and bright percussion | Sparse harmonics, sharp attacks and unusual spectra |
| Jazz, voice and sparse instruments | 3-5 | Silence, exposed noise and relatively narrow spectra |
| Speech and recordings you made yourself | 3-5; mono and stereo if available | Known capture history and narrow-band content |
| Live/ambient recordings | 3-5 | Applause, room noise, reverb and changing background |
| Dark/older recordings, vinyl and cassette captures | 5-8 total if available | Difficult source profiles; provenance must be recorded |

Counts can overlap: a self-recorded acoustic track can cover several categories.
Collect from multiple sessions, albums and production chains, not mostly one
album. Prefer native captures or production exports with a documented chain for
the strongest controls. Record any lossy samples/plugins/imports you know about.

## Technical coverage across the collection

- Mostly 16-bit/44.1 kHz and 24-bit/44.1 or 48 kHz.
- A few independently sourced 24-bit/88.2 or 96 kHz recordings. Native 192 kHz
  material is optional; do not upsample files to fill this category.
- Mono, ordinary stereo, dual mono, a quiet/silent channel and unusual stereo
  phase behavior. Synthetic variants can fill missing channel edge cases.
- WAV/FLAC first; ALAC/M4A and AIFF are useful format checks. APE/WavPack, DSD,
  floating-point WAV and surround/multiple-stream files belong in the later
  compatibility set until their analysis paths are explicitly supported.
- Keep full tracks for coverage checks. Short clips, extreme durations, silence,
  malformed headers and truncated files will also be generated for robustness
  testing without damaging the originals.

## Additional challenge files

If already available, include a small separate collection of:

- Suspected fake lossless files, ideally paired with the original or a documented
  conversion chain.
- Known MQA examples with provenance, preserving all original bits and tags.
  Matched non-MQA/tag-stripped/metadata-only controls will be useful. Synthetic
  sync fixtures alone cannot validate real MQA identification.
- Difficult high-bitrate AAC, HE-AAC, Opus or Vorbis examples with encoder/history
  information, especially examples the Python implementation misses.
- Alternate releases of the same track, labelled as alternate masters unless
  there is evidence they share the same source. Different releases need not be
  identical recordings or controlled codec descendants.

An official download, CD rip, FLAC extension or 24-bit label does not by itself
prove an absence of earlier lossy processing. Unknown-history recordings remain
useful challenge cases but cannot be labelled genuine solely because an analyzer
passes them. Likewise, a low-pass wall alone is not ground truth for a fake.

## Variations we will generate

Use a representative development subset first, then extend the matrix. Exact
encoder implementations, versions, flags and availability will be recorded.
Unavailable codecs/settings must be marked skipped, never silently substituted.

| Family | Planned variations |
| --- | --- |
| Lossless controls | Same integer PCM in WAV, FLAC, ALAC and AIFF where supported; verify decoded sample identity |
| MP3 | LAME 64/96/128/192/256/320 kbps CBR and representative VBR settings |
| AAC-LC | 96/128/192/256/320 kbps; more than one encoder where available; default/TNS-enabled cases as well as the existing TNS-disabled control |
| HE-AAC | Representative low bitrates from an encoder supporting the actual profile; distinguish from AAC-LC |
| Opus | 48/64/96/128/192 kbps with explicit encoder settings |
| Vorbis | q2/q4/q6/q8/q10 and transient-heavy source material |
| Hidden lossy history | Encode each selected lossy variant, decode and store as 16/24-bit lossless PCM; retain direct lossy files as separate cases |
| Gain and editing | Gain changes, sample-offset trims, fades, EQ/lowpass, limiting and clipping |
| Bit depth | Integer zero padding, rounding, dither, noise shaping and float conversions with known processing steps |
| Sample rate | Documented downsample/upsample chains using different resamplers, including lossless resampling controls |
| Compound histories | Selected repeated/mixed codec chains; gain/dither/EQ/resampling after a lossy encode |
| Partial histories | Short lossy sections at the beginning, middle and end; different durations and positions between normal probe points |
| Robustness | Silence/near silence, short inputs, corrupt/truncated files, misleading metadata, multiple streams and channel layouts |

Apply relevant mastering effects to both clean sources and their lossy
descendants. This separates false positives caused by mastering from lost
detection sensitivity after processing. Processed lossless controls remain
negative for *lossy codec ancestry*, even when resampling/bit-depth findings are
expected. Unknown parent history remains unknown after a lossless conversion.

## Notes to include with each original

A simple text file is enough; we can turn it into a manifest during ingestion:

```text
File:
Source / download URL / recording session:
Album or session group:
History known, claimed or unknown:
Known capture/export/transcode steps:
Expected special features (if known):
Original file preserved: yes/no
Permission to redistribute: yes/no/unknown
Other notes:
```

We will add file hashes, measured format details, source/parent IDs, transform
recipes and dataset assignments. File names and tags are not labels. Private
recordings can stay local; public reproducibility can use synthetic or
redistributable fixtures.

Suggested local drop locations:

```text
corpus/local/sources/       originals with provenance notes
corpus/local/challenges/    suspected or uncertain-history examples
corpus/local/generated/     controlled variants created by tooling
corpus/local/results/       manifests, reports and measurements
```

The `corpus/local/` tree is ignored by the root Git configuration. These are
collection folders, not evaluation splits; do not manually distribute related
copies between them as if they were independent examples.

## Local intake manifest

Once originals and provenance notes are ready, create a JSON manifest locally.
Paths in each entry are relative to `root`, which is resolved from the manifest's
directory. Assign group/split IDs explicitly before running analysis:

```json
{
  "version": 1,
  "root": ".",
  "entries": [
    {
      "id": "capture-001",
      "group_id": "session-001",
      "split": "development",
      "file": "sources/capture-001.wav",
      "history": "documented",
      "notes": "sources/capture-001-provenance.txt"
    }
  ]
}
```

Save this example structure as `corpus/local/manifest.json`, replacing its paths,
IDs and declarations with actual collection information. IDs use letters,
numbers, underscores/hyphens (1–64 characters). Splits are `development`,
`validation`, `locked_test` or `challenge`. History is `documented`, `claimed`
or `unknown`; the last two belong in `challenge`. "Documented" means notes were
supplied for review, not that this tool verified a lossless history.

For a controlled descendant, keep the parent's group/split, add `parent_id`
pointing to its manifest ID and a nonempty `recipe` list. Each recipe step records
`tool`, `version` and the exact `arguments` array. Originals omit both fields.
Never upgrade an uncertain original's history after encoding it to lossless.

```text
python scripts/ingest_corpus.py corpus/local/manifest.json --output corpus/local/results/intake-001
```

This optional stdlib tool fingerprints originals/notes and the release binary,
runs full-file Rust measurements for non-locked entries, preserves explicit
failures, and checks declared/file/decoded-PCM duplicates across groups. Locked
entries are hashed and reserved without analysis. Every run requires a new output
directory; receipts remain ignored/local. See
[CORPUS_INTAKE_VALIDATION.md](CORPUS_INTAKE_VALIDATION.md) for coverage, checks and
limits. Independent FFmpeg decoding remains a separate validation step; intake
does not verify source labels, generate transformations or evaluate accuracy.

## Evaluation discipline

Assign development, validation and locked-test groups before tuning. Keep every
original, crop, alternate encoding and processed descendant in the same group;
group related sessions/albums together where feasible. Reserve genuinely unseen
sources. Grouping prevents related versions leaking between tuning and testing;
see [grouped evaluation](https://scikit-learn.org/stable/modules/cross_validation.html#cross-validation-iterators-for-grouped-data).

Tune on development/validation groups only. Freeze the implementation and policy
before scoring the locked test set. If test failures guide a later fix, that set
becomes regression/development evidence and a new untouched test set is needed
for the next independent accuracy claim. See [test-data leakage](https://scikit-learn.org/stable/common_pitfalls.html#data-leakage).

Report source counts, false positives, misses, abstentions and detection coverage
by codec and processing family. Report uncertainty with dependence between
derivatives accounted for. A result of no observed false positives in a small
collection is not a claim that false positives cannot occur.
