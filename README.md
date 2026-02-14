# Zaco Compiler

A TypeScript-to-native compiler with Rust-like ownership semantics. Zaco extends TypeScript syntax with ownership annotations (`owned`, `ref`, `mut ref`, `clone`) to catch memory bugs at compile time, then compiles to native code via Cranelift — no garbage collector needed.

## Features

- **TypeScript-compatible syntax** — parses classes, interfaces, generics, arrow functions, destructuring, async/await, decorators, and more
- **Ownership tracking** — Rust-inspired `owned`, `ref`, `mut ref`, `clone` annotations with compile-time move/borrow checking
- **Native compilation** — full pipeline from TypeScript source to native executable via Cranelift (no LLVM dependency)
- **Module system** — Node.js-compatible module resolution with import/export, circular dependency detection, and multi-file compilation
- **Built-in standard library** — Math, JSON, console, String/Array methods implemented in C runtime
- **Node.js core modules** — fs, path, process, os implemented in Rust (Tokio-based) for async I/O support
- **Rich error reporting** — colorful, Rust-style error messages with source spans via ariadne

## Architecture

```
Source (.ts)
    │
    ▼
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  Lexer   │───▶│  Parser  │───▶│ Type     │───▶│  IR      │───▶│ Codegen  │───▶│  Linker  │
│ (zaco-   │    │ (zaco-   │    │ Checker  │    │ Lowering │    │ (zaco-   │    │          │
│  lexer)  │    │  parser) │    │ (zaco-   │    │ (zaco-ir)│    │  codegen)│    │          │
└──────────┘    └──────────┘    │  typeck) │    └──────────┘    └──────────┘    └──────────┘
                                └──────────┘
```

The compiler is organized as a Rust workspace with 7 crates:

| Crate | Description |
|-------|-------------|
| `zaco-ast` | AST node definitions, types, spans |
| `zaco-lexer` | Tokenizer for TypeScript + Zaco extensions |
| `zaco-parser` | Recursive descent parser with Pratt expression parsing |
| `zaco-typeck` | Type checker with ownership inference and tracking |
| `zaco-ir` | Intermediate representation (CFG-based) + AST-to-IR lowering |
| `zaco-codegen` | Cranelift native code generation |
| `zaco-driver` | CLI frontend, module resolver, and compilation pipeline |

### Runtime

```
runtime/
├── zaco_runtime.c              # C runtime (memory, strings, arrays, Math, JSON, console)
└── zaco_runtime_rs/            # Rust runtime (Tokio-based async I/O)
    └── src/
        ├── lib.rs              # FFI exports
        ├── event_loop.rs       # Tokio event loop
        ├── fs.rs               # File system (sync + async)
        ├── path.rs             # Path operations
        ├── process_api.rs      # Process API
        ├── os.rs               # OS info
        ├── http.rs             # HTTP (stub)
        ├── promise.rs          # Promise (stub)
        └── events.rs           # EventEmitter (stub)
```

## Installation

### Prerequisites

- Rust 1.75+ (2021 edition)
- C compiler (cc/gcc/clang) for linking the runtime

### Build

```bash
cd zaco-compiler
cargo build --release
```

The binary is produced at `target/release/zaco`.

## Usage

### Compile to native executable

```bash
# Compile to executable
zaco compile input.ts -o output --emit exe

# Run the executable
./output
```

### Other emission modes

```bash
# Emit AST for debugging
zaco compile input.ts --emit ast

# Emit IR for debugging
zaco compile input.ts --emit ir

# Emit object file only
zaco compile input.ts -o output --emit obj

# Verbose mode (shows each compilation phase)
zaco compile input.ts -o output --emit exe -v
```

### Type check only

```bash
# Check types without producing output
zaco check input.ts

# Verbose type checking
zaco check input.ts -v
```

### Debug commands

```bash
# Show lexer tokens
zaco lex input.ts

# Show tokens with positions
zaco lex input.ts -p

# Show parsed AST
zaco parse input.ts
```

## Language Guide

Zaco is TypeScript with ownership annotations. All valid Zaco code is syntactically a superset of TypeScript.

### Basic types

```typescript
let x: number = 42;
let name: string = "hello";
let flag: boolean = true;
let nothing: void = undefined;
```

### Variables and arithmetic

```typescript
let a = 10;
let b = 3;
console.log(a + b);   // 13
console.log(a * b);   // 30
console.log(a % b);   // 1
```

### Functions

```typescript
function add(a: number, b: number): number {
    return a + b;
}

// Arrow functions
const double = (x: number): number => x * 2;
```

### Control flow

```typescript
let x = 42;
if (x > 0) {
    console.log("positive");
} else {
    console.log("non-positive");
}

let sum = 0;
for (let i = 0; i < 10; i = i + 1) {
    sum = sum + i;
}

while (sum > 0) {
    sum = sum - 1;
}
```

### Built-in modules (no import needed)

```typescript
// Math
let pi = Math.PI;
let floor = Math.floor(3.7);    // 3
let sqrt = Math.sqrt(16.0);     // 4
let rand = Math.random();

// JSON
let json = JSON.stringify("hello");
let parsed = JSON.parse(json);

// Console
console.log("info");
console.error("error");
console.warn("warning");

// Process
let cwd = process.cwd();
process.exit(0);
```

### Imports and exports

```typescript
// Import from built-in modules
import { readFileSync } from "fs";
import { join } from "path";

// Local module imports
import { greet } from "./utils";

// Export functions
export function greet(name: string): string {
    return "Hello " + name;
}
```

### Classes and interfaces

