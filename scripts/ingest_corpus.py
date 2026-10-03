"""Snapshot a declared local corpus and run full-file Rust measurements.

Python stdlib only; optional development tooling, outside the offline Rust core.
No source labels are inferred. Notes and reports remain private in corpus/local.
"""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
PRIVATE = ROOT / "corpus/local"
SPLITS = {"development", "validation", "locked_test", "challenge"}
HISTORIES = {"documented": 0, "claimed": 1, "unknown": 2}
ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]{0,63}\Z")
SHA = re.compile(r"[0-9a-f]{64}\Z")


class IntakeError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise IntakeError(message)


def object_keys(value, required, optional=()):
    require(isinstance(value, dict), "Expected a JSON object")
    require(required <= value.keys(), f"Missing fields: {sorted(required - value.keys())}")
    require(value.keys() <= required | set(optional), "Unexpected manifest fields")


def text(value):
    return isinstance(value, str) and bool(value.strip())


def unique_object(pairs):
    out = {}
    for key, value in pairs:
        require(key not in out, f"Duplicate JSON key: {key}")
        out[key] = value
    return out


def parse_json(data):
    def invalid_constant(value):
        raise IntakeError(f"Non-finite JSON constant: {value}")
    def finite_float(value):
        number = float(value)
        require(math.isfinite(number), "Non-finite JSON number")
        return number
    return json.loads(data, object_pairs_hook=unique_object, parse_constant=invalid_constant,
                      parse_float=finite_float)


def fingerprint(path):
    before = path.stat()
    digest = hashlib.sha256()
    count = 0
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            digest.update(chunk)
            count += len(chunk)
    after = path.stat()
    require((before.st_size, before.st_mtime_ns) == (after.st_size, after.st_mtime_ns)
            and count == before.st_size, f"File changed while hashing: {path}")
    return {"sha256": digest.hexdigest(), "bytes": count}


def local_file(root, name):
    require(text(name) and not Path(name).is_absolute(), "File paths must be relative to root")
    path = (root / name).resolve(strict=True)
    require(path.is_relative_to(root) and path.is_file(), f"File outside root or not a file: {name}")
    return path


def load_manifest(path):
    manifest = parse_json(path.read_bytes())
    object_keys(manifest, {"version", "root", "entries"})
    require(type(manifest["version"]) is int and manifest["version"] == 1, "Expected manifest version 1")
    require(text(manifest["root"]), "root must name the local source directory")
    root = (path.parent / manifest["root"]).resolve(strict=True)
    require(root.is_dir(), "root is not a directory")
    entries = manifest["entries"]
    require(isinstance(entries, list) and bool(entries), "entries must be a nonempty list")
    by_id, groups, paths, hashes = {}, {}, set(), {}
    prepared = []
    for entry in entries:
        object_keys(entry, {"id", "group_id", "split", "file", "history", "notes"},
                    {"parent_id", "recipe"})
        for key in ("id", "group_id"):
            require(isinstance(entry[key], str) and ID.fullmatch(entry[key]), f"Invalid {key}")
        require(entry["id"] not in by_id, f"Duplicate id: {entry['id']}")
        require(isinstance(entry["split"], str) and entry["split"] in SPLITS, "Invalid split")
        require(isinstance(entry["history"], str) and entry["history"] in HISTORIES, "Invalid history")
        require(entry["history"] == "documented" or entry["split"] == "challenge",
                "Claimed/unknown histories belong in challenge, not evaluation splits")
        group = entry["group_id"]
        require(group not in groups or groups[group] == entry["split"], f"Group crosses splits: {group}")
        groups[group] = entry["split"]
        audio = local_file(root, entry["file"])
        notes = local_file(root, entry["notes"])
        require(audio != notes, "Audio and provenance notes must be different files")
        require(audio not in paths, f"Duplicate file path: {entry['file']}")
        paths.add(audio)
        require(notes.stat().st_size <= 1024 * 1024, "Provenance notes exceed 1 MiB")
        notes_hash = fingerprint(notes)
        require(notes_hash["bytes"] <= 1024 * 1024, "Provenance notes exceed 1 MiB")
        note_text = notes.read_text(encoding="utf-8-sig")
        require(fingerprint(notes) == notes_hash, "Provenance notes changed during snapshot")
        require(text(note_text), "Provenance notes must be nonempty UTF-8 text")
        audio_hash = fingerprint(audio)
        digest = audio_hash["sha256"]
        require(digest not in hashes or hashes[digest] == group, "Identical files assigned to different groups")
        hashes[digest] = group
        recipe = entry.get("recipe", [])
        require(isinstance(recipe, list), "recipe must be a list")
        parent = entry.get("parent_id")
        require(parent is None or (isinstance(parent, str) and ID.fullmatch(parent)), "Invalid parent_id")
        require(bool(recipe) == (parent is not None), "Derivatives require a parent and recipe; originals have neither")
        for step in recipe:
            object_keys(step, {"tool", "version", "arguments"})
            require(text(step["tool"]) and text(step["version"]), "Recipe tool/version must be recorded")
            require(isinstance(step["arguments"], list)
                    and all(isinstance(arg, str) for arg in step["arguments"]), "Recipe arguments must be strings")
        record = dict(entry, parent_id=parent, recipe=recipe, file_fingerprint=audio_hash,
                      notes_fingerprint=notes_hash, notes_text=note_text)
        by_id[entry["id"]] = record
        prepared.append((record, audio, notes))
    for record, _, _ in prepared:
        visited = {record["id"]}
        current = record
        while current["parent_id"] is not None:
            parent = current["parent_id"]
            require(parent in by_id, f"Missing parent: {parent}")
            require(parent not in visited, "Cyclic parent lineage")
            visited.add(parent)
            previous = by_id[parent]
            require(previous["group_id"] == current["group_id"], "Parent and descendant must share a group")
            require(HISTORIES[current["history"]] >= HISTORIES[previous["history"]],
                    "A transform cannot upgrade an uncertain parent history")
            current = previous
    return manifest, prepared, groups


