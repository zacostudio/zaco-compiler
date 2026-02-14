//! Error types for code generation

use std::fmt;

/// Error type for code generation failures
#[derive(Debug, Clone)]
pub struct CodegenError {
    pub message: String,
}

impl CodegenError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Codegen error: {}", self.message)
    }
}

impl std::error::Error for CodegenError {}

impl From<String> for CodegenError {
    fn from(s: String) -> Self {
        Self { message: s }
    }
}

impl From<&str> for CodegenError {
    fn from(s: &str) -> Self {
        Self {
            message: s.to_string(),
        }
    }
}
