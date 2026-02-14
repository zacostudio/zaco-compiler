//! Integration tests for the Zaco compiler pipeline.
//!
//! These tests compile TypeScript source code to executables and verify the output.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Get the path to the compiled `zaco` binary.
fn zaco_binary() -> PathBuf {
    // When running `cargo test`, the binary is in the same target directory
    let mut path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    path.push("zaco");
    path
}

/// Compile a TypeScript snippet and run the resulting executable, returning stdout.
fn compile_and_run(source: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir().join(format!("zaco_test_{}", id));
    let _ = fs::create_dir_all(&temp_dir);

    let input_path = temp_dir.join("test_input.ts");
    let output_path = temp_dir.join("test_output");

    fs::write(&input_path, source).expect("Failed to write test input");

    // Compile
    let zaco = zaco_binary();
    let compile_output = Command::new(&zaco)
        .arg("compile")
        .arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .arg("--emit")
        .arg("exe")
        // Set working directory to workspace root so runtime is found
        .current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap(),
        )
        .output()
        .expect("Failed to run zaco compiler");

    if !compile_output.status.success() {
        let stderr = String::from_utf8_lossy(&compile_output.stderr);
        let stdout = String::from_utf8_lossy(&compile_output.stdout);
        panic!(
            "Compilation failed!\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }

    // Run
    let run_output = Command::new(&output_path)
        .output()
        .expect("Failed to run compiled executable");

    // Clean up
    let _ = fs::remove_file(&input_path);
    let _ = fs::remove_file(&output_path);

    String::from_utf8_lossy(&run_output.stdout).to_string()
}

/// Compile a TypeScript snippet and return IR output.
fn compile_to_ir(source: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static IR_COUNTER: AtomicUsize = AtomicUsize::new(1000);
    let id = IR_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir().join(format!("zaco_test_{}", id));
    let _ = fs::create_dir_all(&temp_dir);

    let input_path = temp_dir.join("test_input.ts");
    fs::write(&input_path, source).expect("Failed to write test input");

    let zaco = zaco_binary();
    let output = Command::new(&zaco)
        .arg("compile")
        .arg(&input_path)
        .arg("--emit")
        .arg("ir")
        .current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap(),
        )
        .output()
        .expect("Failed to run zaco compiler");

    let _ = fs::remove_file(&input_path);

    String::from_utf8_lossy(&output.stdout).to_string()
}

// ============================================================================
// Hello World
// ============================================================================

