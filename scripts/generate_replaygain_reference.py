"""Freeze the pinned pure ReplayGain oracle, refusing to overwrite any fixture."""
import ast
import argparse
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "c6ecce2296256b516709d87088896d1be913908c"


def pinned_function(name, namespace):
    reference = ROOT / "reference/audio-forensic"
    assert subprocess.check_output(["git", "-C", str(reference), "rev-parse", "HEAD"], text=True).strip() == BASELINE
    assert not subprocess.check_output(["git", "-C", str(reference), "status", "--porcelain"], text=True).strip()
    tree = ast.parse((reference / "audio_forensic.py").read_text(encoding="utf-8"))
    function = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == name)
    for arg in function.args.args:
        arg.annotation = None
    function.returns = None
    exec(compile(ast.Module(body=[function], type_ignores=[]), str(reference / "audio_forensic.py"), "exec"), namespace)
    return namespace[name]


def fixture():
    oracle = pinned_function("audit_replaygain", {"re": re})
    vectors = [("", ""), ("", "-18"), ("0 dB", ""), ("+2.00 dB", "-16"),
               ("-2.00 dB", "-20"), ("0 dB", "-18.999"), ("0 dB", "-19"),
               ("0 dB", "-20.999"), ("0 dB", "-21"), ("1e2 dB", "-6"),
               ("++2dB", "-16"), ("-1.25", "-18"), ("-1.35", "-18"),
               ("broken", "-18"), (".--", "-18"), ("+4 dB", "broken"),
               ("  +2 dB  ", "-16"), ("\u0662 dB", "-16"), ("\uff12 dB", "-16"),
               ("0", "-18.125"), ("0", "-18.375")]
    return {"reference_commit": BASELINE,
            "oracle": "pinned pure audit_replaygain AST; no file/extractor execution",
            "cases": [{"stored": s, "lufs": l,
                       "expected": list(oracle(type("Tags", (), {"replaygain_track_gain": s})(), l))}
                      for s, l in vectors]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=Path, default=ROOT / "tests/fixtures/replaygain_reference.json")
    args = parser.parse_args()
    data = fixture()
    if args.check:
        assert json.loads(args.output.read_text(encoding="utf-8")) == data, "frozen oracle differs"
    else:
        with args.output.open("x", encoding="utf-8") as out:
            out.write(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    print(f"PASS: {len(data['cases'])} pinned ReplayGain vectors; no overwritten oracle")


if __name__ == "__main__":
    main()
