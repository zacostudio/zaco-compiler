# Changelog

All notable changes to the Zaco Compiler project are documented in this file.

## [Unreleased]

### Session 1 — Code Review & Language Feature Implementation

#### New Language Features (9 features)

- **Logical AND/OR short-circuit** (`&&`, `||`): Conditional branches with JS/TS semantics — returns operand value, not just boolean
- **Switch statement**: If-else chain lowering with fall-through semantics and `break_stack` support
- **For-in / For-of loops**: Array iteration via `zaco_array_length` / `zaco_array_get` runtime calls
- **Try/Catch/Finally**: Exception handling using `setjmp`/`longjmp` (MAX_TRY_DEPTH=64, single-threaded)
- **Throw statement**: Calls `zaco_throw` which `longjmp`s to nearest try context
- **Classes**: Struct-based representation with prefixed functions, constructors, methods, field access, inheritance (`extends`), `super` calls, method override/forwarding
- **Closures**: Heap-allocated environment struct, capture by value, `Array.map`/`filter`/`forEach` callbacks
- **Ternary operator** (`cond ? then : else`): Branch-based conditional expression lowering that yields a value
- **String equality**: `==`/`!=` on strings uses `zaco_str_eq` runtime call instead of pointer comparison

#### Bug Fixes — Type Checker (10 fixes)

- **Param type annotation**: `resolve_param_type()` checks both `Pattern::Ident.type_annotation` and `Param.type_annotation`
- **TypeRef member access**: `check_member` resolves `TypeRef` to `Class`/`Interface` type for property lookup
- **Unresolved TypeRef assignability**: Generic type params (`T`, `U`) treated as compatible with any type
- **Class inheritance in typeck**: `check_class_decl` inherits parent fields/methods via `extends`
- **Return type validation**: Validates return expressions against declared return types
- **Duplicate detection**: `let`/`const` redeclaration in same scope produces error (`var` allowed)
- **Type::TypeRef**: Now carries `{ name, type_args }` — preserves generic arguments
- **Type::Promise**: `Promise(Box<Type>)` canonical form, `await` unwraps correctly
- **is_assignable**: Handles Union, TypeRef resolution, Function (contravariant params), Promise covariance
- **Async return type**: Type checker unwraps Promise wrapper when checking return value in async functions

#### Bug Fixes — IR Lowerer (10 fixes)

- **String + number concat**: Converts f64 to string via `zaco_f64_to_str` before `StrConcat`
- **Method return type inference**: `infer_expr_type` looks up class method return types for `obj.method()` calls
- **Class param type resolution**: `ast_type_to_ir` resolves class names to `IrType::Struct(struct_id)`
- **Recursive call return type**: `current_function` tracks name/return_type for self-calls
- **Default return value type**: Uses `F64(0.0)` for number return types instead of `I64(0)`
- **main() name conflict**: User-defined `main()` renamed to `_user_main` to avoid collision with compiler wrapper
- **Boolean printing**: `infer_expr_type` handles `Unary(Not)` → `Bool` and `Ternary` → then_branch type
- **Cranelift verifier i8/i64 mismatch**: `UnaryOp::Not` uses `BinaryOp::Eq`/`Ne` comparison with `I64(0)` instead of direct `i8`

#### Bug Fixes — Codegen (3 fixes)

- **Cranelift verifier**: Pre-verify with `cranelift::codegen::verify_function` for detailed error messages
- **zaco_str_eq declaration**: Added Cranelift function declaration for string content comparison
- **seal_all_blocks()**: Used for arbitrary CFG shapes from switch/try-catch

#### Bug Fixes — Runtime (2 fixes)

- **Number formatting**: `zaco_print_f64` detects whole numbers and prints without decimal/scientific notation
- **parseInt/parseFloat/isNaN/isFinite**: Added Cranelift declarations + lowerer mappings for global functions

### Session 2 — Module Resolution & Integration Tests

#### Bug Fixes — Module System (3 fixes)

- **Missing import = compile error**: `ResolvedModule::PackageNotFound` changed from warning (skip) to compile error with descriptive message
- **package.json number parsing**: Added `JsonValue::Number(f64)` variant and `parse_number()` to handle integers, negatives, and decimals in package.json
- **Extensionless entry resolution**: `types`/`module`/`main`/`exports` fields in package.json now fall back to `try_resolve_file()` for paths without extensions

#### Tests

- Added 4 regression tests for module resolution (npm_resolver, package_json)
- Test suite: 107 → 118 tests

