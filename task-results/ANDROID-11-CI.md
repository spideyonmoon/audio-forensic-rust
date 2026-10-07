# Android 11 support and CI — 2026-10-07

Owner expanded Alfred to Android 11–16 and requested heavy checks in GitHub
Actions to avoid laptop builds. Existing A01/A02 work is preserved. No core/DSP
source, fixtures, private audio, memory policy, chunking or throttling changes.

All five Gradle module minimums and the Windows native link target move to API
30; compile/target remains 36. Separate Linux ARM64/x86_64 builds use pinned
NDK/Rust, locked dependencies, checksum verification, lint and all-native
ELF/ZIP/signature checks. The workflow's API 30–36 matrix checks workspace and
JNI bootstrap; phone acceptance and later workflow functionality remain pending.
The contract now specifies API guards for foreground-service types and Android
13+ notification permission, and expanded A07 OS/device coverage.

Local checks: Python script syntax passed; `git diff --check` passed. Initial
YAML check could not run because PyYAML is absent; do not count it as passed.
Initial Bash syntax check hit sandbox process denial; authorized retry recorded
below. No local Gradle/Cargo rebuild, core regression or phone test was run.

Remote build/emulator results and publication status will be recorded below.

Bash syntax passed on authorized retry. Isolated source-only staging and staged
whitespace passed; no ignored build/audio/evidence paths were staged. Commit is
on local `codex/android11-ci`; current checkout and normal staging area preserved.
GitHub rejected push because the OAuth token lacks `workflow` scope. No branch
was published and no CI job was started. `gh auth refresh -h github.com -s workflow`
is awaiting owner browser device authorization. After authorization, push this
branch and inspect the Alfred Android compatibility run, repairing any failures.
No Android 11 build/runtime success is claimed yet.

Owner completed workflow authorization; branch publication succeeded. First
Actions run 37631085115 failed before compilation: setup-android requested the
retired SDK `tools` package. Explicit `platform-tools` configuration repairs
that setup; retry pending. The redundant unchanged-core run was canceled;
core CI now ignores app-only/documentation changes. No private audio uploaded.

Retry 37632592927 linked ARM64 API-30 native code, then stopped before APK
assembly on three missing Gradle metadata pins (Guava parent 33.3.1-jre POM,
JUnit BOM 5.10.2 module, coroutines BOM 1.8.0 POM). Downloaded those exact
metadata files from Maven Central and matched its published SHA1 before adding
their SHA256 entries individually. Existing checksums/dependency versions were
not replaced and strict verification stays enabled. Retry pending. Cross-platform
APK verifier regression passed on the existing API-34 A02 APK; this does not
validate the new API-30 APK. Unchanged-core CI run 37632592799 passed.

Run 37633427012 passed the three metadata pins and reached resource compilation,
then rejected missing Linux aapt2 8.11.1-12782657 checksum. Reviewed the exact
Google Maven artifact against its published SHA1 and added its SHA256 alongside
the existing Windows pin. No verification disabled. Retry pending.
