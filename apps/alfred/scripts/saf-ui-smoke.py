"""Exercise real DocumentsUI read-grant offers against debug generated inputs."""
import re
import time
import xml.etree.ElementTree as ET
from pathlib import Path
import subprocess


def adb(*args):
    return subprocess.check_output(["adb", *args], text=True, stderr=subprocess.STDOUT)


def run(output: Path):
    def nodes():
        adb("shell", "uiautomator", "dump", "/sdcard/alfred-saf.xml")
        xml = adb("shell", "cat", "/sdcard/alfred-saf.xml")
        (output / "saf-current.xml").write_text(xml)
        return list(ET.fromstring(xml).iter("node"))

    def match(label, timeout=30):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            for node in nodes():
                if node.attrib.get("text", "").casefold() == label.casefold() or node.attrib.get("content-desc", "").casefold() == label.casefold():
                    return node
            time.sleep(1)
        raise AssertionError(f"SAF UI label missing: {label}")

    def tap(node, long=False):
        left, top, right, bottom = map(int, re.findall(r"\d+", node.attrib["bounds"]))
        x, y = str((left + right) // 2), str((top + bottom) // 2)
        if long:
            adb("shell", "input", "swipe", x, y, x, y, "1000")
        else:
            adb("shell", "input", "tap", x, y)

    def click(label):
        tap(match(label))

    def top():
        for _ in range(3):
            adb("shell", "input", "swipe", "300", "300", "300", "1100", "250")

    def root():
        # Roots update asynchronously on a freshly booted emulator. Verify the
        # destination, and reacquire coordinates if the drawer reordered while
        # the automation was tapping a row.
        for attempt in range(4):
            current = nodes()
            if any(n.attrib.get("text", "") == "same.flac" for n in current):
                return
            drawer = next((n for n in current if n.attrib.get("content-desc", "") in {"Show roots", "Open navigation drawer"}), None)
            if drawer is not None:
                tap(drawer)
            match("Alfred generated inputs")
            time.sleep(2)  # Let roots discovery finish before using row bounds.
            provider = match("Alfred generated inputs")
            (output / f"saf-roots-{attempt}.xml").write_bytes((output / "saf-current.xml").read_bytes())
            tap(provider)
            try:
                match("same.flac", timeout=10)
                return
            except AssertionError:
                pass
        raise AssertionError("Generated provider root did not open")

    def checked(count):
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            current = nodes()
            texts = [n.attrib.get("text", "") for n in current]
            if f"{count} selected documents" in texts and any("Input checks complete" in text for text in texts):
                assert any("persisted" in text for text in texts), texts
                return
            time.sleep(1)
        raise AssertionError(f"SAF acquisition did not finish for {count} inputs")

    adb("shell", "am", "force-stop", "dev.alfred.workspace.debug")
    adb("shell", "am", "start", "-W", "-n", "dev.alfred.workspace.debug/dev.alfred.workspace.MainActivity")
    click("Choose one document")
    root()
    click("same.flac")
    checked(1)
    (output / "saf-single.xml").write_bytes((output / "saf-current.xml").read_bytes())
    top()
    click("Choose documents")
    root()
    tap(match("same.flac"), long=True)
    click("same.wav")
    # DocumentsUI exposes the confirmation affordance as text or description.
    click("Select")
    checked(2)
    (output / "saf-multiple.xml").write_bytes((output / "saf-current.xml").read_bytes())
    top()
    click("Choose folder")
    root()
    click("Use this folder")
    click("Allow")
    checked(3)
    (output / "saf-folder.xml").write_bytes((output / "saf-current.xml").read_bytes())


if __name__ == "__main__":
    run(Path("emulator-evidence"))
