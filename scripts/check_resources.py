"""Generate duration-scaled WAVs using constant memory and measure the Rust CLI.

Writes only generated files under ignored corpus/local/generated/resource-check.
Run after the release build. Requires FFmpeg for independent PCM hash checks.
"""
from array import array
import math
from pathlib import Path
import subprocess
import sys
import wave

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "corpus/local/generated/resource-check"
OUT.mkdir(parents=True, exist_ok=True)
rate = 48000
block = array("h", (value for i in range(rate) for value in (
    int(16000 * math.sin(2 * math.pi * 997 * i / rate)),
    int(12000 * math.sin(2 * math.pi * 1379 * i / rate)))))
if sys.byteorder != "little":
    block.byteswap()
data = block.tobytes()
for seconds in (10, 120, 600):
    with wave.open(str(OUT / f"synthetic-{seconds:04d}s.wav"), "wb") as handle:
        handle.setparams((2, 2, rate, 0, "NONE", "not compressed"))
        for _ in range(seconds):
            handle.writeframesraw(data)
subprocess.run([sys.executable, str(ROOT / "scripts/validate_local.py"), str(OUT),
                "--output", str(ROOT / "corpus/local/results/resource-check"),
                "--source-history", "generated sine tones; scripts/check_resources.py"], check=True)
