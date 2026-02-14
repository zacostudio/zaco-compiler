//! IR type system and related definitions.

use std::fmt;

use crate::StructId;

/// IR type system representing all possible types in the IR.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IrType {
    /// 64-bit signed integer (TypeScript number integers)
    I64,
    /// 64-bit floating point (TypeScript number)
    F64,
    /// Boolean type
    Bool,
    /// Pointer to heap-allocated data
    Ptr,
    /// Void/unit type (no value)
    Void,
    /// String type (heap-allocated, reference-counted)
    Str,
    /// Array type containing elements of a specific type
    Array(Box<IrType>),
    /// Struct type identified by StructId
    Struct(StructId),
    /// Function pointer with signature
    FuncPtr(FuncSignature),
    /// Promise type wrapping the resolved value type
    Promise(Box<IrType>),
}

impl IrType {
    /// Returns true if this type requires heap allocation.
    pub fn is_heap_allocated(&self) -> bool {
        matches!(
            self,
            IrType::Str | IrType::Array(_) | IrType::Struct(_)
        )
    }

    /// Returns true if this type is a pointer type.
    pub fn is_pointer(&self) -> bool {
        matches!(self, IrType::Ptr | IrType::Str | IrType::Array(_) | IrType::Struct(_) | IrType::FuncPtr(_) | IrType::Promise(_))
    }

    /// Returns the size in bytes of this type (approximate for IR purposes).
    pub fn size_bytes(&self) -> usize {
        match self {
            IrType::I64 => 8,
            IrType::F64 => 8,
            IrType::Bool => 1,
            IrType::Ptr => 8,
            IrType::Void => 0,
            IrType::Str => 8, // Pointer size
            IrType::Array(_) => 8, // Pointer size
            IrType::Struct(_) => 8, // Pointer size
            IrType::FuncPtr(_) => 8, // Pointer size
            IrType::Promise(_) => 8, // Pointer size
        }
    }
}

impl fmt::Display for IrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrType::I64 => write!(f, "i64"),
            IrType::F64 => write!(f, "f64"),
            IrType::Bool => write!(f, "bool"),
            IrType::Ptr => write!(f, "ptr"),
            IrType::Void => write!(f, "void"),
            IrType::Str => write!(f, "str"),
            IrType::Array(ty) => write!(f, "[{}]", ty),
            IrType::Struct(id) => write!(f, "{}", id),
            IrType::FuncPtr(sig) => {
                write!(f, "fn(")?;
                for (i, param) in sig.params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", sig.return_type)
            }
            IrType::Promise(ty) => write!(f, "Promise<{}>", ty),
        }
    }
}

/// Function signature describing parameter and return types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncSignature {
    /// Parameter types
    pub params: Vec<IrType>,
    /// Return type
    pub return_type: Box<IrType>,
}

/// Compile-time constant values.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    /// 64-bit signed integer constant
    I64(i64),
    /// 64-bit floating point constant
    F64(f64),
    /// Boolean constant
    Bool(bool),
    /// String literal constant (index into module's string_literals)
    Str(String),
    /// Null pointer constant
    Null,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Logical
    And,
    Or,

    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
        };
        write!(f, "{}", s)
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    /// Arithmetic negation (-)
    Neg,
    /// Logical negation (!)
    Not,
    /// Bitwise negation (~)
    BitNot,
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            UnOp::Neg => "-",
            UnOp::Not => "!",
            UnOp::BitNot => "~",
        };
        write!(f, "{}", s)
    }
}
