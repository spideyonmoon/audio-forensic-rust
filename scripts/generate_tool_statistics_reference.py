"""Freeze generated-only actual FFmpeg 7.1.1 / SoX 14.4.2 outputs before Rust.

No private files. Existing fixtures are never overwritten. Logs and audio are
ignored local receipts; the public fixture holds compact recipes and tool text.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import struct
import subprocess
import numpy as np
from generate_report_schema import ROOT


def cases():
    out = []
    def add(name, pattern, repeat, encoding="f64", rate=8000, extra=(), prefix=None):
        out.append(dict(name=name, rate=rate, encoding=encoding, channels=len(pattern[0]),
                        segments=[dict(pattern=pattern, repeat=repeat), *extra], prefix_frames=prefix))
    def seg(pattern, repeat):
        return dict(pattern=pattern, repeat=repeat)
    add("silence", [[0, 0]], 8000)
    add("constant", [[.25, -.125]], 8001)
    add("one_sample", [[.25]], 1)
    for n in (399, 400, 401):
        add(f"window_{n}", [[.25]], n)
    add("unit_trough_omitted", [[.25, 1]], 399)
    add("plateau_runs", [[.5], [.5], [0], [-.25], [-.25], [-.25], [0]], 1000)
    add("impulse", [[.75], *[[0]]*31], 251)
    add("zeros_signs", [[0], [.25], [0], [-.25], [0]], 1600)
    sine = [[float(.7*np.sin(2*np.pi*i/64)), float(.2*np.cos(2*np.pi*i/64))] for i in range(64)]
    add("unequal_sine", sine, 125)
    rng = np.random.default_rng(23003)
    add("seeded_noise", rng.uniform(-.5, .5, (257, 2)).tolist(), 32)
    add("odd_rate", [[.5], [-.125], [0]], 4000, rate=11025)
    add("high_rate", [[.5, -.25], [-.5, .25]], 9601, rate=384000)
    for enc in ("s16", "s24", "s32", "f32", "f64"):
        add("precision_"+enc, [[.5, -.25], [-1, 1-2**-15], [2**-16, -2**-16], [0, 0]], 2001, enc)
    for enc in ("f32", "f64"):
        add("conversion_"+enc, [[1.25, -1.5], [.2+2**-33, -.2-2**-33], [2**-32, -2**-32], [1, -1]], 2000, enc)
    for seconds in (2, 6, 9):
        add("dr_exact_"+str(seconds), [[.5], [-.5]], seconds*4000)
    add("dr_short_tail", [[.5], [-.5]], 12001)
    add("dr_varying_tail", [[.8, .6], *[[0, .2]]*15], 1500,
        extra=[seg([[v, .2], *[[0, 0]]*15], 1500) for v in (.6, .4, .2, .1)] + [seg([[.3, .1], *[[0, 0]]*15], 50)])
    add("dr_equal_bin_overshoot", [[.5], [-.5]], 48001)
    add("prefix", [[.8], *[[0]]*15], 6000, extra=[seg([[.1]], 1000)], prefix=72800)
    return out


def deck(case):
    return np.concatenate([np.tile(np.array(s["pattern"], dtype=np.float64), (s["repeat"], 1)) for s in case["segments"]])


def wav(data, case):
    enc, chans, rate = case["encoding"], case["channels"], case["rate"]
    bits = int(enc[1:]); width = bits//8
    if enc.startswith("s"):
        raw = b"".join((max(-(2**(bits-1)), min(2**(bits-1)-1, int(x*2**(bits-1)))) & (2**bits-1)).to_bytes(width, "little") for x in data.ravel())
    else:
        raw = data.astype("<f"+str(width)).tobytes()
    fmt = struct.pack("<HHIIHH", 1 if enc.startswith("s") else 3, chans, rate, rate*chans*width, chans*width, bits)
    body = b"WAVEfmt "+struct.pack("<I", 16)+fmt+b"data"+struct.pack("<I", len(raw))+raw
    return b"RIFF"+struct.pack("<I", len(body))+body


def astats(text):
    channels, overall, current = [], {}, None
    for line in text.splitlines():
        line = re.sub(r"^\[Parsed_astats_[^]]+\]\s*", "", line)
        if re.fullmatch(r"Channel: \d+", line):
            current = {}; channels.append(current)
        elif line == "Overall":
            current = overall
        elif current is not None and ": " in line:
            key, val = line.split(": ", 1); current[key] = val
    return dict(channels=channels, overall=overall)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--output", type=Path, required=True)
    a = p.parse_args(); a.output.mkdir(parents=True, exist_ok=False)
    ff = shutil.which("ffmpeg"); sox = r"C:\tools\sox\sox-14.4.2\sox.exe"
    versions = [subprocess.check_output([ff, "-version"], text=True).splitlines()[0], subprocess.check_output([sox, "--version"], text=True).strip()]
    assert "7.1.1" in versions[0] and "14.4.2" in versions[1], versions
    vectors = cases()
    for c in vectors:
        data = deck(c); path = a.output/(c["name"]+".wav"); path.write_bytes(wav(data, c))
        selected = data[:c["prefix_frames"]] if c["prefix_frames"] else data
        oracle = a.output/(c["name"]+"-selected.wav"); oracle.write_bytes(wav(selected, c))
        logs = {}
        for tool in ("astats", "drmeter", "sox"):
            cmd = [sox, str(oracle), "-n", "stat"] if tool == "sox" else [ff, "-hide_banner", "-v", "verbose", "-i", str(oracle), "-af", tool, "-f", "null", "-"]
            r = subprocess.run(cmd, capture_output=True, text=True, timeout=90); assert r.returncode == 0, r.stderr
            logs[tool] = r.stderr; (a.output/(c["name"]+"-"+tool+".txt")).write_text(r.stderr, encoding="utf-8")
        sox_values = dict(re.findall(r"^([^:\n]+):\s+([^\n]+)$", logs["sox"], re.M))
        dr_channels = re.findall(r"Channel \d+: DR: (\S+)", logs["drmeter"])
        dr_overall = re.findall(r"Overall DR: (\S+)", logs["drmeter"])
        match = re.search(r"DR:\s+([\d.]+)", logs["drmeter"])
        legacy_dr = "DR"+str(int(float(match[1]))) if match else "N/A"
        pcm_type = "s32le" if c["encoding"].startswith("s") else "f64le"
        pcm = subprocess.check_output([ff, "-v", "error", "-i", str(oracle), "-f", pcm_type, "-c:a", "pcm_"+pcm_type, "pipe:1"])
        c["expected"] = dict(frames=len(selected), pcm_sha256=hashlib.sha256(pcm).hexdigest(),
                              astats=astats(logs["astats"]), sox=sox_values, dr_channels=dr_channels,
                              dr_overall=dr_overall[0] if dr_overall else None, legacy_dr=legacy_dr)
    sources = {f.name: hashlib.sha256(f.read_bytes()).hexdigest() for f in (ROOT/"target/p03a-tool-stats/oracle").iterdir() if f.suffix in (".c", ".h")}
    artifact = dict(method="ffmpeg-7.1.1-sox-14.4.2-v1", versions=versions, source_sha256=sources, cases=vectors)
    with (ROOT/"tests/fixtures/tool_statistics_reference.json").open("x", encoding="utf-8") as f:
        json.dump(artifact, f, indent=2, allow_nan=False); f.write("\n")
    (a.output/"fixture.json").write_text(json.dumps(artifact, indent=2), encoding="utf-8")
    print(f"PASS: froze {len(vectors)} generated actual-tool cases")


if __name__ == "__main__":
    main()
