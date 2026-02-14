//! EventEmitter implementation (Node.js compatible)
//! Provides a simple pub/sub event system

use std::collections::HashMap;
use std::os::raw::{c_char, c_void};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicI64, Ordering};

/// Callback function type — fix #6: receives event data pointer
type Callback = extern "C" fn(*mut c_void, *mut c_void);

/// Listener entry
#[derive(Clone)]
struct Listener {
    callback: Callback,
    context: usize,
    once: bool,
}

/// EventEmitter structure
struct EventEmitter {
    listeners: HashMap<String, Vec<Listener>>,
}

impl EventEmitter {
    fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    fn on(&mut self, event: &str, callback: Callback, context: *mut c_void) {
        let listener = Listener {
            callback,
            context: context as usize,
            once: false,
        };
        self.listeners
            .entry(event.to_string())
            .or_insert_with(Vec::new)
            .push(listener);
    }

    fn once(&mut self, event: &str, callback: Callback, context: *mut c_void) {
        let listener = Listener {
            callback,
            context: context as usize,
            once: true,
        };
        self.listeners
            .entry(event.to_string())
            .or_insert_with(Vec::new)
            .push(listener);
    }

    /// Collect listeners for emission — returns cloned list and removes once listeners
    fn take_listeners_for_emit(&mut self, event: &str) -> Vec<Listener> {
        let listeners = match self.listeners.get_mut(event) {
            Some(l) => l,
            None => return Vec::new(),
        };

        let snapshot: Vec<Listener> = listeners.clone();

        // Remove once listeners
        listeners.retain(|l| !l.once);

        snapshot
    }

    fn remove_all(&mut self, event: &str) {
        self.listeners.remove(event);
    }

    fn listener_count(&self, event: &str) -> i64 {
        self.listeners
            .get(event)
            .map(|l| l.len() as i64)
            .unwrap_or(0)
    }

    fn remove_listener(&mut self, event: &str, callback: Callback) -> bool {
        if let Some(listeners) = self.listeners.get_mut(event) {
            if let Some(pos) = listeners.iter().position(|l| {
                l.callback as usize == callback as usize
            }) {
                listeners.remove(pos);
                return true;
            }
        }
        false
    }
}

/// Global registry of EventEmitters
static EMITTERS: Mutex<Option<HashMap<i64, Arc<Mutex<EventEmitter>>>>> = Mutex::new(None);
static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

/// Initialize the global emitter registry
fn ensure_registry() {
    let mut registry = EMITTERS.lock().unwrap();
    if registry.is_none() {
        *registry = Some(HashMap::new());
    }
}

/// Create a new EventEmitter
#[no_mangle]
pub extern "C" fn zaco_events_new() -> i64 {
    ensure_registry();

    let handle = NEXT_HANDLE.fetch_add(1, Ordering::SeqCst);
    let emitter = Arc::new(Mutex::new(EventEmitter::new()));

    let mut registry = EMITTERS.lock().unwrap();
    if let Some(ref mut map) = *registry {
        map.insert(handle, emitter);
    }

    handle
}

/// Register an event listener
#[no_mangle]
pub extern "C" fn zaco_events_on(
    emitter: i64,
    event: *const c_char,
    callback: Callback,
    context: *mut c_void,
) {
    let event_str = unsafe { crate::cstr_to_str(event) };

    let registry = EMITTERS.lock().unwrap();
    if let Some(ref map) = *registry {
        if let Some(emitter) = map.get(&emitter) {
            let mut em = emitter.lock().unwrap();
            em.on(event_str, callback, context);
        }
    }
}

/// Register a one-time event listener
#[no_mangle]
pub extern "C" fn zaco_events_once(
    emitter: i64,
    event: *const c_char,
    callback: Callback,
    context: *mut c_void,
) {
    let event_str = unsafe { crate::cstr_to_str(event) };

    let registry = EMITTERS.lock().unwrap();
    if let Some(ref map) = *registry {
        if let Some(emitter) = map.get(&emitter) {
            let mut em = emitter.lock().unwrap();
            em.once(event_str, callback, context);
        }
    }
}

/// Emit an event — fix #4: clone listeners, drop lock, THEN invoke callbacks to prevent deadlock
/// Fix #6: pass data to callbacks
#[no_mangle]
pub extern "C" fn zaco_events_emit(
    emitter: i64,
    event: *const c_char,
    data: *mut c_void,
) -> i64 {
    let event_str = unsafe { crate::cstr_to_str(event) };

    // Clone the emitter Arc while holding registry lock, then drop registry lock
    let emitter_arc = {
        let registry = EMITTERS.lock().unwrap();
        match *registry {
            Some(ref map) => map.get(&emitter).cloned(),
            None => None,
        }
    };

    let emitter_arc = match emitter_arc {
        Some(e) => e,
        None => return 0,
    };

    // Take snapshot of listeners (and remove once listeners) while holding emitter lock
    let listeners = {
        let mut em = emitter_arc.lock().unwrap();
        em.take_listeners_for_emit(event_str)
    };
    // Emitter lock is now dropped — callbacks can safely call events API

    let mut count = 0i64;
    for listener in &listeners {
        (listener.callback)(listener.context as *mut c_void, data);
        count += 1;
    }

    count
}

/// Remove all listeners for an event
#[no_mangle]
pub extern "C" fn zaco_events_remove_all(emitter: i64, event: *const c_char) {
    let event_str = unsafe { crate::cstr_to_str(event) };

    let registry = EMITTERS.lock().unwrap();
    if let Some(ref map) = *registry {
        if let Some(emitter) = map.get(&emitter) {
            let mut em = emitter.lock().unwrap();
            em.remove_all(event_str);
        }
    }
}

/// Get listener count for an event
#[no_mangle]
pub extern "C" fn zaco_events_listener_count(emitter: i64, event: *const c_char) -> i64 {
    let event_str = unsafe { crate::cstr_to_str(event) };

    let registry = EMITTERS.lock().unwrap();
    if let Some(ref map) = *registry {
        if let Some(emitter) = map.get(&emitter) {
            let em = emitter.lock().unwrap();
            return em.listener_count(event_str);
        }
    }
    0
}

/// Remove a specific listener
#[no_mangle]
pub extern "C" fn zaco_events_remove_listener(
    emitter: i64,
    event: *const c_char,
    callback: Callback,
) -> i64 {
    let event_str = unsafe { crate::cstr_to_str(event) };

    let registry = EMITTERS.lock().unwrap();
    if let Some(ref map) = *registry {
        if let Some(emitter) = map.get(&emitter) {
            let mut em = emitter.lock().unwrap();
            return if em.remove_listener(event_str, callback) { 1 } else { 0 };
        }
    }
    0
}

/// Get all event names
#[no_mangle]
pub extern "C" fn zaco_events_event_names(emitter: i64) -> *mut c_char {
    let registry = EMITTERS.lock().unwrap();
    if let Some(ref map) = *registry {
        if let Some(emitter) = map.get(&emitter) {
            let em = emitter.lock().unwrap();
            let names: Vec<String> = em.listeners.keys().cloned().collect();
            let joined = names.join("\n");
            return crate::zaco_compatible_str_new(&joined);
        }
    }
    std::ptr::null_mut()
}

/// Destroy an EventEmitter
#[no_mangle]
pub extern "C" fn zaco_events_destroy(emitter: i64) {
    let mut registry = EMITTERS.lock().unwrap();
    if let Some(ref mut map) = *registry {
        map.remove(&emitter);
    }
}
