//! Module system definitions for the AST

use super::*;

/// Module item (top-level in a module)
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleItem {
    /// Import declaration
    Import(ImportDecl),

    /// Export declaration
    Export(ExportDecl),

    /// Statement
    Stmt(Node<Stmt>),

    /// Declaration
    Decl(Node<Decl>),
}

/// Import declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub specifiers: Vec<ImportSpecifier>,
    pub source: String,
    pub type_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportSpecifier {
    /// import name from "module"
    Default(Node<Ident>),

    /// import * as name from "module"
    Namespace(Node<Ident>),

    /// import { name } from "module" or import { name as alias } from "module"
    Named {
        imported: Node<Ident>,
        local: Option<Node<Ident>>,
        type_only: bool,
    },
}

/// Export declaration
#[derive(Debug, Clone, PartialEq)]
pub enum ExportDecl {
    /// export { name }
    Named {
        specifiers: Vec<ExportSpecifier>,
        source: Option<String>,
        type_only: bool,
    },

    /// export default expr
    Default(Node<Expr>),

    /// export default declaration
    DefaultDecl(Box<Node<Decl>>),

    /// export * from "module"
    All {
        source: String,
        as_name: Option<Node<Ident>>,
        type_only: bool,
    },

    /// export declaration
    Decl(Box<Node<Decl>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportSpecifier {
    pub local: Node<Ident>,
    pub exported: Option<Node<Ident>>,
    pub type_only: bool,
}

/// Root AST node - represents a complete source file
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Node<ModuleItem>>,
    pub span: Span,
}
