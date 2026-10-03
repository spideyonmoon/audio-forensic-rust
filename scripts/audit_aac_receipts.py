"""Audit existing private AAC receipts against pinned Python/SciPy arithmetic.

Optional development tooling; requires NumPy, SciPy, FFmpeg and the pinned
reference. It never changes audio, reference fixtures, detector gates or reports.
Matched-anchor results use the Rust applicability rule and exact f64 basis. The
unaltered Python method is also recorded using its ordinary float32 input path.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
REFERENCE = ROOT / "reference/audio-forensic"
COMMIT = "c6ecce2296256b516709d87088896d1be913908c"


def fingerprint(path):
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def reference_clean():
    head = subprocess.check_output(["git", "-C", str(REFERENCE), "rev-parse", "HEAD"], text=True).strip()
    dirty = subprocess.check_output(["git", "-C", str(REFERENCE), "status", "--porcelain"], text=True).strip()
    if head != COMMIT or dirty:
        raise ValueError("Reference must be clean and pinned")


def matched_search(engine, basis, probes, np, ndtr, ndtri):
    """SciPy MDCT and analytical gamma on the exact reported anchor windows."""
    edges = np.asarray(engine._SWB_LONG_44_48, dtype=int)
    counts = np.diff(edges)
    mu, sigma = counts / 12, np.sqrt(counts / 180)
    gamma = mu + sigma * ndtri(.01 + .99 * ndtr(-mu / sigma))
    phases = np.arange(0, 1024, 8)
    window = engine._kbd_window(2048)
    eligible_curves, flagged_curves, rms_errors = [], [], []
    for probe in probes:
        anchor = probe["anchor_frame"]
        assert probe["interval"] == {"start_frame": anchor - 512, "end_frame": anchor - 512 + 3064}
        rms_errors.append(abs(float(np.sqrt(np.mean(basis[anchor:anchor + 2048] ** 2))) - probe["anchor_rms"]))
        frames = basis[anchor - 512 + phases[:, None] + np.arange(2048)] * 32768
        coefficients = np.abs(engine._mdct_batch(frames, window))
        powers = np.add.reduceat(coefficients ** 2, edges[:-1], axis=1) / counts
        energetic = powers > np.maximum(1, powers.max(axis=1, keepdims=True) * 1e-6)
        maxima = np.maximum(np.maximum.reduceat(coefficients, edges[:-1], axis=1), 1e-12)
        deadzone = 16 + 4 / 3 * np.log2(maxima)
        powered = coefficients ** .75
        eligible, flagged = [], []
        for scale_index in range(8):
            scales = np.exp2(-3 * (.3 + .4 * scale_index / 7) * deadzone / 16)
            scaled = powered * np.repeat(scales, counts, axis=1)
            errors = np.add.reduceat((np.rint(scaled) - scaled) ** 2, edges[:-1], axis=1)
            populated = np.add.reduceat((scaled >= .5).astype(int), edges[:-1], axis=1) >= counts * .5
            usable = energetic & populated
            eligible.append(usable.sum(axis=1))
            flagged.append(((errors < gamma) & usable).sum(axis=1))
        eligible_curves.append(np.asarray(eligible))
        flagged_curves.append(np.asarray(flagged))
    if len(probes) < 4:
        return dict(score=None, phase_offset=None, scalefactor_index=None, eligible_probes=0,
                    selected_eligible=[None] * len(probes), selected_flagged=[None] * len(probes),
                    max_anchor_rms_error=max(rms_errors, default=0))
    eligible = np.asarray(eligible_curves)
    flagged = np.asarray(flagged_curves)
    supported = eligible >= 16
    means = np.where(supported, flagged / 49, 0).mean(axis=0)
    available = supported.sum(axis=0) >= 4
    if not available.any():
        return dict(score=None, phase_offset=None, scalefactor_index=None, eligible_probes=0,
                    selected_eligible=[None] * len(probes), selected_flagged=[None] * len(probes),
                    max_anchor_rms_error=max(rms_errors, default=0))
    scale, phase = np.unravel_index(np.argmax(np.where(available, means, -1)), means.shape)
    return dict(score=float(means[scale, phase]), phase_offset=int(phases[phase]), scalefactor_index=int(scale),
                eligible_probes=int(supported[:, scale, phase].sum()),
                selected_eligible=eligible[:, scale, phase].tolist(), selected_flagged=flagged[:, scale, phase].tolist(),
                max_anchor_rms_error=max(rms_errors, default=0))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--intake", type=Path, action="append", required=True)
    parser.add_argument("--id-pattern", required=True, help="Full-match regular expression for receipt IDs")
    parser.add_argument("--output", type=Path, required=True, help="New directory under corpus/local")
    args = parser.parse_args()
    output = args.output.resolve()
    if not output.is_relative_to((ROOT / "corpus/local").resolve()) or output == (ROOT / "corpus/local").resolve():
        raise ValueError("Evidence output must remain beneath corpus/local")
    pattern = re.compile(args.id_pattern)
    reference_clean()
    sys.path.insert(0, str(REFERENCE))
    import numpy as np
    import scipy
    from scipy.special import ndtr, ndtri
    from audio_forensic import SpectralEngine

    selected = {}
    repeated_identical_receipts = 0
    for intake in args.intake:
        summary = json.loads((intake / "summary.json").read_text(encoding="utf-8"))
        snapshot = json.loads((intake / "manifest-snapshot.json").read_text(encoding="utf-8"))
        entries = {e["id"]: e for e in snapshot["entries"]}
        for item in summary["records"]:
            if not pattern.fullmatch(item["id"]):
                continue
            candidate = (Path(snapshot["resolved_root"]) / entries[item["id"]]["file"],
                         intake / item["report"], entries[item["id"]])
            if item["id"] in selected:
                previous = selected[item["id"]]
                if (previous[0].resolve() != candidate[0].resolve()
                        or previous[2] != candidate[2]
                        or fingerprint(previous[1]) != fingerprint(candidate[1])):
                    raise ValueError("Selected ID has conflicting receipts")
                repeated_identical_receipts += 1
                continue
            selected[item["id"]] = candidate
    if not selected:
        raise ValueError("No receipt IDs matched")
    output.mkdir(parents=True, exist_ok=False)
    results = []
    for ident, (audio, report_path, entry) in selected.items():
        report_hash = fingerprint(report_path)
        report = json.loads(report_path.read_text(encoding="utf-8"))[0]
        assert report["status"] == "analyzed" and report["stream"]["integer_pcm"]
        assert report["ancestry_verdict"] == "INCONCLUSIVE" and report["evidence_index"] is None
        source_hash = fingerprint(audio)
        assert source_hash == entry["file_fingerprint"]["sha256"]
        rate, channels = report["stream"]["sample_rate"], report["stream"]["channels"]
        assert rate in (44100, 48000) and channels in (1, 2)
        limit = max(a["search_limit_frames"] for a in report["aac"])
        assert 0 < limit <= rate * 180
        command = ["ffmpeg", "-nostdin", "-xerror", "-v", "error", "-i", str(audio),
                   "-map", "0:a:0", "-vn", "-c:a", "pcm_s32le"]
        pcm_hash = subprocess.check_output(command + ["-f", "hash", "-hash", "sha256", "pipe:1"], timeout=600).decode().strip().split("=", 1)[1]
        assert pcm_hash == report["coverage"]["decoded_pcm_sha256"]
        raw = subprocess.check_output(command + ["-af", f"atrim=end_sample={limit}", "-f", "s32le", "pipe:1"], timeout=120)
        assert len(raw) == limit * channels * 4
        native = np.frombuffer(raw, dtype="<i4").reshape(-1, channels).astype(np.float64) / 2**31
        bases = {"mono": native[:, 0]} if channels == 1 else {
            "mid": (native[:, 0] + native[:, 1]) * .5, "side": (native[:, 0] - native[:, 1]) * .5}
        engine = SpectralEngine(audio, rate, channels=1)
        comparisons = []
        for actual in report["aac"]:
            basis = bases[actual["basis"]]
            expected = matched_search(engine, basis, actual["probes"], np, ndtr, ndtri)
            # The reference expects float32 input. Passing contiguous f64 would
            # alias its in-place scaling/energy scratch with the source array.
            direct_input = basis.astype(np.float32)
            input_hash = hashlib.sha256(direct_input.tobytes()).hexdigest()
            direct = engine._mdct_quant_error(direct_input, None)
            assert hashlib.sha256(direct_input.tobytes()).hexdigest() == input_hash
            checks = dict(score=(expected["score"] is None and actual["lattice_score"] is None)
                          or (expected["score"] is not None and actual["lattice_score"] is not None
                              and abs(expected["score"] - actual["lattice_score"]) <= 1e-12),
                          phase=expected["phase_offset"] == actual["phase_offset"],
                          scale=expected["scalefactor_index"] == actual["scalefactor_index"],
                          eligible=expected["eligible_probes"] == actual["eligible_probes"],
                          rms=expected["max_anchor_rms_error"] <= 1e-10,
                          votes=expected["selected_eligible"] == [p["selected_eligible_bands"] for p in actual["probes"]]
                          and expected["selected_flagged"] == [p["selected_flagged_bands"] for p in actual["probes"]])
            comparisons.append(dict(basis=actual["basis"], rust=actual, matched=expected,
                                    original_python_score_f32_basis=direct, checks=checks, passed=all(checks.values())))
            print(ident, actual["basis"], "Rust=", actual["lattice_score"], "Python=", direct,
                  "matched checks=", comparisons[-1]["passed"], flush=True)
        assert fingerprint(audio) == source_hash and fingerprint(report_path) == report_hash
        results.append(dict(id=ident, file=str(audio), file_sha256=source_hash, report_sha256=report_hash,
                            decoded_pcm_sha256=pcm_hash, comparisons=comparisons))
        (output / "results.json").write_text(json.dumps(results, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    reference_clean()
    summary = dict(reference_commit=COMMIT, numpy=np.__version__, scipy=scipy.__version__, files=len(results),
                   basis_comparisons=sum(len(r["comparisons"]) for r in results),
                   repeated_identical_receipts=repeated_identical_receipts,
                   passed=all(c["passed"] for r in results for c in r["comparisons"]),
                   scope="Matched reported anchors on exact f64 bases; original Python also run on its ordinary f32 input path; source histories remain unverified",
                   ancestry_accuracy_evaluated=False)
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary), flush=True)
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
