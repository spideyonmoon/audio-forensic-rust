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
        coordinates.pressure = 0;
        send(inject, bridge, down, MotionEvent.ACTION_UP, pointer, coordinates);
    }
    private void send(Method inject, Object bridge, long down, int action,
                      MotionEvent.PointerProperties pointer, MotionEvent.PointerCoords coordinates) throws Exception {
        MotionEvent event = MotionEvent.obtain(down, SystemClock.uptimeMillis(), action, 1,
            new MotionEvent.PointerProperties[] {pointer}, new MotionEvent.PointerCoords[] {coordinates},
            0, 0, 1, 1, 0, 0, InputDevice.SOURCE_TOUCHSCREEN, 0);
        try {
            if (!((Boolean) inject.invoke(bridge, event, true))) throw new AssertionError("Finger injection failed");
        } finally { event.recycle(); }
    }
}
