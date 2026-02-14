use std::os::raw::c_char;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[no_mangle]
pub extern "C" fn zaco_path_join(a: *const c_char, b: *const c_char) -> *mut c_char {
    let path = Path::new(unsafe { crate::cstr_to_str(a) }).join(unsafe { crate::cstr_to_str(b) });
    crate::zaco_compatible_str_new(&path.to_string_lossy())
}

#[no_mangle]
pub extern "C" fn zaco_path_resolve(p: *const c_char) -> *mut c_char {
    let path = Path::new(unsafe { crate::cstr_to_str(p) });
    match std::fs::canonicalize(path) {
        Ok(abs) => crate::zaco_compatible_str_new(&abs.to_string_lossy()),
        Err(_) => crate::zaco_compatible_str_new(unsafe { crate::cstr_to_str(p) }),
    }
}

#[no_mangle]
pub extern "C" fn zaco_path_dirname(p: *const c_char) -> *mut c_char {
    let path = Path::new(unsafe { crate::cstr_to_str(p) });
    crate::zaco_compatible_str_new(&path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default())
}

#[no_mangle]
pub extern "C" fn zaco_path_basename(p: *const c_char) -> *mut c_char {
    let path = Path::new(unsafe { crate::cstr_to_str(p) });
    crate::zaco_compatible_str_new(&path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
}

#[no_mangle]
pub extern "C" fn zaco_path_extname(p: *const c_char) -> *mut c_char {
    let path = Path::new(unsafe { crate::cstr_to_str(p) });
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    crate::zaco_compatible_str_new(&ext)
}

#[no_mangle]
pub extern "C" fn zaco_path_is_absolute(p: *const c_char) -> i64 {
    if Path::new(unsafe { crate::cstr_to_str(p) }).is_absolute() { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn zaco_path_normalize(p: *const c_char) -> *mut c_char {
    let path_str = unsafe { crate::cstr_to_str(p) };
    let path = PathBuf::from(path_str);
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => { components.pop(); }
            std::path::Component::CurDir => {}
            _ => components.push(comp),
        }
    }
    let normalized: PathBuf = components.iter().collect();
    crate::zaco_compatible_str_new(&normalized.to_string_lossy())
}

/// Wrapper to allow caching raw pointers in OnceLock (pointer is heap-allocated, lives for program lifetime)
struct SendSyncPtr(*mut c_char);
unsafe impl Send for SendSyncPtr {}
unsafe impl Sync for SendSyncPtr {}

/// path.sep — returns the platform separator (cached, fix #12)
#[no_mangle]
pub extern "C" fn zaco_path_sep() -> *mut c_char {
    static CACHED: OnceLock<SendSyncPtr> = OnceLock::new();
    CACHED.get_or_init(|| SendSyncPtr(crate::zaco_compatible_str_new(std::path::MAIN_SEPARATOR_STR))).0
}
