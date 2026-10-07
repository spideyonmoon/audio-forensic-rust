#!/usr/bin/env bash
set -euo pipefail
app_root="$(cd "$(dirname "$0")/.." && pwd)"
repo_root="$(cd "$app_root/../.." && pwd)"
abi="${1:-arm64-v8a}"
case "$abi" in
  arm64-v8a) target=aarch64-linux-android; compiler=aarch64-linux-android30-clang ;;
  x86_64) target=x86_64-linux-android; compiler=x86_64-linux-android30-clang ;;
  *) echo "Unsupported ABI: $abi" >&2; exit 1 ;;
esac
sdk="${ANDROID_HOME:?ANDROID_HOME is required}"
ndk="$sdk/ndk/30.0.16248370"
linker="$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/$compiler"
test -x "$linker"
export RUSTUP_TOOLCHAIN=1.85.0
export "CARGO_TARGET_$(echo "$target" | tr '[:lower:]-' '[:upper:]_')_LINKER=$linker"
cd "$app_root/native"
cargo build --release --locked --target "$target"
mkdir -p "$app_root/app/build/generated/jniLibs/$abi"
cp "target/$target/release/libalfred_native.so" "$app_root/app/build/generated/jniLibs/$abi/"
key="$repo_root/.tools/alfred/debug.keystore"
if [[ ! -f "$key" ]]; then
  mkdir -p "$(dirname "$key")"
  keytool -genkeypair -keystore "$key" -storepass android -alias androiddebugkey -keypass android -dname 'CN=Android Debug,O=Android,C=US' -keyalg RSA -keysize 2048 -validity 10000
fi
cd "$app_root"
bash gradlew --no-daemon --console=plain "-PnativeAbi=$abi" :app:assembleDebug :app:lintDebug
python3 scripts/verify_apk.py app/build/outputs/apk/debug/app-debug.apk --sdk "$sdk" --abi "$abi"
mkdir -p "build/ci/$abi"
cp app/build/outputs/apk/debug/{app-debug.apk,alignment.json} "build/ci/$abi/"
python3 - "$abi" "$target" <<'PY'
import json, subprocess, sys
from pathlib import Path
abi, target = sys.argv[1:]
receipt = dict(revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
               native_api=30, compile_target_api=36, ndk='30.0.16248370',
               rust=subprocess.check_output(['rustc','--version'],text=True).strip(),
               abi=abi, target=target, panic='unwind', runtime='not tested')
Path(f'build/ci/{abi}/build.json').write_text(json.dumps(receipt,indent=2)+'\n')
PY
