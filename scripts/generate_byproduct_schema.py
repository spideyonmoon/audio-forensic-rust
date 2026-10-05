"""Export the separate v1 byproduct schema; preserve measurement JSON."""
import argparse
import json
import re
from generate_report_schema import ROOT, build_schema


def schema():
    source = (ROOT / "src/model.rs").read_text(encoding="utf-8") + "\n" + (ROOT / "src/byproducts.rs").read_text(encoding="utf-8")
    assert "pub const BYPRODUCT_VERSION: u32 = 1;" in source
    definitions = build_schema(source)["$defs"]
    used = {}
    def include(name):
        if name in used:
            return
        used[name] = definitions[name]
        for child in re.findall(r'"\$ref": "#/\$defs/(\w+)"', json.dumps(definitions[name])):
            include(child)
    include("ByproductReport")
    props = used["ByproductReport"]["properties"]
    props["byproduct_version"] = {"const": 1}
    props["contract_version"] = {"const": 1}
    props["method_id"] = {"const": "python-reference-c6ecce2-byproducts-v1"}
    used["ReferenceSilence"]["properties"]["sections"]["maxItems"] = 1024
    used["NativeLevelDisplay"]["properties"]["channels"].update(minItems=1, maxItems=2)
    used["ReferenceNoiseFloor"]["properties"]["retained_block_limit"] = {"const": 864000}
    used["ByproductReport"]["allOf"] = [{
        "if": {"properties": {"status": {"const": "analyzed"}}},
        "then": {"properties": {key: {"$ref": f"#/$defs/{name}"} for key, name in
                                [("stream", "StreamInfo"), ("coverage", "Coverage"),
                                 ("reference", "ReferenceByproducts"), ("native_levels", "NativeLevelDisplay")]}},
        "else": {"properties": {key: {"type": "null"} for key in ["coverage", "reference", "native_levels"]}}
    }]
    used["DisplayValue"]["allOf"] = [{
        "if": {"properties": {"availability": {"const": "available"}}},
        "then": {"properties": {"value": {"type": "number"}, "display": {"type": "string"}, "reason": {"type": "null"}}},
        "else": {"properties": {"value": {"type": "null"}, "display": {"type": "null"}, "reason": {"type": "string"}}}
    }]
    return {"$schema": "https://json-schema.org/draft/2020-12/schema", "$id": "urn:audio-forensic:byproducts:1",
            "$ref": "#/$defs/ByproductReport", "$defs": used}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    artifact = schema()
    data = json.dumps(artifact, indent=2, allow_nan=False) + "\n"
    path = ROOT / "schemas/byproducts-1.schema.json"
    if args.check:
        assert path.read_text(encoding="utf-8") == data, "byproduct schema drift"
    else:
        with path.open("x", encoding="utf-8") as out:
            out.write(data)
    print(f"PASS: byproducts-1.schema.json, {len(artifact['$defs'])} definitions")


if __name__ == "__main__":
    main()
