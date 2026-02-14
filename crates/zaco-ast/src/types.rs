//! Type definitions for the AST

use super::*;
use std::fmt;

/// Type expression
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Primitive types: number, string, boolean, void, null, undefined, any, never, unknown
    Primitive(PrimitiveType),

    /// Array type: T[]
    Array(Box<Node<Type>>),

    /// Tuple type: [T1, T2, ...]
    Tuple(Vec<Node<Type>>),

    /// Union type: T1 | T2 | ...
    Union(Vec<Node<Type>>),

    /// Intersection type: T1 & T2 & ...
    Intersection(Vec<Node<Type>>),

    /// Function type: (args) => ReturnType
    Function(FunctionType),

    /// Generic type: Array<T>
    Generic {
        base: Box<Node<Type>>,
        type_args: Vec<Node<Type>>,
    },

    /// Type reference: SomeType
    TypeRef {
        name: Node<Ident>,
        type_args: Option<Vec<Node<Type>>>,
    },

    /// Object type: { prop1: Type1, prop2: Type2 }
    Object(ObjectType),

    /// Literal type: "hello" | 42 | true
    Literal(LiteralType),

    /// Parenthesized type
    Paren(Box<Node<Type>>),

    /// Type with ownership annotation (Zaco extension)
    WithOwnership {
        base: Box<Node<Type>>,
        ownership: Ownership,
    },

    /// Conditional type: T extends U ? X : Y
    Conditional {
        check_type: Box<Node<Type>>,
        extends_type: Box<Node<Type>>,
        true_type: Box<Node<Type>>,
        false_type: Box<Node<Type>>,
    },

    /// Mapped type: { [K in keyof T]: V }
    Mapped {
        type_param: Node<Ident>,
        constraint: Box<Node<Type>>,
        name_type: Option<Box<Node<Type>>>,
        value_type: Box<Node<Type>>,
        readonly: Option<MappedModifier>,
        optional: Option<MappedModifier>,
    },

    /// Template literal type: `hello ${string}`
    TemplateLiteral {
        parts: Vec<String>,
        types: Vec<Node<Type>>,
    },

    /// Indexed access type: T[K]
    IndexedAccess {
        object_type: Box<Node<Type>>,
        index_type: Box<Node<Type>>,
    },

    /// keyof type: keyof T
    Keyof(Box<Node<Type>>),

    /// typeof type: typeof expr
    TypeofType(Box<Node<Type>>),

    /// infer type: infer T (used in conditional types)
    Infer(Node<Ident>),

    /// Import type: import("module").Type
    ImportType {
        argument: String,
        qualifier: Option<Box<Node<Type>>>,
        type_args: Option<Vec<Node<Type>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Number,
    String,
    Boolean,
    Void,
    Null,
    Undefined,
    Any,
    Never,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub type_params: Option<Vec<TypeParam>>,
    pub params: Vec<FunctionTypeParam>,
    pub return_type: Box<Node<Type>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionTypeParam {
    pub name: Option<Node<Ident>>,
    pub ty: Node<Type>,
    pub optional: bool,
    pub ownership: Option<Ownership>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectType {
    pub members: Vec<ObjectTypeMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectTypeMember {
    Property {
        name: PropertyName,
        ty: Node<Type>,
        optional: bool,
        readonly: bool,
    },
    Method {
        name: PropertyName,
        type_params: Option<Vec<TypeParam>>,
        params: Vec<FunctionTypeParam>,
        return_type: Node<Type>,
        optional: bool,
    },
    IndexSignature {
        key_name: Node<Ident>,
        key_type: Node<Type>,
        value_type: Node<Type>,
    },
    CallSignature {
        type_params: Option<Vec<TypeParam>>,
        params: Vec<FunctionTypeParam>,
        return_type: Node<Type>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralType {
    String(String),
    Number(f64),
    Boolean(bool),
}

/// Type parameter (generic)
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: Node<Ident>,
    pub constraint: Option<Box<Node<Type>>>,
    pub default: Option<Box<Node<Type>>>,
}

/// Modifier for mapped types (+/- readonly, +/- optional)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MappedModifier {
    /// Add the modifier (+readonly, +?)
    Add,
    /// Remove the modifier (-readonly, -?)
    Remove,
    /// Keep the modifier as-is (readonly, ?)
    Present,
}

// Display implementations

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::Number => write!(f, "number"),
            PrimitiveType::String => write!(f, "string"),
            PrimitiveType::Boolean => write!(f, "boolean"),
            PrimitiveType::Void => write!(f, "void"),
            PrimitiveType::Null => write!(f, "null"),
            PrimitiveType::Undefined => write!(f, "undefined"),
            PrimitiveType::Any => write!(f, "any"),
            PrimitiveType::Never => write!(f, "never"),
            PrimitiveType::Unknown => write!(f, "unknown"),
        }
    }
}
