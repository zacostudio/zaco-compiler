# Zaco Rust Runtime

Node.js-compatible runtime library for the Zaco TypeScript compiler, written in Rust with Tokio for async I/O.

## Overview

This crate provides runtime support for Node.js APIs in Zaco-compiled TypeScript programs:

- **Event Loop**: Tokio-based async runtime
- **File System**: `fs` module (sync + async operations)
- **Path**: `path` module (all standard operations)
- **Process**: `process` module (exit, cwd, env, argv, pid, platform, arch)
- **OS**: `os` module (platform, arch, homedir, tmpdir, hostname, cpus, totalmem, EOL)
- **HTTP**: `http` module (stub - to be implemented)
- **Events**: `EventEmitter` (stub - to be implemented)
- **Promises**: Promise infrastructure (stub - to be implemented)

## Building

```bash
# Build the static library
cargo build --release

# Or use the build script
./build.sh
```

Output: `target/release/libzaco_runtime_rs.a` (18MB static library)

## Linking

### macOS

```bash
cc -o program program.c target/release/libzaco_runtime_rs.a \
   -framework CoreFoundation -framework Security -lpthread -ldl
```

### Linux

```bash
cc -o program program.c target/release/libzaco_runtime_rs.a \
   -lpthread -ldl -lm
```

### Windows (MSVC)

```cmd
link /OUT:program.exe program.obj libzaco_runtime_rs.lib ws2_32.lib advapi32.lib userenv.lib
```

## API Reference

### Runtime Management

```c
void zaco_runtime_init(void);       // Initialize Tokio runtime (call once at startup)
void zaco_runtime_shutdown(void);   // Shutdown runtime (call at program exit)
```

### Path Module

```c
char* zaco_path_join(const char* a, const char* b);
char* zaco_path_resolve(const char* p);
char* zaco_path_dirname(const char* p);
char* zaco_path_basename(const char* p);
char* zaco_path_extname(const char* p);
long long zaco_path_is_absolute(const char* p);
char* zaco_path_normalize(const char* p);
char* zaco_path_sep(void);
```

### File System Module (Sync)

```c
char* zaco_fs_read_file_sync(const char* path, const char* encoding);
long long zaco_fs_write_file_sync(const char* path, const char* data);
long long zaco_fs_exists_sync(const char* path);
long long zaco_fs_mkdir_sync(const char* path, long long recursive);
long long zaco_fs_rmdir_sync(const char* path);
long long zaco_fs_unlink_sync(const char* path);
long long zaco_fs_stat_size(const char* path);
long long zaco_fs_stat_is_file(const char* path);
long long zaco_fs_stat_is_dir(const char* path);
char* zaco_fs_readdir_sync(const char* path);  // Returns newline-separated list
```

### File System Module (Async)

```c
void zaco_fs_read_file_async(const char* path, const char* encoding, long long callback_id);
// More async functions to be added
```

### Process Module

```c
void zaco_process_exit(long long code);
char* zaco_process_cwd(void);
char* zaco_process_env_get(const char* key);
long long zaco_process_pid(void);
char* zaco_process_platform(void);
char* zaco_process_arch(void);
char* zaco_process_argv(void);  // Returns newline-separated args
```

### OS Module

```c
char* zaco_os_platform(void);
char* zaco_os_arch(void);
char* zaco_os_homedir(void);
char* zaco_os_tmpdir(void);
char* zaco_os_hostname(void);
long long zaco_os_cpus(void);
long long zaco_os_totalmem(void);
char* zaco_os_eol(void);
```

## Memory Management

**Important**: All functions returning `char*` allocate memory using `CString::into_raw()`. The caller is responsible for freeing this memory using `free()`.

Example:

```c
char* result = zaco_path_join("/usr", "local");
printf("Result: %s\n", result);
free(result);  // Must free!
```

## Testing

```bash
# Build and run the test program
cc -o test_runtime test_runtime.c target/release/libzaco_runtime_rs.a \
   -framework CoreFoundation -framework Security -lpthread -ldl

./test_runtime
```

Expected output:
```
=== Zaco Rust Runtime Test ===

1. Initializing Tokio runtime...
   ✓ Runtime initialized

2. Testing path module:
   ...
   ✓ Path operations working

3. Testing process module:
   ...
   ✓ Process operations working

4. Testing os module:
   ...
   ✓ OS operations working

5. Testing fs module:
   ...
   ✓ FS operations working

6. Shutting down runtime...
   ✓ Runtime shutdown complete

=== All tests passed! ===
```

## Implementation Status

### Fully Implemented
- ✅ Path module (all operations)
- ✅ Process module (cwd, env, argv, pid, platform, arch, exit)
- ✅ OS module (platform, arch, homedir, tmpdir, hostname, cpus, totalmem, EOL)
- ✅ File System (sync operations)
- ✅ Tokio runtime initialization

### Partially Implemented
- 🚧 File System (async operations - basic structure in place, callback integration needed)

### Stub/TODO
- ⏳ HTTP module (needs hyper integration)
- ⏳ Events module (EventEmitter)
- ⏳ Promise module (state machine integration with async/await lowering)

## Architecture

```
┌─────────────────────────────────────────┐
│  Zaco Compiled Code (Cranelift)        │
│  - TypeScript → IR → Native Code       │
└─────────────────┬───────────────────────┘
                  │
                  │ FFI calls
                  │
┌─────────────────▼───────────────────────┐
│  Rust Runtime (libzaco_runtime_rs.a)   │
│  ┌──────────────────────────────────┐  │
│  │  Event Loop (Tokio)              │  │
│  └──────────────────────────────────┘  │
│  ┌──────────────────────────────────┐  │
│  │  Node.js API Modules             │  │
│  │  - fs, path, process, os, etc    │  │
│  └──────────────────────────────────┘  │
└─────────────────┬───────────────────────┘
                  │
                  │ System calls
                  │
┌─────────────────▼───────────────────────┐
│  Operating System                       │
│  - File I/O, Networking, etc            │
└─────────────────────────────────────────┘
```

## Dependencies

- `tokio` 1.x (full features) - Async runtime
- `libc` 0.2 - System call bindings

## Integration with Zaco Compiler

The compiler's codegen phase will:

1. Generate calls to `zaco_runtime_init()` at program startup
2. Lower Node.js API calls to FFI calls to this runtime
3. Link this static library alongside the C runtime (`zaco_runtime.c`)
4. Generate calls to `zaco_runtime_shutdown()` before program exit

Example lowering:

```typescript
// TypeScript
const content = fs.readFileSync('file.txt', 'utf8');
```

```c
// Lowered to (pseudo-IR)
char* content = zaco_fs_read_file_sync("file.txt", "utf8");
// ... use content ...
free(content);
```

## Platform Support

- ✅ macOS (aarch64, x86_64)
- ✅ Linux (x86_64, aarch64)
- 🚧 Windows (needs testing)

## License

MIT
