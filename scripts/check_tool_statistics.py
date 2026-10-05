"""Read frozen generated tool controls; exact PCM, domains and numeric tolerances.

Never regenerates expected outputs. Writes only a new ignored receipt directory.
"""
import argparse
import copy
import hashlib
import json
import math
import re
from functools import cache
from pathlib import Path
import subprocess
from types import SimpleNamespace
from jsonschema import Draft202012Validator
from generate_tool_statistics_reference import ROOT, deck, wav
from generate_replaygain_reference import pinned_function

AST_KEYS = dict(dc_offset="DC offset", peak_dbfs="Peak level dB", rms_dbfs="RMS level dB",
                rms_peak_dbfs="RMS peak dB", rms_trough_dbfs="RMS trough dB", flat_factor_db="Flat factor",
                peak_count="Peak count", absolute_peak_count="Abs Peak count", noise_floor_dbfs="Noise floor dB", entropy="Entropy",
                crest_linear="Crest factor", amplitude_range_db="Dynamic range", zero_crossings_rate="Zero crossings rate")
LEGACY = dict(peak_db="peak_dbfs", rms_db="rms_dbfs", rms_peak_db="rms_peak_dbfs", rms_trough_db="rms_trough_dbfs",
              noise_floor_db="noise_floor_dbfs", dynamic_range_db="amplitude_range_db", crest_factor_db="crest_linear",
              flat_factor="flat_factor_db", peak_count="peak_count", sox_entropy="entropy", dc_offset="dc_offset",
              zero_crossings_rate="zero_crossings_rate")


@cache
def profile_oracle():
    namespace={"re":re,"LoudnessProfile":SimpleNamespace}
    namespace["_run"]=lambda _:SimpleNamespace(stderr=namespace["stderr"])
    return pinned_function("extract_loudness",namespace),namespace


def pinned_profile(expected):
    oracle,namespace=profile_oracle()
    # Preserve the actual insertion/print order frozen from stderr, including
    # Abs Peak count after Peak count. This executes the unanchored source regex.
    lines=[f"{key}: {val}" for lane in expected["astats"]["channels"]+[expected["astats"]["overall"]] for key,val in lane.items()]
    lines += [f"Channel {i+1}: DR: {val}" for i,val in enumerate(expected["dr_channels"])]
    if expected["dr_overall"] is not None: lines.append("Overall DR: "+expected["dr_overall"])
    namespace["stderr"]="\n".join(lines)
    return oracle(Path("generated.wav"))


def scalar(v, oracle, tolerance, context, maxima):
    expected = float("nan") if "#" in oracle else float(oracle)
    if v["value"] is not None:
        assert math.isfinite(expected), (context, v, oracle)
        delta = abs(v["value"]-expected)
        assert delta <= tolerance, (context, v["value"], expected, delta, tolerance)
        category = "db" if tolerance == .05 else "dr" if tolerance == .1 else "printed_volume" if tolerance == .000501 else "linear"
        maxima[category] = max(maxima[category], delta)
    else:
        assert v["reason"] and v["display"] is None and v["availability"] != "available"
        # Nonphysical finite astats sentinel and undefined one-sample SoX values
        # are deliberate typed corrections; their tool text remains audit data.
        assert not math.isfinite(expected) or "sentinel" in v["reason"] or "Requires" in v["reason"] or "zero power" in v["reason"], (context, v, oracle)


