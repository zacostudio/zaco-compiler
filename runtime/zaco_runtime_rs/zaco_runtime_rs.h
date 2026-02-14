/**
 * Zaco Rust Runtime - C Header File
 *
 * Include this header when linking against libzaco_runtime_rs.a
 * All functions use C ABI and are compatible with Cranelift codegen.
 *
 * Linking on macOS requires:
 *   -framework CoreFoundation -framework Security -framework SystemConfiguration -lpthread -ldl
 *
 * Example:
 *   cc -o test test.c -L target/release -lzaco_runtime_rs \
 *      -framework CoreFoundation -framework Security -framework SystemConfiguration -lpthread -ldl
 */

#ifndef ZACO_RUNTIME_RS_H
#define ZACO_RUNTIME_RS_H

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Runtime Management
// ============================================================================

/**
 * Initialize the Tokio async runtime.
 * MUST be called once at program startup before any other runtime functions.
 */
void zaco_runtime_init(void);

/**
 * Shutdown the async runtime.
 * Call at program exit to ensure all pending tasks complete.
 */
void zaco_runtime_shutdown(void);

// ============================================================================
// Path Module (path.*)
// ============================================================================

/**
 * Join two path segments.
 * Returns: Allocated string (caller must free).
 */
char* zaco_path_join(const char* a, const char* b);

/**
 * Resolve a path to an absolute path.
 * Returns: Allocated string (caller must free).
 */
char* zaco_path_resolve(const char* p);

/**
 * Get the directory name of a path.
 * Returns: Allocated string (caller must free).
 */
char* zaco_path_dirname(const char* p);

/**
 * Get the base name of a path.
 * Returns: Allocated string (caller must free).
 */
char* zaco_path_basename(const char* p);

/**
 * Get the file extension (including the dot).
 * Returns: Allocated string (caller must free).
 */
char* zaco_path_extname(const char* p);

/**
 * Check if a path is absolute.
 * Returns: 1 if absolute, 0 otherwise.
 */
long long zaco_path_is_absolute(const char* p);

/**
 * Normalize a path (resolve . and .. components).
 * Returns: Allocated string (caller must free).
 */
char* zaco_path_normalize(const char* p);

/**
 * Get the platform path separator.
 * Returns: Allocated string (caller must free).
 */
char* zaco_path_sep(void);

// ============================================================================
// File System Module - Sync (fs.*Sync)
// ============================================================================

/**
 * Read a file synchronously.
 * Returns: Allocated string with file contents (caller must free), or NULL on error.
 */
char* zaco_fs_read_file_sync(const char* path, const char* encoding);

/**
 * Write a file synchronously.
 * Returns: 0 on success, -1 on error.
 */
long long zaco_fs_write_file_sync(const char* path, const char* data);

/**
 * Check if a file exists.
 * Returns: 1 if exists, 0 otherwise.
 */
long long zaco_fs_exists_sync(const char* path);

/**
 * Create a directory.
 * recursive: 1 to create parent directories, 0 otherwise.
 * Returns: 0 on success, -1 on error.
 */
long long zaco_fs_mkdir_sync(const char* path, long long recursive);

/**
 * Remove a directory.
 * Returns: 0 on success, -1 on error.
 */
long long zaco_fs_rmdir_sync(const char* path);

/**
 * Remove a file.
 * Returns: 0 on success, -1 on error.
 */
long long zaco_fs_unlink_sync(const char* path);

/**
 * Get file size in bytes.
 * Returns: File size, or -1 on error.
 */
long long zaco_fs_stat_size(const char* path);

/**
 * Check if path is a regular file.
 * Returns: 1 if file, 0 otherwise.
 */
long long zaco_fs_stat_is_file(const char* path);

/**
 * Check if path is a directory.
 * Returns: 1 if directory, 0 otherwise.
 */
long long zaco_fs_stat_is_dir(const char* path);

/**
 * Read directory contents.
 * Returns: Newline-separated list of file names (caller must free), or NULL on error.
 */
char* zaco_fs_readdir_sync(const char* path);

// ============================================================================
// File System Module - Async
// ============================================================================

/**
 * Read a file asynchronously.
 * callback_id: Identifier for the callback to invoke when complete.
 * Note: Callback mechanism not yet implemented.
 */
void zaco_fs_read_file_async(const char* path, const char* encoding, long long callback_id);

// ============================================================================
// Process Module (process.*)
// ============================================================================

/**
 * Exit the process with the given code.
 */
void zaco_process_exit(long long code);

/**
 * Get current working directory.
 * Returns: Allocated string (caller must free).
 */
char* zaco_process_cwd(void);

/**
 * Get environment variable value.
 * Returns: Allocated string with value (caller must free), or NULL if not found.
 */
char* zaco_process_env_get(const char* key);

/**
 * Get process ID.
 * Returns: Process ID.
 */
long long zaco_process_pid(void);

/**
 * Get platform name (e.g., "macos", "linux", "windows").
 * Returns: Allocated string (caller must free).
 */
char* zaco_process_platform(void);

/**
 * Get architecture name (e.g., "x86_64", "aarch64").
 * Returns: Allocated string (caller must free).
 */
char* zaco_process_arch(void);

/**
 * Get command-line arguments.
 * Returns: Newline-separated list of arguments (caller must free).
 */
char* zaco_process_argv(void);

