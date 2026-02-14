use std::os::raw::c_char;
use std::ffi::CStr;
use std::fs;

// === Sync API ===

#[no_mangle]
pub extern "C" fn zaco_fs_read_file_sync(path: *const c_char, _encoding: *const c_char) -> *mut c_char {
    let path_str = unsafe { crate::cstr_to_str(path) };
    match fs::read_to_string(path_str) {
        Ok(content) => crate::zaco_compatible_str_new(&content),
        Err(e) => {
            eprintln!("Error reading file '{}': {}", path_str, e);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn zaco_fs_write_file_sync(path: *const c_char, data: *const c_char) -> i64 {
    let path_str = unsafe { crate::cstr_to_str(path) };
    let data_str = unsafe { crate::cstr_to_str(data) };
    match fs::write(path_str, data_str) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error writing file '{}': {}", path_str, e);
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn zaco_fs_exists_sync(path: *const c_char) -> i64 {
    let path_str = unsafe { crate::cstr_to_str(path) };
    if std::path::Path::new(path_str).exists() { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn zaco_fs_mkdir_sync(path: *const c_char, recursive: i64) -> i64 {
    let path_str = unsafe { crate::cstr_to_str(path) };
    let result = if recursive != 0 {
        fs::create_dir_all(path_str)
    } else {
        fs::create_dir(path_str)
    };
    match result {
        Ok(()) => 0,
        Err(e) => { eprintln!("Error creating dir '{}': {}", path_str, e); -1 }
    }
}

#[no_mangle]
pub extern "C" fn zaco_fs_rmdir_sync(path: *const c_char) -> i64 {
    let path_str = unsafe { crate::cstr_to_str(path) };
    match fs::remove_dir(path_str) {
        Ok(()) => 0,
        Err(e) => { eprintln!("Error removing dir '{}': {}", path_str, e); -1 }
    }
}

#[no_mangle]
pub extern "C" fn zaco_fs_unlink_sync(path: *const c_char) -> i64 {
    let path_str = unsafe { crate::cstr_to_str(path) };
    match fs::remove_file(path_str) {
        Ok(()) => 0,
        Err(e) => { eprintln!("Error removing file '{}': {}", path_str, e); -1 }
    }
}

#[no_mangle]
pub extern "C" fn zaco_fs_stat_size(path: *const c_char) -> i64 {
    let path_str = unsafe { crate::cstr_to_str(path) };
    match fs::metadata(path_str) {
        Ok(meta) => meta.len() as i64,
        Err(_) => -1
    }
}

#[no_mangle]
pub extern "C" fn zaco_fs_stat_is_file(path: *const c_char) -> i64 {
    let path_str = unsafe { crate::cstr_to_str(path) };
    match fs::metadata(path_str) {
        Ok(meta) => if meta.is_file() { 1 } else { 0 },
        Err(_) => 0
    }
}

#[no_mangle]
pub extern "C" fn zaco_fs_stat_is_dir(path: *const c_char) -> i64 {
    let path_str = unsafe { crate::cstr_to_str(path) };
    match fs::metadata(path_str) {
        Ok(meta) => if meta.is_dir() { 1 } else { 0 },
        Err(_) => 0
    }
}

#[no_mangle]
pub extern "C" fn zaco_fs_readdir_sync(path: *const c_char) -> *mut c_char {
    let path_str = unsafe { crate::cstr_to_str(path) };
    match fs::read_dir(path_str) {
        Ok(entries) => {
            let names: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            // Return as newline-separated string for simplicity
            crate::zaco_compatible_str_new(&names.join("\n"))
        }
        Err(e) => {
            eprintln!("Error reading dir '{}': {}", path_str, e);
            std::ptr::null_mut()
        }
    }
}

// === Async API (callback-based) ===

/// Async readFile: reads file on a background thread, then calls callback(err, data).
/// callback signature: extern "C" fn(err: *const c_char, data: *const c_char)
#[no_mangle]
pub extern "C" fn zaco_fs_read_file(
    path: *const c_char,
    _encoding: *const c_char,
    callback: extern "C" fn(*const c_char, *const c_char),
) {
    let path_string = unsafe {
        if path.is_null() {
            String::new()
        } else {
            CStr::from_ptr(path).to_string_lossy().to_string()
        }
    };

    std::thread::spawn(move || {
        match fs::read_to_string(&path_string) {
            Ok(content) => {
                let data_ptr = crate::zaco_compatible_str_new(&content);
                callback(std::ptr::null(), data_ptr);
            }
            Err(e) => {
                let err_msg = format!("Error reading '{}': {}", path_string, e);
                let err_ptr = crate::zaco_compatible_str_new(&err_msg);
                callback(err_ptr, std::ptr::null());
            }
        }
    });
}
