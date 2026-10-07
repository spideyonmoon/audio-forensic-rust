//! App-owned A02 load/version bootstrap. A03 implements handles and feature work.
use std::ffi::c_void;

// No JNI pointers are dereferenced, no objects/strings cross this bootstrap.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_alfred_shared_NativeBootstrap_nativeHostVersion(
    _env: *mut c_void,
    _object: *mut c_void,
) -> i32 {
    std::panic::catch_unwind(|| {
        let token = audio_forensic::CancellationToken::default();
        if token.is_cancelled() { 0 } else { 1 }
    })
    .unwrap_or(0)
}
