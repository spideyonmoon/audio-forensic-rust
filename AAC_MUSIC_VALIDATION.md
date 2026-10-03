# AAC music coverage audit — 2026-10-02

Engine/schema/policy remain `0.18.0` / `0.18.0` / `observations-only-v18`.
No detector implementation, threshold or ancestry label changed. This follow-up
investigates the known added-AAC piano conversion that produced no lattice hit
in the private collection experiment. Read `AAC_VALIDATION.md` for the original
numerical contract and `CORPUS_INTAKE_VALIDATION.md` for collection provenance.

## Reusable comparison tooling

`scripts/audit_aac_receipts.py` reads existing intake receipts without invoking
the Rust analyzer again. It requires the clean pinned Python baseline, NumPy,
SciPy and FFmpeg as optional development dependencies. They are not Rust runtime
dependencies. Output must be a new directory below ignored `corpus/local`.

For each selected analyzed integer-PCM file, the tool verifies the supplied
encoded fingerprint and independently checks the full decoded s32le hash with
strict FFmpeg decoding. It captures at most the reported 180-second AAC search
prefix, constructs the same exact f64 mono or stereo M/S basis, and uses the
pinned Python KBD/MDCT plus SciPy analytical gamma on the exact reported anchors.
The matched calculation explicitly applies Rust's minimum four usable probes
rule. It checks score, winning phase/scale, probe eligibility/flag counts and RMS.
The unaltered reference method also runs on its ordinary float32 input path;
that independent selector/basis result is recorded separately from matched
arithmetic. Sources/reports are fingerprinted again afterward.

Identical parent entries and byte-identical reports repeated across receipts are
counted once. Conflicting repetitions are rejected. The tool does not infer
ancestry, rewrite expected values, tune a policy or generate accuracy labels.

## Initial audit checks

- **44 basis comparisons passed**: 18 M/S comparisons across nine existing music
  baselines/AAC exports; 20 across ten new setting descendants; six mono
  numerical/abstention controls. These are related cases, not independent sources.
- Maximum matched-anchor score error: **0**, allowance `1e-12`. Winning phase,
  scale, eligible-probe count and per-probe selected band counts matched exactly.
- Maximum anchor RMS error: **2.609024107869118e-15**, allowance `1e-10`.
- The original reference's normal f32-path numerical scores matched Rust within
  **2.7755575615628914e-17** on these scored cases. Silence returned -1 in Python;
  the tone returned zero. Rust correctly abstained with null scores on both.
- Four reused generated fixtures matched their unchanged `aac_reference.json`
  PCM hashes, score tolerances and anchor positions: clean/AAC at 44.1/48 kHz.
- Three repeated parent receipts were accepted only after exact consistency
  checks. Two negative controls correctly rejected conflicting reports and an
  output directory outside `corpus/local`, before creating their output folders.
- All independently compared PCM hashes matched. Ten new setting descendants
  passed full EOF, two-pass intake, lineage and immutable-source checks; the
  14-entry receipt also includes four existing originals/baselines.
- Source/notes and pinned reference were preserved. Rust source/manifest and the
  staged-index hash were verified unchanged. Python syntax compilation passed.
  The full Rust suite was not rerun because no Rust code changed. Android
  linking/device checks, locked evaluation, calibration, upload, commit and push
  were not run. No task jobs remain running.

## Coverage finding

The original added-AAC piano excerpt remains below the inherited 0.10 threshold
in both implementations. Its mid/side scores are about 0.07015/0.04719 at both
16/24-bit final export depths. This is a shared detector coverage limit on a
known added codec stage, not a measured port disagreement. The other two music
groups retain their earlier AAC hits on the active winning basis.

Five additional AAC256 encoder configurations isolate TNS on/off and M/S
auto/off/on from the same piano clip, each exported at 16 and 24 bits. Including
the existing TNS-on/auto case gives six configurations. All remain below 0.10;
forcing M/S raises the maximum mid score to about 0.09184 but does not produce
a hit. Disabling TNS did not restore detection. A setting flag and changed PCM
do not by themselves establish which AAC tool was exercised in each frame.