def compare(product, case, maxima):
    report, t, expected = product["measurement"], product["tool_statistics"], case["expected"]
    assert report["status"] == t["status"] == "analyzed", report["diagnostics"]
    assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
    assert report["coverage"] == t["coverage"] == product["byproducts"]["coverage"]
    assert report["stream"] == t["stream"] == product["byproducts"]["stream"]
    assert report["coverage"]["decoded_pcm_sha256"] == expected["pcm_sha256"]
    assert report["coverage"]["analyzed_frames"] == expected["frames"]
    assert report["coverage"]["analysis_passes"] == 2
    m = t["measurements"]; a = m["astats"]
    assert len(a["channels"]) == case["channels"]
    assert a["window_frames"] == int(.05*case["rate"]+.5)
    assert [c["channel_index"] for c in a["channels"]] == list(range(case["channels"]))
    for fields, original in [(ch["fields"], e) for ch, e in zip(a["channels"], expected["astats"]["channels"])] + [(a["overall"], expected["astats"]["overall"])]:
        for key, v in fields.items():
            if key == "crest_db":
                if v["value"] is not None:
                    assert abs(v["value"]-20*math.log10(fields["crest_linear"]["value"])) < 1e-10
                continue
            label = AST_KEYS[key]
            if label not in original:
                assert key == "rms_trough_dbfs" and v["legacy_text"] is None
                continue
            scalar(v, original[label], .05 if key.endswith("dbfs") or key.endswith("_db") else 1e-5, (case["name"], key), maxima)
            if v["legacy_text"] is not None and math.isfinite(float(original[label])):
                assert abs(float(v["legacy_text"])-float(original[label])) <= (.05 if "dB" in v["unit"] else 1e-5)
        if "crest_linear" in fields and fields["crest_linear"]["value"] is None:
            assert float(original["Crest factor"]) == 1
    for ch, e in zip(a["channels"], expected["astats"]["channels"]):
        assert ch["frames"] == expected["frames"]
        assert ch["minimum_samples"]+ch["maximum_samples"] == int(e["Peak count"])
        for key, label in [("zero_crossings", "Zero crossings"), ("absolute_peak_samples", "Abs Peak count"), ("noise_floor_windows", "Noise floor count")]:
            assert ch[key] == int(e[label]), (case["name"], key, ch[key], e[label])
    profile,legacy_dr = pinned_profile(expected)
    assert legacy_dr == expected["legacy_dr"]
    for name in LEGACY:
        want=getattr(profile,name) or None
        assert m["legacy_loudness_profile"][name] == want, (case["name"], name, m["legacy_loudness_profile"][name], want)
    s = m["sox"]; assert len(s["fields"]) == 15
    assert s["channel_sample_count"] == expected["frames"]*case["channels"]
    for key, v in s["fields"].items():
        label = v["source_label"]
        if label not in expected["sox"]:
            assert key == "volumeAdjustment" and v["value"] is None and v["legacy_text"] is None
        elif key == "roughFrequency" and v["value"] is None:
            assert v["reason"] and v["legacy_text"] == expected["sox"][label]
        else:
            scalar(v, expected["sox"][label], .000501 if key == "volumeAdjustment" else 1e-5, (case["name"], key), maxima)
    d = m["dr"]; assert d["legacy_python_label"] == expected["legacy_dr"], (case["name"], d, expected)
    assert len(d["channels"]) == case["channels"]
    assert [c["channel_index"] for c in d["channels"]] == list(range(case["channels"]))
    assert d["block_frames"] == 3*case["rate"]
    assert d["complete_blocks_before_tail"] == expected["frames"]//d["block_frames"]
    assert d["trailing_frames"] == expected["frames"]%d["block_frames"]
    assert d["selected_block_target"] == round(.2*d["complete_blocks_before_tail"])
    for ch, oracle in zip(d["channels"], expected["dr_channels"]):
        scalar(ch["numeric_dr"], oracle, .1, (case["name"], "channel DR"), maxima)
    if expected["dr_overall"] is None:
        assert d["overall"]["value"] is None and d["trailing_frames"] == 0
    else:
        scalar(d["overall"], expected["dr_overall"], .1, (case["name"], "overall DR"), maxima)
    if d["overall"]["value"] is not None:
        assert d["overall_integer_label"] == "DR"+str(int(d["overall"]["value"]))
        reference_dr = float(expected["dr_overall"])
        if abs(reference_dr-round(reference_dr)) > 1e-4:
            assert d["overall_integer_label"] == "DR"+str(int(reference_dr)), (case["name"], "independent overall label")


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--output", type=Path, required=True)
    p.add_argument("--binary", type=Path, default=ROOT/"target/release/examples/read_tool_statistics.exe")
    p.add_argument("--check-saved", action="store_true", help="recheck saved reports; no decoding or oracle execution")
    a = p.parse_args()
    fixture = (ROOT/"tests/fixtures/tool_statistics_reference.json").read_bytes()
    if a.check_saved:
        assert (a.output/"expected.json").read_bytes() == fixture, "saved oracle drift"
    else:
        a.output.mkdir(parents=True, exist_ok=False)
        (a.output/"expected.json").write_bytes(fixture)
    validators = {}
    for key, filename in [("measurement", "analysis-report-0.18.0.schema.json"), ("byproducts", "byproducts-1.schema.json"), ("tool_statistics", "tool-statistics-1.schema.json")]:
        schema = json.loads((ROOT/"schemas"/filename).read_text(encoding="utf-8")); Draft202012Validator.check_schema(schema)
        validators[key] = Draft202012Validator(schema)
    maxima = dict(db=0., linear=0., dr=0., printed_volume=0.); records=[]
    for c in json.loads(fixture)["cases"]:
        path = a.output/(c["name"]+".wav"); audio=wav(deck(c), c)
        if a.check_saved:
            product=json.loads((a.output/(c["name"]+".json")).read_text(encoding="utf-8"))
        else:
            path.write_bytes(audio)
            command=[str(a.binary), str(path)]
            if c["prefix_frames"]: command.append(str(c["prefix_frames"]/c["rate"]))
            r = subprocess.run(command, capture_output=True, text=True, timeout=90)
            (a.output/(c["name"]+".json")).write_text(r.stdout, encoding="utf-8")
            (a.output/(c["name"]+".stderr.txt")).write_text(r.stderr, encoding="utf-8")
            assert r.returncode == 0, r.stderr
            product=json.loads(r.stdout)
        for key, validator in validators.items(): validator.validate(product[key])
        compare(product, c, maxima)
        if c["name"].startswith("conversion_"):
            assert product["tool_statistics"]["measurements"]["sox"]["conversion_clips"] == 4000
        assert path.read_bytes() == audio
        records.append(dict(name=c["name"], wav_sha256=hashlib.sha256(audio).hexdigest(), pcm_sha256=c["expected"]["pcm_sha256"]))
    baseline=json.loads((a.output/"unequal_sine.json").read_text(encoding="utf-8"))["tool_statistics"]
    rejected=[]
    for name, mutate in [("wrong_version", lambda x: x.update(statistics_version=2)),
                         ("missing_product", lambda x: x.update(measurements=None)),
                         ("unavailable_finite", lambda x: x["measurements"]["sox"]["fields"]["meanNorm"].update(availability="inapplicable")),
                         ("missing_astats_key", lambda x: x["measurements"]["astats"]["overall"].pop("noise_floor_dbfs")),
                         ("missing_audit_key", lambda x: x["measurements"]["legacy_loudness_profile"].pop("peak_count")),
                         ("missing_sox_key", lambda x: x["measurements"]["sox"]["fields"].pop("rmsDelta"))]:
        bad=copy.deepcopy(baseline); mutate(bad); assert not validators["tool_statistics"].is_valid(bad), name; rejected.append(name)
    summary=dict(result="PASS", cases=len(records), max_errors=maxima, rejected_schema_mutations=rejected,
                 binary_sha256=hashlib.sha256(a.binary.read_bytes()).hexdigest(), fixture_sha256=hashlib.sha256(fixture).hexdigest(), records=records)
    if not a.check_saved: (a.output/"summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps({k:v for k,v in summary.items() if k != "records"}))


if __name__ == "__main__":
    main()
