# Version 0.2 detector validation — 2026-09-30

This milestone adds segment-wall observations, overlapping wall-profile
candidates and MQA signalling candidates. All ancestry verdicts remain
`INCONCLUSIVE`; evidence indices remain null. Tests establish numerical and
implementation behavior, not source-provenance accuracy.

## Automated checks

- 29 Rust tests passed: 9 detector unit tests, 13 core/CLI integration tests,
  5 detector integration tests and 2 original parity/integrity tests.
- Formatting checks and Clippy passed with warnings denied. The optimized Windows
  executable built.
- Android ARM64 library checking passed with default CLI features disabled.
  Native linking and execution on a phone remain untested.

New checks cover all three MQA candidate planes at both supported integer
depths; signed 24-bit samples through the decoder; marker/payload boundaries;
truncated payload fields; the three-second cap; and bounded retention of 64
candidates while counting additional matches. No synthetic marker is treated as
format confirmation. Real independently characterized MQA fixtures are still needed.

Segment tests cover bounded, non-overlapping sample intervals across sample
rates and durations, overlapping profile matches, silence, isolated tones,
short input, high-frequency walls, mixed probes, independent stereo channels
and prefix limits. A filtered-noise control produces a wall without any lossy
encoding; this is an expected observation, not an authenticity accusation.

## Numerical reference

Four two-second mono 24-bit fixtures contain full-band noise, a 16.75 kHz wall,
a 20.2 kHz wall, and a 19.55 kHz wall at 48 kHz. Other fixtures use 44.1 kHz.
All are generated signals, not recordings. `scripts/generate_detector_reference.py`
requires the unchanged Python reference commit
`c6ecce2296256b516709d87088896d1be913908c` and records NumPy's version.

Cutoff, cliff, high-band RMS relative level and sample peak come from the pinned
Python `_segment_voting` method's first probe. Above-cutoff relative level and
occupied-band fraction are separately computed in NumPy for the new gates.
Rust comparisons pass at 0.001 Hz for cutoff, 0.0001 dB for spectral levels,
and 1e-10 for peak dBFS and occupied-band fraction. These tolerances measure
arithmetic agreement, not detector confidence. Missing frequency bands and
unavailable cliffs use null rather than Python's placeholder zero.

## Deliberate policy differences from Python

- Sample positions are deterministic and evenly spaced across the actual decoded
  interval, including the start and last complete two-second clip. At most 36
  non-overlapping clips per channel are retained. Up to nine clips are used on
  shorter tracks when duration permits; the count then grows with duration.
- A probe needs peak at least -50 dBFS, cutoff at least 8 kHz and at least 30%
  occupancy above -45 dB relative to its spectral peak between 1 kHz and
  cutoff minus 1 kHz. These are provisional applicability gates.
- A wall additionally needs cutoff below min(22.5 kHz, 98% of Nyquist), cliff
  greater than 30 dB and RMS magnitude below -75 dB relative to the spectral
  peak in the band from cutoff plus 800 Hz to Nyquist minus 100 Hz.
- The original fixed 18.5 kHz high band is measured separately. It includes
  pass-band content for high-frequency walls and cannot serve as their void test.
- Fewer than three eligible probes means inconclusive aggregate evidence.
  Sparse sampling does not estimate duration ratios or precise splice boundaries.
- All matching legacy codec profiles are retained at 44.1/48 kHz. Other rates
  report that lookup as unsupported. Wall observations and profile lookup share
  the same evidence family; neither is a unique codec/bitrate identification.
- MQA observations report complete/partial reverse-engineered fields, skip
  metadata claims and omit the Python score override. The rate field does not
  establish an original sample rate. Packet-consistency confirmation is pending.

These gates were established against engineering controls. They have not been
tuned or calibrated against the supplied music collection.

## Generated encoder experiment

`python scripts/check_detectors.py` generates 18-second mono controls, encodes
noise with FFmpeg, decodes it to 24-bit PCM WAV, then compares Rust PCM hashes
with an independent FFmpeg decode. All eight PCM comparisons passed.

