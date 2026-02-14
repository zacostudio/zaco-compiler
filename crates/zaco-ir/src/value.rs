//! Value, place, and right-value definitions.

use crate::{BinOp, Constant, IrType, LocalId, StructId, TempId, UnOp};

/// Represents a value that can be used in computations.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Compile-time constant
    Const(Constant),
    /// Local variable reference
    Local(LocalId),
    /// Temporary value reference
    Temp(TempId),
}

/// Projection applied to a place (field access, array indexing, dereference).
#[derive(Debug, Clone, PartialEq)]
pub enum Projection {
    /// Access a struct field by index
    Field(usize),
    /// Array/pointer indexing
    Index(Value),
    /// Pointer dereference
    Deref,
}

/// A place represents a location in memory that can be read from or written to.
/// Places are composed of a base value and a series of projections.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    /// Base value (local, temp, or const)
    pub base: Value,
    /// Projections applied to the base (field access, indexing, deref)
    pub projections: Vec<Projection>,
}

impl Place {
    /// Creates a simple place from a value with no projections.
    pub fn from_value(value: Value) -> Self {
        Place {
            base: value,
            projections: Vec::new(),
        }
    }

    /// Creates a place from a local variable.
    pub fn from_local(local: LocalId) -> Self {
        Place {
            base: Value::Local(local),
            projections: Vec::new(),
        }
    }

    /// Creates a place from a temporary.
    pub fn from_temp(temp: TempId) -> Self {
        Place {
            base: Value::Temp(temp),
            projections: Vec::new(),
        }
    }

    /// Adds a field projection.
    pub fn field(mut self, index: usize) -> Self {
        self.projections.push(Projection::Field(index));
        self
    }

    /// Adds an index projection.
    pub fn index(mut self, index: Value) -> Self {
        self.projections.push(Projection::Index(index));
        self
    }

    /// Adds a dereference projection.
    pub fn deref(mut self) -> Self {
        self.projections.push(Projection::Deref);
        self
    }
}

/// Right-hand side of an assignment - represents a computation.
#[derive(Debug, Clone, PartialEq)]
pub enum RValue {
    /// Use a value directly
    Use(Value),

    /// Binary operation
    BinaryOp {
        op: BinOp,
        left: Value,
        right: Value,
    },

    /// Unary operation
    UnaryOp {
        op: UnOp,
        operand: Value,
    },

    /// Type cast
    Cast {
        value: Value,
        ty: IrType,
    },

    /// Struct initialization
    StructInit {
        struct_id: StructId,
        fields: Vec<Value>,
    },

    /// Array initialization
    ArrayInit(Vec<Value>),

    /// String concatenation
    StrConcat(Vec<Value>),
}
