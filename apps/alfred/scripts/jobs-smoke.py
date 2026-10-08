"""Generated-only A05 real service/rotation/kill/storage checks, APIs 30-36."""
import json
import subprocess
import time
from pathlib import Path

PACKAGE = "dev.alfred.workspace.debug"
ACTIVITY = PACKAGE + "/dev.alfred.workspace.JobSmokeActivity"


def adb(*args):
    return subprocess.check_output(["adb", *args], text=True, stderr=subprocess.STDOUT)


def receipt(name, predicate=lambda value: value.get("passed"), seconds=60):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        try:
            value = json.loads(adb("shell", "run-as", PACKAGE, "cat", "files/" + name))
            if value.get("passed") is False:
                raise AssertionError(value)
            if predicate(value):
                return value
        except (subprocess.CalledProcessError, json.JSONDecodeError):
            pass
        # A target receipt may be absent because the Activity recorded its real
        # failure in the aggregate receipt. Do not hide that behind a timeout.
        try:
            aggregate = json.loads(adb("shell", "run-as", PACKAGE, "cat", "files/job-smoke.json"))
            if aggregate.get("passed") is False:
                raise AssertionError(aggregate)
        except (subprocess.CalledProcessError, json.JSONDecodeError):
            pass
        time.sleep(0.25)
    raise AssertionError("Bounded receipt wait expired: " + name)


def start(mode, committed=False):
    adb("shell", "run-as", PACKAGE, "rm", "-f", "files/job-smoke.json", "files/job-marker.json",
        "files/job-recovery.json", "files/job-denied.json", "files/job-timeout.json")
    adb("shell", "am", "start", "-W", "-n", ACTIVITY, "--es", "mode", mode,
        "--ez", "committed", str(committed).lower())


def run(output):
    output = Path(output)
    api = int(adb("shell", "getprop", "ro.build.version.sdk").strip())
    evidence = {"api": api, "checks": []}
    if api >= 33:
        adb("shell", "pm", "revoke", PACKAGE, "android.permission.POST_NOTIFICATIONS")
    start("controls")
    controls = receipt("job-controls.json")
    receipt("job-smoke.json")
    evidence["checks"].append({"controls": controls, "notifications": "denied" if api >= 33 else "not_applicable"})

    start("background")
    marker = receipt("job-marker.json", lambda value: value["phase"] == "background")
    adb("shell", "am", "start", "-W", "-n", PACKAGE + "/dev.alfred.workspace.MainActivity")
    adb("shell", "settings", "put", "system", "accelerometer_rotation", "0")
    adb("shell", "settings", "put", "system", "user_rotation", "1")
    adb("shell", "input", "keyevent", "KEYCODE_HOME")
    adb("shell", "input", "keyevent", "KEYCODE_SLEEP")
    completed = receipt("job-marker.json", lambda value: value["phase"] == "completed")
    assert completed["attempt_id"] == marker["attempt_id"], (marker, completed)
    adb("shell", "input", "keyevent", "KEYCODE_WAKEUP")
    adb("shell", "wm", "dismiss-keyguard")
    adb("shell", "settings", "put", "system", "user_rotation", "0")
    if api >= 31:
        adb("shell", "wm", "user-rotation", "lock", "0")
    evidence["checks"].append({"rotation_background_screen_off": "completed", "attempt": completed["attempt_id"]})

    for phase in ["copy", "native", "finalizing", "completed"]:
        start(phase)
        marker = receipt("job-marker.json", lambda value: value["phase"] == phase)
        adb("shell", "am", "force-stop", PACKAGE)
        start("recover", committed=phase == "completed")
        recovery = receipt("job-recovery.json")
        receipt("job-smoke.json")
        evidence["checks"].append({"kill_phase": phase, "recovery": recovery})

    start("denied")
    adb("shell", "input", "keyevent", "KEYCODE_HOME")
    evidence["checks"].append({"background_start": receipt("job-denied.json")})
    receipt("job-smoke.json")

    if api >= 35:
        try:
            adb("shell", "device_config", "put", "activity_manager", "media_processing_fgs_timeout_duration", "5000")
            start("timeout")
            receipt("job-marker.json", lambda value: value["phase"] == "timeout")
            adb("shell", "input", "keyevent", "KEYCODE_HOME")
            timeout = receipt("job-timeout.json", seconds=60)
            receipt("job-smoke.json")
            services = adb("shell", "dumpsys", "activity", "services", PACKAGE)
            assert "dev.alfred.shared.JobService" not in services, services
            evidence["checks"].append({"platform_media_processing_timeout": timeout})
        finally:
            adb("shell", "device_config", "delete", "activity_manager", "media_processing_fgs_timeout_duration")
    else:
        evidence["checks"].append({"platform_media_processing_timeout": "not_applicable_before_api35"})
    (output / "jobs-smoke.json").write_text(json.dumps(evidence, indent=2))
    # Denial was exercised above. Restore test-environment state for the existing
    # picker suite, whose generated row should not be pushed below the fold.
    if api >= 33:
        adb("shell", "pm", "grant", PACKAGE, "android.permission.POST_NOTIFICATIONS")
    if api >= 31:
        adb("shell", "wm", "user-rotation", "lock", "0")
    time.sleep(1)


if __name__ == "__main__":
    run("emulator-evidence")
