# Alfred extraction — 2026-10-09

## Alfred repository extraction — 2026-10-09

Owner requested separation now. Alfred development moved to
`C:\Users\Bishal\code\alfred`, remote https://github.com/spideyonmoon/alfred,
branch `codex/extract-android`. App/shared/feature/native-adapter source and Android
workflows belong there. This repository retains the independently consumable
Rust engine, schemas, generated DSP fixtures and core tests. Alfred pins engine
5c5ce00d44f6759dd6a7319804b5f7a21079d1b4 through a Git dependency and its own lockfile.
No core refactor or private-audio transfer. Historical Android records stay here;
subsequent Android work starts with the Alfred HANDOFF.md.

The owner rejected the scaffold's visual quality and is drafting the replacement
UI on a scratchpad. No redesign is included in extraction; A06 functional checks
are not visual approval. A07 physical acceptance/A08 release remain pending in
Alfred. Extraction CI 37888964074 is running at Alfred commit 0388042; no extraction
acceptance claim yet. Static lock/source/syntax/link controls passed. Windows Rust
checks failed in host dependency linking (ld exit 204 including documented GCC
retry); Linux CI carries those checks. Local Kotlin/lint is running.

Tracked old app source/workflows are removed only as a repository move. Ignored
old build/cache directories are retained under apps/alfred; no recursive cleanup
or private recording deletion. Existing owner cleanup edits remain unstaged.
