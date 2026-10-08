"""Inspect all packaged native ELF objects, ZIP alignment and APK signature.

Build evidence only; runtime loading on 4/16 KiB Android is A03/A07.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import zipfile


def inspect(apk, sdk, abi="arm64-v8a", smoke_assets=False):
    machine = {"arm64-v8a": 183, "x86_64": 62}[abi]
    records = []
    with zipfile.ZipFile(apk) as archive, open(apk, "rb") as raw:
        if smoke_assets:
            fixtures = Path(__file__).resolve().parents[3] / "tests/fixtures"
            for name in ("noise16.wav", "noise16.flac", "clip_noise.wav", "alac/8000-16-1-tail.m4a"):
                assert archive.read(f"assets/{name}") == (fixtures / name).read_bytes(), name
        for info in archive.infolist():
            if not info.filename.endswith(".so"):
                continue
            assert info.filename.startswith(f"lib/{abi}/"), info.filename
            assert info.compress_type == zipfile.ZIP_STORED, "native libraries must be uncompressed"
            raw.seek(info.header_offset)
            header = raw.read(30)
            assert header[:4] == b"PK\x03\x04"
            name_bytes, extra_bytes = struct.unpack_from("<HH", header, 26)
            offset = info.header_offset + 30 + name_bytes + extra_bytes
            assert offset % 16384 == 0, (info.filename, offset)
            data = archive.read(info)
            assert data[:6] == b"\x7fELF\x02\x01", "expected little-endian ELF64"
            assert struct.unpack_from("<H", data, 18)[0] == machine, f"expected {abi}"
            phoff = struct.unpack_from("<Q", data, 32)[0]
            entsize, count = struct.unpack_from("<HH", data, 54)
            loads = []
            segments = []
            dynamic = None
            for index in range(count):
                kind, flags, file_offset, vaddr, _, filesz, memsz, align = struct.unpack_from(
                    "<IIQQQQQQ", data, phoff + index * entsize
                )
                segments.append((kind, file_offset, vaddr, filesz))
                if kind == 2:
                    dynamic = (file_offset, filesz)
                if kind == 1:
                    assert align >= 16384 and (vaddr - file_offset) % 16384 == 0
                    loads.append(dict(offset=file_offset, address=vaddr, alignment=align))
            assert loads, "no LOAD segments"
            assert dynamic, "missing dynamic table"
            entries = []
            for position in range(dynamic[0], sum(dynamic), 16):
                tag, value = struct.unpack_from("<QQ", data, position)
                if tag == 0:
                    break
                entries.append((tag, value))
            string_address = next(value for tag, value in entries if tag == 5)
            string_size = next(value for tag, value in entries if tag == 10)
            string_offset = next(offset + string_address - address for kind, offset, address, size in segments
                                 if kind == 1 and address <= string_address < address + size)
            strings = data[string_offset:string_offset + string_size]
            needed = []
            for tag, value in entries:
                if tag == 1:
                    end = strings.index(b"\0", value)
                    needed.append(strings[value:end].decode("ascii"))
            records.append(dict(path=info.filename, bytes=len(data), sha256=hashlib.sha256(data).hexdigest(),
                                zip_offset=offset, loads=loads, needed=needed))
    assert any(r["path"] == f"lib/{abi}/libalfred_native.so" for r in records), records
    assert all(r["path"].startswith(f"lib/{abi}/") for r in records), records
    system = {"libc.so", "libdl.so", "libm.so", "liblog.so", "libandroid.so"}
    packaged = {Path(record["path"]).name for record in records}
    for record in records:
        assert set(record["needed"]) <= system | packaged, record["needed"]
    tools = Path(sdk) / "build-tools/36.0.0"
    subprocess.run([str(tools / ("zipalign.exe" if os.name == "nt" else "zipalign")), "-c", "-P", "16", "-v", "4", str(apk)], check=True,
                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    subprocess.run([str(tools / ("apksigner.bat" if os.name == "nt" else "apksigner")), "verify", "--verbose", str(apk)], check=True)
    result = dict(apk_sha256=hashlib.sha256(Path(apk).read_bytes()).hexdigest(), native=records,
                  zipalign="passed", signature="passed", runtime="not tested")
    output = Path(apk).parent / "alignment.json"
    output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("apk", type=Path)
    parser.add_argument("--sdk", required=True)
    parser.add_argument("--abi", choices=("arm64-v8a", "x86_64"), default="arm64-v8a")
    parser.add_argument("--smoke-assets", action="store_true")
    args = parser.parse_args()
    inspect(args.apk, args.sdk, args.abi, args.smoke_assets)
