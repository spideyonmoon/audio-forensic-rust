"""Refuse APK reuse unless all compilation/packaging inputs match Git exactly."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


def verify(directory, if_unchanged=False):
    revisions = set()
    for abi in ("arm64-v8a", "x86_64"):
        root = directory / abi
        receipt = json.loads((root / "build.json").read_text())
        revision = receipt["revision"]
        assert re.fullmatch(r"[0-9a-f]{40}", revision), "invalid build revision"
        revisions.add(revision)
        assert receipt["abi"] == abi and receipt["native_api"] == 30
        alignment = json.loads((root / "alignment.json").read_text())
        # The original verifier checked ELF/ZIP/signatures/assets. Bind reuse to
        # those exact APK bytes rather than trusting only a filename/run ID.
        with (root / "app-debug.apk").open("rb") as apk:
            actual = hashlib.file_digest(apk, "sha256").hexdigest()
        assert actual == alignment["apk_sha256"], "APK verification binding mismatch"
    assert len(revisions) == 1, "ABI revisions differ"
    revision = revisions.pop()
    # Only documentation and runtime-harness scripts may differ. Build scripts,
    # Gradle pins, Kotlin, native/core source and fixture assets must be identical.
    # **/*.md did not exclude root-level Markdown on the CI Git version.
    # Both scopes are documentation; Kotlin/native/packaging remain compared.
    paths = [".", ":(exclude,glob)*.md", ":(exclude)**/*.md", ":(exclude).github/workflows/*",
             ":(exclude)apps/alfred/scripts/emulator-smoke.py",
             ":(exclude)apps/alfred/scripts/jobs-smoke.py",
             ":(exclude)apps/alfred/scripts/features-ui-smoke.py",
             ":(exclude)apps/alfred/scripts/saf-ui-smoke.py",
             ":(exclude)apps/alfred/scripts/saf-finger-tap/**",
             ":(exclude)apps/alfred/scripts/verify_reused_build.py"]
    difference = subprocess.run(["git", "diff", "--quiet", revision, "HEAD", "--", *paths])
    if difference.returncode == 1 and if_unchanged:
        print("Compiled inputs changed; require the full APK build workflow.")
        return False
    difference.check_returncode()
    print(f"Verified APK/source reuse from {revision}")
    return True


if __name__ == "__main__":
    sys.exit(0 if verify(Path(sys.argv[1]), "--if-unchanged" in sys.argv[2:]) else 2)
