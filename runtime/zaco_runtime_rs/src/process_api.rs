use std::ffi::CStr;
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn zaco_process_exit(code: i64) {
    std::process::exit(code as i32);
}

#[no_mangle]
pub extern "C" fn zaco_process_cwd() -> *mut c_char {
    match std::env::current_dir() {
        Ok(path) => crate::zaco_compatible_str_new(&path.to_string_lossy()),
        Err(_) => crate::zaco_compatible_str_new(""),
    }
}

/// Fix #5: null pointer check before CStr::from_ptr
#[no_mangle]
pub extern "C" fn zaco_process_env_get(key: *const c_char) -> *mut c_char {
    if key.is_null() {
        return std::ptr::null_mut();
    }
    let key_str = unsafe { CStr::from_ptr(key).to_str().unwrap_or("") };
    match std::env::var(key_str) {
        Ok(val) => crate::zaco_compatible_str_new(&val),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn zaco_process_pid() -> i64 {
    std::process::id() as i64
}

#[no_mangle]
pub extern "C" fn zaco_process_platform() -> *mut c_char {
    crate::zaco_compatible_str_new(std::env::consts::OS)
}

#[no_mangle]
pub extern "C" fn zaco_process_arch() -> *mut c_char {
    crate::zaco_compatible_str_new(std::env::consts::ARCH)
}

// process.argv - returns newline-separated args
#[no_mangle]
pub extern "C" fn zaco_process_argv() -> *mut c_char {
    let args: Vec<String> = std::env::args().collect();
    crate::zaco_compatible_str_new(&args.join("\n"))
}
