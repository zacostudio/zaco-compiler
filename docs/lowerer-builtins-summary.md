# IR Lowerer: Built-in Module Recognition and Import/Export Lowering

## Summary

Extended the IR lowerer (`crates/zaco-ir/src/lower.rs`) to recognize built-in module calls and handle imports/exports. This enables the compiler to properly lower TypeScript/JavaScript standard library calls to appropriate runtime functions.

## Changes Made

### 1. Import Tracking (`imported_bindings` HashMap)

Added a `HashMap<String, String>` to the `Lowerer` struct to track imported function names and their source modules:

```rust
/// Maps imported names to their source module
/// e.g., "readFileSync" → "fs", "join" → "path"
imported_bindings: HashMap<String, String>,
```

### 2. Import Declaration Lowering

Implemented `lower_import()` to process import declarations and populate the `imported_bindings` map:

- Handles named imports: `import { readFileSync } from "fs"`
- Handles default imports: `import fs from "fs"`
- Handles namespace imports: `import * as fs from "fs"`

### 3. Export Declaration Lowering

Implemented `lower_export()` to process export declarations:

- Marks exported functions as `is_public = true`
- Handles `export function ...`
- Handles `export default ...`

### 4. Built-in Module Recognition

Extended `lower_call()` to recognize and lower calls to built-in modules:

#### Math Module (16 methods + 2 constants)

**Methods:**
- `Math.floor(x)` → `zaco_math_floor(x)`
- `Math.ceil(x)` → `zaco_math_ceil(x)`
- `Math.round(x)` → `zaco_math_round(x)`
- `Math.abs(x)` → `zaco_math_abs(x)`
- `Math.sqrt(x)` → `zaco_math_sqrt(x)`
- `Math.pow(x, y)` → `zaco_math_pow(x, y)`
- `Math.sin(x)` → `zaco_math_sin(x)`
- `Math.cos(x)` → `zaco_math_cos(x)`
- `Math.tan(x)` → `zaco_math_tan(x)`
- `Math.log(x)` → `zaco_math_log(x)`
- `Math.log2(x)` → `zaco_math_log2(x)`
- `Math.log10(x)` → `zaco_math_log10(x)`
- `Math.random()` → `zaco_math_random()`
- `Math.min(a, b)` → `zaco_math_min(a, b)`
- `Math.max(a, b)` → `zaco_math_max(a, b)`
- `Math.trunc(x)` → `zaco_math_trunc(x)`

**Constants:**
- `Math.PI` → `Constant::F64(3.141592653589793)`
- `Math.E` → `Constant::F64(2.718281828459045)`

#### JSON Module (2 methods)

- `JSON.parse(s)` → `zaco_json_parse(s)`
- `JSON.stringify(v)` → `zaco_json_stringify(v)`

#### Console Module (extended)

- `console.log(...)` → `zaco_print_*` + `zaco_println_str`
- `console.error(...)` → `zaco_console_error_*` + `zaco_println_str`
- `console.warn(...)` → `zaco_console_warn_*` + `zaco_println_str`
- `console.info(...)` → same as `console.log`

#### Process Module (5 methods)

- `process.exit(code)` → `zaco_process_exit(code)`
- `process.cwd()` → `zaco_process_cwd()`
- `process.pid` → `zaco_process_pid()`
- `process.platform` → `zaco_process_platform()`
- `process.arch` → `zaco_process_arch()`

#### fs Module (4 functions, when imported)

- `readFileSync(path, encoding)` → `zaco_fs_read_file_sync(path, encoding)`
- `writeFileSync(path, data)` → `zaco_fs_write_file_sync(path, data)`
- `existsSync(path)` → `zaco_fs_exists_sync(path)`
- `mkdirSync(path, opts)` → `zaco_fs_mkdir_sync(path, recursive)`

#### path Module (5 functions, when imported)

- `join(a, b)` → `zaco_path_join(a, b)`
- `resolve(p)` → `zaco_path_resolve(p)`
- `dirname(p)` → `zaco_path_dirname(p)`
- `basename(p)` → `zaco_path_basename(p)`
- `extname(p)` → `zaco_path_extname(p)`

#### os Module (6 functions, when imported)

- `os.platform()` → `zaco_os_platform()`
- `os.arch()` → `zaco_os_arch()`
- `os.homedir()` → `zaco_os_homedir()`
- `os.tmpdir()` → `zaco_os_tmpdir()`
- `os.hostname()` → `zaco_os_hostname()`
- `os.cpus()` → `zaco_os_cpus()`

### 5. Extern Function Declaration

Added `ensure_extern()` helper method to automatically declare external runtime functions:

```rust
fn ensure_extern(&mut self, name: &str, params: Vec<IrType>, ret: IrType)
```

This ensures that all built-in runtime functions are properly declared in the IR module's `extern_functions` list, which codegen needs.

### 6. Helper Methods

Implemented specialized lowering methods:

- `lower_console_method()` - Handles console.log/error/warn
- `lower_math_method()` - Handles Math.* calls
- `lower_json_method()` - Handles JSON.parse/stringify
- `lower_process_method()` - Handles process.* calls
- `lower_imported_function_call()` - Handles fs/path/os imported functions

## Tests Added

Added 4 new test cases:

1. **test_lower_math_floor** - Verifies Math.floor() is lowered correctly
2. **test_lower_math_pi** - Verifies Math.PI constant is recognized
3. **test_lower_import_fs** - Verifies import tracking and fs function lowering
4. **test_lower_export_function** - Verifies exported functions are marked public

All tests pass successfully.

## Examples

Created two example files demonstrating the new functionality:

1. **examples/builtin_modules.ts** - Demonstrates Math, JSON, console, and process module usage
2. **examples/import_modules.ts** - Demonstrates import tracking for fs and path modules

## Integration

The lowerer now properly:

1. Tracks all imports and associates function names with their modules
2. Recognizes built-in global objects (Math, JSON, console, process)
3. Recognizes imported module functions (fs, path, os)
4. Automatically declares external functions in the IR module
5. Marks exported functions as public for linking

## Runtime Function Requirements

The runtime (C or Rust) must implement all the `zaco_*` functions declared by the lowerer:

- Math functions: `zaco_math_floor`, `zaco_math_ceil`, etc.
- JSON functions: `zaco_json_parse`, `zaco_json_stringify`
- Console functions: `zaco_console_error_str`, `zaco_console_warn_str`, etc.
- Process functions: `zaco_process_exit`, `zaco_process_cwd`, etc.
- fs functions: `zaco_fs_read_file_sync`, `zaco_fs_write_file_sync`, etc.
- path functions: `zaco_path_join`, `zaco_path_dirname`, etc.
- os functions: `zaco_os_platform`, `zaco_os_arch`, etc.

## Future Enhancements

Potential improvements:

1. Handle `process.env.KEY` member access (currently only handles method calls)
2. Support more Math methods (asin, acos, atan, exp, etc.)
3. Support Array/String built-in methods
4. Handle default export values (not just functions)
5. Support re-exports (`export { x } from "module"`)
6. Handle namespace imports as objects (`import * as fs` → `fs.readFileSync`)

## Testing

```bash
# Run IR lowerer tests
cargo test -p zaco-ir

# Check compilation
cargo check -p zaco-ir

# Verify integration
cargo check
```

All tests pass and the entire project compiles successfully.