A private limited header experiment inspected eight controlled AAC-LC streams
against the [FFmpeg n7.1.1 decoder field order](https://raw.githubusercontent.com/FFmpeg/FFmpeg/n7.1.1/libavcodec/aac/aacdec.c).
It parsed the first SCE/CPE window descriptor after optional fill elements in
all 20,680 packets, recording unsupported layouts as errors rather than guessing.
It is not a complete AAC/TNS parser or a new production decoder. In the original
piano stream, 2,566/2,585 descriptors use the ordinary long KBD window, with seven
short-window descriptors and six each of the start/stop sequences. Packet-time
context at the selected anchors is predominantly ordinary long KBD; short-block
dominance is not supported by this observation. This does not prove the cause
of the miss. Further controlled source/excerpt/stereo/noise-tool comparisons are
needed before proposing a changed detector or threshold.

At the winning phase, the piano's selected probes average roughly 27–29 eligible
bands out of the reference's fixed 49-band denominator. Those raw counts are
already exposed in reports. Eligibility and source dependence require evaluation;
changing the denominator simply to make this case hit would not be validation.

## Evidence and reproducibility

All detailed music values, commands, encoded intermediates, header observations
and receipts remain local in
`corpus/local/results/aac-music-audit-20261002/`. `audit-final/` is the authoritative
original-music comparison; `settings/intake` and `settings/audit` cover the new
descendants; `generated-controls/` covers the generated controls. Aggregate and
immutability results are `aggregate-checks.json` and `final-checks.json`.
Dependencies were Python 3.13, NumPy 2.3.5, SciPy 1.17.1 and FFmpeg 7.1.1.

An initial invocation rejected intentionally repeated parent IDs. An initial
completed harness run also passed contiguous f64 into a reference method whose
ordinary source input is f32: its in-place energy scratch aliased that temporary
array, invalidating its direct-reference scores. Those outputs were preserved
in `audit/`; `INITIAL_ATTEMPTS.md` explains their exclusion. The corrected run
keeps exact f64 for the matched calculation, uses f32 for the original method
and verifies its source array remains unchanged. No original file or historical
oracle was edited. A final immutability assertion initially included an expected
in-progress `HANDOFF.md` edit; the corrected check distinguishes that document
update from unchanged implementation/index files.

Commands from the project root:

```text
python scripts/audit_aac_receipts.py --intake corpus/local/results/private-additions-20261002/variants-intake --intake corpus/local/results/private-additions-20261002/s24-intake --id-pattern ".*-clip60(?:-aac-256(?:-s24)?)?" --output corpus/local/results/aac-music-audit-20261002/audit-final
python scripts/ingest_corpus.py corpus/local/results/aac-music-audit-20261002/settings/manifest.json --binary corpus/local/results/private-additions-20261002/audio-forensic.exe --output corpus/local/results/aac-music-audit-20261002/settings/intake --deadline-seconds 600
python scripts/audit_aac_receipts.py --intake corpus/local/results/aac-music-audit-20261002/settings/intake --id-pattern "jarrett-aac256-.*" --output corpus/local/results/aac-music-audit-20261002/settings/audit
python scripts/audit_aac_receipts.py --intake corpus/local/results/aac-music-audit-20261002/generated-controls/intake --id-pattern ".*" --output corpus/local/results/aac-music-audit-20261002/generated-controls/audit
python -m py_compile scripts/audit_aac_receipts.py
```

These directories already exist; choose new output names to rerun. The private
header script and generation commands are preserved with the experiment rather
than published with audio. All supplied parents retain claimed/unknown history;
the known added codec stages support scoped coverage observations, not a clean
negative corpus, false-positive rate or calibrated ancestry accuracy. Every
report stays `INCONCLUSIVE` with evidence index null.

## Fixed excerpts and intensity-stereo/PNS controls — 2026-10-02

The continuation generated **27 new FLAC controls** in a fresh private batch:
18 first-excerpt AAC descendants across the three supplied source groups, three
later native piano trims and six AAC descendants from those trims. Twelve AAC256
intermediates each supply paired 16/24-bit exports. TNS is enabled and M/S is
automatic; intensity-stereo/PNS switches are requested as off/on, on/off and
off/off. The existing on/on exports are independently rechecked. Later piano
excerpts use exact native frames at 180–240, 600–660 and 1200–1260 seconds, chosen
before running detectors. The design is saved in private `plan.json`.

All **33 lineage intake entries passed** full-file two-pass analysis, including
six unchanged original/first-trim parents. All **72 matched M/S basis comparisons
passed** across 36 clips: 54 comparisons on the 27 new files and 18 repeated
comparisons on nine existing baseline/default files. Together with the initial
audit, this adds 54 comparisons and brings the distinct matched-basis coverage
to 98; it does not add 54 independent recordings. Six repeated parent receipts
were accepted only after exact consistency checks.

- Maximum matched-anchor score error: **0**, allowance `1e-12`; phase, scale,
  eligible probes and selected eligible/flagged band counts match exactly.
- Maximum ordinary reference f32-path score error: **5.551115123125783e-17**.
- Maximum anchor RMS error: **2.609024107869118e-15**, allowance `1e-10`.
- All 36 clip PCM hashes matched independent strict FFmpeg decoding. The three
  full originals were independently decoded again and also matched exactly.
  All six native first/later trims matched their exact original parent frames.
- All 39 generation commands passed. Independent decoding of twelve AAC
  intermediates verified both export depths' lengths and native stereo geometry.
- **264 existing input/implementation files**, the staged index and pinned
  reference were verified unchanged. Python syntax and Git whitespace checks
  passed. No Rust code changed, so the Rust suite was not rerun. No upload,
  commit, push, calibration, locked evaluation or Android link/device check ran.

The following values are the maximum across M/S and both final export depths
for the first 60-second excerpt. Every Miles Davis and Daft Punk configuration
retains a provisional hit; every piano configuration stays below 0.10.

| Requested intensity stereo / PNS | Miles Davis | Piano | Daft Punk |
| --- | ---: | ---: | ---: |
| On / on, existing default | 0.156888 | 0.070153 | 0.145408 |
| Off / on | 0.156888 | 0.071429 | 0.141582 |
| On / off | 0.158163 | 0.070153 | 0.145408 |
| Off / off | 0.150510 | 0.070153 | 0.104592 |

All nine new ablation ADTS audio streams differ from their same-source default
stream, so these controls changed compressed audio rather than only MP4 metadata.
They do not parse per-frame tool use or isolate a single causal mechanism.
Disabling intensity stereo/PNS does not restore the piano observation here.

| Piano excerpt start | Native trim maximum | AAC16 maximum | AAC24 maximum |
| --- | ---: | ---: | ---: |
| 180 s | 0.012755 | 0.068878 | 0.068878 |
| 600 s | 0.014031 | 0.063776 | 0.066327 |
| 1200 s | 0.016582 | 0.057398 | 0.057398 |

The later piano misses reproduce in the pinned f32 method as well. Both bases
have sixteen eligible probes at their winning combination, so this is not a
missing-probe abstention. The result broadens the observed coverage limit beyond
the opening excerpt. Upstream history remains claimed; native trims are not
verified negative ancestry controls. Neither table establishes sensitivity,
specificity, probabilities or a reason to lower the threshold/denominator.

An initial final-check harness incorrectly required AAC descendants to contain
exactly 2,646,000 frames. It exited 1 on that assertion; intake and PCM comparison
had already passed. All 24 new AAC exports retain **16 additional tail frames**,
matching independently decoded intermediate lengths. Native trims remain exact.
The corrected check compares each export to its decoded AAC intermediate, and
records the padding explicitly. No audio, reports or oracle was regenerated;
`INITIAL_ATTEMPT.md` preserves the failed assumption.

Evidence is local in `corpus/local/results/aac-coverage-controls-20261002/`:
`commands.json`, `manifest.json`, `intake/`, `audit/`, `coverage-table.json`,
`pcm-and-excerpt-checks.json`, `decoded-length-checks.json`,
`compressed-stream-checks.json` and `final-checks.json`. Private generation and
verification scripts plus exact commands are saved in its `EXPERIMENT.md`.
The copied binary remains SHA-256
`6595c60dd4147d20d71ad0252df44d8688277dec47b490ed71867d8629a529e4`.
These receipts remain ignored local files, outside source history and external
backups.

## Integer-phase diagnostic — 2026-10-02

After closing the encoder/excerpt audit, a separate private diagnostic expanded
the phase grid to **all 1024 integer offsets**. It kept the same reported anchors,
exact f64 M/S bases, KBD alpha=4 window, eight scales, band gates, four-probe rule
and 49-band denominator. Transforms ran in 128-phase batches; it read seven
additional samples beyond each reported captured span, within the original
selector's conservative end bound. It changed no production report or detector.

All **20 basis diagnostics on ten clips passed**: the four piano AAC24 excerpts
plus six unchanged generated seed-84 controls (ordinary AAC320, its 137-sample
trim, clean pink noise, MP3, Opus and Vorbis descendants). The original eight-step
subset matched every saved Rust score, phase, scale and selected probe vote.
Every source/report fingerprint and independent full PCM hash remained exact;
the clean pinned reference was checked before and after.

- All eight piano basis maxima are **unchanged**; every fine-grid winner remains
  phase 512. Thus phase-grid spacing does not explain these particular misses on
  the existing selected anchors/window/scales. This does not diagnose every
  component of the detector.
- The known generated **137-sample trim miss is recovered** by the diagnostic:
  mid/side scores rise from 0.016582/0.016582 to **0.184949/0.197704**, with winning
  phase **375 = 512 − 137**. These are wider-search observations under the
  inherited threshold, not new production hits. The untrimmed AAC control's
  maxima remain 0.178571/0.202806 at phase 512.
- Clean pink noise and its MP3/Opus/Vorbis descendants remain below 0.10; the
  highest fine-grid maximum among these four related controls is **0.021684**.
  Several maxima rise slightly with the additional search opportunities. These
  cases do not calibrate the expanded search or its false-hit rate.

The existing production phase-grid limitation therefore remains documented,
while the piano coverage limit requires a different explanation. Deploying a
finer grid needs separate CPU/storage/cancellation measurements and broader
generated/independent false-hit controls. No threshold, denominator, phase grid,
engine version or policy changed. `fine-phase/plan.json`, `results.json` and
`summary.json`, plus private `phase_diagnostic.py`, retain the exact diagnostic
design and outputs. `post-diagnostic-checks.json` verifies the same 264 inputs,
staged index and pinned reference after this diagnostic. Python syntax and final
preservation checks passed. No task jobs remain running.
