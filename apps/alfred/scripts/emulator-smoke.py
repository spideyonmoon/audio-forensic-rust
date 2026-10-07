"""CI scaffold install/start/JNI smoke only; no analysis or device acceptance."""
from pathlib import Path
import subprocess
import time
import xml.etree.ElementTree as ET


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
    while time.monotonic() < deadline:
        adb("shell", "uiautomator", "dump", "/sdcard/alfred-smoke.xml")
        adb("pull", "/sdcard/alfred-smoke.xml", str(output / "ui.xml"))
        texts = [node.attrib.get("text", "") for node in ET.parse(output / "ui.xml").iter()]
        if any("Native host v1 loaded" in text for text in texts):
            assert "Choose documents" in texts and "Choose folder" in texts, texts
            break
        time.sleep(2)
    else:
        raise AssertionError("Workspace/JNI bootstrap did not become ready")
    (output / "device.txt").write_text(adb("shell", "getprop"))
finally:
    (output / "logcat.txt").write_text(adb("logcat", "-d"))