def validate_report(report, returncode):
    require(isinstance(report, list) and len(report) == 1 and isinstance(report[0], dict),
            "Expected exactly one CLI report")
    report = report[0]
    require(report.get("ancestry_verdict") == "INCONCLUSIVE" and "evidence_index" in report
            and report["evidence_index"] is None, "CLI report violates observations-only policy")
    require(all(text(report.get(key)) for key in ("engine_version", "schema_version", "policy_version"))
            and report["policy_version"].startswith("observations-only-"), "Missing report versions/policy")
    require(isinstance(report.get("status"), str)
            and report["status"] in {"analyzed", "unsupported", "failed", "cancelled", "timed_out"},
            "Unknown CLI status")
    expected_exit = 0 if report["status"] == "analyzed" else 130 if report["status"] == "cancelled" else 1
    require(returncode == expected_exit, "CLI exit/status mismatch")
    if report["status"] == "analyzed":
        coverage = report.get("coverage")
        require(isinstance(coverage, dict) and coverage.get("reached_end") is True
                and coverage.get("requested_max_seconds") is None and coverage.get("start_seconds") == 0
                and coverage.get("analysis_passes") == 2, "Intake requires full-file two-pass coverage")
        require(isinstance(coverage.get("decoded_pcm_sha256"), str)
                and SHA.fullmatch(coverage["decoded_pcm_sha256"]), "Missing decoded PCM hash")
        require(type(coverage.get("analyzed_frames")) is int and coverage["analyzed_frames"] > 0
                and coverage.get("hash_sample_encoding") in ("s32le_msb_aligned", "f64le"),
                "Missing PCM count/encoding")
    return report


def save_json(path, value):
    with path.open("x", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, allow_nan=False)
        handle.write("\n")


