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
