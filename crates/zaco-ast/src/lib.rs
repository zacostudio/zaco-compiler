//! # Zaco AST
//!
//! Abstract Syntax Tree definitions for the Zaco compiler.
//! Supports TypeScript syntax with Rust-style ownership annotations.

use std::fmt;

// =============================================================================
// Core Types (kept in lib.rs - used by all modules)
// =============================================================================

/// Source location information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub file_id: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, file_id: usize) -> Self {
        Self { start, end, file_id }
    }

    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            file_id: self.file_id,
        }
    }
}

/// AST node wrapper that includes span information
#[derive(Debug, Clone, PartialEq)]
pub struct Node<T> {
    pub span: Span,
    pub value: T,
}

impl<T> Node<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { span, value }
    }
}

/// Identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident {
    pub name: String,
}

impl Ident {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// =============================================================================
// Ownership Annotations (Zaco Extension)
// =============================================================================

/// Ownership kind for Rust-style ownership annotations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OwnershipKind {
    /// Owned value (default)
    Owned,
    /// Immutable reference (&)
    Ref,
    /// Mutable reference (&mut)
    MutRef,
    /// Inferred by compiler
    Inferred,
}

impl fmt::Display for OwnershipKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwnershipKind::Owned => write!(f, "owned"),
            OwnershipKind::Ref => write!(f, "&"),
            OwnershipKind::MutRef => write!(f, "&mut"),
            OwnershipKind::Inferred => write!(f, "inferred"),
        }
    }
}

/// Ownership annotation
#[derive(Debug, Clone, PartialEq)]
pub struct Ownership {
    pub kind: OwnershipKind,
    pub span: Span,
}

// =============================================================================
// Module Declarations
// =============================================================================

pub mod types;
pub mod expr;
pub mod stmt;
pub mod decl;
pub mod module;

// =============================================================================
// Re-exports (critical for maintaining backward compatibility)
// =============================================================================

pub use types::*;
pub use expr::*;
pub use stmt::*;
pub use decl::*;
pub use module::*;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_span() -> Span {
        Span::new(0, 0, 0)
    }

    #[test]
    fn test_basic_types() {
        let _num_type = Type::Primitive(PrimitiveType::Number);
        assert_eq!(format!("{}", PrimitiveType::Number), "number");

        let array_type = Type::Array(Box::new(Node::new(
            Type::Primitive(PrimitiveType::String),
            dummy_span(),
        )));

        assert!(matches!(array_type, Type::Array(_)));
    }

    #[test]
    fn test_ownership() {
        let ownership = Ownership {
            kind: OwnershipKind::Ref,
            span: dummy_span(),
        };

        assert_eq!(ownership.kind, OwnershipKind::Ref);
        assert_eq!(format!("{}", ownership.kind), "&");
    }

    #[test]
    fn test_expressions() {
        let literal = Expr::Literal(Literal::Number(42.0));
        assert!(matches!(literal, Expr::Literal(Literal::Number(42.0))));

        let ident = Expr::Ident(Ident::new("x"));
        assert!(matches!(ident, Expr::Ident(_)));
    }

    #[test]
    fn test_binary_op() {
        let add = BinaryOp::Add;
        assert_eq!(format!("{}", add), "+");

        let eq = BinaryOp::StrictEq;
        assert_eq!(format!("{}", eq), "===");
    }

    #[test]
    fn test_var_decl() {
        let decl = VarDecl {
            kind: VarDeclKind::Let,
            declarations: vec![
                VarDeclarator {
                    pattern: Node::new(
                        Pattern::Ident {
                            name: Node::new(Ident::new("x"), dummy_span()),
                            type_annotation: None,
                            ownership: None,
                        },
                        dummy_span(),
                    ),
                    init: Some(Node::new(
                        Expr::Literal(Literal::Number(10.0)),
                        dummy_span(),
                    )),
                }
            ],
        };

        assert_eq!(decl.kind, VarDeclKind::Let);
        assert_eq!(decl.declarations.len(), 1);
    }

    #[test]
    fn test_function_decl() {
        let func = FunctionDecl {
            name: Node::new(Ident::new("foo"), dummy_span()),
            type_params: None,
            params: vec![],
            return_type: Some(Box::new(Node::new(
                Type::Primitive(PrimitiveType::Void),
                dummy_span(),
            ))),
            body: Some(Node::new(
                BlockStmt { stmts: vec![] },
                dummy_span(),
            )),
            is_async: false,
            is_generator: false,
            is_declare: false,
        };

        assert_eq!(func.name.value.name, "foo");
        assert!(!func.is_async);
    }

    #[test]
    fn test_class_decl() {
        let class = ClassDecl {
            name: Node::new(Ident::new("MyClass"), dummy_span()),
            type_params: None,
            extends: None,
            implements: vec![],
            members: vec![
                ClassMember::Property {
                    name: PropertyName::Ident(Node::new(Ident::new("field"), dummy_span())),
                    type_annotation: Some(Box::new(Node::new(
                        Type::Primitive(PrimitiveType::String),
                        dummy_span(),
                    ))),
                    ownership: Some(Ownership {
                        kind: OwnershipKind::Owned,
                        span: dummy_span(),
                    }),
                    init: None,
                    access: AccessModifier::Private,
                    is_static: false,
                    is_readonly: false,
                    is_abstract: false,
                    is_optional: false,
                    is_override: false,
                    decorators: vec![],
                }
            ],
            is_abstract: false,
            is_declare: false,
            decorators: vec![],
        };

        assert_eq!(class.name.value.name, "MyClass");
        assert_eq!(class.members.len(), 1);
    }

    #[test]
    fn test_ownership_annotation() {
        let pattern = Pattern::Ident {
            name: Node::new(Ident::new("x"), dummy_span()),
            type_annotation: Some(Box::new(Node::new(
                Type::Primitive(PrimitiveType::Number),
                dummy_span(),
            ))),
            ownership: Some(Ownership {
                kind: OwnershipKind::MutRef,
                span: dummy_span(),
            }),
        };

        if let Pattern::Ident { ownership, .. } = pattern {
            assert!(ownership.is_some());
            assert_eq!(ownership.unwrap().kind, OwnershipKind::MutRef);
        } else {
            panic!("Expected ident pattern");
        }
    }

    #[test]
    fn test_clone_expr() {
        let clone_expr = Expr::Clone(Box::new(Node::new(
            Expr::Ident(Ident::new("value")),
            dummy_span(),
        )));

        assert!(matches!(clone_expr, Expr::Clone(_)));
    }
}
