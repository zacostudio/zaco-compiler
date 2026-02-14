//! Promise implementation for async/await support

use std::sync::{Mutex, Condvar};
use std::ffi::c_void;

/// Promise state
#[derive(Clone, Copy, PartialEq)]
enum PromiseState {
    Pending,
    Resolved,
    Rejected,
}

/// Promise implementation using condition variables for blocking await
pub struct ZacoPromise {
    state: Mutex<PromiseState>,
    value: Mutex<Option<*mut c_void>>,
    condvar: Condvar,
}

impl ZacoPromise {
    fn new() -> Self {
        ZacoPromise {
            state: Mutex::new(PromiseState::Pending),
            value: Mutex::new(None),
            condvar: Condvar::new(),
        }
    }

    fn resolve(&self, value: *mut c_void) {
        let mut state = self.state.lock().unwrap();
        if *state == PromiseState::Pending {
            *state = PromiseState::Resolved;
            *self.value.lock().unwrap() = Some(value);
            self.condvar.notify_all();
        }
    }

    fn reject(&self, error: *mut c_void) {
        let mut state = self.state.lock().unwrap();
        if *state == PromiseState::Pending {
            *state = PromiseState::Rejected;
            *self.value.lock().unwrap() = Some(error);
            self.condvar.notify_all();
        }
    }

    fn wait(&self) -> *mut c_void {
        let mut state = self.state.lock().unwrap();
        while *state == PromiseState::Pending {
            state = self.condvar.wait(state).unwrap();
        }
        self.value.lock().unwrap().unwrap_or(std::ptr::null_mut())
    }
}

unsafe impl Send for ZacoPromise {}
unsafe impl Sync for ZacoPromise {}

/// Create a new pending promise
#[no_mangle]
pub extern "C" fn zaco_promise_new() -> *mut ZacoPromise {
    Box::into_raw(Box::new(ZacoPromise::new()))
}

/// Resolve a promise with a value
#[no_mangle]
pub extern "C" fn zaco_promise_resolve(promise: *mut ZacoPromise, value: *mut c_void) {
    if promise.is_null() {
        return;
    }
    unsafe {
        (*promise).resolve(value);
    }
}

/// Reject a promise with an error
#[no_mangle]
pub extern "C" fn zaco_promise_reject(promise: *mut ZacoPromise, error: *mut c_void) {
    if promise.is_null() {
        return;
    }
    unsafe {
        (*promise).reject(error);
    }
}

/// Block on a promise until it resolves or rejects (returns the value/error)
#[no_mangle]
pub extern "C" fn zaco_async_block_on(promise: *mut ZacoPromise) -> *mut c_void {
    if promise.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        (*promise).wait()
    }
}

/// Spawn an async task (simplified version - just calls fn and resolves promise)
/// In a real implementation, this would use tokio::spawn
#[no_mangle]
pub extern "C" fn zaco_async_spawn(
    fn_ptr: extern "C" fn(*mut c_void) -> *mut c_void,
    arg: *mut c_void,
) -> *mut ZacoPromise {
    let promise = ZacoPromise::new();
    let promise_ptr = Box::into_raw(Box::new(promise));

    // For now, execute synchronously
    // TODO: Use tokio::spawn for true async execution
    let result = fn_ptr(arg);

    unsafe {
        (*promise_ptr).resolve(result);
    }

    promise_ptr
}

/// Free a promise
#[no_mangle]
pub extern "C" fn zaco_promise_free(promise: *mut ZacoPromise) {
    if !promise.is_null() {
        unsafe {
            let _ = Box::from_raw(promise);
        }
    }
}
