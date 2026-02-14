use std::os::raw::c_char;
use std::sync::OnceLock;

/// Wrapper to allow caching raw pointers in OnceLock
struct SendSyncPtr(*mut c_char);
unsafe impl Send for SendSyncPtr {}
unsafe impl Sync for SendSyncPtr {}

/// os.platform() — cached (fix #12)
#[no_mangle]
pub extern "C" fn zaco_os_platform() -> *mut c_char {
    static CACHED: OnceLock<SendSyncPtr> = OnceLock::new();
    CACHED.get_or_init(|| SendSyncPtr(crate::zaco_compatible_str_new(std::env::consts::OS))).0
}

/// os.arch() — cached (fix #12)
#[no_mangle]
pub extern "C" fn zaco_os_arch() -> *mut c_char {
    static CACHED: OnceLock<SendSyncPtr> = OnceLock::new();
    CACHED.get_or_init(|| SendSyncPtr(crate::zaco_compatible_str_new(std::env::consts::ARCH))).0
}

#[no_mangle]
pub extern "C" fn zaco_os_homedir() -> *mut c_char {
    match std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        Ok(dir) => crate::zaco_compatible_str_new(&dir),
        Err(_) => crate::zaco_compatible_str_new(""),
    }
}

#[no_mangle]
pub extern "C" fn zaco_os_tmpdir() -> *mut c_char {
    crate::zaco_compatible_str_new(&std::env::temp_dir().to_string_lossy())
}

#[no_mangle]
pub extern "C" fn zaco_os_hostname() -> *mut c_char {
    let mut buf = vec![0u8; 256];
    unsafe {
        libc::gethostname(buf.as_mut_ptr() as *mut i8, buf.len());
    }
    let hostname = String::from_utf8_lossy(&buf).trim_end_matches('\0').to_string();
    crate::zaco_compatible_str_new(&hostname)
}

#[no_mangle]
pub extern "C" fn zaco_os_cpus() -> i64 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(1)
}

#[no_mangle]
pub extern "C" fn zaco_os_totalmem() -> i64 {
    #[cfg(target_os = "macos")]
    {
        let mut size: u64 = 0;
        let mut len = std::mem::size_of::<u64>();
        let mib = [libc::CTL_HW, libc::HW_MEMSIZE];
        unsafe {
            libc::sysctl(
                mib.as_ptr() as *mut _,
                2,
                &mut size as *mut u64 as *mut _,
                &mut len,
                std::ptr::null_mut(),
                0,
            );
        }
        size as i64
    }
    #[cfg(not(target_os = "macos"))]
    { 0 }
}

/// os.EOL — cached (fix #12)
#[no_mangle]
pub extern "C" fn zaco_os_eol() -> *mut c_char {
    static CACHED: OnceLock<SendSyncPtr> = OnceLock::new();
    CACHED.get_or_init(|| {
        #[cfg(windows)]
        { SendSyncPtr(crate::zaco_compatible_str_new("\r\n")) }
        #[cfg(not(windows))]
        { SendSyncPtr(crate::zaco_compatible_str_new("\n")) }
    }).0
}
