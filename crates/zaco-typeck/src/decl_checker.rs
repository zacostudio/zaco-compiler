//! Declaration checking methods

use zaco_ast::{
    ClassDecl, ClassMember, Decl, EnumDecl, Expr, FunctionDecl, InterfaceDecl,
    ObjectTypeMember, Param, Pattern, Span, TypeAliasDecl,
};
use crate::checker::TypeChecker;
use crate::error::TypeError;
use crate::types::Type;
use crate::ownership::{OwnershipState, VarInfo};
use crate::helpers::TypeHelpers;

impl TypeChecker {
    pub(crate) fn check_decl(&mut self, decl: &Decl, span: &Span) -> Result<(), TypeError> {
        match decl {
            Decl::Function(func) => self.check_function_decl(func, span),
            Decl::Class(class) => self.check_class_decl(class, span),
            Decl::Interface(interface) => self.check_interface_decl(interface, span),
            Decl::TypeAlias(alias) => self.check_type_alias(alias, span),
            Decl::Enum(enum_decl) => self.check_enum_decl(enum_decl, span),
            Decl::Module(_) => Ok(()), // Module declarations are handled separately
            Decl::Var(var_decl) => self.check_var_decl(var_decl, span),
        }
    }

    fn check_function_decl(
        &mut self,
        func: &FunctionDecl,
        _span: &Span,
    ) -> Result<(), TypeError> {
        // Convert parameters to types
        let mut param_types = Vec::new();
        for param in &func.params {
            let param_ty = self.resolve_param_type(param)?;
            param_types.push(param_ty);
        }

        // Get return type
        let return_type = if let Some(ret_ty) = &func.return_type {
            self.convert_ast_type(&ret_ty.value)?
        } else {
            Type::Void
        };

        let func_type = Type::Function {
            params: param_types,
            return_type: Box::new(return_type),
        };

        // Declare function in environment
        self.env.declare(
            func.name.value.name.clone(),
            VarInfo {
                ty: func_type,
                ownership: OwnershipState::Owned,
                is_mutable: false,
                is_initialized: true,
            },
        );

        // Check function body
        if let Some(body) = &func.body {
            self.env.push_scope();

            // Track the declared return type for return-statement validation
            let prev_return_type = self.current_return_type.take();
            if let Some(ret_ty) = &func.return_type {
                let rt = self.convert_ast_type(&ret_ty.value)?;
                // Don't validate returns against Void — it just means no meaningful return
                if rt != Type::Void {
                    self.current_return_type = Some(rt);
                }
            }

            // Declare parameters in function scope
            for param in &func.params {
                self.check_param(param)?;
            }

            self.check_block_stmt(&body.value, &body.span)?;
            self.env.pop_scope();

            // Restore previous return type (for nested functions)
            self.current_return_type = prev_return_type;
        }

        Ok(())
    }

    pub(crate) fn check_param(&mut self, param: &Param) -> Result<(), TypeError> {
        let param_ty = self.resolve_param_type(param)?;

        // Extract parameter name and declare it
        match &param.pattern.value {
            Pattern::Ident { name, ownership, .. } => {
                let ownership_state = if let Some(own) = ownership {
                    TypeHelpers::convert_ownership(&own.kind)
                } else {
                    OwnershipState::Owned
                };

                self.env.declare(
                    name.value.name.clone(),
                    VarInfo {
                        ty: param_ty,
                        ownership: ownership_state,
                        is_mutable: true, // Parameters are mutable by default
                        is_initialized: true,
                    },
                );
            }
            _ => {
                // Handle destructuring patterns
                // For now, simplified handling
            }
        }

        Ok(())
    }

    /// Extract the effective type annotation from a Param.
    /// The parser may place the type annotation on either `Param.type_annotation`
    /// or `Pattern::Ident.type_annotation` depending on the parsing context.
    pub(crate) fn resolve_param_type(&self, param: &Param) -> Result<Type, TypeError> {
        // First check Param-level type annotation
        if let Some(type_ann) = &param.type_annotation {
            return self.convert_ast_type(&type_ann.value);
        }
        // Fall back to Pattern::Ident-level type annotation
        if let Pattern::Ident { type_annotation: Some(type_ann), .. } = &param.pattern.value {
            return self.convert_ast_type(&type_ann.value);
        }
        Ok(Type::Unknown)
    }