// ============================================================================
// OS Module (os.*)
// ============================================================================

/**
 * Get platform name.
 * Returns: Allocated string (caller must free).
 */
char* zaco_os_platform(void);

/**
 * Get architecture name.
 * Returns: Allocated string (caller must free).
 */
char* zaco_os_arch(void);

/**
 * Get user's home directory.
 * Returns: Allocated string (caller must free).
 */
char* zaco_os_homedir(void);

/**
 * Get temporary directory path.
 * Returns: Allocated string (caller must free).
 */
char* zaco_os_tmpdir(void);

/**
 * Get hostname.
 * Returns: Allocated string (caller must free).
 */
char* zaco_os_hostname(void);

/**
 * Get number of CPU cores.
 * Returns: Number of cores.
 */
long long zaco_os_cpus(void);

/**
 * Get total system memory in bytes.
 * Returns: Total memory, or 0 if not available.
 */
long long zaco_os_totalmem(void);

/**
 * Get end-of-line marker for the platform.
 * Returns: Allocated string (caller must free). "\n" on Unix, "\r\n" on Windows.
 */
char* zaco_os_eol(void);

// ============================================================================
// HTTP Module (http.*)
// ============================================================================

/**
 * Perform HTTP GET request (synchronous).
 * Returns: Response body (caller must free), or NULL on error.
 */
char* zaco_http_get(const char* url);

/**
 * Perform HTTP POST request (synchronous).
 * url: Target URL
 * body: Request body (JSON, text, etc.)
 * content_type: Content-Type header (e.g., "application/json")
 * Returns: Response body (caller must free), or NULL on error.
 */
char* zaco_http_post(const char* url, const char* body, const char* content_type);

/**
 * Perform HTTP PUT request (synchronous).
 * url: Target URL
 * body: Request body
 * content_type: Content-Type header
 * Returns: Response body (caller must free), or NULL on error.
 */
char* zaco_http_put(const char* url, const char* body, const char* content_type);

/**
 * Perform HTTP DELETE request (synchronous).
 * Returns: Response body (caller must free), or NULL on error.
 */
char* zaco_http_delete(const char* url);

/**
 * Perform HTTP GET and return only the status code.
 * Returns: HTTP status code (200, 404, etc.), or -1 on error.
 */
long long zaco_http_get_status(const char* url);

/**
 * Perform HTTP GET and return response headers as JSON.
 * Returns: JSON string of headers (caller must free), or NULL on error.
 */
char* zaco_http_get_headers(const char* url);

/**
 * Perform HTTP GET asynchronously.
 * callback: Function to call when complete: void callback(i64 status, char* body, void* context)
 * context: User data to pass to callback
 */
typedef void (*zaco_http_callback)(long long status, char* body, void* context);
void zaco_http_get_async(const char* url, zaco_http_callback callback, void* context);

// ============================================================================
// Events Module (EventEmitter)
// ============================================================================

/**
 * Callback function type for event listeners.
 */
typedef void (*zaco_event_callback)(void* context);

/**
 * Create a new EventEmitter.
 * Returns: Handle to the emitter.
 */
long long zaco_events_new(void);

/**
 * Register an event listener (persistent).
 * emitter: Handle from zaco_events_new
 * event: Event name
 * callback: Function to call when event is emitted
 * context: User data to pass to callback
 */
void zaco_events_on(long long emitter, const char* event, zaco_event_callback callback, void* context);

/**
 * Register a one-time event listener (removed after first call).
 * emitter: Handle from zaco_events_new
 * event: Event name
 * callback: Function to call when event is emitted
 * context: User data to pass to callback
 */
void zaco_events_once(long long emitter, const char* event, zaco_event_callback callback, void* context);

/**
 * Emit an event.
 * emitter: Handle from zaco_events_new
 * event: Event name
 * data: Data to pass to listeners
 * Returns: Number of listeners called
 */
long long zaco_events_emit(long long emitter, const char* event, void* data);

/**
 * Remove all listeners for an event.
 * emitter: Handle from zaco_events_new
 * event: Event name
 */
void zaco_events_remove_all(long long emitter, const char* event);

/**
 * Get the number of listeners for an event.
 * emitter: Handle from zaco_events_new
 * event: Event name
 * Returns: Number of listeners
 */
long long zaco_events_listener_count(long long emitter, const char* event);

/**
 * Remove a specific listener.
 * emitter: Handle from zaco_events_new
 * event: Event name
 * callback: The callback to remove
 * Returns: 1 if removed, 0 if not found
 */
long long zaco_events_remove_listener(long long emitter, const char* event, zaco_event_callback callback);

/**
 * Get all event names.
 * emitter: Handle from zaco_events_new
 * Returns: Newline-separated list of event names (caller must free), or NULL
 */
char* zaco_events_event_names(long long emitter);

/**
 * Destroy an EventEmitter and free its resources.
 * emitter: Handle from zaco_events_new
 */
void zaco_events_destroy(long long emitter);

// ============================================================================
// Promise Module - STUB
// ============================================================================

/**
 * Promise handle (opaque).
 */
typedef struct ZacoPromise ZacoPromise;

/**
 * Create a new Promise.
 * Returns: Handle to the promise, or NULL.
 * Note: Not yet implemented.
 */
ZacoPromise* zaco_promise_new(void);

#ifdef __cplusplus
}
#endif

#endif // ZACO_RUNTIME_RS_H
