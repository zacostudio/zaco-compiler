//! Zaco Rust Runtime — Node.js compatible API implementations
//! All functions are exposed as C-compatible symbols for Cranelift codegen.

mod event_loop;
mod promise;
mod fs;
mod path;
mod process_api;
mod os;
mod http;
mod events;
mod timer;

pub use event_loop::*;
pub use promise::*;
pub use fs::*;
pub use path::*;
pub use process_api::*;
pub use os::*;
pub use http::*;
pub use events::*;
pub use timer::*;

use std::ffi::CStr;
use std::os::raw::c_char;

/// Helper: Convert C string pointer to Rust &str
/// Used by all submodules via `crate::cstr_to_str`
pub(crate) unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() { return ""; }
    CStr::from_ptr(ptr).to_str().unwrap_or("")
}

/// Allocate a string using the same memory layout as the C runtime's zaco_alloc.
/// Layout: [ref_count: i64 = 1][size: i64 = len][data: char[len+1]]
/// Returns a pointer to the data portion (offset 16), compatible with zaco_free/zaco_rc_inc/zaco_rc_dec.
pub(crate) fn zaco_compatible_str_new(s: &str) -> *mut c_char {
    let len = s.len();
    let total = 16 + len + 1; // header + data + null terminator
    unsafe {
        let layout = std::alloc::Layout::from_size_align(total, 8).unwrap();
        let base = std::alloc::alloc_zeroed(layout);
        if base.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        // Write ref_count = 1 at offset 0
        *(base as *mut i64) = 1;
        // Write size = len at offset 8
        *((base as *mut i64).add(1)) = len as i64;
        // Copy string data at offset 16
        let data_ptr = base.add(16);
        std::ptr::copy_nonoverlapping(s.as_ptr(), data_ptr, len);
        // Null terminator (already zeroed, but be explicit)
        *data_ptr.add(len) = 0;
        data_ptr as *mut c_char
    }
}

/// Initialize the Tokio runtime (called once at program start)
#[no_mangle]
pub extern "C" fn zaco_runtime_init() {
    event_loop::init_runtime();
}

/// Shutdown the runtime and run pending tasks
#[no_mangle]
pub extern "C" fn zaco_runtime_shutdown() {
    event_loop::shutdown_runtime();
}
