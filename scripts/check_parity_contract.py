"""Read-only P01 inventory checks; no reference execution or oracle updates."""

import ast
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parents[1]
BASELINE = "c6ecce2296256b516709d87088896d1be913908c"


def check() -> None:
    reference = ROOT / "reference/audio-forensic"
    sha = subprocess.check_output(
        ["git", "-C", str(reference), "rev-parse", "HEAD"], text=True
    ).strip()
    assert sha == BASELINE, ("reference baseline changed", sha)
    assert not subprocess.check_output(
        ["git", "-C", str(reference), "status", "--porcelain"], text=True
    ).strip(), "reference checkout is not clean"
    tree = ast.parse((reference / "audio_forensic.py").read_text(encoding="utf-8"))
    expected = {}
    for node in tree.body:
        if isinstance(node, ast.FunctionDef):
            expected["method:" + node.name] = node.lineno
        elif isinstance(node, ast.ClassDef):
            dataclass = any(
                isinstance(d, ast.Name) and d.id == "dataclass"
                or isinstance(d, ast.Call)
                and isinstance(d.func, ast.Name) and d.func.id == "dataclass"
                for d in node.decorator_list
            )
            for member in node.body:
                if dataclass and isinstance(member, ast.AnnAssign):
                    expected[f"field:{node.name}.{member.target.id}"] = member.lineno
                elif isinstance(member, ast.FunctionDef):
                    expected[f"method:{node.name}.{member.name}"] = member.lineno
    parity = (ROOT / "PYTHON_PARITY.md").read_text(encoding="utf-8")
    rows = re.findall(
        r"^\| `((?:field|method):[^`]+)` \| `audio_forensic.py:(\d+)` \| (G\d+) \|$",
        parity, re.M,
    )
    actual = {name: int(line) for name, line, _ in rows}
    assert len(actual) == len(rows), "duplicate inventory key"
    assert actual == expected, {
        "missing": sorted(expected.keys() - actual.keys()),
        "extra": sorted(actual.keys() - expected.keys()),
        "wrong_lines": [k for k in actual.keys() & expected.keys()
                        if actual[k] != expected[k]],
    }
    groups = re.findall(r"^\| (G\d+) \| (.+)$", parity, re.M)
    assert len(groups) == 30 and len(dict(groups)) == 30
    assert {group for _, _, group in rows} <= dict(groups).keys()
    cards = (ROOT / "ROADMAP_TASKS.md").read_text(encoding="utf-8")
    task_ids = set(re.findall(r"^## ([A-Z]\d+[a-z]?)\b", cards, re.M))
    for group, row in groups:
        cells = row.split(" | ")
        assert len(cells) == 3, (group, "bad mapping columns")
        assert ";" in cells[-1], (group, "missing acceptance oracle")
        owners = re.findall(r"\b(?:P|F|A)\d+[a-z]?\b", cells[-1])
        assert owners and set(owners) <= task_ids, (group, owners)
    rules = re.findall(r"^\| (R\d+) \|", parity, re.M)
    assert rules == [f"R{i:02}" for i in range(1, 35)]
    roadmap = (ROOT / "ROADMAP.md").read_text(encoding="utf-8")
    board = re.findall(
        r"^\| ((?:P|F|A)\d+[a-z]?) \| [^|]+ \| [^|]+ \| ([^|]+) \| ([^|]+) \|$",
        roadmap, re.M,
    )
    graph = {}
    for task, dependencies, _ in board:
        assert task in task_ids and task not in graph, task
        graph[task] = re.findall(r"\b(?:P|F|A)\d+[a-z]?\b", dependencies)
    # Initial launch excludes owner-deferred F02; its future scope stays mapped.
    # The board abbreviates this gate as P02–P07 plus required child tasks.
    graph["P08"] = ["P02", "P03", "P03a", "P04a", "P04b", "P04c",
                    "P05", "P06", "P07", "F01"]
    def visit(task, path):
        assert task in graph, ("missing board dependency", task)
        assert task not in path, ("dependency cycle", path, task)
        for dependency in graph[task]:
            visit(dependency, path + [task])
    for task in graph:
        visit(task, [])
    for filename in ("PYTHON_PARITY.md", "RELEASE_CONTRACT.md", "ROADMAP.md",
                     "ROADMAP_TASKS.md", "task-results/P01.md"):
        content = (ROOT / filename).read_text(encoding="utf-8")
        for link in re.findall(r"\]\(([^)]+)\)", content):
            if "://" in link or link.startswith("#"):
                continue
            target = link.split("#")[0]
            assert ((ROOT / filename).parent / target).exists(), (filename, link)
    print(f"PASS: {sum(k.startswith('field:') for k in actual)} fields, "
          f"{sum(k.startswith('method:') for k in actual)} methods, "
          "30 mapped groups, 34 rule rows, task owners, acyclic board and local links; "
          "reference pinned and clean. This is inventory validation, not DSP parity.")


if __name__ == "__main__":
    check()
