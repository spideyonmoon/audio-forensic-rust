package com.android.uiautomator.testrunner;
import android.os.Bundle;
import com.android.uiautomator.core.UiDevice;
/** Compile-only signatures. These stubs are never packaged; Android's shell
 * uiautomator.jar supplies their implementations and privileged UIAutomation.
 */
public class UiAutomatorTestCase {
    public Bundle getParams() { throw new UnsupportedOperationException(); }
    public UiDevice getUiDevice() { throw new UnsupportedOperationException(); }
}
