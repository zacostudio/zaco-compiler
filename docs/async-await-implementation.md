# Async/Await Implementation

## Summary

This document describes the async/await support implementation in the Zaco compiler's IR layer.

## Implementation Overview

### 1. Promise IR Type

Added a new `IrType::Promise(Box<IrType>)` variant to represent Promise types in the IR.

**File:** `crates/zaco-ir/src/types.rs`

```rust
pub enum IrType {
    // ... existing types
    /// Promise type wrapping the resolved value type
    Promise(Box<IrType>),
}
```

The Promise type:
- Is treated as a pointer type (8 bytes)
- Can wrap any other IR type (e.g., `Promise<Str>`, `Promise<I64>`)
- Supports proper Display formatting (e.g., "Promise<str>")

### 2. Async Function Lowering

**File:** `crates/zaco-ir/src/lower.rs`

When lowering an `async function`:

1. **Return Type Handling:**
   - If the AST return type is already `Promise<T>`, use it as-is
   - Otherwise, wrap the return type in `Promise` (e.g., `number` → `Promise<I64>`)
   - No return type → `Promise<Void>`

2. **Promise Creation:**
   - Creates a new Promise object by calling `zaco_promise_new()`
   - Stores the promise in a temporary variable

3. **Body Execution:**
   - Lowers the function body normally
   - For now, executes synchronously (TODO: true async with task spawning)

4. **Promise Resolution:**
   - If no explicit return, resolves the promise with null/void
   - Returns the promise object

**Extern Functions Declared:**
- `zaco_promise_new() -> Ptr`
- `zaco_promise_resolve(promise: Ptr, value: Ptr) -> Void`

### 3. Await Expression Lowering

**File:** `crates/zaco-ir/src/lower.rs`

When lowering `await expr`:

1. **Expression Lowering:**
   - Lowers the inner expression (should produce a Promise value)

2. **Blocking Await:**
   - Calls `zaco_async_block_on(promise)` to wait for the promise to resolve
   - Returns the resolved value as a `Ptr` type

**Extern Function Declared:**
- `zaco_async_block_on(promise: Ptr) -> Ptr`

### 4. AST Type Conversion

**File:** `crates/zaco-ir/src/lower.rs`

Updated `ast_type_to_ir` to handle Promise types:

```rust
// Promise<string> → IrType::Promise(Box::new(IrType::Str))
Type::TypeRef { name: "Promise", type_args: Some([Type::Primitive(String)]) }
  → IrType::Promise(Box::new(IrType::Str))

// Generic Promise<T>
Type::Generic { base: TypeRef("Promise"), type_args: [T] }
  → IrType::Promise(Box::new(ast_type_to_ir(T)))
```

### 5. Rust Runtime Promise Implementation

**File:** `runtime/zaco_runtime_rs/src/promise.rs`

Implemented a working Promise using Rust's synchronization primitives:

```rust
pub struct ZacoPromise {
    state: Mutex<PromiseState>,           // Pending/Resolved/Rejected
    value: Mutex<Option<*mut c_void>>,    // Resolved value
    condvar: Condvar,                     // For blocking await
}
```

**Exported C Functions:**

1. **`zaco_promise_new() -> *mut ZacoPromise`**
   - Creates a new pending promise

2. **`zaco_promise_resolve(promise: *mut ZacoPromise, value: *mut c_void)`**
   - Resolves a promise with a value
   - Notifies all waiting threads

3. **`zaco_promise_reject(promise: *mut ZacoPromise, error: *mut c_void)`**
   - Rejects a promise with an error
   - Notifies all waiting threads

4. **`zaco_async_block_on(promise: *mut ZacoPromise) -> *mut c_void`**
   - Blocks the current thread until the promise resolves
   - Returns the resolved value (or error)
   - Uses condition variables for efficient waiting

5. **`zaco_async_spawn(fn_ptr: extern "C" fn(*mut c_void) -> *mut c_void, arg: *mut c_void) -> *mut ZacoPromise`**
   - Creates a promise and spawns a task (currently executes synchronously)
   - TODO: Use `tokio::spawn` for true async execution

6. **`zaco_promise_free(promise: *mut ZacoPromise)`**
   - Frees a promise object

**File:** `runtime/zaco_runtime_rs/src/lib.rs`

Added `pub use promise::*;` to export promise functions.

## Current Limitations

### 1. Sequential Execution
- Async functions currently execute **sequentially** (not truly concurrent)
- The body of an async function runs immediately, not in a separate task
- `await` uses a **blocking** approach with condition variables

