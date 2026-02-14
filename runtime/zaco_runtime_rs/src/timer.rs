//! Timer functions: setTimeout, setInterval, clearTimeout, clearInterval

use std::os::raw::c_void;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Mutex, Arc, OnceLock};
use std::collections::HashMap;
use std::time::Duration;

static NEXT_TIMER_ID: AtomicI64 = AtomicI64::new(1);

struct TimerEntry {
    cancelled: AtomicBool,
}

fn timers() -> &'static Mutex<HashMap<i64, Arc<TimerEntry>>> {
    static TIMERS: OnceLock<Mutex<HashMap<i64, Arc<TimerEntry>>>> = OnceLock::new();
    TIMERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// setTimeout(callback, context, delay_ms) -> timer_id
/// Calls callback(context) after delay_ms milliseconds on a background thread.
#[no_mangle]
pub extern "C" fn zaco_set_timeout(
    callback: extern "C" fn(*mut c_void),
    context: *mut c_void,
    delay_ms: i64,
) -> i64 {
    let id = NEXT_TIMER_ID.fetch_add(1, Ordering::SeqCst);
    let entry = Arc::new(TimerEntry {
        cancelled: AtomicBool::new(false),
    });

    {
        let mut t = timers().lock().unwrap();
        t.insert(id, entry.clone());
    }

    // context pointer needs to be sendable across threads
    let ctx = context as usize;
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(delay_ms as u64));
        if !entry.cancelled.load(Ordering::SeqCst) {
            callback(ctx as *mut c_void);
        }
        // Clean up
        if let Ok(mut t) = timers().lock() {
            t.remove(&id);
        }
    });

    id
}

/// setInterval(callback, context, delay_ms) -> timer_id
/// Calls callback(context) repeatedly every delay_ms milliseconds.
#[no_mangle]
pub extern "C" fn zaco_set_interval(
    callback: extern "C" fn(*mut c_void),
    context: *mut c_void,
    delay_ms: i64,
) -> i64 {
    let id = NEXT_TIMER_ID.fetch_add(1, Ordering::SeqCst);
    let entry = Arc::new(TimerEntry {
        cancelled: AtomicBool::new(false),
    });

    {
        let mut t = timers().lock().unwrap();
        t.insert(id, entry.clone());
    }

    let ctx = context as usize;
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(delay_ms as u64));
            if entry.cancelled.load(Ordering::SeqCst) {
                break;
            }
            callback(ctx as *mut c_void);
        }
        // Clean up
        if let Ok(mut t) = timers().lock() {
            t.remove(&id);
        }
    });

    id
}

/// clearTimeout(timer_id)
#[no_mangle]
pub extern "C" fn zaco_clear_timeout(timer_id: i64) {
    if let Ok(t) = timers().lock() {
        if let Some(entry) = t.get(&timer_id) {
            entry.cancelled.store(true, Ordering::SeqCst);
        }
    }
}

/// clearInterval(timer_id) — same as clearTimeout
#[no_mangle]
pub extern "C" fn zaco_clear_interval(timer_id: i64) {
    zaco_clear_timeout(timer_id);
}
