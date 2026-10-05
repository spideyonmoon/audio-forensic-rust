"""Export separate metadata/audit schemas; never modify the measurement schema."""
import argparse
import json
from pathlib import Path
import re
from generate_report_schema import build_schema

ROOT = Path(__file__).resolve().parents[1]


def schemas():
    source = (ROOT / "src/model.rs").read_text(encoding="utf-8") + "\n" + (ROOT / "src/metadata.rs").read_text(encoding="utf-8")
    definitions = build_schema(source)["$defs"]
    assert re.search(r"pub const METADATA_VERSION: u32 = 1;", source)
    for name, identity, filename in [("MetadataReport", "metadata", "metadata-1.schema.json"),
                                      ("ReplayGainAudit", "replaygain-audit", "replaygain-audit-1.schema.json")]:
        used = {}
        def include(key):
            if key in used:
                return
            used[key] = definitions[key]
            for child in re.findall(r'"\$ref": "#/\$defs/(\w+)"', json.dumps(definitions[key])):
                include(child)
        include(name)
        if name == "MetadataReport":
            props = used[name]["properties"]
            props["metadata_version"] = {"const": 1}
            props["contract_version"] = {"const": 1}
            props["entries"]["maxItems"] = 1024
            # Byte limits/accounting and index binding are checked semantically.
            used[name]["allOf"] = [{"if": {"properties": {"status": {"const": "available"}}},
                                     "then": {"properties": {"technical": {"$ref": "#/$defs/TechnicalMetadata"}}}}]
        else:
            props = used[name]["properties"]
            props["audit_version"] = {"const": 1}
            props["method_id"] = {"const": "python-reference-c6ecce2-replaygain-v1"}
        yield filename, {"$schema": "https://json-schema.org/draft/2020-12/schema",
                         "$id": f"urn:audio-forensic:{identity}:1", "$ref": f"#/$defs/{name}", "$defs": used}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for filename, schema in schemas():
        path = ROOT / "schemas" / filename
        data = json.dumps(schema, indent=2, allow_nan=False) + "\n"
        if args.check:
            assert path.read_text(encoding="utf-8") == data, f"schema drift: {filename}"
        else:
            with path.open("x", encoding="utf-8") as out:
                out.write(data)
        print(f"PASS: {filename}, {len(schema['$defs'])} definitions")


if __name__ == "__main__":
    main()
