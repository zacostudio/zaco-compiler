//! Statement definitions for the AST

use super::*;
use std::fmt;

/// Statement
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Expression statement
    Expr(Node<Expr>),

    /// Variable declaration: let/const/var name: Type = value
    VarDecl(VarDecl),

    /// Return statement
    Return(Option<Node<Expr>>),

    /// If statement
    If {
        condition: Node<Expr>,
        then_stmt: Box<Node<Stmt>>,
        else_stmt: Option<Box<Node<Stmt>>>,
    },

    /// For statement
    For {
        init: Option<ForInit>,
        condition: Option<Node<Expr>>,
        update: Option<Node<Expr>>,
        body: Box<Node<Stmt>>,
    },

    /// For-in statement: for (left in right) body
    ForIn {
        left: ForInLeft,
        right: Node<Expr>,
        body: Box<Node<Stmt>>,
    },

    /// For-of statement: for (left of right) body
    ForOf {
        left: ForInLeft,
        right: Node<Expr>,
        body: Box<Node<Stmt>>,
        is_await: bool,
    },

    /// While statement
    While {
        condition: Node<Expr>,
        body: Box<Node<Stmt>>,
    },

    /// Do-while statement
    DoWhile {
        body: Box<Node<Stmt>>,
        condition: Node<Expr>,
    },

    /// Block statement: { stmts }
    Block(BlockStmt),

    /// Break statement
    Break(Option<Node<Ident>>),

    /// Continue statement
    Continue(Option<Node<Ident>>),

    /// Throw statement
    Throw(Node<Expr>),

    /// Try-catch-finally statement
    Try {
        block: Node<BlockStmt>,
        catch: Option<CatchClause>,
        finally: Option<Node<BlockStmt>>,
    },

    /// Switch statement
    Switch {
        discriminant: Node<Expr>,
        cases: Vec<SwitchCase>,
    },

    /// Labeled statement
    Labeled {
        label: Node<Ident>,
        stmt: Box<Node<Stmt>>,
    },

    /// Empty statement: ;
    Empty,

    /// Debugger statement
    Debugger,
}

/// Block statement
#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt {
    pub stmts: Vec<Node<Stmt>>,
}

/// Variable declaration
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub kind: VarDeclKind,
    pub declarations: Vec<VarDeclarator>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarDeclKind {
    Let,
    Const,
    Var,
    Using,
    AwaitUsing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDeclarator {
    pub pattern: Node<Pattern>,
    pub init: Option<Node<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForInit {
    VarDecl(VarDecl),
    Expr(Node<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForInLeft {
    VarDecl(VarDecl),
    Pattern(Node<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub param: Option<Node<Pattern>>,
    pub body: Node<BlockStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub test: Option<Node<Expr>>,
    pub consequent: Vec<Node<Stmt>>,
}

/// Pattern for destructuring
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Identifier pattern
    Ident {
        name: Node<Ident>,
        type_annotation: Option<Box<Node<Type>>>,
        ownership: Option<Ownership>,
    },

    /// Array pattern: [a, b, ...rest]
    Array {
        elements: Vec<Option<Node<Pattern>>>,
        rest: Option<Box<Node<Pattern>>>,
    },

    /// Object pattern: { a, b: c, ...rest }
    Object {
        properties: Vec<ObjectPatternProperty>,
        rest: Option<Box<Node<Pattern>>>,
    },

    /// Assignment pattern: pattern = default_value
    Assignment {
        pattern: Box<Node<Pattern>>,
        default: Box<Node<Expr>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectPatternProperty {
    pub key: PropertyName,
    pub value: Node<Pattern>,
    pub shorthand: bool,
}

// Display implementations

impl fmt::Display for VarDeclKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VarDeclKind::Let => write!(f, "let"),
            VarDeclKind::Const => write!(f, "const"),
            VarDeclKind::Var => write!(f, "var"),
            VarDeclKind::Using => write!(f, "using"),
            VarDeclKind::AwaitUsing => write!(f, "await using"),
        }
    }
}
