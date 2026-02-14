//! Typed AST (output of type checking)

use zaco_ast::{Decl, Expr, Span, Stmt};
use crate::types::Type;

/// Typed expression with inferred type information
#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub expr: Expr,
    pub ty: Type,
    pub span: Span,
}

/// Typed statement
#[derive(Debug, Clone, PartialEq)]
pub struct TypedStmt {
    pub stmt: Stmt,
    pub span: Span,
}

/// Typed program (output of type checking)
#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    pub items: Vec<TypedModuleItem>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedModuleItem {
    Import,
    Export,
    Stmt(TypedStmt),
    Decl(TypedDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedDecl {
    pub decl: Decl,
    pub span: Span,
}
