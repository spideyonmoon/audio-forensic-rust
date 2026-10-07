//! Alfred-owned JNI transport. The portable core contains no Android state.
pub mod operation;
pub mod storage;
pub mod transport;

use jni::{
    JNIEnv,
    objects::{JByteArray, JObject},
    sys::{jbyteArray, jint, jlong},
};
use serde_json::{Value, json};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::OnceLock,
};
use transport::{Bridge, CONTROL_LIMIT, Result, fail};
static BRIDGE: OnceLock<Bridge> = OnceLock::new();
fn bridge() -> &'static Bridge {
    BRIDGE.get_or_init(Bridge::default)
}

pub fn encode(result: Result<Value>) -> Vec<u8> {
    let value = match result {
        Ok(v) => json!({"ok":true,"value":v}),
        Err(e) => json!({"ok":false,"error":e}),
    };
    let bytes = serde_json::to_vec(&value).unwrap_or_default();
    if bytes.len() <= CONTROL_LIMIT {
        bytes
    } else {
        serde_json::to_vec(
            &json!({"ok":false,"error":fail("size_limit","Control response exceeds 64 KiB")}),
        )
        .unwrap_or_default()
    }
}
fn entry(mut env: JNIEnv, run: impl FnOnce(&mut JNIEnv) -> Result<Value>) -> jbyteArray {
    // Catch conversion/registry/encoding/output allocation panics at every entry.
    catch_unwind(AssertUnwindSafe(|| {
        let result = catch_unwind(AssertUnwindSafe(|| run(&mut env)))
            .unwrap_or_else(|_| Err(fail("worker_panicked", "Native entry panicked")));
        env.byte_array_from_slice(&encode(result))
            .map(|a| a.into_raw())
            .unwrap_or(std::ptr::null_mut())
    }))
    .unwrap_or(std::ptr::null_mut())
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_alfred_shared_NativeBootstrap_nativeHostVersion(
    _env: JNIEnv,
    _object: JObject,
) -> jint {
    catch_unwind(|| 1).unwrap_or(0)
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_alfred_shared_NativeTransport_describe(
    env: JNIEnv,
    _object: JObject,
) -> jbyteArray {
    entry(env, |_| Ok(bridge().describe()))
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_alfred_shared_NativeTransport_start(
    env: JNIEnv,
    _object: JObject,
    request: JByteArray,
) -> jbyteArray {
    entry(env, |env| {
        let n = env
            .get_array_length(&request)
            .map_err(|_| fail("invalid_request", "Missing request bytes"))?;
        if n < 0 || n as usize > CONTROL_LIMIT {
            return Err(fail("size_limit", "Control request exceeds 64 KiB"));
        }
        let bytes = env
            .convert_byte_array(request)
            .map_err(|_| fail("invalid_request", "Cannot read request bytes"))?;
        bridge().start(&bytes).map(|h| json!({"handle":h}))
    })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_alfred_shared_NativeTransport_poll(
    env: JNIEnv,
    _object: JObject,
    handle: jlong,
) -> jbyteArray {
    entry(env, |_| bridge().poll(handle))
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_alfred_shared_NativeTransport_cancel(
    env: JNIEnv,
    _object: JObject,
    handle: jlong,
) -> jbyteArray {
    entry(env, |_| bridge().cancel(handle))
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_alfred_shared_NativeTransport_close(
    env: JNIEnv,
    _object: JObject,
    handle: jlong,
) -> jbyteArray {
    entry(env, |_| Ok(bridge().close(handle)))
}
