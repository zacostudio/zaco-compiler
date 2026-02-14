//! HTTP client implementation using reqwest
//! Provides both synchronous and asynchronous HTTP operations

use std::os::raw::{c_char, c_void};
use std::collections::HashMap;

use crate::event_loop;

/// HTTP GET request (synchronous)
#[no_mangle]
pub extern "C" fn zaco_http_get(url: *const c_char) -> *mut c_char {
    let url_str = unsafe { crate::cstr_to_str(url) };
    if url_str.is_empty() {
        return std::ptr::null_mut();
    }

    match reqwest::blocking::get(url_str) {
        Ok(response) => match response.text() {
            Ok(body) => crate::zaco_compatible_str_new(&body),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// HTTP POST request (synchronous)
#[no_mangle]
pub extern "C" fn zaco_http_post(
    url: *const c_char,
    body: *const c_char,
    content_type: *const c_char,
) -> *mut c_char {
    let url_str = unsafe { crate::cstr_to_str(url) };
    let body_str = unsafe { crate::cstr_to_str(body) };
    let content_type_str = unsafe { crate::cstr_to_str(content_type) };

    if url_str.is_empty() {
        return std::ptr::null_mut();
    }

    let client = reqwest::blocking::Client::new();
    let mut request = client.post(url_str);

    if !content_type_str.is_empty() {
        request = request.header("Content-Type", content_type_str);
    }

    match request.body(body_str.to_string()).send() {
        Ok(response) => match response.text() {
            Ok(body) => crate::zaco_compatible_str_new(&body),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// HTTP GET with status code
#[no_mangle]
pub extern "C" fn zaco_http_get_status(url: *const c_char) -> i64 {
    let url_str = unsafe { crate::cstr_to_str(url) };
    if url_str.is_empty() {
        return -1;
    }

    match reqwest::blocking::get(url_str) {
        Ok(response) => response.status().as_u16() as i64,
        Err(_) => -1,
    }
}

/// HTTP GET response headers (returns JSON string of headers)
#[no_mangle]
pub extern "C" fn zaco_http_get_headers(url: *const c_char) -> *mut c_char {
    let url_str = unsafe { crate::cstr_to_str(url) };
    if url_str.is_empty() {
        return std::ptr::null_mut();
    }

    match reqwest::blocking::get(url_str) {
        Ok(response) => {
            let mut headers_map: HashMap<String, String> = HashMap::new();
            for (name, value) in response.headers() {
                if let Ok(value_str) = value.to_str() {
                    headers_map.insert(name.to_string(), value_str.to_string());
                }
            }

            match serde_json::to_string(&headers_map) {
                Ok(json) => crate::zaco_compatible_str_new(&json),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Callback function type for async operations
type AsyncCallback = extern "C" fn(i64, *mut c_char, *mut c_void);

/// Async HTTP GET (uses Tokio)
#[no_mangle]
pub extern "C" fn zaco_http_get_async(
    url: *const c_char,
    callback: AsyncCallback,
    context: *mut c_void,
) {
    let url_str = unsafe { crate::cstr_to_str(url) }.to_string();
    let context_addr = context as usize;

    event_loop::spawn(async move {
        let result = match reqwest::get(&url_str).await {
            Ok(response) => match response.text().await {
                Ok(body) => crate::zaco_compatible_str_new(&body),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        };

        callback(0, result, context_addr as *mut c_void);
    });
}

/// HTTP PUT request (synchronous)
#[no_mangle]
pub extern "C" fn zaco_http_put(
    url: *const c_char,
    body: *const c_char,
    content_type: *const c_char,
) -> *mut c_char {
    let url_str = unsafe { crate::cstr_to_str(url) };
    let body_str = unsafe { crate::cstr_to_str(body) };
    let content_type_str = unsafe { crate::cstr_to_str(content_type) };

    if url_str.is_empty() {
        return std::ptr::null_mut();
    }

    let client = reqwest::blocking::Client::new();
    let mut request = client.put(url_str);

    if !content_type_str.is_empty() {
        request = request.header("Content-Type", content_type_str);
    }

    match request.body(body_str.to_string()).send() {
        Ok(response) => match response.text() {
            Ok(body) => crate::zaco_compatible_str_new(&body),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// HTTP DELETE request (synchronous)
#[no_mangle]
pub extern "C" fn zaco_http_delete(url: *const c_char) -> *mut c_char {
    let url_str = unsafe { crate::cstr_to_str(url) };
    if url_str.is_empty() {
        return std::ptr::null_mut();
    }

    let client = reqwest::blocking::Client::new();
    match client.delete(url_str).send() {
        Ok(response) => match response.text() {
            Ok(body) => crate::zaco_compatible_str_new(&body),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}
