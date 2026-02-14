//! Expression definitions for the AST

use super::*;
use std::fmt;

/// Expression
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal values
    Literal(Literal),

    /// Identifier
    Ident(Ident),

    /// Binary operation: left op right
    Binary {
        left: Box<Node<Expr>>,
        op: BinaryOp,
        right: Box<Node<Expr>>,
    },

    /// Unary operation: op expr
    Unary {
        op: UnaryOp,
        expr: Box<Node<Expr>>,
    },

    /// Assignment: target = value
    Assignment {
        target: Box<Node<Expr>>,
        op: AssignmentOp,
        value: Box<Node<Expr>>,
    },

    /// Function call: callee(args)
    Call {
        callee: Box<Node<Expr>>,
        type_args: Option<Vec<Node<Type>>>,
        args: Vec<Node<Expr>>,
    },

    /// Member access: object.property
    Member {
        object: Box<Node<Expr>>,
        property: Node<Ident>,
        computed: bool,
    },

    /// Index access: object[index]
    Index {
        object: Box<Node<Expr>>,
        index: Box<Node<Expr>>,
    },

    /// Array literal: [elem1, elem2, ...]
    Array(Vec<Option<Node<Expr>>>),

    /// Object literal: { key1: value1, key2: value2 }
    Object(Vec<ObjectProperty>),

    /// Arrow function: (params) => body
    Arrow {
        type_params: Option<Vec<TypeParam>>,
        params: Vec<Param>,
        return_type: Option<Box<Node<Type>>>,
        body: ArrowBody,
    },

    /// Function expression: function name?(params) { body }
    Function {
        name: Option<Node<Ident>>,
        type_params: Option<Vec<TypeParam>>,
        params: Vec<Param>,
        return_type: Option<Box<Node<Type>>>,
        body: Box<Node<BlockStmt>>,
        is_async: bool,
    },

    /// Ternary/conditional: condition ? then_expr : else_expr
    Ternary {
        condition: Box<Node<Expr>>,
        then_expr: Box<Node<Expr>>,
        else_expr: Box<Node<Expr>>,
    },

    /// Template literal: `hello ${expr}`
    Template {
        parts: Vec<String>,
        exprs: Vec<Node<Expr>>,
    },

    /// New expression: new Constructor(args)
    New {
        callee: Box<Node<Expr>>,
        type_args: Option<Vec<Node<Type>>>,
        args: Vec<Node<Expr>>,
    },

    /// Type cast: expr as Type
    TypeCast {
        expr: Box<Node<Expr>>,
        ty: Box<Node<Type>>,
    },

    /// Await expression: await expr
    Await(Box<Node<Expr>>),

    /// Parenthesized expression
    Paren(Box<Node<Expr>>),

    /// This expression
    This,

    /// Super expression
    Super,

    /// Clone expression (Zaco extension): clone expr
    Clone(Box<Node<Expr>>),

    /// Sequence expression: expr1, expr2, ...
    Sequence(Vec<Node<Expr>>),

    /// Spread expression: ...expr (in call args, array literals)
    Spread(Box<Node<Expr>>),

    /// Optional chaining call: expr?.(args)
    OptionalCall {
        callee: Box<Node<Expr>>,
        type_args: Option<Vec<Node<Type>>>,
        args: Vec<Node<Expr>>,
    },

    /// Optional chaining index: expr?.[index]
    OptionalIndex {
        object: Box<Node<Expr>>,
        index: Box<Node<Expr>>,
    },

    /// Optional chaining member: expr?.prop (already handled by Member with QuestionDot, but explicit)
    OptionalMember {
        object: Box<Node<Expr>>,
        property: Node<Ident>,
    },

    /// Tagged template literal: tag`template`
    TaggedTemplate {
        tag: Box<Node<Expr>>,
        parts: Vec<String>,
        exprs: Vec<Node<Expr>>,
    },

    /// Satisfies expression: expr satisfies Type
    Satisfies {
        expr: Box<Node<Expr>>,
        ty: Box<Node<Type>>,
    },

    /// Non-null assertion: expr!
    NonNullAssertion(Box<Node<Expr>>),

    /// Meta property: new.target, import.meta
    MetaProperty {
        meta: Node<Ident>,
        property: Node<Ident>,
    },

    /// Yield expression: yield expr, yield* expr
    Yield {
        argument: Option<Box<Node<Expr>>>,
        delegate: bool,
    },
}

/// Literal values
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    RegExp { pattern: String, flags: String },
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,

    // Comparison
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    // Logical
    And,
    Or,
    NullishCoalesce,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    LeftShift,
    RightShift,
    UnsignedRightShift,

    // Other
    In,
    InstanceOf,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
    BitNot,
    TypeOf,
    Void,
    Delete,
    PreIncrement,
    PreDecrement,
    PostIncrement,
    PostDecrement,
}

/// Assignment operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignmentOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
    PowAssign,
    LeftShiftAssign,
    RightShiftAssign,
    UnsignedRightShiftAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    AndAssign,
    OrAssign,
    NullishAssign,
}

/// Arrow function body
#[derive(Debug, Clone, PartialEq)]
pub enum ArrowBody {
    Expr(Box<Node<Expr>>),
    Block(Box<Node<BlockStmt>>),
}

/// Object property
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectProperty {
    Property {
        key: PropertyName,
        value: Node<Expr>,
        shorthand: bool,
    },
    Method {
        key: PropertyName,
        type_params: Option<Vec<TypeParam>>,
        params: Vec<Param>,
        return_type: Option<Box<Node<Type>>>,
        body: Node<BlockStmt>,
    },
    Spread(Node<Expr>),
}

/// Property name
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyName {
    Ident(Node<Ident>),
    String(String),
    Number(f64),
    Computed(Box<Node<Expr>>),
}

// Display implementations

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Pow => "**",
            BinaryOp::Eq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::StrictEq => "===",
            BinaryOp::StrictNotEq => "!==",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
            BinaryOp::NullishCoalesce => "??",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::LeftShift => "<<",
            BinaryOp::RightShift => ">>",
            BinaryOp::UnsignedRightShift => ">>>",
            BinaryOp::In => "in",
            BinaryOp::InstanceOf => "instanceof",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UnaryOp::Plus => "+",
            UnaryOp::Minus => "-",
            UnaryOp::Not => "!",
            UnaryOp::BitNot => "~",
            UnaryOp::TypeOf => "typeof",
            UnaryOp::Void => "void",
            UnaryOp::Delete => "delete",
            UnaryOp::PreIncrement => "++",
            UnaryOp::PreDecrement => "--",
            UnaryOp::PostIncrement => "++",
            UnaryOp::PostDecrement => "--",
        };
        write!(f, "{}", s)
    }
}
