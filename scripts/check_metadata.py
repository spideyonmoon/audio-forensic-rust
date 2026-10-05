"""Generated-only P02 differential checks. Freeze expected values before Rust."""
import argparse
from dataclasses import make_dataclass, field, asdict
import hashlib
import json
from pathlib import Path
import re
import subprocess
from generate_replaygain_reference import ROOT, BASELINE, pinned_function


def flac(entries):
    original = (ROOT / "tests/fixtures/noise16.flac").read_bytes()
    offset = 4
    while True:
        last = original[offset] & 128
        length = int.from_bytes(original[offset+1:offset+4], "big")
        offset += 4 + length
        if last:
            break
    vendor = b"generated P02 vendor"
    comments = len(vendor).to_bytes(4, "little") + vendor + len(entries).to_bytes(4, "little")
    for key, value in entries:
        text = (key + "=" + value).encode("utf-8")
        comments += len(text).to_bytes(4, "little") + text
    return original[:4] + bytes([0]) + original[5:42] + bytes([132]) + len(comments).to_bytes(3, "big") + comments + original[offset:]


def chunk(key, value):
    return key + len(value).to_bytes(4, "little") + value + b"\0" * (len(value) % 2)


def wav():
    fmt = bytes.fromhex("0100010080bb00000077010002001000")
    info = b"INFO" + chunk(b"INAM", b"Generated title\0") + chunk(b"IART", b"Generated artist\0") + chunk(b"ICMT", b"joint stereo quoted\0") + chunk(b"ZZZZ", b"unknown text\0")
    body = b"WAVE" + chunk(b"fmt ", fmt) + chunk(b"data", b"\0" * 96000) + chunk(b"LIST", info)
    return b"RIFF" + len(body).to_bytes(4, "little") + body


