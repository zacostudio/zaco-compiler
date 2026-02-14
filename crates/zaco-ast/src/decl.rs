//! Declaration definitions for the AST

use super::*;
use std::fmt;

/// Top-level declaration
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    /// Function declaration
    Function(FunctionDecl),

    /// Class declaration
    Class(ClassDecl),

    /// Interface declaration
    Interface(InterfaceDecl),

    /// Type alias declaration
    TypeAlias(TypeAliasDecl),

    /// Enum declaration
    Enum(EnumDecl),

    /// Module/namespace declaration
    Module(ModuleDecl),

    /// Variable declaration
    Var(VarDecl),
}

/// Function declaration
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: Node<Ident>,
    pub type_params: Option<Vec<TypeParam>>,
    pub params: Vec<Param>,
    pub return_type: Option<Box<Node<Type>>>,
    pub body: Option<Node<BlockStmt>>,
    pub is_async: bool,
    pub is_generator: bool,
    pub is_declare: bool,
}

/// Function parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub pattern: Node<Pattern>,
    pub type_annotation: Option<Box<Node<Type>>>,
    pub ownership: Option<Ownership>,
    pub optional: bool,
    pub is_rest: bool,
}

/// Class declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: Node<Ident>,
    pub type_params: Option<Vec<TypeParam>>,
    pub extends: Option<ClassExtends>,
    pub implements: Vec<Node<Type>>,
    pub members: Vec<ClassMember>,
    pub is_abstract: bool,
    pub is_declare: bool,
    pub decorators: Vec<Node<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassExtends {
    pub base: Box<Node<Expr>>,
    pub type_args: Option<Vec<Node<Type>>>,
}

/// Class member
#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    /// Constructor
    Constructor {
        params: Vec<Param>,
        body: Option<Node<BlockStmt>>,
        access: AccessModifier,
    },

    /// Method
    Method {
        name: PropertyName,
        type_params: Option<Vec<TypeParam>>,
        params: Vec<Param>,
        return_type: Option<Box<Node<Type>>>,
        body: Option<Node<BlockStmt>>,
        access: AccessModifier,
        is_static: bool,
        is_async: bool,
        is_abstract: bool,
        is_optional: bool,
        is_override: bool,
        decorators: Vec<Node<Expr>>,
    },

    /// Property/field
    Property {
        name: PropertyName,
        type_annotation: Option<Box<Node<Type>>>,
        ownership: Option<Ownership>,
        init: Option<Node<Expr>>,
        access: AccessModifier,
        is_static: bool,
        is_readonly: bool,
        is_abstract: bool,
        is_optional: bool,
        is_override: bool,
        decorators: Vec<Node<Expr>>,
    },

    /// Getter
    Getter {
        name: PropertyName,
        return_type: Option<Box<Node<Type>>>,
        body: Option<Node<BlockStmt>>,
        access: AccessModifier,
        is_static: bool,
        is_abstract: bool,
    },

    /// Setter
    Setter {
        name: PropertyName,
        param: Param,
        body: Option<Node<BlockStmt>>,
        access: AccessModifier,
        is_static: bool,
        is_abstract: bool,
    },

    /// Index signature
    IndexSignature {
        key_name: Node<Ident>,
        key_type: Node<Type>,
        value_type: Node<Type>,
        is_readonly: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessModifier {
    Public,
    Private,
    Protected,
}

/// Interface declaration
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: Node<Ident>,
    pub type_params: Option<Vec<TypeParam>>,
    pub extends: Vec<Node<Type>>,
    pub members: Vec<ObjectTypeMember>,
    pub is_declare: bool,
}

/// Type alias declaration
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasDecl {
    pub name: Node<Ident>,
    pub type_params: Option<Vec<TypeParam>>,
    pub ty: Node<Type>,
    pub is_declare: bool,
}

/// Enum declaration
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: Node<Ident>,
    pub members: Vec<EnumMember>,
    pub is_const: bool,
    pub is_declare: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumMember {
    pub name: Node<Ident>,
    pub init: Option<Node<Expr>>,
}

/// Module/namespace declaration
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDecl {
    pub name: ModuleName,
    pub body: ModuleBody,
    pub is_declare: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleName {
    Ident(Node<Ident>),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleBody {
    Block(Vec<Node<ModuleItem>>),
    Namespace(Box<Node<ModuleDecl>>),
}

// Display implementations

impl fmt::Display for AccessModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessModifier::Public => write!(f, "public"),
            AccessModifier::Private => write!(f, "private"),
            AccessModifier::Protected => write!(f, "protected"),
        }
    }
}
