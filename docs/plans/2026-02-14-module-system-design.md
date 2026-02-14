# Zaco Module System Design (Bun/Deno Strategy)

## Decisions

| Item | Decision |
|------|----------|
| Strategy | Bun/Deno — native Node.js API implementation |
| Runtime | C (existing) + Rust (async/IO via Tokio) |
| Async | Tokio-based event loop (Deno approach) |
| Module Resolution | Node.js-compatible algorithm |
| NPM | Pure TS/JS packages only |

## Phases

### Phase 1: Local Module System
- ModuleResolver: relative/absolute path, index.ts fallback, .ts/.js extension
- DependencyGraph: topological sort, cycle detection
- Multi-file compilation: Lex→Parse→TypeCheck→Lower per file → IR merge → single binary
- Type checker: import binding validation, export symbol tables

### Phase 2: Built-in Standard Library
- Math: floor, ceil, round, random, sin, cos, etc. (C math.h)
- JSON: parse, stringify (C implementation)
- console: error, warn, debug (C runtime)
- String/Array prototype methods

### Phase 3: Node.js Core Modules (Rust Runtime)
- fs: readFileSync/writeFileSync + async (Tokio)
- path: join, resolve, dirname, basename (pure Rust)
- process: env, argv, exit, cwd (Rust std)
- http: createServer, request (Tokio + hyper)
- os: platform, cpus, homedir (Rust std)
- events: EventEmitter (Rust)

### Phase 4: NPM Package Support
- package.json parser
- node_modules resolution algorithm
- .d.ts type definition loading

## Async/Await Strategy
- async fn → state machine (coroutine) lowering
- await → Tokio future registration + poll
- Promise → runtime Promise object (Rust struct)
- Top-level: tokio::runtime::Runtime::block_on()

## Runtime Architecture
```
runtime/
├── zaco_runtime.c          # Existing C runtime
└── zaco_runtime_rs/        # New Rust runtime (Tokio-based)
    ├── Cargo.toml
    └── src/
        ├── lib.rs          # #[no_mangle] extern "C" exports
        ├── event_loop.rs   # Tokio event loop
        ├── promise.rs      # Promise implementation
        ├── fs.rs           # File system API
        ├── path.rs         # Path operations
        ├── http.rs         # HTTP server/client
        ├── process_api.rs  # Process API
        ├── os.rs           # OS info
        └── events.rs       # EventEmitter
```
