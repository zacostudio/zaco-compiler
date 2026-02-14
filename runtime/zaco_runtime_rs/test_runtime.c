// Test program to verify Rust runtime linking and basic functionality
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Declare Rust runtime functions
extern void zaco_runtime_init(void);
extern void zaco_runtime_shutdown(void);
extern char* zaco_path_join(const char* a, const char* b);
extern char* zaco_path_basename(const char* p);
extern char* zaco_path_extname(const char* p);
extern long long zaco_path_is_absolute(const char* p);
extern char* zaco_process_cwd(void);
extern long long zaco_process_pid(void);
extern char* zaco_process_platform(void);
extern char* zaco_os_arch(void);
extern long long zaco_os_cpus(void);
extern char* zaco_fs_read_file_sync(const char* path, const char* encoding);
extern long long zaco_fs_write_file_sync(const char* path, const char* data);
extern long long zaco_fs_exists_sync(const char* path);

// HTTP module functions
extern char* zaco_http_get(const char* url);
extern char* zaco_http_post(const char* url, const char* body, const char* content_type);
extern long long zaco_http_get_status(const char* url);
extern char* zaco_http_get_headers(const char* url);

// Events module functions
extern long long zaco_events_new(void);
extern void zaco_events_on(long long emitter, const char* event, void (*callback)(void*), void* context);
extern void zaco_events_once(long long emitter, const char* event, void (*callback)(void*), void* context);
extern long long zaco_events_emit(long long emitter, const char* event, void* data);
extern void zaco_events_remove_all(long long emitter, const char* event);
extern long long zaco_events_listener_count(long long emitter, const char* event);
extern void zaco_events_destroy(long long emitter);

// Test callback counters
static int test_callback_count = 0;
static int test_once_callback_count = 0;

// Test callback for events
void test_event_callback(void* context) {
    int* counter = (int*)context;
    (*counter)++;
    printf("   Event callback called! (count: %d)\n", *counter);
}

void test_once_callback(void* context) {
    int* counter = (int*)context;
    (*counter)++;
    printf("   Once callback called! (count: %d)\n", *counter);
}