    fn check_class_decl(&mut self, class: &ClassDecl, _span: &Span) -> Result<(), TypeError> {
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        // Inherit fields and methods from parent class (if extends)
        if let Some(ref extends) = class.extends {
            if let Expr::Ident(parent_ident) = &extends.base.value {
                if let Some(parent_type) = self.env.lookup_type(&parent_ident.name) {
                    if let Type::Class { fields: parent_fields, methods: parent_methods, .. } = parent_type {
                        fields.extend(parent_fields.clone());
                        methods.extend(parent_methods.clone());
                    }
                }
            }
        }

        for member in &class.members {
            match member {
                ClassMember::Property {
                    name,
                    type_annotation,
                    ..
                } => {
                    let prop_name = TypeHelpers::property_name_to_string(name);
                    let prop_ty = if let Some(type_ann) = type_annotation {
                        self.convert_ast_type(&type_ann.value)?
                    } else {
                        Type::Unknown
                    };
                    fields.push((prop_name, prop_ty));
                }
                ClassMember::Method {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let method_name = TypeHelpers::property_name_to_string(name);
                    let mut param_types = Vec::new();
                    for param in params {
                        let param_ty = self.resolve_param_type(param)?;
                        param_types.push(param_ty);
                    }

                    let ret_ty = if let Some(ret_ty) = return_type {
                        self.convert_ast_type(&ret_ty.value)?
                    } else {
                        Type::Void
                    };

                    let method_ty = Type::Function {
                        params: param_types,
                        return_type: Box::new(ret_ty),
                    };
                    methods.push((method_name, method_ty));
                }
                _ => {} // Handle other members
            }
        }

        let class_type = Type::Class {
            name: class.name.value.name.clone(),
            fields,
            methods,
        };

        self.env.define_class(class.name.value.name.clone(), class_type.clone());

        // Register generic type parameter names if present
        if let Some(ref type_params) = class.type_params {
            let param_names: Vec<String> = type_params.iter()
                .map(|tp| tp.name.value.name.clone())
                .collect();
            self.env.define_type_params(class.name.value.name.clone(), param_names);
        }

        // Also declare constructor
        self.env.declare(
            class.name.value.name.clone(),
            VarInfo {
                ty: class_type,
                ownership: OwnershipState::Owned,
                is_mutable: false,
                is_initialized: true,
            },
        );

        Ok(())
    }

    fn check_interface_decl(
        &mut self,
        interface: &InterfaceDecl,
        _span: &Span,
    ) -> Result<(), TypeError> {
        let mut properties = Vec::new();

        for member in &interface.members {
            match member {
                ObjectTypeMember::Property {
                    name,
                    ty,
                    optional,
                    ..
                } => {
                    let prop_name = TypeHelpers::property_name_to_string(name);
                    let prop_ty = self.convert_ast_type(&ty.value)?;
                    properties.push((prop_name, prop_ty, *optional));
                }
                _ => {} // Handle other members
            }
        }

        let interface_type = Type::Interface {
            name: interface.name.value.name.clone(),
            properties,
        };

        self.env
            .define_interface(interface.name.value.name.clone(), interface_type);

        // Register generic type parameter names if present
        if let Some(ref type_params) = interface.type_params {
            let param_names: Vec<String> = type_params.iter()
                .map(|tp| tp.name.value.name.clone())
                .collect();
            self.env.define_type_params(interface.name.value.name.clone(), param_names);
        }

        Ok(())
    }

    fn check_type_alias(&mut self, alias: &TypeAliasDecl, _span: &Span) -> Result<(), TypeError> {
        let ty = self.convert_ast_type(&alias.ty.value)?;
        self.env.define_type_alias(alias.name.value.name.clone(), ty);
        Ok(())
    }

    fn check_enum_decl(&mut self, enum_decl: &EnumDecl, _span: &Span) -> Result<(), TypeError> {
        let members: Vec<String> = enum_decl
            .members
            .iter()
            .map(|m| m.name.value.name.clone())
            .collect();

        let enum_type = Type::Enum {
            name: enum_decl.name.value.name.clone(),
            members,
        };

        self.env.define_enum(enum_decl.name.value.name.clone(), enum_type);
        Ok(())
    }
}
