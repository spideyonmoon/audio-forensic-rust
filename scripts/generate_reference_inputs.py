"""Freeze P04c generated controls before Rust implementation; --check is read-only.

Optional development dependencies: pinned Python, NumPy/SciPy, FFmpeg. Runtime
Rust never imports these. Old fixtures/oracles are never written by this helper.
"""
import argparse
import hashlib
import json
from pathlib import Path
import random
import subprocess
import sys

import numpy as np
import scipy

ROOT = Path(__file__).resolve().parents[1]
REF = ROOT / "reference/audio-forensic"
COMMIT = "c6ecce2296256b516709d87088896d1be913908c"
if subprocess.check_output(["git", "-C", str(REF), "rev-parse", "HEAD"], text=True).strip() != COMMIT:
    raise SystemExit("Wrong reference commit")
if subprocess.check_output(["git", "-C", str(REF), "status", "--porcelain", "--untracked-files=no"], text=True).strip():
    raise SystemExit("Dirty reference")
sys.path.insert(0, str(REF))
from audio_forensic import SpectralEngine


def decoded(name):
    path = ROOT / "tests/fixtures" / name
    meta = json.loads(subprocess.check_output(["ffprobe", "-v", "error", "-select_streams", "a:0",
        "-show_entries", "stream=sample_rate,channels", "-of", "json", str(path)]))["streams"][0]
    rate, channels = int(meta["sample_rate"]), int(meta["channels"])
    raw = subprocess.check_output(["ffmpeg", "-nostdin", "-v", "error", "-i", str(path),
        "-c:a", "pcm_s32le", "-f", "s32le", "pipe:1"])
    return rate, (np.frombuffer(raw, dtype="<i4").reshape(-1, channels).astype(float) / 2**31).astype(np.float32), hashlib.sha256(raw).hexdigest()


