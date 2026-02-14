//! Helper methods for type checking

use std::collections::HashMap;
use zaco_ast::{OwnershipKind, PrimitiveType, PropertyName};
use crate::types::{LiteralType, Type};
use crate::ownership::OwnershipState;
use crate::env::TypeEnv;

/// Helper methods for type conversion and checking
pub struct TypeHelpers;

impl TypeHelpers {
    pub fn convert_primitive(prim: &PrimitiveType) -> Type {
        match prim {
            PrimitiveType::Number => Type::Number,
            PrimitiveType::String => Type::String,
            PrimitiveType::Boolean => Type::Boolean,
            PrimitiveType::Void => Type::Void,
            PrimitiveType::Null => Type::Null,
            PrimitiveType::Undefined => Type::Undefined,
            PrimitiveType::Any => Type::Any,
            PrimitiveType::Never => Type::Never,
            PrimitiveType::Unknown => Type::Unknown,
        }
    }

    pub fn convert_literal_type(lit: &zaco_ast::LiteralType) -> LiteralType {
        match lit {
            zaco_ast::LiteralType::String(s) => LiteralType::String(s.clone()),
            zaco_ast::LiteralType::Number(n) => LiteralType::Number(*n),
            zaco_ast::LiteralType::Boolean(b) => LiteralType::Boolean(*b),
        }
    }

    pub fn convert_ownership(kind: &OwnershipKind) -> OwnershipState {
        match kind {
            OwnershipKind::Owned => OwnershipState::Owned,
            OwnershipKind::Ref => OwnershipState::Borrowed,
            OwnershipKind::MutRef => OwnershipState::MutBorrowed,
            OwnershipKind::Inferred => OwnershipState::Owned,
        }
    }

    pub fn property_name_to_string(name: &PropertyName) -> String {
        match name {
            PropertyName::Ident(ident) => ident.value.name.clone(),
            PropertyName::String(s) => s.clone(),
            PropertyName::Number(n) => n.to_string(),
            PropertyName::Computed(_) => "__computed__".to_string(),
        }
    }

    /// Resolve a TypeRef to its underlying type using the environment.
    /// Returns the resolved type, or the original type if no resolution is found.
    pub fn resolve_type<'a>(ty: &'a Type, env: Option<&'a TypeEnv>) -> &'a Type {
        if let (Type::TypeRef { name, .. }, Some(env)) = (ty, env) {
            if let Some(resolved) = env.lookup_type(name) {
                return resolved;
            }
        }
        ty
    }

    #[allow(dead_code)]
    pub fn is_assignable(from: &Type, to: &Type) -> bool {
        Self::is_assignable_with_env(from, to, None)
    }

    pub fn is_assignable_with_env(from: &Type, to: &Type, env: Option<&TypeEnv>) -> bool {
        // Resolve TypeRef before comparison
        let from = Self::resolve_type(from, env);
        let to = Self::resolve_type(to, env);

        if from == to {
            return true;
        }

        match (from, to) {
            // Any is compatible with everything
            (_, Type::Any) | (Type::Any, _) => true,
            // Unresolved TypeRef (generic type parameters like T, U) are compatible with anything
            (Type::TypeRef { .. }, _) | (_, Type::TypeRef { .. }) => true,
            // Everything is assignable to Unknown
            (_, Type::Unknown) => true,
            // Never is assignable to everything (bottom type)
            (Type::Never, _) => true,
            // Null and Undefined are distinct — do NOT treat as interchangeable
            // Literal types widen to their base types
            (Type::Literal(lit), Type::Number) if matches!(lit, LiteralType::Number(_)) => true,
            (Type::Literal(lit), Type::String) if matches!(lit, LiteralType::String(_)) => true,
            (Type::Literal(lit), Type::Boolean) if matches!(lit, LiteralType::Boolean(_)) => true,
            // Array covariance
            (Type::Array(from_elem), Type::Array(to_elem)) => {
                Self::is_assignable_with_env(from_elem, to_elem, env)
            }
            // Promise covariance
            (Type::Promise(from_inner), Type::Promise(to_inner)) => {
                Self::is_assignable_with_env(from_inner, to_inner, env)
            }
            // Source is a union: ALL members must be assignable to target
            (Type::Union(members), _) => {
                members.iter().all(|m| Self::is_assignable_with_env(m, to, env))
            }
            // Target is a union: source must be assignable to ANY member
            (_, Type::Union(members)) => {
                members.iter().any(|m| Self::is_assignable_with_env(from, m, env))
            }
            // Function assignability (basic: same arity, contravariant params, covariant return)
            (
                Type::Function { params: from_params, return_type: from_ret },
                Type::Function { params: to_params, return_type: to_ret },
            ) => {
                if from_params.len() != to_params.len() {
                    return false;
                }
                // Params are contravariant (simplified: just check assignable in either direction)
                for (fp, tp) in from_params.iter().zip(to_params.iter()) {
                    if !Self::is_assignable_with_env(tp, fp, env) && !Self::is_assignable_with_env(fp, tp, env) {
                        return false;
                    }
                }
                Self::is_assignable_with_env(from_ret, to_ret, env)
            }
            _ => false,
        }
    }

