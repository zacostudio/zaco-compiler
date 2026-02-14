# Zaco Driver - CLI Compiler Driver

The main CLI interface for the Zaco TypeScript compiler with ownership semantics.

## Features

### Commands

1. **compile** - Full compilation pipeline
   ```bash
   zaco compile input.ts [-o output] [--emit ast|ir|obj|exe] [--target <triple>]
   ```

2. **check** - Type checking only (no code generation)
   ```bash
   zaco check input.ts
   ```

3. **lex** - Show tokens (debugging)
   ```bash
   zaco lex input.ts [--positions]
   ```

4. **parse** - Show AST (debugging)
   ```bash
   zaco parse input.ts [--pretty]
   ```

### Compilation Pipeline

The driver orchestrates a 6-phase compilation pipeline:

1. **Lexing** - Source code → Tokens
2. **Parsing** - Tokens → AST
3. **Type Checking** - AST → Typed AST (with ownership inference)
4. **IR Lowering** - Typed AST → IR Module
5. **Code Generation** - IR → Object file
6. **Linking** - Object file → Executable (via system linker)

### Error Reporting

Beautiful error messages powered by [ariadne](https://crates.io/crates/ariadne):

- Color-coded diagnostics
- Source context with line numbers
- Precise error location highlighting
- Multi-error support (doesn't stop at first error)

Error categories:
- `E0001` - Lexical errors
- `E1000` - Parse errors
- `E2000` - Type errors
- `E3000` - Ownership errors
- `W0000` - Warnings

### Emit Modes

- `ast` - Pretty-print the AST (debug)
- `ir` - Pretty-print the IR (debug)
- `obj` - Output object file only (no linking)
- `exe` - Full executable (default)

### Platform Support

- **macOS**: Uses `cc` (clang) for linking
- **Linux**: Uses `cc` (gcc/clang) for linking
- **Windows**: Uses `link.exe` (MSVC) for linking

## Examples

```bash
# Compile to executable
zaco compile main.ts -o app

# Type check only
zaco check main.ts

# Show tokens with positions
zaco lex main.ts --positions

# Compile with verbose output
zaco compile main.ts -o app --verbose

# Emit object file only
zaco compile main.ts --emit obj -o main.o

# Target specific platform
zaco compile main.ts --target x86_64-unknown-linux-gnu
```

## Implementation Status

### Completed
- ✅ CLI structure with clap
- ✅ File reading and error handling
- ✅ Lexer integration
- ✅ Beautiful error reporting with ariadne
- ✅ Multi-phase pipeline orchestration
- ✅ Linker integration (ready for use)
- ✅ Platform-specific linker selection
- ✅ Verbose mode
- ✅ Debug commands (lex, parse)

### Pending (Awaiting Implementation)
- ⏳ Parser integration (zaco-parser pending)
- ⏳ Type checker integration (zaco-typeck pending)
- ⏳ IR lowering integration (zaco-ir pending)
- ⏳ Code generation integration (zaco-codegen pending)

## Architecture

```
main.rs
├── CLI Definition (clap)
│   ├── Commands enum
│   ├── EmitMode enum
│   └── Cli struct
│
├── Command Handlers
│   ├── compile_command() - Full compilation
│   ├── check_command() - Type checking
│   ├── lex_command() - Token display
│   └── parse_command() - AST display
│
├── Error Reporting
│   ├── report_lexer_errors() - Lexical errors
│   ├── report_parse_error() - Parse errors
│   ├── report_type_error() - Type errors
│   ├── report_ownership_error() - Ownership errors
│   ├── report_warning() - Warnings
│   └── report_help() - Suggestions
│
└── Utilities
    ├── read_source_file() - File I/O
    └── link_executable() - Linker invocation
```

## Error Handling Strategy

1. **No Panic Policy**: All errors are reported gracefully
2. **Multiple Errors**: Collect and report all errors, don't stop at first
3. **Source Context**: Always show source code with errors
4. **Actionable Messages**: Clear, specific error messages
5. **Exit Codes**: 0 for success, 1 for errors

## Future Enhancements

- [ ] Watch mode for incremental compilation
- [ ] Parallel compilation of modules
- [ ] Caching of intermediate results
- [ ] LSP server integration
- [ ] REPL mode
- [ ] Optimization levels (-O0, -O1, -O2, -O3)
- [ ] Debug info generation (DWARF)
- [ ] Cross-compilation support
- [ ] Package manager integration

## Dependencies

- `clap` - CLI argument parsing
- `ariadne` - Beautiful error diagnostics
- `zaco-ast` - AST definitions
- `zaco-lexer` - Tokenization
- `zaco-parser` - Parsing (pending)
- `zaco-typeck` - Type checking (pending)
- `zaco-ir` - IR generation (pending)
- `zaco-codegen` - Code generation (pending)
