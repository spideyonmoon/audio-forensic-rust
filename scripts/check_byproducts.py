"""Generated-only decoded P03 differential, exact PCM and offline schemas."""
import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import shutil
import struct
import subprocess
import numpy as np
from jsonschema import Draft202012Validator
from generate_byproduct_reference import ROOT, samples


def hash_bytes(data):
    return hashlib.sha256(data).hexdigest()


def wav(deck, rate, integer=False):
    channels = deck.shape[1]
    width = 3 if integer else 8
    if integer:
        raw = b"".join((int(x * 2**23) & 0xffffff).to_bytes(3, "little") for x in deck.ravel())
    else:
        raw = deck.astype("<f8").tobytes()
    fmt = struct.pack("<HHIIHH", 1 if integer else 3, channels, rate, rate*channels*width, channels*width, width*8)
    body = b"WAVEfmt " + struct.pack("<I", len(fmt)) + fmt + b"data" + struct.pack("<I", len(raw)) + raw
    return b"RIFF" + struct.pack("<I", len(body)) + body


def compare(p, case):
    r, b, e = p["measurement"], p["byproducts"]["reference"], case["expected"]
    assert r["status"] == p["byproducts"]["status"] == "analyzed"
    assert r["ancestry_verdict"] == "INCONCLUSIVE" and r["evidence_index"] is None
    assert r["coverage"] == p["byproducts"]["coverage"]
    assert r["stream"] == p["byproducts"]["stream"]
    assert b["interval"] == dict(start_frame=0, end_frame=e["frames"])
    assert b["phase"]["display"] == (e["phase"][0] or None)
    assert b["phase"]["legacy_verdict"] == (e["phase"][1] or None)
    a = b["phase"]["mean_correlation"]
    if e["correlation"] is None:
        assert a is None
    else:
        assert abs(a-e["correlation"]) <= 1e-10
    assert b["phase"]["valid_blocks"] == e["valid_blocks"]
    assert b["ceiling_count"]["display"] == e["clipping"][0]
    assert b["ceiling_count"]["legacy_verdict"] == e["clipping"][1]
    assert b["silence"]["total_percent"]["display"] == e["silence_pct"]
    assert [[s["interval"]["start_frame"], s["interval"]["end_frame"]] for s in b["silence"]["sections"]] == e["runs"]
    assert [s["legacy_display"] for s in b["silence"]["sections"]] == e["sections"]
    assert b["silence"]["total_silent_frames"] == sum(end-start for start, end in e["runs"])
    assert abs(b["silence"]["total_percent"]["value"] - sum(end-start for start, end in e["runs"])/e["frames"]*100) <= 1e-12
    n = b["noise_floor_fallback"]
    assert n["level_dbfs"]["display"] == (e["floor"] or None)
    assert n["positive_blocks"] == e["positive_blocks"]
    if e["percentile_rms"] is None:
        assert n["percentile_rms"] is None
    else:
        assert abs(n["percentile_rms"] / e["percentile_rms"] - 1) <= 1e-12
        assert abs(n["level_dbfs"]["value"] - 20*math.log10(e["percentile_rms"])) <= 1e-9
    levels = p["byproducts"]["native_levels"]
    assert levels["loudness_interval"] == r["loudness"]["interval"]
    assert levels["loudness_range_interval"] == r["loudness"]["range"]["interval"]
    for channel, level in zip(r["channels"], levels["channels"]):
        assert level["dc_offset_amplitude"]["value"] == channel["dc_offset"]
        assert level["sample_crest_linear"]["value"] == channel["crest_factor_linear"]
        assert level["sample_crest_db"]["value"] == channel["crest_factor_db"]
        for scalar, field in [("peak", "sample_peak_dbfs"), ("rms", "rms_dbfs")]:
            expected = 20*math.log10(channel[scalar]) if channel[scalar] > 0 else None
            assert level[field]["value"] == expected or (expected is not None and abs(level[field]["value"]-expected) <= 1e-10)
            assert level[field]["display"] == (f"{expected:.2f}" if expected is not None else None)
        true_peak = next(t for t in r["true_peak"] if t["channel_index"] == channel["channel_index"])
        assert level["estimated_true_peak_dbtp"]["value"] == true_peak["estimated_peak_dbtp"]
    lufs = r["loudness"]["integrated_lufs"]
    for field, target in [("fixed_minus16_delta_db", -16), ("fixed_minus14_delta_db", -14)]:
        assert levels[field]["value"] == (target-lufs if lufs is not None else None)
        assert levels[field]["display"] == (f"{target-lufs:+.1f} dB" if lufs is not None else None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/examples/read_byproducts.exe")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--check-saved", action="store_true", help="read-only recheck of saved final reports; no audio execution")
    args = parser.parse_args()
    if args.check_saved:
        fixture = json.loads((args.output / "expected.json").read_text(encoding="utf-8"))
        assert (args.output / "expected.json").read_bytes() == (ROOT / "tests/fixtures/byproduct_reference.json").read_bytes()
        maxima = dict(phase_absolute=0.0, percentile_rms_relative=0.0, floor_db_absolute=0.0)
        for case in fixture["cases"]:
            product = json.loads((args.output / (case["name"] + ".json")).read_text(encoding="utf-8"))
            compare(product, case)
            b, e = product["byproducts"]["reference"], case["expected"]
            if e["correlation"] is not None:
                maxima["phase_absolute"] = max(maxima["phase_absolute"], abs(b["phase"]["mean_correlation"]-e["correlation"]))
            if e["percentile_rms"] is not None:
                n = b["noise_floor_fallback"]
                maxima["percentile_rms_relative"] = max(maxima["percentile_rms_relative"], abs(n["percentile_rms"]/e["percentile_rms"]-1))
                maxima["floor_db_absolute"] = max(maxima["floor_db_absolute"], abs(n["level_dbfs"]["value"]-20*math.log10(e["percentile_rms"])))
        print(json.dumps(dict(result="PASS", saved_cases=len(fixture["cases"]), max_errors=maxima)))
        return
    args.output.mkdir(parents=True, exist_ok=False)
    original = (ROOT / "tests/fixtures/byproduct_reference.json").read_bytes()
    (args.output / "expected.json").write_bytes(original)
    vectors = json.loads(original)["cases"]
    ffmpeg = shutil.which("ffmpeg") or str(ROOT / ".tools/ffmpeg/bin/ffmpeg.exe")
    version = subprocess.check_output([ffmpeg, "-version"], text=True).splitlines()[0]
    schemas = {}
    for key, filename in [("measurement", "analysis-report-0.18.0.schema.json"), ("byproducts", "byproducts-1.schema.json")]:
        artifact = json.loads((ROOT / "schemas" / filename).read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(artifact)
        schemas[key] = Draft202012Validator(artifact)
    binary_hash = hash_bytes(args.binary.read_bytes())
    records = []
    for case in vectors:
        deck = samples(case)
        # One exact 24-bit integer case additionally proves the fixed ceiling is
        # different from the native precision's full-scale count.
        integer = case["name"] == "precision24_fixed_ceiling"
        path = args.output / (case["name"] + ".wav")
        audio = wav(deck, case["rate"], integer)
        path.write_bytes(audio)
        command = [str(args.binary), str(path)]
        ffcommand = [ffmpeg, "-v", "error", "-i", str(path)]
        if case["prefix_frames"] is not None:
            span = str(case["prefix_frames"] / case["rate"])
            command.append(span)
            ffcommand += ["-t", span]
        encoding = "s32le" if integer else "f64le"
        pcm = subprocess.check_output(ffcommand + ["-f", encoding, "-c:a", "pcm_"+encoding, "pipe:1"])
        receipt = subprocess.run(command, capture_output=True, text=True, encoding="utf-8", timeout=90)
        (args.output / (case["name"] + ".stderr.txt")).write_text(receipt.stderr, encoding="utf-8")
        (args.output / (case["name"] + ".json")).write_text(receipt.stdout, encoding="utf-8")
        assert receipt.returncode == 0, (case["name"], receipt.stderr)
        product = json.loads(receipt.stdout)
        for key, validator in schemas.items():
            validator.validate(product[key])
        compare(product, case)
        coverage = product["measurement"]["coverage"]
        assert coverage["decoded_pcm_sha256"] == hash_bytes(pcm), case["name"]
        assert coverage["analyzed_frames"] == len(pcm) // (case["channels"] * (4 if integer else 8))
        if integer:
            assert all(c["full_scale_samples"] == 0 for c in product["measurement"]["channels"])
            assert product["byproducts"]["reference"]["ceiling_count"]["samples_at_or_above_threshold"] == len(deck)*2
        assert path.read_bytes() == audio
        records.append(dict(name=case["name"], input_sha256=hash_bytes(audio), pcm_sha256=hash_bytes(pcm), frames=coverage["analyzed_frames"]))
    rejected = []
    baseline = json.loads((args.output / "dual_mono.json").read_text(encoding="utf-8"))["byproducts"]
    for name, mutate in [
        ("unknown_version", lambda p: p.update(byproduct_version=2)),
        ("failed_with_reference", lambda p: p.update(status="failed")),
        ("available_null", lambda p: p["native_levels"]["integrated_lufs"].update(value=None)),
        ("too_many_sections", lambda p: p["reference"]["silence"].update(sections=[baseline["reference"]["silence"]["sections"][0]]*1025)),
    ]:
        # Dual mono has no silence sections; use the independently saved silence
        # report for this one structural mutation.
        p = copy.deepcopy(baseline)
        if name == "too_many_sections":
            sample = json.loads((args.output / "silence.json").read_text(encoding="utf-8"))["byproducts"]["reference"]["silence"]["sections"][0]
            p["reference"]["silence"]["sections"] = [sample]*1025
        else:
            mutate(p)
        assert list(schemas["byproducts"].iter_errors(p)), name
        rejected.append(name)
    assert hash_bytes(args.binary.read_bytes()) == binary_hash
    assert (ROOT / "tests/fixtures/byproduct_reference.json").read_bytes() == original
    summary = dict(cases=records, count=len(records), exact_pcm=len(records), schema_rejections=rejected,
                   ffmpeg=version, numpy=np.__version__, binary_sha256=binary_hash,
                   fixture_sha256=hash_bytes(original), result="PASS")
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2)+"\n", encoding="utf-8")
    print(f"PASS: {len(records)} decoded generated cases, exact FFmpeg PCM hashes, pinned byproducts/native mapping, both schemas, {len(rejected)} schema rejections")


if __name__ == "__main__":
    main()
