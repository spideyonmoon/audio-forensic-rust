"""Run the release CLI on local WAV/FLAC files and compare decoded PCM with FFmpeg.

Outputs remain in ignored corpus/local/results. This measures decoder agreement,
runtime and process memory, not detector accuracy or source authenticity.
"""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


class MemoryCounters(ctypes.Structure):
    _fields_ = [("cb", ctypes.c_ulong), ("PageFaultCount", ctypes.c_ulong)] + [
        (name, ctypes.c_size_t) for name in (
            "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage",
            "QuotaPagedPoolUsage", "QuotaPeakNonPagedPoolUsage", "QuotaNonPagedPoolUsage",
            "PagefileUsage", "PeakPagefileUsage")]


def working_set_peak(process):
    if os.name != "nt":
        return None
    counters = MemoryCounters()
    counters.cb = ctypes.sizeof(counters)
    get_info = ctypes.windll.psapi.GetProcessMemoryInfo
    get_info.argtypes = [ctypes.c_void_p, ctypes.POINTER(MemoryCounters), ctypes.c_ulong]
    get_info.restype = ctypes.c_int
    if get_info(int(process._handle), ctypes.byref(counters), counters.cb):
        return counters.PeakWorkingSetSize
    return None


def file_hash(path):
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="A file or a directory of WAV/FLAC files")
    parser.add_argument("--max-seconds", type=float)
    parser.add_argument("--source-history", default="unverified; user describes collection as CD/vinyl rips")
    parser.add_argument("--output", type=Path, default=ROOT / "corpus/local/results/local-validation")
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release" / ("audio-forensic.exe" if os.name == "nt" else "audio-forensic"))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    files = [args.input] if args.input.is_file() else sorted(p for p in args.input.iterdir() if p.suffix.lower() in (".wav", ".flac") and p.is_file())
    if not files:
        raise SystemExit("No WAV/FLAC inputs found")
    records = []
    for index, path in enumerate(files):
        report_path = args.output / f"{index:03d}.json"
        command = [str(args.binary), "--json"]
        if args.max_seconds is not None:
            command += ["--max-seconds", str(args.max_seconds)]
        command.append(str(path.resolve()))
        started = time.perf_counter()
        peak = None
        with report_path.open("wb") as out, (args.output / f"{index:03d}.log").open("wb") as err:
            process = subprocess.Popen(command, stdout=out, stderr=err)
            while process.poll() is None:
                observed = working_set_peak(process)
                if observed is not None:
                    peak = max(peak or 0, observed)
                time.sleep(0.01)
            process.wait()
        elapsed = time.perf_counter() - started
        report = json.loads(report_path.read_text(encoding="utf-8"))[0]
        reference_hash = None
        hash_matches = None
        ffmpeg_error = None
        if report["stream"] is not None:
            encoding = "pcm_s32le" if report["stream"]["integer_pcm"] else "pcm_f64le"
            command = ["ffmpeg", "-nostdin", "-xerror", "-v", "error", "-i", str(path), "-map", "0:a:0", "-vn"]
            if args.max_seconds is not None:
                command += ["-t", str(args.max_seconds)]
            command += ["-c:a", encoding, "-f", "hash", "-hash", "sha256", "pipe:1"]
            result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=600)
            if result.returncode == 0:
                reference_hash = result.stdout.strip().split("=", 1)[1]
                if report["coverage"] is not None:
                    hash_matches = reference_hash == report["coverage"]["decoded_pcm_sha256"]
            else:
                ffmpeg_error = result.stderr
        record = {"file": path.name, "source_file_sha256": file_hash(path),
            "source_history": args.source_history,
            "use": "decoding and robustness only; not accuracy ground truth",
            "status": report["status"], "exit_code": process.returncode, "seconds": round(elapsed, 4),
            "peak_working_set_bytes": peak, "sample_rate": (report.get("stream") or {}).get("sample_rate"),
            "bits_per_sample": (report.get("stream") or {}).get("bits_per_sample"),
            "coverage": report["coverage"], "ffmpeg_pcm_sha256": reference_hash,
            "pcm_hash_matches_ffmpeg": hash_matches, "ffmpeg_error": ffmpeg_error,
            "diagnostics": report["diagnostics"]}
        records.append(record)
        (args.output / "summary.json").write_text(json.dumps(records, indent=2) + "\n", encoding="utf-8")
        print(f"[{index+1}/{len(files)}] {path.name}: {report['status']}, PCM match={hash_matches}, {elapsed:.2f}s", flush=True)
    ok = all(r["status"] == "analyzed" and r["pcm_hash_matches_ffmpeg"] is True for r in records)
    print(f"{'PASS' if ok else 'FAIL'}: {len(records)} files; results: {args.output}")
    raise SystemExit(0 if ok else 1)


if __name__ == "__main__":
    main()
