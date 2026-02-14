//! Type checking errors

use std::fmt;
use zaco_ast::Span;

use crate::types::Type;

/// Type error kinds
#[derive(Debug, Clone, PartialEq)]
pub enum TypeErrorKind {
    /// Type mismatch
    TypeMismatch {
        expected: Type,
        found: Type,
    },
    /// Undefined variable
    UndefinedVariable(String),
    /// Undefined type
    UndefinedType(String),
    /// Use after move
    UseAfterMove(String),
    /// Borrow conflict (trying to borrow mutably while borrowed)
    BorrowConflict(String),
    /// Cannot assign to immutable variable
    AssignToImmutable(String),
    /// Missing initialization
    UninitializedVariable(String),
    /// Duplicate declaration
    DuplicateDeclaration(String),
    /// Invalid operation
    InvalidOperation(String),
    /// Arity mismatch (function call)
    ArityMismatch {
        expected: usize,
        found: usize,
    },
    /// Property not found
    PropertyNotFound {
        ty: Type,
        property: String,
    },
    /// Cannot call non-function
    NotCallable(Type),
    /// Cannot index non-array/object
    NotIndexable(Type),
    /// Generic error message
    Generic(String),
}

/// Type error with location information
#[derive(Debug, Clone, PartialEq)]
pub struct TypeError {
    pub kind: TypeErrorKind,
    pub span: Span,
}

impl TypeError {
    pub fn new(kind: TypeErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Type error at {}:{}: {}",
            self.span.start, self.span.end, self.kind
        )
    }
}

impl fmt::Display for TypeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeErrorKind::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {:?}, found {:?}", expected, found)
            }
            TypeErrorKind::UndefinedVariable(name) => {
                write!(f, "undefined variable '{}'", name)
            }
            TypeErrorKind::UndefinedType(name) => {
                write!(f, "undefined type '{}'", name)
            }
            TypeErrorKind::UseAfterMove(name) => {
                write!(f, "use of moved value '{}'", name)
            }
            TypeErrorKind::BorrowConflict(name) => {
                write!(f, "cannot borrow '{}' mutably while borrowed", name)
            }
            TypeErrorKind::AssignToImmutable(name) => {
                write!(f, "cannot assign to immutable variable '{}'", name)
            }
            TypeErrorKind::UninitializedVariable(name) => {
                write!(f, "variable '{}' used before initialization", name)
            }
            TypeErrorKind::DuplicateDeclaration(name) => {
                write!(f, "duplicate declaration of '{}'", name)
            }
            TypeErrorKind::InvalidOperation(msg) => {
                write!(f, "invalid operation: {}", msg)
            }
            TypeErrorKind::ArityMismatch { expected, found } => {
                write!(
                    f,
                    "argument count mismatch: expected {}, found {}",
                    expected, found
                )
            }
            TypeErrorKind::PropertyNotFound { ty, property } => {
                write!(f, "property '{}' not found on type {:?}", property, ty)
            }
            TypeErrorKind::NotCallable(ty) => {
                write!(f, "cannot call value of type {:?}", ty)
            }
            TypeErrorKind::NotIndexable(ty) => {
                write!(f, "cannot index value of type {:?}", ty)
            }
            TypeErrorKind::Generic(msg) => write!(f, "{}", msg),
        }
    }
}