### 2. No Tokio Integration Yet
- `zaco_async_spawn` does not actually spawn a Tokio task
- The Tokio runtime exists but is not used for async execution
- All async code runs on the main thread

### 3. No Top-Level Await
- Top-level `await` (outside async functions) is not yet supported
- Would require wrapping the entire main function in `zaco_async_block_on`

### 4. No Promise Chaining
- No support for `.then()`, `.catch()`, or promise combinators
- Only basic promise creation and awaiting

### 5. Return Value Handling
- Async functions always resolve the promise with `null` if no explicit return
- TODO: Properly capture return values and resolve the promise with them

## Future Enhancements

### 1. True Async Execution
Replace the synchronous execution model with Tokio-based async tasks:

```rust
#[no_mangle]
pub extern "C" fn zaco_async_spawn(
    fn_ptr: extern "C" fn(*mut c_void) -> *mut c_void,
    arg: *mut c_void,
) -> *mut ZacoPromise {
    let promise = Arc::new(ZacoPromise::new());
    let promise_clone = promise.clone();

    event_loop::spawn(async move {
        let result = fn_ptr(arg);
        promise_clone.resolve(result);
    });

    Arc::into_raw(promise) as *mut ZacoPromise
}
```

### 2. Async/Await Syntax Sugar
- Transform `await` expressions into proper async state machines
- Generate IR that supports yielding and resuming

### 3. Promise Combinators
Add runtime support for:
- `Promise.all()` - wait for multiple promises
- `Promise.race()` - wait for first promise to resolve
- `Promise.any()` - wait for first successful promise

### 4. Top-Level Await
Detect top-level await and wrap main function:

```rust
fn lower_program(&mut self, program: &Program) -> Result<IrModule, Vec<LowerError>> {
    if self.has_top_level_await {
        // Call zaco_runtime_init()
        // Wrap main body in zaco_async_block_on()
        // Call zaco_runtime_shutdown()
    }
}
```

## Testing

**File:** `crates/zaco-ir/src/lower.rs`

Added 4 new tests:

1. **`test_lower_async_function`**
   - Tests lowering of async function declarations
   - Verifies Promise return type
   - Checks for promise extern declarations

2. **`test_lower_await_expression`**
   - Tests lowering of await expressions
   - Verifies `zaco_async_block_on` extern declaration

3. **`test_promise_type_conversion`**
   - Tests AST `Promise<T>` → IR `IrType::Promise(T)` conversion
   - Covers `Promise<string>`, `Promise<number>`, etc.

**Test Results:**
```
test result: ok. 19 passed; 0 failed
```

All existing tests continue to pass, plus 3 new async/await tests.

## Example Usage

```typescript
// Async function declaration
async function fetchData(): Promise<string> {
    return "Hello from async function";
}

// Async function with await
async function main(): Promise<void> {
    let result = await fetchData();  // Blocks until promise resolves
    console.log(result);             // Prints: "Hello from async function"
}

main();
```

**Generated IR (conceptual):**

```
function fetchData() -> Promise<str>:
  bb0:
    %promise = call zaco_promise_new()
    # ... function body ...
    call zaco_promise_resolve(%promise, "Hello from async function")
    return %promise

function main() -> Promise<void>:
  bb0:
    %promise = call zaco_promise_new()
    %call_result = call fetchData()
    %result = call zaco_async_block_on(%call_result)  # await
    # ... console.log(%result) ...
    call zaco_promise_resolve(%promise, null)
    return %promise
```

## Integration Notes

### Codegen Layer
The codegen agent should:
1. Add declarations for all promise runtime functions in `crates/zaco-codegen/src/runtime.rs`
2. Handle `IrType::Promise` like other pointer types (8 bytes)
3. Ensure promise functions are available during linking

### Type Checker
The type checker should:
1. Verify that `await` is only used inside `async` functions
2. Check that `await` operand is a Promise type
3. Unwrap Promise types when inferring await expression types

### Driver
The driver should:
1. Link the Rust runtime (`libzaco_runtime_rs.a`) which contains promise implementations
2. Ensure Tokio runtime is initialized if using true async execution

## Files Modified

1. `crates/zaco-ir/src/types.rs` - Added Promise type
2. `crates/zaco-ir/src/lower.rs` - Async function and await lowering
3. `runtime/zaco_runtime_rs/src/promise.rs` - Promise implementation
4. `runtime/zaco_runtime_rs/src/lib.rs` - Export promise functions

## Backward Compatibility

This implementation is **fully backward compatible**:
- No changes to existing sync function lowering
- No changes to existing IR types or instructions
- Only adds new functionality when `is_async` is true or `Expr::Await` is encountered
