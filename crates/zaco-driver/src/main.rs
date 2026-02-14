use ariadne::{Color, Label, Report, ReportKind, Source};
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use zaco_lexer::{Lexer, Token, TokenKind};

use zaco_driver::{ModuleResolver, ResolvedModule, DepGraph};
use zaco_driver::dts_loader;

#[derive(Parser)]
#[command(
    name = "zaco",
    version = "0.1.0",
    about = "Zaco TypeScript Compiler with ownership semantics",
    long_about = "A TypeScript compiler that adds Rust-like ownership tracking\nto catch memory bugs at compile time."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a TypeScript file to an executable
    Compile {
        /// Input TypeScript file
        input: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// What to emit
        #[arg(long, default_value = "exe")]
        emit: EmitMode,

        /// Target triple (e.g., x86_64-apple-darwin)
        #[arg(long)]
        target: Option<String>,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Type check a TypeScript file without compiling
    Check {
        /// Input TypeScript file
        input: PathBuf,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Lex a TypeScript file and show tokens (debug)
    Lex {
        /// Input TypeScript file
        input: PathBuf,

        /// Show token positions
        #[arg(short, long)]
        positions: bool,
    },

    /// Parse a TypeScript file and show AST (debug)
    Parse {
        /// Input TypeScript file
        input: PathBuf,

        /// Pretty print the AST
        #[arg(short, long)]
        pretty: bool,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum EmitMode {
    /// Emit AST (debug output)
    Ast,
    /// Emit IR (debug output)
    Ir,
    /// Emit object file only
    Obj,
    /// Emit executable (default)
    Exe,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Compile {
            input,
            output,
            emit,
            target,
            verbose,
        } => compile_command(input, output, emit, target, verbose),
        Commands::Check { input, verbose } => check_command(input, verbose),
        Commands::Lex { input, positions } => lex_command(input, positions),
        Commands::Parse { input, pretty } => parse_command(input, pretty),
    }
}

fn compile_command(
    input: PathBuf,
    output: Option<PathBuf>,
    emit: EmitMode,
    target: Option<String>,
    verbose: bool,
) -> ExitCode {
    if verbose {
        println!("Compiling: {}", input.display());
        if let Some(ref t) = target {
            println!("Target: {}", t);
        }
        println!("Emit mode: {:?}", emit);
    }

    // Canonicalize input path
    let input = match input.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error resolving input path: {}", e);
            return ExitCode::FAILURE;
        }
    };

    // Build dependency graph by discovering all imports
    if verbose {
        println!("\n[Phase 0] Discovering module dependencies...");
    }

    let mut dep_graph = DepGraph::new();
    let base_dir = input.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let resolver = ModuleResolver::new(base_dir);
    let mut parse_cache: HashMap<PathBuf, (String, Program)> = HashMap::new();

    match discover_modules(&input, &resolver, &mut dep_graph, verbose, &mut parse_cache) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Module discovery error: {}", e);
            return ExitCode::FAILURE;
        }
    }

    dep_graph.set_entry(input.clone());

    // Check for circular dependencies
    if let Err(e) = dep_graph.detect_cycles() {
        eprintln!("Error: {}", e);
        return ExitCode::FAILURE;
    }

    // Get compilation order (topological sort)
    let compilation_order = match dep_graph.topological_sort() {
        Ok(order) => order,
        Err(e) => {
            eprintln!("Error determining compilation order: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if verbose {
        println!("  Discovered {} modules", compilation_order.len());
        for (i, module_path) in compilation_order.iter().enumerate() {
            println!("    {}. {}", i + 1, module_path.display());
        }
    }

    // Compile each module in order and collect IR modules (preserving compilation order)
    let mut module_irs: Vec<(PathBuf, zaco_ir::IrModule)> = Vec::new();
    let mut func_id_offset: usize = 0;
    let mut struct_id_offset: usize = 0;

    for module_path in &compilation_order {
        if verbose {
            println!("\n[Compiling] {}", module_path.display());
        }

        // Entry module (the user's input file) gets "main" wrapper;
        // all other modules get "__module_init_<name>" wrappers.
        let is_entry = *module_path == input;
        let module_name = if is_entry {
            None
        } else {
            Some(module_path_to_init_name(module_path))
        };

        let ir_module = match compile_single_module(
            module_path,
            &emit,
            verbose,
            &mut parse_cache,
            module_name.as_deref(),
            func_id_offset,
            struct_id_offset,
        ) {
            Ok(ir) => ir,
            Err(_) => return ExitCode::FAILURE,
        };

        // Update offsets for the next module to avoid FuncId/StructId collisions
        func_id_offset = ir_module.next_func_id;
        struct_id_offset = ir_module.next_struct_id;

        module_irs.push((module_path.clone(), ir_module));
    }

    // Merge all IR modules into one
    if verbose {
        println!("\n[Phase 4.5] Merging IR modules...");
    }

    let mut merged_ir = merge_ir_modules(module_irs);

    // Inject calls to __module_init_* functions at the start of "main"'s entry block.
    // This ensures all dependency modules' top-level code runs before the entry module.
    inject_module_init_calls(&mut merged_ir);

    if verbose {
        println!(
            "  {} functions, {} string literals",
            merged_ir.functions.len(),
            merged_ir.string_literals.len()
        );
    }

    if matches!(emit, EmitMode::Ir) {
        dump_ir(&merged_ir);
        return ExitCode::SUCCESS;
    }

    // Phase 5: IR → Native Code (Cranelift)
    if verbose {
        println!("\n[Phase 5] Generating native code...");
    }

    let codegen = match zaco_codegen::CodeGenerator::new() {
        Ok(cg) => cg,
        Err(e) => {
            eprintln!("Codegen initialization error: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let object_bytes = match codegen.compile_module(&merged_ir) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Codegen error: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if verbose {
        println!("  {} bytes of object code generated", object_bytes.len());
    }

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let stem = input.file_stem().unwrap_or_default().to_string_lossy();
        PathBuf::from(stem.to_string())
    });

    if matches!(emit, EmitMode::Obj) {
        let obj_path = output_path.with_extension("o");
        match fs::write(&obj_path, &object_bytes) {
            Ok(_) => {
                println!("Object file written to: {}", obj_path.display());
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("Error writing object file: {}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    // Phase 6: Linking
    if verbose {
        println!("\n[Phase 6] Linking...");
    }

    // Find the runtime source
    let runtime_path = find_runtime_source(&input);

    match link_executable(&object_bytes, &output_path, runtime_path.as_deref(), verbose) {
        Ok(_) => {
            println!("Executable written to: {}", output_path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Linking error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn check_command(input: PathBuf, verbose: bool) -> ExitCode {
    if verbose {
        println!("Type checking: {}", input.display());
    }

    let source = match read_source_file(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let filename = input.to_string_lossy().to_string();

    // Lex
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    let has_errors = tokens.iter().any(|t| t.kind == TokenKind::Error);
    if has_errors {
        report_lexer_errors(&tokens, &filename, &source);
        return ExitCode::FAILURE;
    }

    // Parse
    let mut parser = zaco_parser::Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(prog) => prog,
        Err(errors) => {
            for err in &errors {
                report_error(
                    "E1000",
                    "Parse error",
                    &err.message,
                    err.span.start,
                    err.span.end,
                    &filename,
                    &source,
                );
            }
            return ExitCode::FAILURE;
        }
    };

    // Type check
    match zaco_typeck::check_program(&program) {
        Ok(_) => {
            println!("Type check passed!");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for err in &errors {
                let msg = err.kind.to_string();
                report_error(
                    "E2000",
                    "Type error",
                    &msg,
                    err.span.start,
                    err.span.end,
                    &filename,
                    &source,
                );
            }
            ExitCode::FAILURE
        }
    }
}

fn lex_command(input: PathBuf, positions: bool) -> ExitCode {
    let source = match read_source_file(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let filename = input.to_string_lossy().to_string();

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    println!("Tokens for {}:\n", filename);
    println!("{}", "=".repeat(80));

    for (i, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Eof {
            println!("\n{:4} | {:?}", i, token.kind);
            break;
        }

        if positions {
            println!(
                "{:4} | {:20?} | {:?} | {}..{}",
                i, token.kind, token.value, token.span.start, token.span.end
            );
        } else {
            println!("{:4} | {:20?} | {:?}", i, token.kind, token.value);
        }
    }

    println!("{}", "=".repeat(80));
    println!("\nTotal tokens: {}", tokens.len());

    let error_count = tokens.iter().filter(|t| t.kind == TokenKind::Error).count();
    if error_count > 0 {
        println!("\nLexer errors found: {}", error_count);
        report_lexer_errors(&tokens, &filename, &source);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn parse_command(input: PathBuf, _pretty: bool) -> ExitCode {
    let source = match read_source_file(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let filename = input.to_string_lossy().to_string();

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    let has_errors = tokens.iter().any(|t| t.kind == TokenKind::Error);
    if has_errors {
        report_lexer_errors(&tokens, &filename, &source);
        return ExitCode::FAILURE;
    }

    let mut parser = zaco_parser::Parser::new(tokens);
    match parser.parse_program() {
        Ok(program) => {
            println!("{:#?}", program);
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for err in &errors {
                report_error(
                    "E1000",
                    "Parse error",
                    &err.message,
                    err.span.start,
                    err.span.end,
                    &filename,
                    &source,
                );
            }
            ExitCode::FAILURE
        }
    }
}

// Helper functions

fn read_source_file(path: &PathBuf) -> io::Result<String> {
    fs::read_to_string(path)
}

fn report_lexer_errors(tokens: &[Token], filename: &str, source: &str) {
    for token in tokens.iter().filter(|t| t.kind == TokenKind::Error) {
        report_error(
            "E0001",
            "Lexical error",
            &token.value,
            token.span.start,
            token.span.end,
            filename,
            source,
        );
    }
}

fn report_error(code: &str, title: &str, message: &str, start: usize, end: usize, filename: &str, source: &str) {
    let span = (filename, start..end);
    Report::build(ReportKind::Error, span.clone())
        .with_code(code)
        .with_message(title)
        .with_label(
            Label::new(span)
                .with_message(message)
                .with_color(Color::Red),
        )
        .finish()
        .print((filename, Source::from(source)))
        .unwrap();
}

/// Find the runtime C source file, searching common locations.
fn find_runtime_source(input_path: &PathBuf) -> Option<PathBuf> {
    // 1. Check ZACO_RUNTIME_C environment variable
    if let Ok(env_path) = std::env::var("ZACO_RUNTIME_C") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Search relative paths
    let candidates = [
        // Relative to the input file's directory
        input_path
            .parent()
            .map(|p| p.join("runtime/zaco_runtime.c")),
        // Relative to CWD
        Some(PathBuf::from("runtime/zaco_runtime.c")),
    ];

    for candidate in candidates.iter().flatten() {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    // 3. Try to find via the executable's location
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // Try sibling directory (e.g., installed layout)
            let runtime = exe_dir.join("../runtime/zaco_runtime.c");
            if runtime.exists() {
                return Some(runtime);
            }
            // Try share directory (e.g., /usr/local/share/zaco/runtime)
            let runtime = exe_dir.join("../share/zaco/runtime/zaco_runtime.c");
            if runtime.exists() {
                return Some(runtime);
            }
        }
    }

    None
}

/// Find the Rust runtime static library (.a), searching common locations.
fn find_rust_runtime(c_runtime_path: &std::path::Path) -> Option<PathBuf> {
    // 1. Check ZACO_RUNTIME_RS environment variable
    if let Ok(env_path) = std::env::var("ZACO_RUNTIME_RS") {
        let p = PathBuf::from(env_path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Derive from C runtime location (sibling directory)
    if let Some(runtime_dir) = c_runtime_path.parent() {
        let candidate = runtime_dir.join("zaco_runtime_rs/target/release/libzaco_runtime_rs.a");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // 3. Relative to CWD
    let cwd_candidate = PathBuf::from("runtime/zaco_runtime_rs/target/release/libzaco_runtime_rs.a");
    if cwd_candidate.exists() {
        return Some(cwd_candidate);
    }

    // 4. Relative to compiler executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let candidate = exe_dir.join("../runtime/zaco_runtime_rs/target/release/libzaco_runtime_rs.a");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Dump IR module for --emit ir.
fn dump_ir(module: &zaco_ir::IrModule) {
    for func in &module.functions {
        println!("fn {}(", func.name);
        for (i, (id, ty)) in func.params.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("{}: {}", id, ty);
        }
        println!(") -> {} {{", func.return_type);

        for block in &func.blocks {
            println!("  {}:", block.id);
            for instr in &block.instructions {
                println!("    {:?}", instr);
            }
            println!("    {:?}", block.terminator);
        }
        println!("}}");
        println!();
    }

    if !module.string_literals.is_empty() {
        println!("String literals:");
        for (i, s) in module.string_literals.iter().enumerate() {
            println!("  [{}] {:?}", i, s);
        }
    }
}

fn link_executable(
    object_bytes: &[u8],
    output_path: &PathBuf,
    runtime_path: Option<&std::path::Path>,
    verbose: bool,
) -> io::Result<()> {
    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let temp_obj = temp_dir.join(format!("zaco_temp_{}.o", pid));
    fs::write(&temp_obj, object_bytes)?;

    let linker = "cc";

    let mut cmd = Command::new(linker);
    cmd.arg("-o").arg(output_path);

    // On macOS, suppress linker warnings about missing platform load
    // commands in Cranelift-generated object files (Cranelift doesn't
    // emit Mach-O platform metadata).
    if cfg!(target_os = "macos") {
        cmd.arg("-Wl,-w");
    }

    // Add the compiled object file
    cmd.arg(&temp_obj);

    // Compile and link the C runtime if available
    if let Some(rt_path) = runtime_path {
        if verbose {
            println!("  Using C runtime: {}", rt_path.display());
        }
        // Compile runtime.c to .o and link together
        let temp_rt_obj = temp_dir.join(format!("zaco_runtime_{}.o", pid));
        let rt_status = Command::new("cc")
            .args(["-c", "-o"])
            .arg(&temp_rt_obj)
            .arg(rt_path)
            .status()?;

        if !rt_status.success() {
            let _ = fs::remove_file(&temp_obj);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to compile runtime.c",
            ));
        }
        cmd.arg(&temp_rt_obj);

        // Link the Rust runtime static library
        let rust_runtime_lib = find_rust_runtime(rt_path);

        if let Some(ref rust_runtime_lib) = rust_runtime_lib {
            if verbose {
                println!("  Using Rust runtime: {}", rust_runtime_lib.display());
            }
            cmd.arg(&rust_runtime_lib);

            // Add required linker flags for Rust runtime on macOS
            if cfg!(target_os = "macos") {
                cmd.arg("-framework").arg("CoreFoundation");
                cmd.arg("-framework").arg("Security");
                cmd.arg("-framework").arg("SystemConfiguration");
                cmd.arg("-lpthread");
                cmd.arg("-ldl");
            }
        } else {
            eprintln!("Warning: Rust runtime library not found");
            eprintln!("To build it, run: cd runtime/zaco_runtime_rs && cargo build --release");
        }

        let status = cmd.status()?;

        // Clean up temp files
        let _ = fs::remove_file(&temp_obj);
        let _ = fs::remove_file(&temp_rt_obj);

        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Linker exited with status: {}", status),
            ))
        }
    } else {
        // No runtime — link just the object file (will fail if runtime symbols are referenced)
        if verbose {
            println!("  Warning: Runtime not found, linking without it");
        }
        let status = cmd.status()?;
        let _ = fs::remove_file(&temp_obj);

        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Linker exited with status: {}", status),
            ))
        }
    }
}

// ============================================================================
// Module system helper functions
// ============================================================================

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use zaco_ast::{ExportDecl, ImportDecl, ModuleItem, Program};

/// Discover all modules starting from an entry point.
/// Returns a cache of parsed programs to avoid re-parsing during compilation.
fn discover_modules(
    entry: &Path,
    resolver: &ModuleResolver,
    graph: &mut DepGraph,
    verbose: bool,
    parse_cache: &mut HashMap<PathBuf, (String, Program)>,
) -> Result<(), String> {
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();

    queue.push_back(entry.to_path_buf());

    while let Some(current_path) = queue.pop_front() {
        if visited.contains(&current_path) {
            continue;
        }
        visited.insert(current_path.clone());

        // Read and parse the module
        let source = fs::read_to_string(&current_path).map_err(|e| {
            format!(
                "Failed to read module {}: {}",
                current_path.display(),
                e
            )
        })?;

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize();

        let has_errors = tokens.iter().any(|t| t.kind == TokenKind::Error);
        if has_errors {
            return Err(format!(
                "Lexer errors in module: {}",
                current_path.display()
            ));
        }

        let mut parser = zaco_parser::Parser::new(tokens);
        let program = parser.parse_program().map_err(|errors| {
            format!(
                "Parse errors in module {}: {}",
                current_path.display(),
                errors
                    .iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

        // Extract imports and exports
        let (imports, exports) = extract_imports_exports(&program);

        // Resolve imports to module paths
        let mut dependencies = Vec::new();
        for import in &imports {
            match resolver.resolve(&import.source, &current_path) {
                Ok(ResolvedModule::LocalFile(path)) => {
                    dependencies.push(path.clone());
                    queue.push_back(path);
                }
                Ok(ResolvedModule::Builtin(name)) => {
                    if verbose {
                        println!("  Note: Skipping built-in module: {}", name);
                    }
                }
                Ok(ResolvedModule::Package(path)) => {
                    // NPM package resolved successfully
                    if verbose {
                        println!("  Resolved NPM package '{}' to: {}", import.source, path.display());
                    }

                    // If it's a .d.ts file, load type declarations but don't compile
                    if path.extension().and_then(|s| s.to_str()) == Some("ts")
                        && path.to_string_lossy().ends_with(".d.ts") {
                        if verbose {
                            println!("  Loading type declarations from: {}", path.display());
                        }
                        // Load declarations for type checking
                        match dts_loader::DtsLoader::load_declarations(&path) {
                            Ok(decls) => {
                                if verbose {
                                    println!("    Loaded {} type declarations", decls.len());
                                }
                                // Type declarations are loaded but not added to compilation queue
                            }
                            Err(e) => {
                                if verbose {
                                    println!("    Warning: Failed to load .d.ts: {}", e);
                                }
                            }
                        }
                    } else {
                        // Regular package file - add to compilation queue
                        dependencies.push(path.clone());
                        queue.push_back(path);
                    }
                }
                Ok(ResolvedModule::PackageNotFound { name: pkg_name, reason }) => {
                    return Err(format!(
                        "Cannot resolve import '{}' in {}: package '{}' not found ({})",
                        import.source,
                        current_path.display(),
                        pkg_name,
                        reason,
                    ));
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to resolve import '{}' in {}: {}",
                        import.source,
                        current_path.display(),
                        e
                    ));
                }
            }
        }

        graph.add_module(current_path.clone(), dependencies, exports);

        // Cache the parsed program to avoid re-parsing during compilation
        parse_cache.insert(current_path, (source, program));
    }

    Ok(())
}

/// Extract imports and exports from a program AST
fn extract_imports_exports(program: &Program) -> (Vec<ImportDecl>, HashSet<String>) {
    let mut imports = Vec::new();
    let mut exports = HashSet::new();

    for item in &program.items {
        match &item.value {
            ModuleItem::Import(import_decl) => {
                imports.push(import_decl.clone());
            }
            ModuleItem::Export(export_decl) => {
                extract_export_names(export_decl, &mut exports);
            }
            _ => {}
        }
    }

    (imports, exports)
}

/// Extract exported names from an export declaration
fn extract_export_names(export_decl: &ExportDecl, exports: &mut HashSet<String>) {
    match export_decl {
        ExportDecl::Named { specifiers, .. } => {
            for spec in specifiers {
                let name = if let Some(ref exported) = spec.exported {
                    exported.value.name.clone()
                } else {
                    spec.local.value.name.clone()
                };
                exports.insert(name);
            }
        }
        ExportDecl::Default(_) | ExportDecl::DefaultDecl(_) => {
            exports.insert("default".to_string());
        }
        ExportDecl::All { as_name, .. } => {
            if let Some(ref name) = as_name {
                exports.insert(name.value.name.clone());
            }
        }
        ExportDecl::Decl(decl) => {
            // Extract the name from the declaration
            use zaco_ast::Decl;
            match &decl.value {
                Decl::Function(func) => {
                    exports.insert(func.name.value.name.clone());
                }
                Decl::Var(var_decl) => {
                    for declarator in &var_decl.declarations {
                        if let zaco_ast::Pattern::Ident { name, .. } = &declarator.pattern.value {
                            exports.insert(name.value.name.clone());
                        }
                    }
                }
                Decl::Class(class) => {
                    exports.insert(class.name.value.name.clone());
                }
                Decl::TypeAlias(alias) => {
                    exports.insert(alias.name.value.name.clone());
                }
                Decl::Interface(iface) => {
                    exports.insert(iface.name.value.name.clone());
                }
                Decl::Enum(enum_decl) => {
                    exports.insert(enum_decl.name.value.name.clone());
                }
                _ => {}
            }
        }
    }
}

/// Compile a single module (typecheck, lower to IR).
/// Uses cached parse results when available to avoid re-parsing.
fn compile_single_module(
    module_path: &Path,
    emit: &EmitMode,
    verbose: bool,
    parse_cache: &mut HashMap<PathBuf, (String, Program)>,
    module_name: Option<&str>,
    func_id_offset: usize,
    struct_id_offset: usize,
) -> Result<zaco_ir::IrModule, ()> {
    // Use cached parse result if available, otherwise parse from scratch
    let (source, program) = if let Some(cached) = parse_cache.remove(module_path) {
        cached
    } else {
        let source = fs::read_to_string(module_path).map_err(|e| {
            eprintln!("Error reading {}: {}", module_path.display(), e);
        })?;

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize();

        let has_errors = tokens.iter().any(|t| t.kind == TokenKind::Error);
        if has_errors {
            let filename = module_path.to_string_lossy().to_string();
            report_lexer_errors(&tokens, &filename, &source);
            return Err(());
        }

        let mut parser = zaco_parser::Parser::new(tokens);
        let program = match parser.parse_program() {
            Ok(prog) => prog,
            Err(errors) => {
                let filename = module_path.to_string_lossy().to_string();
                for err in &errors {
                    report_error(
                        "E1000",
                        "Parse error",
                        &err.message,
                        err.span.start,
                        err.span.end,
                        &filename,
                        &source,
                    );
                }
                return Err(());
            }
        };

        (source, program)
    };

    let filename = module_path.to_string_lossy().to_string();

    if matches!(emit, EmitMode::Ast) {
        println!("AST for {}:", filename);
        println!("{:#?}", program);
    }

    // Phase 3: Type checking
    let _typed_program = match zaco_typeck::check_program(&program) {
        Ok(typed) => typed,
        Err(errors) => {
            for err in &errors {
                let msg = err.kind.to_string();
                report_error(
                    "E2000",
                    "Type error",
                    &msg,
                    err.span.start,
                    err.span.end,
                    &filename,
                    &source,
                );
            }
            return Err(());
        }
    };

    // Phase 4: AST → IR lowering
    let lowerer = {
        let l = zaco_ir::lower::Lowerer::new()
            .with_func_id_offset(func_id_offset)
            .with_struct_id_offset(struct_id_offset)
            .with_file_path(module_path.to_string_lossy().into_owned());
        if let Some(name) = module_name {
            l.with_module_name(name.to_string())
        } else {
            l
        }
    };
    let ir_module = match lowerer.lower_program(&program) {
        Ok(module) => module,
        Err(errors) => {
            for err in &errors {
                report_error(
                    "E3000",
                    "Lowering error",
                    &err.message,
                    err.span.start,
                    err.span.end,
                    &filename,
                    &source,
                );
            }
            return Err(());
        }
    };

    if verbose {
        println!(
            "  Compiled: {} ({} functions)",
            filename,
            ir_module.functions.len()
        );
    }

    Ok(ir_module)
}

/// Merge multiple IR modules into a single module (order-preserving).
///
/// User-defined functions are all included (no name-based dedup — each module
/// now has uniquely-named wrappers via `__module_init_<name>` prefixing).
/// Only extern function *declarations* are deduplicated (safe — they're just declarations).
fn merge_ir_modules(
    module_irs: Vec<(PathBuf, zaco_ir::IrModule)>,
) -> zaco_ir::IrModule {
    let mut merged = zaco_ir::IrModule::new();

    for (_path, ir_module) in module_irs {
        // Merge all user-defined functions without name-based dedup
        for func in ir_module.functions {
            merged.add_function(func);
        }

        // Merge structs
        for struct_def in ir_module.structs {
            merged.add_struct(struct_def);
        }

        // Merge globals
        for (name, ty, init) in ir_module.globals {
            merged.add_global(name, ty, init);
        }

        // Merge string literals
        for lit in ir_module.string_literals {
            merged.intern_string(lit);
        }

        // Merge extern function declarations (deduplicate by name — safe for declarations)
        for ext_func in ir_module.extern_functions {
            if !merged
                .extern_functions
                .iter()
                .any(|ef| ef.name == ext_func.name)
            {
                merged.extern_functions.push(ext_func);
            }
        }
    }

    merged
}

/// Derive a safe init function name from a module's file path.
/// Includes the parent directory for readability plus a hash suffix of the full
/// path to guarantee uniqueness even when multiple modules share the same
/// parent+stem (e.g., `x/a/index.ts` vs `y/a/index.ts`).
/// e.g., "x/a/index.ts" → "a_index_1a2b3c4d"
/// Characters that aren't alphanumeric or underscore are replaced with '_'.
fn module_path_to_init_name(path: &Path) -> String {
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let readable = if parent.is_empty() {
        stem.to_string()
    } else {
        format!("{}_{}", parent, stem)
    };

    let sanitized: String = readable
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();

    // Append hash of full path for uniqueness
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let hash = hasher.finish();

    format!("{}_{:08x}", sanitized, hash as u32)
}

/// Inject calls to all `__module_init_*` functions at the start of "main"'s entry block.
/// This ensures dependency modules' top-level code runs before the entry module's code.
fn inject_module_init_calls(module: &mut zaco_ir::IrModule) {
    // Collect names of all __module_init_* functions
    let init_names: Vec<String> = module
        .functions
        .iter()
        .filter(|f| f.name.starts_with("__module_init_"))
        .map(|f| f.name.clone())
        .collect();

    if init_names.is_empty() {
        return;
    }

    // Find the "main" function and inject calls at the start of its entry block
    if let Some(main_func) = module.functions.iter_mut().find(|f| f.name == "main") {
        let entry_block = main_func.entry_block;

        // Build Call instructions for each init function
        let mut init_calls: Vec<zaco_ir::Instruction> = Vec::new();
        for name in &init_names {
            init_calls.push(zaco_ir::Instruction::Call {
                dest: None,
                func: zaco_ir::Value::Const(zaco_ir::Constant::Str(name.clone())),
                args: vec![],
            });
        }

        // Prepend init calls before existing instructions in the entry block
        if let Some(block) = main_func.blocks.iter_mut().find(|b| b.id == entry_block) {
            let existing = std::mem::take(&mut block.instructions);
            block.instructions = init_calls;
            block.instructions.extend(existing);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_module_path_to_init_name_no_collision() {
        let name1 = module_path_to_init_name(Path::new("x/a/index.ts"));
        let name2 = module_path_to_init_name(Path::new("y/a/index.ts"));
        assert_ne!(
            name1, name2,
            "Different paths with same stem should produce different init names"
        );

        // Same path should produce same name
        let name3 = module_path_to_init_name(Path::new("x/a/index.ts"));
        assert_eq!(
            name1, name3,
            "Same path should produce same init name"
        );
    }
}
