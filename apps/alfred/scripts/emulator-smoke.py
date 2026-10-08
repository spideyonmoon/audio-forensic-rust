"""CI workspace and generated-input JNI acceptance; not physical acceptance."""
from pathlib import Path
import subprocess
import time
import json
import xml.etree.ElementTree as ET
import importlib.util


def adb(*args):
    return subprocess.check_output(["adb", *args], text=True, stderr=subprocess.STDOUT)


output = Path("emulator-evidence")
output.mkdir(exist_ok=True)
try:
    adb("install", "-r", "apps/alfred/build/ci/x86_64/app-debug.apk")
    adb("logcat", "-c")
    result = adb("shell", "am", "start", "-W", "-n", "dev.alfred.workspace.debug/dev.alfred.workspace.MainActivity")
    (output / "start.txt").write_text(result)
    assert "Status: ok" in result, result
    deadline = time.monotonic() + 60
    picker_labels = set()
    while time.monotonic() < deadline:
        adb("shell", "uiautomator", "dump", "/sdcard/alfred-smoke.xml")
        adb("pull", "/sdcard/alfred-smoke.xml", str(output / "ui.xml"))
        tree = ET.parse(output / "ui.xml")
        texts = [node.attrib.get("text", "") for node in tree.iter()]
        picker_labels.update(text for text in texts if text in {"Choose documents", "Choose folder"})
        if len(picker_labels) == 2 and not (output / "workspace.xml").exists():
            (output / "workspace.xml").write_bytes((output / "ui.xml").read_bytes())
        assert not any("native_load_failed" in text or "unsupported_version" in text for text in texts), texts
        if any("Native host v1 loaded" in text for text in texts):
            assert len(picker_labels) == 2, picker_labels
            break
        # The bootstrap label is below the fold on the runner's small default AVD.
        bounds = tree.getroot().find("node").attrib["bounds"]
        width, height = map(int, bounds.split("][")[1].rstrip("]").split(","))
        adb("shell", "input", "swipe", str(width // 2), str(height * 4 // 5),
            str(width // 2), str(height // 3), "400")
        time.sleep(2)
    else:
        raise AssertionError("Workspace/JNI bootstrap did not become ready")
    (output / "device.txt").write_text(adb("shell", "getprop"))
    adb("shell", "am", "start", "-W", "-n", "dev.alfred.workspace.debug/dev.alfred.workspace.NativeSmokeActivity")
    deadline = time.monotonic() + 180
    while time.monotonic() < deadline:
        try:
            receipt = adb("shell", "run-as", "dev.alfred.workspace.debug", "cat", "files/native-smoke.json")
            parsed = json.loads(receipt)
        except (subprocess.CalledProcessError, json.JSONDecodeError):
            time.sleep(2)
            continue
        (output / "native-smoke.json").write_text(receipt)
        assert parsed["passed"], parsed
        break
    else:
        raise AssertionError("Generated-input JNI smoke timed out")
    adb("shell", "am", "start", "-W", "-n", "dev.alfred.workspace.debug/dev.alfred.workspace.InputSmokeActivity")
    deadline = time.monotonic() + 240
    while time.monotonic() < deadline:
        try:
            receipt = adb("shell", "run-as", "dev.alfred.workspace.debug", "cat", "files/input-smoke.json")
            parsed = json.loads(receipt)
        except (subprocess.CalledProcessError, json.JSONDecodeError):
            time.sleep(2)
            continue
        (output / "input-smoke.json").write_text(receipt)
        assert parsed["passed"], parsed
        break
    else:
        raise AssertionError("Shared input/provider smoke timed out")
    spec = importlib.util.spec_from_file_location("saf_ui", Path(__file__).with_name("saf-ui-smoke.py"))
    saf_ui = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(saf_ui)
    saf_ui.run(output)
finally:
    (output / "logcat.txt").write_text(adb("logcat", "-d"))