    pub fn is_numeric(ty: &Type) -> bool {
        matches!(
            ty,
            Type::Number | Type::Literal(LiteralType::Number(_))
        )
    }

    pub fn is_string(ty: &Type) -> bool {
        matches!(
            ty,
            Type::String | Type::Literal(LiteralType::String(_))
        )
    }

    pub fn union_type(types: Vec<Type>) -> Type {
        if types.is_empty() {
            Type::Never
        } else if types.len() == 1 {
            types[0].clone()
        } else {
            Type::Union(types)
        }
    }

    /// Substitute type parameters with concrete types.
    /// Walks the type tree recursively, replacing Generic/TypeRef names found in `params`
    /// with their concrete types.
    pub fn substitute_type_params(ty: &Type, params: &HashMap<String, Type>) -> Type {
        match ty {
            // A generic type parameter like T — substitute if we have a mapping
            Type::Generic { name, .. } => {
                if let Some(concrete) = params.get(name) {
                    concrete.clone()
                } else {
                    ty.clone()
                }
            }
            // A TypeRef without type_args might be a bare type parameter name (e.g., T)
            Type::TypeRef { name, type_args } if type_args.is_empty() => {
                if let Some(concrete) = params.get(name) {
                    concrete.clone()
                } else {
                    ty.clone()
                }
            }
            // A TypeRef with type_args — substitute inside the args
            Type::TypeRef { name, type_args } => {
                let new_args: Vec<Type> = type_args.iter()
                    .map(|a| Self::substitute_type_params(a, params))
                    .collect();
                Type::TypeRef { name: name.clone(), type_args: new_args }
            }
            // Recurse into composite types
            Type::Array(elem) => {
                Type::Array(Box::new(Self::substitute_type_params(elem, params)))
            }
            Type::Promise(inner) => {
                Type::Promise(Box::new(Self::substitute_type_params(inner, params)))
            }
            Type::Tuple(types) => {
                Type::Tuple(types.iter().map(|t| Self::substitute_type_params(t, params)).collect())
            }
            Type::Union(types) => {
                Type::Union(types.iter().map(|t| Self::substitute_type_params(t, params)).collect())
            }
            Type::Intersection(types) => {
                Type::Intersection(types.iter().map(|t| Self::substitute_type_params(t, params)).collect())
            }
            Type::Function { params: fn_params, return_type } => {
                Type::Function {
                    params: fn_params.iter().map(|t| Self::substitute_type_params(t, params)).collect(),
                    return_type: Box::new(Self::substitute_type_params(return_type, params)),
                }
            }
            Type::Object { properties } => {
                Type::Object {
                    properties: properties.iter()
                        .map(|(name, t, opt)| (name.clone(), Self::substitute_type_params(t, params), *opt))
                        .collect(),
                }
            }
            Type::Class { name, fields, methods } => {
                Type::Class {
                    name: name.clone(),
                    fields: fields.iter()
                        .map(|(n, t)| (n.clone(), Self::substitute_type_params(t, params)))
                        .collect(),
                    methods: methods.iter()
                        .map(|(n, t)| (n.clone(), Self::substitute_type_params(t, params)))
                        .collect(),
                }
            }
            Type::Interface { name, properties } => {
                Type::Interface {
                    name: name.clone(),
                    properties: properties.iter()
                        .map(|(n, t, opt)| (n.clone(), Self::substitute_type_params(t, params), *opt))
                        .collect(),
                }
            }
            // Primitive and other types pass through unchanged
            _ => ty.clone(),
        }
    }
}