| Known processing history | Wall / eligible probes | Observation |
| --- | ---: | --- |
| Full-band noise, no encoder | 0 / 9 | No wall |
| 16.75 kHz low-pass, no encoder | 9 / 9 | MP3 profile overlap despite no lossy encoding |
| LAME MP3, 128 kbps mono | 9 / 9 | Overlapping MP3 and Opus wall profiles |
| LAME MP3, 320 kbps mono | 9 / 9 | Overlapping MP3 and Opus wall profiles |
| FFmpeg AAC, 128 kbps mono | 0 / 9 | No qualifying wall; not detected by this feature |
| Opus, 128 kbps mono | 0 / 9 | Does not pass wall gates; not detected by this feature |
| 12 kHz sine | 0 / 0 | Inconclusive; ineligible narrow-band probes |
| Digital silence | 0 / 0 | Inconclusive; inactive probes |

The MP3 cases have similar observed cutoffs despite different requested bitrates.
The AAC case retains energy to Nyquist; the Opus case has an observed cutoff near
21.03 kHz but fails the wall gates. Absence of this feature does not clear either
known transcode. More codec-specific evidence and real corpus evaluation remain
necessary. Repeated copies of one synthetic source are not independent accuracy
trials. All ancestry results remain inconclusive, including the low-pass control.

Exact encoder commands/version and histories are in
`corpus/local/results/synthetic-detectors-v2/manifest.json`; per-file reports,
PCM comparisons and summarized observations are in that same ignored directory.

## Expanded local collection

The folder contained 34 FLACs when this run started, including nine additions to
the original 25-file collection. All 25 original file hashes are unchanged.
The recordings remain unverified-provenance diagnostic material.

- 31 files completed full analysis with exact decoded-PCM agreement with FFmpeg
  and matching header lengths. Embedded FLAC checksum verification succeeded for
  30; it was unavailable for `7 - Bend the Clock - Dream Theater.flac` and remains
  null in that report. Its decoded PCM still matches FFmpeg exactly.
- The same three files documented in `VALIDATION.md` failed full-stream integrity:
  `01 - Hands Up.flac`, `11 - The APL Song.flac`, and
  `13 - Where Is The Love.flac`. FFmpeg also rejected them. The batch validator's
  exit code 1 reflects these input errors, not failed Rust code tests.
- Successful files span 16/24-bit PCM, 44.1/48/88.2/96/192 kHz and approximately
  96–713 seconds. Local process time was 1.86–16.06 seconds per file; peak working
  set was 19.23–43.08 MiB. These are Windows host observations, not mobile benchmarks.
- Across 62 channels, 25 had too few eligible probes and remained inconclusive;
  35 had enough eligible probes but no qualifying wall; two had mixed probes.
  These counts are neither false-positive rates nor evidence of authenticity.
- `08 - The Manifesto (feat. Trueno and Proof).flac` had five wall probes out of
  17 eligible probes in each channel, with cutoffs around 21.1 kHz and no matching
  legacy codec profile. It remains an unexplained spectral observation; this
  does not establish a splice, codec history, or source authenticity.
- No MQA sync matches were observed in the scanned first three seconds of the
  successfully analyzed files. This does not exclude signalling elsewhere.

Raw reports and independent decoder comparisons are in
`corpus/local/results/detectors-v2/summary.json` and its neighboring numbered JSON
files. The supplied recordings were not modified.

## Duration and memory

The original generated stereo 16-bit/48 kHz duration controls were rerun with
version 0.2. All decoded-PCM hashes still match FFmpeg exactly.

| Duration | Peak working set | Analysis process wall time |
| --- | ---: | ---: |
| 10 seconds | 19.08 MiB | 0.23 seconds |
| 120 seconds | 19.21 MiB | 1.11 seconds |
| 600 seconds | 19.15 MiB | 7.22 seconds |

The two-second FFT and clip buffers increase the footprint relative to version
0.1, while duration scaling remains approximately flat in this experiment.
Windows peak working set was sampled every 10 ms. Timings vary with caching and
system activity; this is not a controlled speed comparison against version 0.1.
Allocations scale with rate/channel/block sizes; this does not impose a hard
process memory cap. Results: `corpus/local/results/resources-v2/summary.json`.

## Remaining gates

Add the remaining resampling, source-profile and codec-transform evidence, then
evaluate aggregation on independent source groups. The encoder misses, misleading
low-pass profile match and music-probe abstentions above must remain visible when
evaluating any future policy. MQA confirmation requires independently characterized
fixtures and defensible structural criteria. Android runtime and UI work follow
core validation; no application or native binding was added in this milestone.
