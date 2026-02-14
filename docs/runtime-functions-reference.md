# Runtime Functions Reference

This document lists all runtime functions that the IR lowerer expects to be implemented in the runtime (C or Rust).

## Math Functions (16 functions)

| TypeScript Call | Runtime Function | Parameters | Return Type |
|----------------|------------------|------------|-------------|
| `Math.floor(x)` | `zaco_math_floor` | `f64` | `f64` |
| `Math.ceil(x)` | `zaco_math_ceil` | `f64` | `f64` |
| `Math.round(x)` | `zaco_math_round` | `f64` | `f64` |
| `Math.abs(x)` | `zaco_math_abs` | `f64` | `f64` |
| `Math.sqrt(x)` | `zaco_math_sqrt` | `f64` | `f64` |
| `Math.pow(x, y)` | `zaco_math_pow` | `f64, f64` | `f64` |
| `Math.sin(x)` | `zaco_math_sin` | `f64` | `f64` |
| `Math.cos(x)` | `zaco_math_cos` | `f64` | `f64` |
| `Math.tan(x)` | `zaco_math_tan` | `f64` | `f64` |
| `Math.log(x)` | `zaco_math_log` | `f64` | `f64` |
| `Math.log2(x)` | `zaco_math_log2` | `f64` | `f64` |
| `Math.log10(x)` | `zaco_math_log10` | `f64` | `f64` |
| `Math.random()` | `zaco_math_random` | - | `f64` |
| `Math.min(a, b)` | `zaco_math_min` | `f64, f64` | `f64` |
| `Math.max(a, b)` | `zaco_math_max` | `f64, f64` | `f64` |
| `Math.trunc(x)` | `zaco_math_trunc` | `f64` | `f64` |

## JSON Functions (2 functions)

| TypeScript Call | Runtime Function | Parameters | Return Type |
|----------------|------------------|------------|-------------|
| `JSON.parse(s)` | `zaco_json_parse` | `const char*` | `const char*` |
| `JSON.stringify(v)` | `zaco_json_stringify` | `void*` | `const char*` |

## Console Functions (12 functions - 3 methods × 4 types)

### console.log / console.info
| TypeScript Call | Runtime Function | Parameters | Return Type |
|----------------|------------------|------------|-------------|
| `console.log(str)` | `zaco_print_str` | `const char*` | `void` |
| `console.log(num)` | `zaco_print_i64` | `int64_t` | `void` |
| `console.log(num)` | `zaco_print_f64` | `double` | `void` |
| `console.log(bool)` | `zaco_print_bool` | `bool` | `void` |

### console.error
| TypeScript Call | Runtime Function | Parameters | Return Type |
|----------------|------------------|------------|-------------|
| `console.error(str)` | `zaco_console_error_str` | `const char*` | `void` |
| `console.error(num)` | `zaco_console_error_i64` | `int64_t` | `void` |
| `console.error(num)` | `zaco_console_error_f64` | `double` | `void` |
| `console.error(bool)` | `zaco_console_error_bool` | `bool` | `void` |

### console.warn
| TypeScript Call | Runtime Function | Parameters | Return Type |
|----------------|------------------|------------|-------------|
| `console.warn(str)` | `zaco_console_warn_str` | `const char*` | `void` |
| `console.warn(num)` | `zaco_console_warn_i64` | `int64_t` | `void` |
| `console.warn(num)` | `zaco_console_warn_f64` | `double` | `void` |
| `console.warn(bool)` | `zaco_console_warn_bool` | `bool` | `void` |

### Newline
| TypeScript Call | Runtime Function | Parameters | Return Type |
|----------------|------------------|------------|-------------|
| (internal) | `zaco_println_str` | `const char*` | `void` |

## Process Functions (5 functions)

