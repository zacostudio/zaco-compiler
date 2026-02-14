//! Internal type representation

/// Internal type representation used by the type checker
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Primitive types
    Number,
    String,
    Boolean,
    Void,
    Null,
    Undefined,
    Any,
    Never,
    Unknown,

    /// Array type
    Array(Box<Type>),

    /// Tuple type
    Tuple(Vec<Type>),

    /// Union type
    Union(Vec<Type>),

    /// Intersection type
    Intersection(Vec<Type>),

    /// Function type
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },

    /// Object type
    Object {
        properties: Vec<(String, Type, bool)>, // (name, type, optional)
    },

    /// Class type
    Class {
        name: String,
        fields: Vec<(String, Type)>,
        methods: Vec<(String, Type)>,
    },

    /// Generic type parameter
    Generic {
        name: String,
        constraint: Option<Box<Type>>,
    },

    /// Type reference (named type, with optional type arguments)
    TypeRef { name: String, type_args: Vec<Type> },

    /// Promise type wrapping the resolved value type
    Promise(Box<Type>),

    /// Literal type
    Literal(LiteralType),

    /// Enum type
    Enum {
        name: String,
        members: Vec<String>,
    },

    /// Interface type (similar to Object but nominal)
    Interface {
        name: String,
        properties: Vec<(String, Type, bool)>,
    },
}

/// Literal types
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralType {
    String(String),
    Number(f64),
    Boolean(bool),
}