```typescript
interface Printable {
    toString(): string;
}

class Animal implements Printable {
    name: string;
    age: number;

    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
    }

    toString(): string {
        return this.name;
    }
}

class Dog extends Animal {
    breed: string;

    constructor(name: string, age: number, breed: string) {
        super(name, age);
        this.breed = breed;
    }
}
```

### Generics

```typescript
class Container<T> {
    private value: T;

    constructor(value: T) {
        this.value = value;
    }

    get(): T {
        return this.value;
    }

    set(newValue: T): void {
        this.value = newValue;
    }
}
```

### Ownership annotations

Zaco's key extension over TypeScript — ownership annotations that enable compile-time memory safety without a garbage collector.

```typescript
class Point {
    x: number;
    y: number;

    constructor(x: number, y: number) {
        this.x = x;
        this.y = y;
    }

    // `ref` borrows the parameter — no ownership transfer
    distanceTo(ref other: Point): number {
        let dx: number = this.x - other.x;
        let dy: number = this.y - other.y;
        return Math.sqrt(dx * dx + dy * dy);
    }
}

function main(): void {
    let a: Point = new Point(0, 0);  // a owns the Point
    let b: Point = new Point(3, 4);  // b owns this Point

    let dist: number = a.distanceTo(b);  // b is borrowed (not moved)

    // Ownership transfer (move)
    let c: Point = a;  // a is moved to c — a is no longer usable

    // Explicit deep copy
    let d: Point = clone b;  // d is an independent copy of b

    console.log(c.x);  // 0
    console.log(d.y);  // 4
}
```

**Ownership keywords:**

| Keyword | Meaning |
|---------|---------|
| `owned` | Explicit owned value (default for local variables) |
| `ref` | Immutable borrow — callee cannot modify or move the value |
| `mut ref` | Mutable borrow — callee can modify but not move |
| `clone` | Deep copy — creates an independent owned copy |

**Ownership rules (enforced at compile time):**
1. Every value has exactly one owner
2. Assignment transfers ownership (move semantics)
3. `ref` parameters borrow without transferring ownership
4. Use `clone` for explicit deep copies
5. Values are automatically dropped when their owner goes out of scope

## Supported Built-in APIs

### C Runtime (60+ functions)

| Module | Functions |
|--------|-----------|
| Math | floor, ceil, round, abs, sqrt, pow, sin, cos, tan, log, log2, log10, random, min, max, trunc, PI, E |
| JSON | parse, stringify |
| Console | log, error, warn (str/i64/f64/bool variants) |
| String | slice, toUpperCase, toLowerCase, trim, indexOf, includes, replace, split, startsWith, endsWith, charAt, repeat, padStart, padEnd |
| Array | slice, concat, indexOf, join, reverse, pop |

### Rust Runtime (39 functions, Tokio-based)

| Module | Functions |
|--------|-----------|
| fs | readFileSync, writeFileSync, existsSync, mkdirSync, rmdirSync, unlinkSync, statSync, readdirSync, readFile (async) |
| path | join, resolve, dirname, basename, extname, isAbsolute, normalize, sep |
| process | exit, cwd, env.get, pid, platform, arch, argv |
| os | platform, arch, homedir, tmpdir, hostname, cpus, totalmem, EOL |

## Examples

See the `examples/` directory:

| File | Description |
|------|-------------|
| `hello.ts` | Basic hello world |
| `fibonacci.ts` | Functions and loops |
| `classes.ts` | Classes, inheritance, generics, interfaces |
| `ownership.ts` | Ownership annotations: `ref`, move, `clone` |
| `builtin_modules.ts` | Math, JSON, console, process built-in usage |
| `import_modules.ts` | Import tracking for fs and path modules |
| `modules/` | Multi-file module system examples |

Run examples:

```bash
# Compile and run hello world
zaco compile examples/hello.ts -o hello --emit exe && ./hello

# Type check
zaco check examples/fibonacci.ts

# View AST
zaco parse examples/classes.ts
```

## Current Status

**Working:**
- Complete lexer (70+ TypeScript keywords, all operators, strings, template literals, BigInt)
- Full recursive-descent parser with Pratt expression parsing (TS5 spec coverage)
- Type checker with ownership inference and move/borrow tracking
- Full compilation pipeline: AST → IR → Cranelift codegen → native executable
- Module system: Node.js-compatible resolution, dependency graph, cycle detection
- Built-in module recognition: Math, JSON, console, process → runtime function calls
- Import/export lowering with multi-file compilation support
- C runtime: 60+ functions (Math, JSON, console, String, Array methods)
- Rust runtime: 39 Tokio-based functions (fs, path, process, os)
- CLI with `compile`, `check`, `lex`, `parse` subcommands
- Error reporting with source spans and colors

**Compilation support:**
- Variables (let/const) and arithmetic (+, -, *, /, %)
- Functions with parameters and return values
- Control flow: if/else, while, for loops
- String literals and concatenation
- Boolean values and comparisons
- Math.* (16 methods + PI, E constants)
- console.log/error/warn with type-appropriate printing
- Import/export declarations

**Not yet implemented:**
- Class/object compilation (parsed and type-checked, not lowered to IR)
- Closures and lambda capture
- Async/await (Tokio runtime ready, state machine lowering pending)
- for-in/for-of loops
- try/catch/finally
- switch statements
- Rust runtime linking in driver (built separately)

**Test suite:** 81 tests passing across all 7 crates.

## Development

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p zaco-parser

# Build in release mode
cargo build --release

# Build the Rust runtime (separate crate)
cd runtime/zaco_runtime_rs && cargo build --release
```

## License

MIT