def ingest(manifest_path, binary, output, deadline):
    manifest_before = fingerprint(manifest_path)
    manifest, prepared, groups = load_manifest(manifest_path)
    require(fingerprint(manifest_path) == manifest_before, "Manifest changed during validation")
    binary_before = fingerprint(binary)
    output.mkdir(parents=True, exist_ok=False)
    save_json(output / "manifest-snapshot.json", dict(manifest, entries=[p[0] for p in prepared],
              resolved_root=str((manifest_path.parent / manifest["root"]).resolve()),
              manifest_path=str(manifest_path)))
    results, errors, pcm_groups = [], [], {}
    for index, (record, audio, notes) in enumerate(prepared):
        result = {"id": record["id"], "group_id": record["group_id"], "split": record["split"]}
        if record["split"] == "locked_test":
            result["status"] = "reserved"
            results.append(result)
            print(f"[{index + 1}/{len(prepared)}] {record['id']}: reserved (no analysis)", flush=True)
            continue
        try:
            require(fingerprint(audio) == record["file_fingerprint"], "Source changed before analysis")
            process = subprocess.run([str(binary), "--json", "--deadline-seconds", str(deadline), str(audio)],
                                     capture_output=True, timeout=deadline + 30)
            (output / f"{index:04d}.json").write_bytes(process.stdout)
            (output / f"{index:04d}.log").write_bytes(process.stderr)
            report = validate_report(parse_json(process.stdout), process.returncode)
            result.update(status=report["status"], exit_code=process.returncode,
                          engine_version=report.get("engine_version"), schema_version=report.get("schema_version"),
                          policy_version=report["policy_version"],
                          stream=report.get("stream"), coverage=report.get("coverage"),
                          diagnostics=report.get("diagnostics"), report=f"{index:04d}.json")
            if report["status"] == "analyzed":
                coverage = report["coverage"]
                pcm = (coverage["hash_sample_encoding"], coverage["decoded_pcm_sha256"])
                group = record["group_id"]
                require(pcm not in pcm_groups or pcm_groups[pcm] == group,
                        "Identical decoded PCM assigned to different groups")
                pcm_groups[pcm] = group
            else:
                errors.append(f"{record['id']}: {report['status']}")
            require(fingerprint(audio) == record["file_fingerprint"], "Source changed during analysis")
        except (IntakeError, OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
            result.update(status="intake_failed", error=str(error))
            if isinstance(error, subprocess.TimeoutExpired):
                (output / f"{index:04d}.json").write_bytes(error.stdout or b"")
                (output / f"{index:04d}.log").write_bytes(error.stderr or b"")
            errors.append(f"{record['id']}: {error}")
        results.append(result)
        print(f"[{index + 1}/{len(prepared)}] {record['id']}: {result['status']}", flush=True)
    # Recheck every source/notes/binary, including inputs whose decoder failed.
    for record, audio, notes in prepared:
        for path, expected in ((audio, record["file_fingerprint"]), (notes, record["notes_fingerprint"])):
            try:
                require(fingerprint(path) == expected, f"Input changed: {path}")
            except (OSError, IntakeError) as error:
                errors.append(str(error))
    for path, expected in ((manifest_path, manifest_before), (binary, binary_before)):
        try:
            require(fingerprint(path) == expected, f"Input changed: {path}")
        except (OSError, IntakeError) as error:
            errors.append(str(error))
    summary = {"intake_version": 1, "created_utc": datetime.now(timezone.utc).isoformat(),
               "manifest_path": str(manifest_path), "manifest_fingerprint": manifest_before,
               "binary_path": str(binary), "binary_fingerprint": binary_before,
               "declared_group_counts": dict(sorted(Counter(groups.values()).items())),
               "status_counts": dict(sorted(Counter(r["status"] for r in results).items())),
               "records": results, "errors": errors, "passed": not errors,
               "accuracy_evaluated": False, "ancestry_labels_verified": False,
               "analysis_scope": "Full files outside locked_test; locked groups are hashed/reserved only",
               "caveat": "Declared groups/history and copied notes require human provenance review; "
                         "hash equality checks cannot detect every related recording or transform. "
                         "Decoded duplicate checks exclude reserved locked groups. "
                         "Rust PCM hashes are not independent-decoder verification."}
    save_json(output / "summary.json", summary)
    return summary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/audio-forensic.exe"
                        if sys.platform == "win32" else ROOT / "target/release/audio-forensic")
    parser.add_argument("--output", type=Path, required=True, help="New directory beneath corpus/local")
    parser.add_argument("--deadline-seconds", type=int, default=600)
    args = parser.parse_args()
    try:
        require(args.deadline_seconds > 0, "Deadline must be positive")
        output = args.output.resolve()
        require(output.is_relative_to(PRIVATE.resolve()) and output != PRIVATE.resolve(),
                "Private intake output must be beneath corpus/local")
        summary = ingest(args.manifest.resolve(strict=True), args.binary.resolve(strict=True), output,
                         args.deadline_seconds)
        print(f"{'PASS' if summary['passed'] else 'FAIL'}: {len(summary['records'])} entries; {output}")
        return 0 if summary["passed"] else 1
    except (IntakeError, OSError, ValueError) as error:
        print(f"Intake error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