#[test]
fn test_hello_world() {
    let output = compile_and_run(r#"console.log("Hello, World!");"#);
    assert_eq!(output.trim(), "Hello, World!");
}

#[test]
fn test_hello_string_only() {
    let output = compile_and_run(r#"console.log("Zaco compiler works!");"#);
    assert_eq!(output.trim(), "Zaco compiler works!");
}

// ============================================================================
// Variables + Arithmetic
// ============================================================================

#[test]
fn test_integer_variables() {
    let output = compile_and_run(
        r#"
let x: number = 42;
let y: number = 10;
console.log(x);
console.log(y);
"#,
    );
    assert_eq!(output.trim(), "42\n10");
}

#[test]
fn test_arithmetic_add() {
    let output = compile_and_run(
        r#"
let a: number = 100;
let b: number = 23;
let sum: number = a + b;
console.log(sum);
"#,
    );
    assert_eq!(output.trim(), "123");
}

#[test]
fn test_arithmetic_sub() {
    let output = compile_and_run(
        r#"
let a: number = 100;
let b: number = 23;
let diff: number = a - b;
console.log(diff);
"#,
    );
    assert_eq!(output.trim(), "77");
}

#[test]
fn test_arithmetic_mul() {
    let output = compile_and_run(
        r#"
let a: number = 7;
let b: number = 6;
let prod: number = a * b;
console.log(prod);
"#,
    );
    assert_eq!(output.trim(), "42");
}

#[test]
fn test_arithmetic_div() {
    let output = compile_and_run(
        r#"
let a: number = 100;
let b: number = 4;
let result: number = a / b;
console.log(result);
"#,
    );
    assert_eq!(output.trim(), "25");
}

#[test]
fn test_arithmetic_mod() {
    let output = compile_and_run(
        r#"
let a: number = 17;
let b: number = 5;
let result: number = a % b;
console.log(result);
"#,
    );
    assert_eq!(output.trim(), "2");
}

#[test]
fn test_multiple_args() {
    let output = compile_and_run(
        r#"
let a: number = 1;
let b: number = 2;
let c: number = 3;
console.log(a, b, c);
"#,
    );
    assert_eq!(output.trim(), "1 2 3");
}

#[test]
fn test_boolean_output() {
    let output = compile_and_run(
        r#"
let t: boolean = true;
let f: boolean = false;
console.log(t);
console.log(f);
"#,
    );
    assert_eq!(output.trim(), "true\nfalse");
}

#[test]
fn test_mixed_output() {
    let output = compile_and_run(
        r#"
console.log("result:", 42);
"#,
    );
    assert_eq!(output.trim(), "result: 42");
}

// ============================================================================
// IR Emission
// ============================================================================

#[test]
fn test_ir_emission() {
    let ir = compile_to_ir(r#"console.log("test");"#);
    assert!(ir.contains("fn main("));
    assert!(ir.contains("zaco_print_str"));
    assert!(ir.contains("Return"));
}

// ============================================================================
// Control Flow
// ============================================================================

#[test]
fn test_if_true() {
    let output = compile_and_run(
        r#"
let x: number = 1;
if (x) {
    console.log("yes");
}
"#,
    );
    assert_eq!(output.trim(), "yes");
}

#[test]
fn test_if_false() {
    let output = compile_and_run(
        r#"
let x: number = 0;
if (x) {
    console.log("yes");
} else {
    console.log("no");
}
"#,
    );
    assert_eq!(output.trim(), "no");
}

// ============================================================================
// Return Code
// ============================================================================

#[test]
fn test_exit_code_zero() {
    let temp_dir = std::env::temp_dir().join("zaco_test_exit_code");
    let _ = fs::create_dir_all(&temp_dir);
    let input_path = temp_dir.join("test_exit.ts");
    let output_path = temp_dir.join("test_exit");

    fs::write(&input_path, r#"console.log("ok");"#).unwrap();

    let zaco = zaco_binary();
    Command::new(&zaco)
        .arg("compile")
        .arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .arg("--emit")
        .arg("exe")
        .current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap(),
        )
        .output()
        .expect("compile failed");

    let status = Command::new(&output_path).status().expect("run failed");
    assert!(status.success(), "Expected exit code 0");

    let _ = fs::remove_file(&input_path);
    let _ = fs::remove_file(&output_path);
}

// ============================================================================
// Switch Statement
// ============================================================================

#[test]
fn test_switch_basic_match() {
    let output = compile_and_run(
        r#"
let x: number = 2;
switch (x) {
    case 1:
        console.log("one");
        break;
    case 2:
        console.log("two");
        break;
    case 3:
        console.log("three");
        break;
}
"#,
    );
    assert_eq!(output.trim(), "two");
}

#[test]
fn test_switch_default_case() {
    let output = compile_and_run(
        r#"
let x: number = 99;
switch (x) {
    case 1:
        console.log("one");
        break;
    default:
        console.log("other");
}
"#,
    );
    assert_eq!(output.trim(), "other");
}

#[test]
fn test_switch_no_match_no_default() {
    let output = compile_and_run(
        r#"
let x: number = 5;
switch (x) {
    case 1:
        console.log("one");
        break;
    case 2:
        console.log("two");
        break;
}
console.log("done");
"#,
    );
    assert_eq!(output.trim(), "done");
}

// ============================================================================
// Switch IR Emission
// ============================================================================

#[test]
fn test_switch_ir_emission() {
    let ir = compile_to_ir(
        r#"
let x: number = 1;
switch (x) {
    case 1:
        console.log("one");
        break;
    default:
        console.log("other");
}
"#,
    );
    // Switch should generate Branch instructions for case matching
    assert!(ir.contains("Branch"), "Switch IR should contain Branch terminators");
}

// ============================================================================
// Module Resolution Failures
// ============================================================================

/// Helper: compile source and expect it to fail. Returns (stdout, stderr).
fn compile_should_fail(source: &str) -> (String, String) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static FAIL_COUNTER: AtomicUsize = AtomicUsize::new(2000);
    let id = FAIL_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir().join(format!("zaco_test_{}", id));
    let _ = fs::create_dir_all(&temp_dir);

    let input_path = temp_dir.join("test_input.ts");
    fs::write(&input_path, source).expect("Failed to write test input");

    let zaco = zaco_binary();
    let output = Command::new(&zaco)
        .arg("compile")
        .arg(&input_path)
        .arg("--emit")
        .arg("ir")
        .current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap(),
        )
        .output()
        .expect("Failed to run zaco compiler");

    let _ = fs::remove_file(&input_path);

    assert!(
        !output.status.success(),
        "Expected compilation to fail but it succeeded"
    );

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn test_unresolved_import_fails_compilation() {
    let (stdout, stderr) = compile_should_fail(
        r#"import { foo } from "definitely-missing-package";
console.log("ok");
"#,
    );
    // Error should mention the missing package (may be in stdout or stderr)
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("not found") || combined.contains("Cannot resolve"),
        "Error message should mention the missing package, got stdout={}, stderr={}",
        stdout, stderr
    );
}

#[test]
fn test_builtin_import_compiles_ok() {
    // Built-in module imports must still compile fine
    let ir = compile_to_ir(
        r#"import { readFileSync } from "fs";
console.log("ok");
"#,
    );
    assert!(ir.contains("fn main("), "Built-in import should compile to IR");
}