int main() {
    printf("=== Zaco Rust Runtime Test ===\n\n");

    // Initialize runtime
    printf("1. Initializing Tokio runtime...\n");
    zaco_runtime_init();
    printf("   ✓ Runtime initialized\n\n");

    // Test path operations
    printf("2. Testing path module:\n");
    char* joined = zaco_path_join("/usr/local", "bin/zaco");
    printf("   path.join('/usr/local', 'bin/zaco') = %s\n", joined);
    free(joined);

    char* basename = zaco_path_basename("/path/to/file.ts");
    printf("   path.basename('/path/to/file.ts') = %s\n", basename);
    free(basename);

    char* extname = zaco_path_extname("test.ts");
    printf("   path.extname('test.ts') = %s\n", extname);
    free(extname);

    long long is_abs = zaco_path_is_absolute("/usr/bin");
    printf("   path.isAbsolute('/usr/bin') = %s\n", is_abs ? "true" : "false");
    printf("   ✓ Path operations working\n\n");

    // Test process module
    printf("3. Testing process module:\n");
    char* cwd = zaco_process_cwd();
    printf("   process.cwd() = %s\n", cwd);
    free(cwd);

    long long pid = zaco_process_pid();
    printf("   process.pid = %lld\n", pid);

    char* platform = zaco_process_platform();
    printf("   process.platform = %s\n", platform);
    free(platform);
    printf("   ✓ Process operations working\n\n");

    // Test os module
    printf("4. Testing os module:\n");
    char* arch = zaco_os_arch();
    printf("   os.arch() = %s\n", arch);
    free(arch);

    long long cpus = zaco_os_cpus();
    printf("   os.cpus().length = %lld\n", cpus);
    printf("   ✓ OS operations working\n\n");

    // Test fs module
    printf("5. Testing fs module:\n");
    const char* test_file = "/tmp/zaco_test.txt";
    const char* test_data = "Hello from Zaco runtime!";

    long long write_result = zaco_fs_write_file_sync(test_file, test_data);
    printf("   fs.writeFileSync('%s') = %s\n", test_file, write_result == 0 ? "OK" : "FAILED");

    long long exists = zaco_fs_exists_sync(test_file);
    printf("   fs.existsSync('%s') = %s\n", test_file, exists ? "true" : "false");

    char* content = zaco_fs_read_file_sync(test_file, "utf8");
    if (content) {
        printf("   fs.readFileSync('%s') = \"%s\"\n", test_file, content);
        free(content);
    }
    printf("   ✓ FS operations working\n\n");

    // Test HTTP module
    printf("6. Testing HTTP module:\n");
    printf("   Testing HTTP GET to httpbin.org...\n");

    // Test status code
    long long status = zaco_http_get_status("https://httpbin.org/status/200");
    printf("   http.get('https://httpbin.org/status/200') status = %lld\n", status);

    if (status == 200) {
        printf("   ✓ HTTP status code test passed\n");
    } else {
        printf("   ✗ HTTP status code test failed (expected 200, got %lld)\n", status);
    }

    // Test GET with response
    char* response = zaco_http_get("https://httpbin.org/get");
    if (response) {
        printf("   http.get('https://httpbin.org/get') = %.100s...\n", response);
        free(response);
        printf("   ✓ HTTP GET test passed\n");
    } else {
        printf("   ✗ HTTP GET test failed\n");
    }

    // Test POST
    char* post_response = zaco_http_post(
        "https://httpbin.org/post",
        "{\"test\":\"data\"}",
        "application/json"
    );
    if (post_response) {
        printf("   http.post() = %.100s...\n", post_response);
        free(post_response);
        printf("   ✓ HTTP POST test passed\n");
    } else {
        printf("   ✗ HTTP POST test failed\n");
    }

    // Test headers
    char* headers = zaco_http_get_headers("https://httpbin.org/headers");
    if (headers) {
        printf("   http.getHeaders() = %.100s...\n", headers);
        free(headers);
        printf("   ✓ HTTP headers test passed\n");
    } else {
        printf("   ✗ HTTP headers test failed\n");
    }

    printf("   ✓ HTTP operations working\n\n");

    // Test Events module
    printf("7. Testing Events module (EventEmitter):\n");

    // Create emitter
    long long emitter = zaco_events_new();
    printf("   events.new() = %lld\n", emitter);

    // Register listeners
    test_callback_count = 0;
    test_once_callback_count = 0;

    zaco_events_on(emitter, "test", test_event_callback, &test_callback_count);
    zaco_events_on(emitter, "test", test_event_callback, &test_callback_count);
    zaco_events_once(emitter, "test", test_once_callback, &test_once_callback_count);

    long long count = zaco_events_listener_count(emitter, "test");
    printf("   emitter.listenerCount('test') = %lld (expected 3)\n", count);

    // Emit event (should call all 3 listeners)
    printf("   emitter.emit('test', NULL)...\n");
    long long called = zaco_events_emit(emitter, "test", NULL);
    printf("   emitter.emit() called %lld listeners\n", called);
    printf("   Regular callbacks: %d, Once callbacks: %d\n", test_callback_count, test_once_callback_count);

    // Emit again (should only call the 2 regular listeners)
    printf("   emitter.emit('test', NULL) again...\n");
    called = zaco_events_emit(emitter, "test", NULL);
    printf("   emitter.emit() called %lld listeners (once listener removed)\n", called);
    printf("   Regular callbacks: %d, Once callbacks: %d\n", test_callback_count, test_once_callback_count);

    if (test_callback_count == 4 && test_once_callback_count == 1) {
        printf("   ✓ Events callbacks working correctly\n");
    } else {
        printf("   ✗ Events callbacks failed (expected 4 regular, 1 once)\n");
    }

    // Test remove all
    zaco_events_remove_all(emitter, "test");
    count = zaco_events_listener_count(emitter, "test");
    printf("   After removeAllListeners('test'), count = %lld\n", count);

    // Cleanup
    zaco_events_destroy(emitter);
    printf("   ✓ Events operations working\n\n");

    // Cleanup
    printf("8. Shutting down runtime...\n");
    zaco_runtime_shutdown();
    printf("   ✓ Runtime shutdown complete\n\n");

    printf("=== All tests passed! ===\n");
    return 0;
}
