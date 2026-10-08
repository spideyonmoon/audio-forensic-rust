package dev.alfred.test;

import com.android.uiautomator.testrunner.UiAutomatorTestCase;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.InputEvent;
import android.view.MotionEvent;
import java.lang.reflect.Method;

/** Shell UIAutomator injects real finger tool properties, unlike API-30 input. */
public class SafFingerTap extends UiAutomatorTestCase {
    public void testTap() throws Exception {
        int x = Integer.parseInt(getParams().getString("x"));
        int y = Integer.parseInt(getParams().getString("y"));
        boolean longer = "true".equals(getParams().getString("long"));
        Method access = getUiDevice().getClass().getDeclaredMethod("getAutomatorBridge");
        access.setAccessible(true);
        Object bridge = access.invoke(getUiDevice());
        Method inject = Class.forName("com.android.uiautomator.core.UiAutomatorBridge")
            .getMethod("injectInputEvent", InputEvent.class, boolean.class);
        MotionEvent.PointerProperties pointer = new MotionEvent.PointerProperties();
        pointer.id = 0;
        pointer.toolType = MotionEvent.TOOL_TYPE_FINGER;
        MotionEvent.PointerCoords coordinates = new MotionEvent.PointerCoords();
        coordinates.x = x; coordinates.y = y; coordinates.pressure = 1; coordinates.size = 1;
        long down = SystemClock.uptimeMillis();
        send(inject, bridge, down, MotionEvent.ACTION_DOWN, pointer, coordinates);
        SystemClock.sleep(longer ? 1000 : 100);
        send(inject, bridge, down, MotionEvent.ACTION_UP, pointer, coordinates);
    }
    private void send(Method inject, Object bridge, long down, int action,
                      MotionEvent.PointerProperties pointer, MotionEvent.PointerCoords coordinates) throws Exception {
        int deviceId = 0;
        for (int id : InputDevice.getDeviceIds()) {
            InputDevice device = InputDevice.getDevice(id);
            if (device != null && device.supportsSource(InputDevice.SOURCE_TOUCHSCREEN)) {
                deviceId = id;
                break;
            }
        }
        MotionEvent event = MotionEvent.obtain(down, SystemClock.uptimeMillis(), action, 1,
            new MotionEvent.PointerProperties[] {pointer}, new MotionEvent.PointerCoords[] {coordinates},
            0, 0, 1, 1, deviceId, 0, InputDevice.SOURCE_TOUCHSCREEN, 0);
        // Match API-30's shell input sender: virtual pointer events explicitly
        // target the default display instead of retaining an invalid display ID.
        MotionEvent.class.getMethod("setDisplayId", int.class).invoke(event, 0);
        try {
            if (!((Boolean) inject.invoke(bridge, event, true))) throw new AssertionError("Finger injection failed");
        } finally { event.recycle(); }
    }
}