| TypeScript Call | Runtime Function | Parameters | Return Type |
|----------------|------------------|------------|-------------|
| `process.exit(code)` | `zaco_process_exit` | `int64_t` | `void` |
| `process.cwd()` | `zaco_process_cwd` | - | `const char*` |
| `process.pid` | `zaco_process_pid` | - | `int64_t` |
| `process.platform` | `zaco_process_platform` | - | `const char*` |
| `process.arch` | `zaco_process_arch` | - | `const char*` |

## fs Module Functions (4 functions)

| TypeScript Call | Runtime Function | Parameters | Return Type |
|----------------|------------------|------------|-------------|
| `readFileSync(path, enc)` | `zaco_fs_read_file_sync` | `const char*, const char*` | `const char*` |
| `writeFileSync(path, data)` | `zaco_fs_write_file_sync` | `const char*, const char*` | `void` |
| `existsSync(path)` | `zaco_fs_exists_sync` | `const char*` | `bool` |
| `mkdirSync(path, opts)` | `zaco_fs_mkdir_sync` | `const char*, int64_t` | `void` |

## path Module Functions (5 functions)

| TypeScript Call | Runtime Function | Parameters | Return Type |
|----------------|------------------|------------|-------------|
| `join(a, b)` | `zaco_path_join` | `const char*, const char*` | `const char*` |
| `resolve(p)` | `zaco_path_resolve` | `const char*` | `const char*` |
| `dirname(p)` | `zaco_path_dirname` | `const char*` | `const char*` |
| `basename(p)` | `zaco_path_basename` | `const char*` | `const char*` |
| `extname(p)` | `zaco_path_extname` | `const char*` | `const char*` |

## os Module Functions (6 functions)

| TypeScript Call | Runtime Function | Parameters | Return Type |
|----------------|------------------|------------|-------------|
| `os.platform()` | `zaco_os_platform` | - | `const char*` |
| `os.arch()` | `zaco_os_arch` | - | `const char*` |
| `os.homedir()` | `zaco_os_homedir` | - | `const char*` |
| `os.tmpdir()` | `zaco_os_tmpdir` | - | `const char*` |
| `os.hostname()` | `zaco_os_hostname` | - | `const char*` |
| `os.cpus()` | `zaco_os_cpus` | - | `void*` |

## Total Functions Required

- **Math**: 16 functions
- **JSON**: 2 functions
- **Console**: 13 functions (including println)
- **Process**: 5 functions
- **fs**: 4 functions
- **path**: 5 functions
- **os**: 6 functions

**Total: 51 runtime functions**

## Implementation Notes

1. All string parameters and return values are `const char*` (C strings)
2. Numeric parameters use `int64_t` for integers and `double` for floats
3. All functions returning strings must return heap-allocated strings (caller responsible for freeing)
4. The `mkdirSync` recursive parameter is mapped to `int64_t` (0 = false, 1 = true)
5. The `os.cpus()` function returns an opaque pointer (array of CPU info objects)

## Example C Runtime Stub

```c
#include <stdio.h>
#include <stdlib.h>
#include <math.h>

// Math functions
double zaco_math_floor(double x) { return floor(x); }
double zaco_math_ceil(double x) { return ceil(x); }
double zaco_math_round(double x) { return round(x); }
// ... etc

// Console functions
void zaco_print_str(const char* s) { printf("%s", s); }
void zaco_print_i64(int64_t n) { printf("%lld", n); }
void zaco_print_f64(double n) { printf("%f", n); }
void zaco_print_bool(bool b) { printf("%s", b ? "true" : "false"); }
void zaco_println_str(const char* s) { printf("%s\n", s); }
// ... etc

// Process functions
void zaco_process_exit(int64_t code) { exit((int)code); }
const char* zaco_process_cwd() { /* impl */ }
// ... etc
```

## Example Rust Runtime Stub

```rust
#[no_mangle]
pub extern "C" fn zaco_math_floor(x: f64) -> f64 {
    x.floor()
}

#[no_mangle]
pub extern "C" fn zaco_print_str(s: *const i8) {
    let s = unsafe { std::ffi::CStr::from_ptr(s) };
    print!("{}", s.to_string_lossy());
}

// ... etc
```