def features(audio, rate):
    mid = audio[:, 0] if audio.shape[1] == 1 else (audio[:, 0] + audio[:, 1]) / np.float32(2)
    engine = SpectralEngine(Path("generated.wav"), rate, channels=audio.shape[1])
    mags, phase, _ = engine._compute_stft(mid)
    active = engine._active_frame_mask(mags)
    selected = mags[active]
    out = {"frames": len(mags), "active_frames": int(active.sum())}
    if len(selected) >= 4:
        bins = engine._freq_bins()
        cutoff = engine._cutoff_per_frame(selected, bins)
        p95 = float(np.percentile(cutoff, 95))
        out.update(cutoff_p95_hz=p95, cutoff_variance_hz2=float(np.var(cutoff)),
            sharpness_db_per_bin=engine._sharpness(selected, bins, p95),
            cliff_depth_db=engine._cliff_depth(selected, bins, p95),
            hf_magnitude_ratio=engine._hf_energy_ratio(selected, bins),
            entropy_bits=engine._spectral_entropy(selected),
            noise_above_cutoff_db=engine._noise_floor_above_cutoff(selected, bins, p95))
        avg, mode, ent = engine._aucdtect_features(selected, phase[active], bins)
        out["scatter"] = {"average_bound_hz": avg, "histogram_mode_hz": mode, "legacy_phase_entropy_bits": ent,
            "stride": len(selected)//2500+1 if len(selected)>2500 else 1,
            "sampled_frames": len(selected[::len(selected)//2500+1]) if len(selected)>2500 else len(selected)}
    return out


def generate():
    out = {"reference_commit": COMMIT, "numpy": np.__version__, "scipy": scipy.__version__,
        "reference_source_sha256": hashlib.sha256((REF/"audio_forensic.py").read_bytes()).hexdigest(),
        "cases": [], "offsets": [], "basis_vectors": [], "transforms": []}
    for name in ["noise16.wav", "wall24.wav", "activity16.wav", "rate_full96.flac"]:
        rate, audio, digest = decoded(name)
        out["cases"].append({"file": name, "rate": rate, "pcm_sha256": digest, "features": features(audio, rate)})
    for seconds in [17, 18, 134, 135, 149, 150, 539, 540, 600, 100000]:
        rate = 48000
        frames = seconds*rate
        count = int(min(36, max(9, frames/rate//15)))
        offsets = []
        if frames >= 2*rate*count:
            step = (frames-2*rate)//(count-1)
            offsets = sorted([i*step for i in range(count-2)] + [random.Random(42).randint(0,frames-2*rate),frames-2*rate])
        out["offsets"].append({"frames": frames, "rate": rate, "count": count, "offsets": offsets})
    for l, r in [(0.123456789, -0.123456788), (0.75, -0.75), (1.00000007, .00000006), (0.5, 0.5), (0, 0.7)]:
        left, right = np.float32(l), np.float32(r)
        mid, side = (left+right)/np.float32(2), (left-right)/np.float32(2)
        out["basis_vectors"].append({"left": l,"right": r,"mid": float(mid), "side": float(side),
            "reconstructed_left": float(mid)+float(side), "reconstructed_right": float(mid)-float(side)})
    # Deterministic nonstationary f32 spectrum: >2500 active rows, no PCM retention
    # needed by Rust. Independent source scatter/phase auditable from this formula.
    n = np.arange(2049, dtype=np.float32)
    rows = []
    phases = []
    for i in range(5003):
        mags = (0.01 + np.abs(np.sin(n*np.float32(.031)+np.float32(i*.073)))).astype(np.float32)
        mags[int(600+i%1100):] *= np.float32(1e-6)
        rows.append(mags)
        phases.append(np.full(2049-853, np.float32((i*.137)%(2*np.pi)-np.pi)))
    eng = SpectralEngine(Path("generated.wav"),48000,channels=1)
    a,b,c=eng._aucdtect_features(np.array(rows), np.array(phases), eng._freq_bins())
    out["long_scatter"] = {"active_frames":5003,"stride":3,"sampled_frames":1668,
        "average_bound_hz":a,"histogram_mode_hz":b,"legacy_phase_entropy_bits":c}
    for name, method in [("aac_256_44100.flac","aac"), ("grid_q6_44100.flac","vorbis")]:
        rate, audio, digest = decoded(name)
        sig=audio[:,0]
        eng=SpectralEngine(Path(name),rate,channels=2)
        for layout in ["mono","dual_mono","antiphase","right_only"]:
            if layout=="mono": mid,side=sig,None
            elif layout=="dual_mono": mid,side=sig,np.zeros_like(sig)
            elif layout=="antiphase": mid,side=np.zeros_like(sig),sig
            else: mid,side=sig/np.float32(2),-sig/np.float32(2)
            if method=="aac":
                scores=[eng._mdct_quant_error(mid,None)]
                if side is not None: scores.append(eng._mdct_quant_error(side,None))
                result={"scores":scores,"winner":("M","S")[int(len(scores)>1 and scores[1]>scores[0])]}
            else:
                score,support,tested,basis=eng._vorbis_grid(mid,side)
                result={"score":score,"support":support,"tested":tested,"winner":basis}
            out["transforms"].append({"file":name,"rate":rate,"method":method,"layout":layout,"pcm_sha256":digest,**result})
    return out


if __name__ == "__main__":
    parser=argparse.ArgumentParser(); parser.add_argument("--check",action="store_true"); args=parser.parse_args()
    path=ROOT/"tests/fixtures/reference_inputs.json"
    expected=generate()
    if args.check:
        assert json.loads(path.read_text(encoding="utf-8")) == expected, "Frozen oracle differs"
        print("Frozen P04c oracle matches pinned source")
    else:
        if path.exists(): raise SystemExit("Refuse to overwrite frozen P04c oracle")
        path.write_text(json.dumps(expected,indent=2,allow_nan=False)+"\n",encoding="utf-8")
        print("Frozen P04c oracle written")
