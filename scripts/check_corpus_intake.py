"""Generated-only end-to-end intake controls with an independent integer PCM hash."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import wave

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "corpus/local/results/corpus-intake-v1-smoke")
    args = parser.parse_args()
    out = args.output.resolve()
    if not out.is_relative_to((ROOT / "corpus/local").resolve()):
        raise SystemExit("Choose a new output directory beneath corpus/local")
    out.mkdir(parents=True, exist_ok=False)
    audio = out / "generated"
    audio.mkdir()
    samples = [(i % 129 - 64) * 123 for i in range(1600)]
    pcm = struct.pack("<" + "h" * len(samples), *samples)
    expected = hashlib.sha256(b"".join(struct.pack("<i", x * 65536) for x in samples)).hexdigest()
    with wave.open(str(audio / "a.wav"), "wb") as handle:
        handle.setparams((1, 2, 8000, 0, "NONE", "not compressed"))
        handle.writeframes(pcm)
    base = (audio / "a.wav").read_bytes()
    junk = b"JUNK" + struct.pack("<I", 4) + b"demo"
    variant = b"RIFF" + struct.pack("<I", len(base) - 8 + len(junk)) + base[8:36] + junk + base[36:]
    (audio / "equivalent.wav").write_bytes(variant)
    with wave.open(str(audio / "reserved.wav"), "wb") as handle:
        handle.setparams((1, 2, 8000, 0, "NONE", "not compressed"))
        handle.writeframes(struct.pack("<" + "h" * len(samples), *(x + 1 for x in samples)))
    (audio / "unsupported.dat").write_bytes(b"generated unsupported signature")
    (audio / "notes.txt").write_text("Synthetic integer ramp controls only. "
                                   "Equivalent WAV adds a JUNK chunk without modifying PCM.\n", encoding="utf-8")
    before = {p.name: digest(p) for p in audio.iterdir()}
    original = dict(id="original", group_id="generated-a", split="development", file="a.wav",
                    history="documented", notes="notes.txt")
    equivalent = dict(original, id="equivalent", file="equivalent.wav", parent_id="original",
                      recipe=[dict(tool="Python struct", version=sys.version.split()[0],
                                   arguments=["insert 4-byte JUNK chunk before WAV data; preserve PCM"])])
    reserved = dict(original, id="reserved", group_id="generated-locked", split="locked_test",
                    file="reserved.wav")
    unsupported = dict(original, id="unsupported", group_id="generated-challenge", split="challenge",
                       history="unknown", file="unsupported.dat")
    controls = []

    def run(name, entries, expected_exit, destination=None):
        manifest = out / f"{name}-manifest.json"
        manifest.write_text(json.dumps(dict(version=1, root="generated", entries=entries), indent=2) + "\n")
        destination = destination or out / name
        command = [sys.executable, str(ROOT / "scripts/ingest_corpus.py"), str(manifest),
                   "--binary", str(args.binary.resolve()), "--output", str(destination), "--deadline-seconds", "30"]
        result = subprocess.run(command, capture_output=True, timeout=120)
        (out / f"{name}.stdout.txt").write_bytes(result.stdout)
        (out / f"{name}.stderr.txt").write_bytes(result.stderr)
        assert result.returncode == expected_exit, (name, result.returncode, result.stderr)
        controls.append(dict(name=name, exit_code=result.returncode, expected_exit=expected_exit))
        return destination

    positive = run("positive", [original, equivalent, reserved], 0)
    summary = json.loads((positive / "summary.json").read_text())
    assert summary["passed"] and summary["status_counts"] == {"analyzed": 2, "reserved": 1}
    assert summary["declared_group_counts"] == {"development": 1, "locked_test": 1}
    assert not summary["accuracy_evaluated"] and not summary["ancestry_labels_verified"]
    snapshot = json.loads((positive / "manifest-snapshot.json").read_text())
    assert snapshot["resolved_root"] == str(audio) and Path(snapshot["manifest_path"]).is_file()
    for record in summary["records"][:2]:
        assert record["coverage"]["decoded_pcm_sha256"] == expected
        assert record["coverage"]["analyzed_frames"] == len(samples)
        assert record["coverage"]["reached_end"] and record["coverage"]["requested_max_seconds"] is None
        report = json.loads((positive / record["report"]).read_text())[0]
        assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
    assert not (positive / "0002.json").exists()
    leak = dict(equivalent, group_id="incorrect-other-group", parent_id=None, recipe=[])
    leaked = run("decoded-leakage", [original, leak], 1)
    assert "Identical decoded PCM" in json.loads((leaked / "summary.json").read_text())["errors"][0]
    failed = run("unsupported", [unsupported, original], 1)
    failed_summary = json.loads((failed / "summary.json").read_text())
    assert [r["status"] for r in failed_summary["records"]] == ["unsupported", "analyzed"]
    rejected = run("split-leakage", [original, dict(equivalent, split="locked_test")], 2)
    assert not rejected.exists()
    preserved = digest(positive / "summary.json")
    run("overwrite", [original, equivalent, reserved], 2, positive)
    assert digest(positive / "summary.json") == preserved
    outside = ROOT / "target/intake-output-must-not-exist"
    assert not outside.exists()
    run("private-boundary", [original], 2, outside)
    assert not outside.exists()
    after = {p.name: digest(p) for p in audio.iterdir()}
    assert before == after, "Generated source/notes files changed"
    evidence = dict(passed=True, controls=controls, source_hashes_unchanged=True,
                    source_hashes=before, expected_s32le_sha256=expected,
                    binary_sha256=digest(args.binary), ancestry_labels_verified=False,
                    independent_hash_method="Generated s16 samples MSB-aligned into signed s32le")
    (out / "summary.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(f"PASS: {len(controls)} end-to-end controls; exact PCM; original hashes unchanged; {out}")


if __name__ == "__main__":
    main()
