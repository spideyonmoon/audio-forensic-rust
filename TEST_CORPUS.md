# Audio collection shopping list

Collect original recordings and provenance notes. We can generate the codec,
container and processing variations reproducibly from these sources. Do not
spend time manually making dozens of differently encoded copies.

## First delivery

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
