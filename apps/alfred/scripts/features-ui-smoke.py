"""Generated-only saved Forensics UI, using the actual Compose navigation."""
import json
import re
import subprocess
import time
import xml.etree.ElementTree as ET


def adb(*args):
    return subprocess.check_output(["adb", *args], text=True, stderr=subprocess.STDOUT)


def run(output, receipt):
    package = "dev.alfred.workspace.debug"
    adb("shell", "am", "force-stop", package)
    adb("shell", "am", "start", "-W", "-n", package + "/dev.alfred.workspace.MainActivity")

    def nodes():
        adb("shell", "uiautomator", "dump", "/sdcard/alfred-feature.xml")
        xml = adb("shell", "cat", "/sdcard/alfred-feature.xml")
        (output / "feature-current.xml").write_text(xml)
        return list(ET.fromstring(xml).iter("node"))

    def swipe(up):
        current = nodes()
        _, _, width, height = map(int, re.findall(r"\d+", current[0].attrib["bounds"]))
        a, b = (height * 4 // 5, height // 3) if up else (height // 3, height * 4 // 5)
        adb("shell", "input", "swipe", str(width // 2), str(a), str(width // 2), str(b), "250")

    def find(text, scroll=True, up=True):
        for _ in range(20):
            found = next((n for n in nodes() if text in n.attrib.get("text", "")), None)
            if found is not None:
                return found
            if scroll:
                swipe(up)
            time.sleep(.5)
        raise AssertionError("Feature UI missing: " + text)

    def click(text, up=True):
        node = find(text, up=up)
        left, top, right, bottom = map(int, re.findall(r"\d+", node.attrib["bounds"]))
        adb("shell", "input", "tap", str((left + right) // 2), str((top + bottom) // 2))
        time.sleep(.5)

    click("Forensics history")
    find("Scores are uncalibrated")
    click("Open completed · " + receipt["generated_attempt"])
    find("Generated abstention")
    click("unknown_future_integer:")
    find("18446744073709551615")
    # Navigation back up to the object precedes choosing a sibling field.
    click("Up one field", up=False)
    click("unknown_null:")
    find("null · unavailable (not zero)")
    (output / "feature-ui-smoke.json").write_text(json.dumps({"passed": True, "checks": 5,
        "history_after_restart": True, "api": int(adb("shell", "getprop", "ro.build.version.sdk"))}))
    adb("shell", "input", "keyevent", "4")
    viewer = receipt.get("viewer")
    if viewer:
        click("Spectrogram history")
        find("Independent bounded P06", up=False)
        click("Open completed · " + viewer["preview_attempt"])
        find("Measurement: analyzed")
        click("3: png")
        find("Scaled viewing preview")
        image = next((n for n in nodes() if n.attrib.get("content-desc") == "Calibrated Rust spectrogram PNG"), None)
        assert image is not None, "Rust PNG was not displayed"
        adb("shell", "screencap", "-p", "/sdcard/alfred-spectrogram.png")
        adb("pull", "/sdcard/alfred-spectrogram.png", str(output / "spectrogram-display.png"))
        click("Open completed · " + viewer["old_attempt"], up=False)
        find("Presentation/encoded bitrate unavailable")
        adb("shell", "input", "keyevent", "4")
        click("Compare history")
        find("Compare saved products", up=False)
        click("Open completed · " + viewer["compare_attempt"])
        click("3: comparison")
        find("Comparison: available")
        find("Winner reported by Rust")
        (output / "viewer-compare-ui.json").write_text(json.dumps({"passed": True,
            "independent_routes_after_restart": True, "Rust_PNG_display": True,
            "saved_product_selection_visible": True, "comparison_history": True}))
        adb("shell", "input", "keyevent", "4")