### Session 3 — Built-in Module Alignment & Array Deallocation

#### Built-in Module Consistency (4 files)

- **typeck/builtins.rs**: Added `http` module registration (get/post/put/delete) and `events` module registration (EventEmitter as opaque type)
- **ir/lower.rs**: Added `process` module mappings (exit, cwd) and `http` module mappings (get/post/put/delete) to `lower_imported_function_call()`
- **driver/resolver.rs**: Removed unsupported modules from `is_builtin()` (https, math, json, util, stream, buffer, crypto, url). Only fs/path/http/os/process/events remain
- **codegen/runtime.rs**: Added Cranelift function declarations for `zaco_http_get`, `zaco_http_post`, `zaco_http_put`, `zaco_http_delete`

#### Array Deallocation (2 files)

- **codegen/runtime.rs**: Added `zaco_array_rc_dec` field to `RuntimeFunctions` struct with Cranelift function declaration
- **codegen/translator.rs**: `RefCountAdjust` handler now checks value type — calls `zaco_array_rc_dec` for arrays instead of generic `zaco_rc_dec`

#### Tests

- Added 2 built-in module export tests (http, events)
- Test suite: 118 → 120 tests

### Session 4 — Multi-Module Compilation & FuncId Collision

#### Multi-Module System Overhaul (3 files)

- **ir/lower.rs**: Added `module_name` field + `with_module_name()` builder — non-entry modules generate `__module_init_<name>` wrapper instead of `main`
- **ir/lower.rs**: Added `with_func_id_offset()` / `with_struct_id_offset()` builder methods — each module gets unique ID ranges to prevent FuncId/StructId collisions across modules
- **ir/module.rs**: Added `next_func_id` / `next_struct_id` fields to `IrModule` — lowerer writes final ID counters so driver can compute offsets for next module
- **driver/main.rs**: `merge_ir_modules()` removed name-based function deduplication (kept extern declaration dedup only)
- **driver/main.rs**: `inject_module_init_calls()` prepends calls to all `__module_init_*` functions at start of `main`'s entry block
- **driver/main.rs**: Multi-module compilation loop tracks running FuncId/StructId offsets across modules
- **driver/main.rs**: `module_path_to_init_name()` includes parent directory to avoid collisions (`a/index.ts` → `a_index`)

#### Imported Function Type Inference (1 file)

- **ir/lower.rs**: Extracted `imported_func_signature()` helper method from `lower_imported_function_call()`
- **ir/lower.rs**: `infer_expr_type()` for `Expr::Call` now checks `imported_bindings` and looks up return type via `imported_func_signature()` — fixes `console.log(get("url"))` using wrong print function

### Summary

| Metric | Before | After |
|--------|--------|-------|
| Test count | 107 | 120 |
| Language features | basic (vars, arithmetic, functions, if/else, while, for, async/await) | +switch, for-in/for-of, try/catch/finally, throw, classes, closures, &&/\|\|, ternary, string equality |
| Built-in modules (full pipeline) | fs, path, os | fs, path, os, process, http, events |
| Module compilation | single-file only | multi-module with init ordering and unique IDs |
| Bug fixes | — | 28 across typeck, IR, codegen, driver, runtime |

### Files Changed

- `crates/zaco-ast/src/` — minor AST node additions for new features
- `crates/zaco-lexer/src/` — no changes
- `crates/zaco-parser/src/` — switch, for-in/for-of, try/catch, class, closure parsing
- `crates/zaco-typeck/src/` — builtins registry, assignability, return type validation, class inheritance
- `crates/zaco-ir/src/lower.rs` — 9 new feature lowerings, 10 bug fixes, multi-module support, imported call type inference
- `crates/zaco-ir/src/module.rs` — IrModule ID offset fields
- `crates/zaco-codegen/src/lib.rs` — verifier integration
- `crates/zaco-codegen/src/translator.rs` — array-specific RC handling, seal_all_blocks
- `crates/zaco-codegen/src/runtime.rs` — 10+ new Cranelift function declarations
- `crates/zaco-driver/src/main.rs` — multi-module merge, init injection, FuncId offsets, compile error handling
- `crates/zaco-driver/src/resolver.rs` — built-in list cleanup, PackageNotFound error propagation
- `crates/zaco-driver/src/package_json.rs` — JSON number parsing
- `crates/zaco-driver/src/npm_resolver.rs` — extensionless entry resolution fallback
- `runtime/zaco_runtime.c` — number formatting, array RC functions