def validate_metadata(report, schema_validator):
    schema_validator.validate(report)
    limits = report["text_limits"]
    assert len(report["entries"]) <= 1024
    retained = 0
    for entry in report["entries"]:
        key_bytes = len(entry["key"].encode("utf-8"))
        value_bytes = len((entry["value"] or "").encode("utf-8"))
        assert key_bytes <= 256 and value_bytes <= 16384
        retained += key_bytes + value_bytes
        if entry["artwork_index"] is not None:
            assert entry["value"] is None and entry["artwork_index"] < len(report["artwork"])
    assert retained == limits["retained_text_utf8_bytes"] <= 1048576
    assert limits["total_text_utf8_bytes"] == retained + limits["omitted_text_utf8_bytes"]
    assert limits["total_entries"] == len(report["entries"]) + limits["omitted_entries"]
    for indices in report["named_tags"].values():
        assert indices and all(0 <= i < len(report["entries"]) for i in indices)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/examples/read_metadata.exe")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    from jsonschema import Draft202012Validator
    from referencing import Registry
    def no_network(uri):
        raise RuntimeError(f"external schema denied: {uri}")
    validators = {name: Draft202012Validator(json.loads((ROOT / "schemas" / filename).read_text()), registry=Registry(retrieve=no_network))
                  for name, filename in [("metadata", "metadata-1.schema.json"), ("audit", "replaygain-audit-1.schema.json")]}
    for validator in validators.values():
        validator.check_schema(validator.schema)
    Tags = make_dataclass("Tags", [(name, str, field(default="")) for name in
        ("title", "album", "date", "album_artist", "artist", "bpm", "comment_quality", "comments", "replaygain_track_gain", "replaygain_album_gain")]
        + [("other", dict, field(default_factory=dict))])
    Technical = make_dataclass("Technical", [(name, str, field(default="")) for name in
        ("bit_rate", "channels", "precision", "sample_rate", "sample_encoding", "duration", "writing_library", "format_profile", "compression_mode", "codec")]
        + [("duration_sec", float, field(default=0.0))])
    def run(command):
        return subprocess.run(command, capture_output=True, text=True, encoding="utf-8", errors="strict", check=False)
    source = (ROOT / "reference/audio-forensic/audio_forensic.py").read_text(encoding="utf-8")
    # Read only the two pinned sets/tuple needed by the extractor/trace functions.
    import ast
    tree = ast.parse(source)
    ns = {"json": json, "re": re, "_run": run, "AudioTags": Tags, "AudioTechnical": Technical}
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id in
            ("_MEDIAINFO_NONTAG_KEYS", "_KNOWN_TAG_KEYS", "_LOSSLESS_EXTS", "_ENCODER_SIGNATURES") for t in node.targets):
            exec(compile(ast.Module(body=[node], type_ignores=[]), "pinned constants", "exec"), ns)
    pinned_function("_prettify_mi_key", ns)
    extractor = pinned_function("extract_mediainfo", ns)
    encoder = pinned_function("detect_encoder_trace", ns)
    rg = pinned_function("audit_replaygain", {"re": re})
    entries = [("TITLE", "Generated title"), ("ALBUM", "Generated album"), ("ARTIST", "Generated artist"),
               ("DATE", "2026"), ("ALBUMARTIST", "Generated album artist"), ("BPM", "99"),
               ("COMMENT", "LAME quoted, not provenance"), ("REPLAYGAIN_TRACK_GAIN", "+2.00 dB"),
               ("REPLAYGAIN_ALBUM_GAIN", "-2.00 dB"), ("X_CUSTOM", "Unicode বাংলা 🎵"), ("MQA", "studio")]
    files = [("named.flac", flac(entries)), ("trailing.wav", wav()), ("duplicate.flac", flac([("Title", "first"), ("TITLE", "second"), ("Unknown", "unchanged=tail")]))]
    expected = {}
    for name, data in files:
        path = args.output / name
        path.write_bytes(data)
        tags, tech = extractor(path)
        mi = run(["mediainfo", "--Output=JSON", str(path)])
        assert mi.returncode == 0, mi.stderr
        (args.output / (name + ".mediainfo.json")).write_text(mi.stdout, encoding="utf-8")
        expected[name] = {"file_sha256": hashlib.sha256(data).hexdigest(), "tags": asdict(tags), "technical": asdict(tech), "encoder_trace": encoder(tags, tech, path)}
    binary = args.binary.resolve()
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
    versions = {"mediainfo": run(["mediainfo", "--Version"]).stdout.strip(), "reference_commit": BASELINE, "binary_sha256": binary_hash}
    (args.output / "expected.json").write_text(json.dumps({"versions": versions, "cases": expected}, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    # Expectations above are frozen before the first Rust run.
    for name, _ in files:
        result = run([str(binary), str(args.output / name)])
        assert result.returncode == 0, result.stderr
        (args.output / (name + ".rust.json")).write_text(result.stdout, encoding="utf-8")
        report = json.loads(result.stdout)
        validate_metadata(report, validators["metadata"])
        assert report["status"] == "available"
        tech = report["technical"]
        oracle = expected[name]
        if name != "duplicate.flac":
            for field_name in ("title", "album", "artist", "date", "album_artist", "bpm", "comments"):
                indices = report["named_tags"].get(field_name, [])
                actual = report["entries"][indices[0]]["value"] if indices else ""
                if name == "trailing.wav" and field_name == "artist":
                    # MediaInfo 24.01 maps RIFF IART to Director; the pinned
                    # extractor leaves artist empty. Verify the exact raw text
                    # against its retained Other field, without discarding IART.
                    assert actual == oracle["tags"]["other"]["Director"] == "Generated artist"
                    continue
                assert actual == oracle["tags"][field_name], (name, field_name, actual, oracle["tags"][field_name])
            assert (report["observations"]["legacy_encoder_trace"] or "") == oracle["encoder_trace"]
        else:
            assert [report["entries"][i]["value"] for i in report["named_tags"]["title"]] == ["first", "second"]
        for raw_name, mi_name in [("declared_sample_rate_hz", "sample_rate"), ("declared_channels", "channels")]:
            assert tech[raw_name] == int(oracle["technical"][mi_name])
        assert tech["declared_precision_bits"] == int(oracle["technical"]["precision"].split("-")[0])
        # MediaInfo duration is printed to milliseconds; headers retain exact frames.
        assert abs(tech["declared_duration_seconds"] - oracle["technical"]["duration_sec"]) <= 0.001
        assert tech["codec"] == oracle["technical"]["codec"]
        if name == "named.flac":
            for field_name, value in [("replaygain_track_gain", "+2.00 dB"), ("replaygain_album_gain", "-2.00 dB")]:
                assert report["entries"][report["named_tags"][field_name][0]]["value"] == value
            # MediaInfo normalizes these into different General/Audio fields;
            # pinned extract_mediainfo misses both. Keep its empty projection
            # frozen, and independently verify the source values.
            mi = json.loads((args.output / (name + ".mediainfo.json")).read_text(encoding="utf-8"))
            tracks = mi["media"]["track"]
            assert float(next(t for t in tracks if t["@type"] == "Audio")["ReplayGain_Gain"]) == 2.0
            assert float(next(t for t in tracks if t["@type"] == "General")["Album_ReplayGain_Gain"]) == -2.0
        assert hashlib.sha256((args.output / name).read_bytes()).hexdigest() == oracle["file_sha256"]
    # Explicit native prefix audit; no full-file loudness substitution.
    result = run([str(binary), str(args.output / "named.flac"), "--audit", "0.5"])
    assert result.returncode == 0, result.stderr
    (args.output / "prefix-audit.json").write_text(result.stdout, encoding="utf-8")
    pair = json.loads(result.stdout)
    validate_metadata(pair["metadata"], validators["metadata"])
    audit = pair["replaygain_audit"]
    validators["audit"].validate(audit)
    assert audit["status"] == "available" and audit["measurement_method"] == "native-rate-bs1770-two-pass-v1"
    assert audit["analyzed_frames"] == int(pair["metadata"]["technical"]["declared_sample_rate_hz"] * 0.5)
    expected_audit = rg(Tags(replaygain_track_gain="+2.00 dB"), str(audit["measured_lufs"]))
    assert audit["legacy_outputs"] == list(expected_audit)
    # Producer schema guards: missing field, unrecognized version, and excess tags.
    import copy
    mutations = []
    for key in ("source", "text_limits"):
        wrong = copy.deepcopy(pair["metadata"]); del wrong[key]; mutations.append(wrong)
    wrong = copy.deepcopy(pair["metadata"]); wrong["metadata_version"] = 2; mutations.append(wrong)
    wrong = copy.deepcopy(pair["metadata"]); wrong["entries"] *= 1000; mutations.append(wrong)
    for wrong in mutations:
        assert not validators["metadata"].is_valid(wrong)
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == binary_hash
    summary = {"status": "passed", "generated_files": 3, "schema_mutations_rejected": len(mutations), "native_prefix_audits": 1, "private_audio_analyzed": 0,
               "documented_mediainfo_projection_differences": ["24.01 ReplayGain_Gain/Album_ReplayGain_Gain not read by pinned extractor", "24.01 RIFF IART exposed as Director; raw artist retained"]}
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary))


if __name__ == "__main__":
    main()
