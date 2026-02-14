//! Expression checking methods

use std::collections::HashMap;
use zaco_ast::{
    ArrowBody, AssignmentOp, BinaryOp, BlockStmt, Expr, Ident, Literal, Node,
    ObjectProperty, Param, Span, UnaryOp,
};
use crate::checker::TypeChecker;
use crate::error::{TypeError, TypeErrorKind};
use crate::types::{LiteralType, Type};
use crate::ownership::OwnershipState;
use crate::helpers::TypeHelpers;

impl TypeChecker {
    pub(crate) fn check_expr(&mut self, expr: &Expr, span: &Span) -> Result<Type, TypeError> {
        match expr {
            Expr::Literal(lit) => Ok(self.check_literal(lit)),
            Expr::Ident(ident) => self.check_ident(&ident.name, span),
            Expr::Binary { left, op, right } => self.check_binary(left, *op, right, span),
            Expr::Unary { op, expr } => self.check_unary(*op, expr, span),
            Expr::Assignment { target, op, value } => {
                self.check_assignment(target, *op, value, span)
            }
            Expr::Call {
                callee,
                type_args: _,
                args,
            } => self.check_call(callee, args, span),
            Expr::Member {
                object,
                property,
                computed,
            } => self.check_member(object, property, *computed, span),
            Expr::Index { object, index } => self.check_index(object, index, span),
            Expr::Array(elements) => self.check_array(elements, span),
            Expr::Object(properties) => self.check_object(properties, span),
            Expr::Arrow {
                params,
                return_type,
                body,
                ..
            } => self.check_arrow(params, return_type.as_ref(), body, span),
            Expr::Function {
                params,
                return_type,
                body,
                ..
            } => self.check_function_expr(params, return_type.as_ref(), body, span),
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => self.check_ternary(condition, then_expr, else_expr, span),
            Expr::Template { parts, exprs } => self.check_template(parts, exprs, span),
            Expr::New {
                callee,
                type_args: _,
                args,
            } => self.check_new(callee, args, span),
            Expr::TypeCast { expr, ty } => self.check_type_cast(expr, ty, span),
            Expr::Await(expr) => {
                let inner_ty = self.check_expr(&expr.value, &expr.span)?;
                // Unwrap Promise type if applicable
                match inner_ty {
                    Type::Promise(inner) => Ok(*inner),
                    Type::TypeRef { ref name, ref type_args } if name == "Promise" => {
                        if let Some(inner) = type_args.first() {
                            Ok(inner.clone())
                        } else {
                            Ok(Type::Void)
                        }
                    }
                    // If not a Promise, return the type as-is (like TypeScript)
                    other => Ok(other),
                }
            }
            Expr::Paren(expr) => self.check_expr(&expr.value, &expr.span),
            Expr::This => Ok(Type::Unknown), // Context-dependent
            Expr::Super => Ok(Type::Unknown), // Context-dependent
            Expr::Clone(expr) => {
                let ty = self.check_expr(&expr.value, &expr.span)?;
                // Clone creates a new owned copy
                Ok(ty)
            }
            Expr::Sequence(exprs) => {
                let mut last_ty = Type::Void;
                for expr in exprs {
                    last_ty = self.check_expr(&expr.value, &expr.span)?;
                }
                Ok(last_ty)
            }
            Expr::Spread(expr) => {
                // Spread in call args or array literals - check the inner expression
                self.check_expr(&expr.value, &expr.span)
            }
            Expr::OptionalCall {
                callee,
                type_args: _,
                args,
            } => {
                // Optional chaining call: expr?.(args) - similar to regular call but returns T | undefined
                let ty = self.check_call(callee, args, span)?;
                Ok(Type::Union(vec![ty, Type::Undefined]))
            }
            Expr::OptionalIndex { object, index } => {
                // Optional chaining index: expr?.[index] - similar to regular index but returns T | undefined
                let ty = self.check_index(object, index, span)?;
                Ok(Type::Union(vec![ty, Type::Undefined]))
            }
            Expr::OptionalMember {
                object,
                property,
                ..
            } => {
                // Optional chaining member: expr?.prop - similar to regular member but returns T | undefined
                let ty = self.check_member(object, property, false, span)?;
                Ok(Type::Union(vec![ty, Type::Undefined]))
            }
            Expr::TaggedTemplate { tag, parts, exprs } => {
                // Tagged template: tag`template` - check tag as function
                let _tag_ty = self.check_expr(&tag.value, &tag.span)?;
                let _template_ty = self.check_template(parts, exprs, span)?;
                // Return type depends on the tag function's return type
                Ok(Type::Unknown)
            }
            Expr::Satisfies { expr, ty } => {
                // Satisfies expression: expr satisfies Type - check expr against ty
                let expr_ty = self.check_expr(&expr.value, &expr.span)?;
                let _target_ty = self.convert_ast_type(&ty.value);
                // TODO: Actually check satisfies constraint
                Ok(expr_ty)
            }
            Expr::NonNullAssertion(expr) => {
                // Non-null assertion: expr! - strip null/undefined from type
                let ty = self.check_expr(&expr.value, &expr.span)?;
                // TODO: Strip null/undefined from union types
                Ok(ty)
            }
            Expr::MetaProperty { meta, property } => {
                // Meta property: new.target, import.meta
                if meta.value.name == "new" && property.value.name == "target" {
                    Ok(Type::Unknown) // Function | undefined
                } else if meta.value.name == "import" && property.value.name == "meta" {
                    Ok(Type::Unknown) // ImportMeta
                } else {
                    Ok(Type::Unknown)
                }
            }
            Expr::Yield { argument, .. } => {
                // Yield expression: yield expr, yield* expr
                if let Some(arg) = argument {
                    self.check_expr(&arg.value, &arg.span)
                } else {
                    Ok(Type::Undefined)
                }
            }
        }
    }

    fn check_literal(&self, lit: &Literal) -> Type {
        match lit {
            Literal::Number(n) => Type::Literal(LiteralType::Number(*n)),
            Literal::String(s) => Type::Literal(LiteralType::String(s.clone())),
            Literal::Boolean(b) => Type::Literal(LiteralType::Boolean(*b)),
            Literal::Null => Type::Null,
            Literal::Undefined => Type::Undefined,
            Literal::RegExp { .. } => Type::Object {
                properties: vec![], // RegExp object
            },
        }
    }

    fn check_ident(&mut self, name: &str, span: &Span) -> Result<Type, TypeError> {
        if let Some(var_info) = self.env.lookup(name) {
            // Check ownership state
            match var_info.ownership {
                OwnershipState::Moved => {
                    return Err(TypeError::new(
                        TypeErrorKind::UseAfterMove(name.to_string()),
                        span.clone(),
                    ));
                }
                OwnershipState::Dropped => {
                    return Err(TypeError::new(
                        TypeErrorKind::UseAfterMove(name.to_string()),
                        span.clone(),
                    ));
                }
                _ => {}
            }

            if !var_info.is_initialized {
                return Err(TypeError::new(
                    TypeErrorKind::UninitializedVariable(name.to_string()),
                    span.clone(),
                ));
            }

            Ok(var_info.ty.clone())
        } else {
            Err(TypeError::new(
                TypeErrorKind::UndefinedVariable(name.to_string()),
                span.clone(),
            ))
        }
    }

    fn check_binary(
        &mut self,
        left: &Node<Expr>,
        op: BinaryOp,
        right: &Node<Expr>,
        _span: &Span,
    ) -> Result<Type, TypeError> {
        let left_ty = self.check_expr(&left.value, &left.span)?;
        let right_ty = self.check_expr(&right.value, &right.span)?;

        match op {
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Pow => {
                // Arithmetic operators
                if TypeHelpers::is_numeric(&left_ty) && TypeHelpers::is_numeric(&right_ty) {
                    Ok(Type::Number)
                } else if matches!(op, BinaryOp::Add) && (TypeHelpers::is_string(&left_ty) || TypeHelpers::is_string(&right_ty)) {
                    Ok(Type::String)
                } else {
                    Ok(Type::Number) // TypeScript-like coercion
                }
            }
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::StrictEq
            | BinaryOp::StrictNotEq
            | BinaryOp::Lt
            | BinaryOp::LtEq
            | BinaryOp::Gt
            | BinaryOp::GtEq => Ok(Type::Boolean),
            BinaryOp::And => {
                // && returns the right operand type (if left is truthy)
                Ok(right_ty)
            }
            BinaryOp::Or | BinaryOp::NullishCoalesce => {
                // || and ?? return a union of both operand types
                Ok(TypeHelpers::union_type(vec![left_ty, right_ty]))
            }
            BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::LeftShift
            | BinaryOp::RightShift
            | BinaryOp::UnsignedRightShift => Ok(Type::Number),
            BinaryOp::In | BinaryOp::InstanceOf => Ok(Type::Boolean),
        }
    }

    fn check_unary(
        &mut self,
        op: UnaryOp,
        expr: &Node<Expr>,
        _span: &Span,
    ) -> Result<Type, TypeError> {
        let _expr_ty = self.check_expr(&expr.value, &expr.span)?;

        match op {
            UnaryOp::Plus | UnaryOp::Minus | UnaryOp::BitNot => Ok(Type::Number),
            UnaryOp::Not => Ok(Type::Boolean),
            UnaryOp::TypeOf => Ok(Type::String),
            UnaryOp::Void => Ok(Type::Undefined),
            UnaryOp::Delete => Ok(Type::Boolean),
            UnaryOp::PreIncrement
            | UnaryOp::PreDecrement
            | UnaryOp::PostIncrement
            | UnaryOp::PostDecrement => Ok(Type::Number),
        }
    }

    fn check_assignment(
        &mut self,
        target: &Node<Expr>,
        op: AssignmentOp,
        value: &Node<Expr>,
        span: &Span,
    ) -> Result<Type, TypeError> {
        let value_ty = self.check_expr(&value.value, &value.span)?;

        // Extract target variable name for ownership tracking
        if let Expr::Ident(ident) = &target.value {
            let var_name = &ident.name;

            if let Some(var_info) = self.env.lookup(var_name) {
                if !var_info.is_mutable {
                    return Err(TypeError::new(
                        TypeErrorKind::AssignToImmutable(var_name.clone()),
                        span.clone(),
                    ));
                }

                // Check type compatibility
                if !TypeHelpers::is_assignable_with_env(&value_ty, &var_info.ty, Some(&self.env)) {
                    return Err(TypeError::new(
                        TypeErrorKind::TypeMismatch {
                            expected: var_info.ty.clone(),
                            found: value_ty.clone(),
                        },
                        span.clone(),
                    ));
                }

                // Handle move semantics
                if matches!(op, AssignmentOp::Assign) {
                    // Simple assignment might move the value
                    // Check if the value is being moved
                    if let Expr::Ident(value_ident) = &value.value {
                        if let Some(value_var) = self.env.lookup(&value_ident.name) {
                            if matches!(value_var.ownership, OwnershipState::Owned) {
                                // Move the value
                                let _ = self.env.update_ownership(
                                    &value_ident.name,
                                    OwnershipState::Moved,
                                );
                            }
                        }
                    }

                    // Update target ownership
                    let _ = self.env.update_ownership(var_name, OwnershipState::Owned);
                }
            } else {
                return Err(TypeError::new(
                    TypeErrorKind::UndefinedVariable(var_name.clone()),
                    span.clone(),
                ));
            }
        }

        Ok(value_ty)
    }

    fn check_call(
        &mut self,
        callee: &Node<Expr>,
        args: &[Node<Expr>],
        span: &Span,
    ) -> Result<Type, TypeError> {
        let callee_ty = self.check_expr(&callee.value, &callee.span)?;

        match &callee_ty {
            Type::Function {
                params,
                return_type,
            } => {
                // Variadic-style: if single param is Any, accept any number of args
                let is_variadic = params.len() == 1 && params[0] == Type::Any;

                if !is_variadic && args.len() != params.len() {
                    return Err(TypeError::new(
                        TypeErrorKind::ArityMismatch {
                            expected: params.len(),
                            found: args.len(),
                        },
                        span.clone(),
                    ));
                }

                // Check argument types
                for (i, arg) in args.iter().enumerate() {
                    let arg_ty = self.check_expr(&arg.value, &arg.span)?;
                    if let Some(param_ty) = params.get(i) {
                        if !TypeHelpers::is_assignable_with_env(&arg_ty, param_ty, Some(&self.env)) {
                            return Err(TypeError::new(
                                TypeErrorKind::TypeMismatch {
                                    expected: param_ty.clone(),
                                    found: arg_ty,
                                },
                                arg.span.clone(),
                            ));
                        }
                    }
                }

                Ok((**return_type).clone())
            }
            Type::Class { name, .. } => {
                // Constructor call
                Ok(Type::TypeRef { name: name.clone(), type_args: vec![] })
            }
            _ => Err(TypeError::new(
                TypeErrorKind::NotCallable(callee_ty),
                span.clone(),
            )),
        }
    }

    fn check_member(
        &mut self,
        object: &Node<Expr>,
        property: &Node<Ident>,
        _computed: bool,
        span: &Span,
    ) -> Result<Type, TypeError> {
        let object_ty = self.check_expr(&object.value, &object.span)?;
        let prop_name = &property.value.name;

        match &object_ty {
            Type::Object { properties } => {
                for (name, ty, _) in properties {
                    if name == prop_name {
                        return Ok(ty.clone());
                    }
                }
                Err(TypeError::new(
                    TypeErrorKind::PropertyNotFound {
                        ty: object_ty,
                        property: prop_name.clone(),
                    },
                    span.clone(),
                ))
            }
            Type::Class { fields, methods, .. } => {
                // Check fields
                for (name, ty) in fields {
                    if name == prop_name {
                        return Ok(ty.clone());
                    }
                }
                // Check methods
                for (name, ty) in methods {
                    if name == prop_name {
                        return Ok(ty.clone());
                    }
                }
                Err(TypeError::new(
                    TypeErrorKind::PropertyNotFound {
                        ty: object_ty,
                        property: prop_name.clone(),
                    },
                    span.clone(),
                ))
            }
            Type::Interface { properties, .. } => {
                for (name, ty, _) in properties {
                    if name == prop_name {
                        return Ok(ty.clone());
                    }
                }
                Err(TypeError::new(
                    TypeErrorKind::PropertyNotFound {
                        ty: object_ty,
                        property: prop_name.clone(),
                    },
                    span.clone(),
                ))
            }
            Type::TypeRef { ref name, ref type_args } => {
                // Build generic substitution map if type_args are provided
                let subst_map = if !type_args.is_empty() {
                    if let Some(param_names) = self.env.get_type_params(name) {
                        let mut map = HashMap::new();
                        for (param_name, arg_type) in param_names.iter().zip(type_args.iter()) {
                            map.insert(param_name.clone(), arg_type.clone());
                        }
                        Some(map)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Resolve TypeRef to the actual class/interface type
                if let Some(resolved) = self.env.lookup_type(name) {
                    match resolved {
                        Type::Class { fields, methods, .. } => {
                            for (fname, fty) in fields {
                                if fname == prop_name {
                                    let result_ty = if let Some(ref map) = subst_map {
                                        TypeHelpers::substitute_type_params(fty, map)
                                    } else {
                                        fty.clone()
                                    };
                                    return Ok(result_ty);
                                }
                            }
                            for (mname, mty) in methods {
                                if mname == prop_name {
                                    let result_ty = if let Some(ref map) = subst_map {
                                        TypeHelpers::substitute_type_params(mty, map)
                                    } else {
                                        mty.clone()
                                    };
                                    return Ok(result_ty);
                                }
                            }
                            Err(TypeError::new(
                                TypeErrorKind::PropertyNotFound {
                                    ty: resolved.clone(),
                                    property: prop_name.clone(),
                                },
                                span.clone(),
                            ))
                        }
                        Type::Interface { properties, .. } => {
                            for (pname, pty, _) in properties {
                                if pname == prop_name {
                                    let result_ty = if let Some(ref map) = subst_map {
                                        TypeHelpers::substitute_type_params(pty, map)
                                    } else {
                                        pty.clone()
                                    };
                                    return Ok(result_ty);
                                }
                            }
                            Err(TypeError::new(
                                TypeErrorKind::PropertyNotFound {
                                    ty: resolved.clone(),
                                    property: prop_name.clone(),
                                },
                                span.clone(),
                            ))
                        }
                        _ => Ok(Type::Any),
                    }
                } else {
                    // Unknown type ref — treat as Any to avoid blocking compilation
                    Ok(Type::Any)
                }
            }
            Type::Enum { ref name, ref members } => {
                // Enum member access: Direction.Up
                if members.contains(&prop_name.to_string()) {
                    return Ok(Type::Enum { name: name.clone(), members: members.clone() });
                }
                Ok(Type::Any)
            }
            Type::Any | Type::Unknown => Ok(Type::Any),
            _ => Err(TypeError::new(
                TypeErrorKind::PropertyNotFound {
                    ty: object_ty,
                    property: prop_name.clone(),
                },
                span.clone(),
            )),
        }
    }

    fn check_index(
        &mut self,
        object: &Node<Expr>,
        index: &Node<Expr>,
        span: &Span,
    ) -> Result<Type, TypeError> {
        let object_ty = self.check_expr(&object.value, &object.span)?;
        let _index_ty = self.check_expr(&index.value, &index.span)?;

        match &object_ty {
            Type::Array(elem_ty) => Ok((**elem_ty).clone()),
            Type::Tuple(types) => {
                // If we can determine index statically, return that type
                // Otherwise, return union of all types
                Ok(TypeHelpers::union_type(types.clone()))
            }
            Type::Object { .. } => Ok(Type::Any), // Object indexing
            Type::Any | Type::Unknown => Ok(Type::Any),
            _ => Err(TypeError::new(
                TypeErrorKind::NotIndexable(object_ty),
                span.clone(),
            )),
        }
    }

    fn check_array(
        &mut self,
        elements: &[Option<Node<Expr>>],
        _span: &Span,
    ) -> Result<Type, TypeError> {
        let mut elem_types = Vec::new();

        for elem in elements {
            if let Some(elem) = elem {
                let elem_ty = self.check_expr(&elem.value, &elem.span)?;
                elem_types.push(elem_ty);
            }
        }

        // Infer array type as union of all element types
        let elem_ty = if elem_types.is_empty() {
            Type::Unknown
        } else if elem_types.len() == 1 {
            elem_types[0].clone()
        } else {
            TypeHelpers::union_type(elem_types)
        };

        Ok(Type::Array(Box::new(elem_ty)))
    }

    fn check_object(
        &mut self,
        properties: &[ObjectProperty],
        _span: &Span,
    ) -> Result<Type, TypeError> {
        let mut props = Vec::new();

        for prop in properties {
            match prop {
                ObjectProperty::Property { key, value, .. } => {
                    let prop_name = TypeHelpers::property_name_to_string(key);
                    let prop_ty = self.check_expr(&value.value, &value.span)?;
                    props.push((prop_name, prop_ty, false));
                }
                ObjectProperty::Method {
                    key,
                    params,
                    return_type,
                    ..
                } => {
                    let method_name = TypeHelpers::property_name_to_string(key);
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
                    props.push((method_name, method_ty, false));
                }
                ObjectProperty::Spread(_) => {
                    // Handle spread
                }
            }
        }

        Ok(Type::Object { properties: props })
    }

    fn check_arrow(
        &mut self,
        params: &[Param],
        return_type: Option<&Box<Node<zaco_ast::Type>>>,
        body: &ArrowBody,
        _span: &Span,
    ) -> Result<Type, TypeError> {
        self.env.push_scope();

        let mut param_types = Vec::new();
        for param in params {
            let param_ty = self.resolve_param_type(param)?;
            param_types.push(param_ty.clone());

            // Declare parameter
            self.check_param(param)?;
        }

        let ret_ty = match body {
            ArrowBody::Expr(expr) => self.check_expr(&expr.value, &expr.span)?,
            ArrowBody::Block(block) => {
                self.check_block_stmt(&block.value, &block.span)?;
                if let Some(ret_ty) = return_type {
                    self.convert_ast_type(&ret_ty.value)?
                } else {
                    Type::Void
                }
            }
        };

        self.env.pop_scope();

        Ok(Type::Function {
            params: param_types,
            return_type: Box::new(ret_ty),
        })
    }

    fn check_function_expr(
        &mut self,
        params: &[Param],
        return_type: Option<&Box<Node<zaco_ast::Type>>>,
        body: &Node<BlockStmt>,
        _span: &Span,
    ) -> Result<Type, TypeError> {
        self.env.push_scope();

        let mut param_types = Vec::new();
        for param in params {
            let param_ty = self.resolve_param_type(param)?;
            param_types.push(param_ty);

            self.check_param(param)?;
        }

        self.check_block_stmt(&body.value, &body.span)?;

        let ret_ty = if let Some(ret_ty) = return_type {
            self.convert_ast_type(&ret_ty.value)?
        } else {
            Type::Void
        };

        self.env.pop_scope();

        Ok(Type::Function {
            params: param_types,
            return_type: Box::new(ret_ty),
        })
    }

    fn check_ternary(
        &mut self,
        condition: &Node<Expr>,
        then_expr: &Node<Expr>,
        else_expr: &Node<Expr>,
        _span: &Span,
    ) -> Result<Type, TypeError> {
        let _cond_ty = self.check_expr(&condition.value, &condition.span)?;
        let then_ty = self.check_expr(&then_expr.value, &then_expr.span)?;
        let else_ty = self.check_expr(&else_expr.value, &else_expr.span)?;

        Ok(TypeHelpers::union_type(vec![then_ty, else_ty]))
    }

    fn check_template(
        &mut self,
        _parts: &[String],
        exprs: &[Node<Expr>],
        _span: &Span,
    ) -> Result<Type, TypeError> {
        for expr in exprs {
            self.check_expr(&expr.value, &expr.span)?;
        }
        Ok(Type::String)
    }

    fn check_new(
        &mut self,
        callee: &Node<Expr>,
        args: &[Node<Expr>],
        _span: &Span,
    ) -> Result<Type, TypeError> {
        let callee_ty = self.check_expr(&callee.value, &callee.span)?;

        // Check constructor arguments
        for arg in args {
            self.check_expr(&arg.value, &arg.span)?;
        }

        match &callee_ty {
            Type::Class { name, .. } => Ok(Type::TypeRef { name: name.clone(), type_args: vec![] }),
            Type::Function { return_type, .. } => Ok((**return_type).clone()),
            _ => Ok(callee_ty),
        }
    }

    fn check_type_cast(
        &mut self,
        expr: &Node<Expr>,
        ty: &Box<Node<zaco_ast::Type>>,
        _span: &Span,
    ) -> Result<Type, TypeError> {
        self.check_expr(&expr.value, &expr.span)?;
        self.convert_ast_type(&ty.value)
    }

    /// Convert AST type to internal type representation
    pub(crate) fn convert_ast_type(&self, ast_ty: &zaco_ast::Type) -> Result<Type, TypeError> {
        match ast_ty {
            zaco_ast::Type::Primitive(prim) => Ok(TypeHelpers::convert_primitive(prim)),
            zaco_ast::Type::Array(elem_ty) => {
                let elem = self.convert_ast_type(&elem_ty.value)?;
                Ok(Type::Array(Box::new(elem)))
            }
            zaco_ast::Type::Tuple(types) => {
                let mut tuple_types = Vec::new();
                for ty in types {
                    tuple_types.push(self.convert_ast_type(&ty.value)?);
                }
                Ok(Type::Tuple(tuple_types))
            }
            zaco_ast::Type::Union(types) => {
                let mut union_types = Vec::new();
                for ty in types {
                    union_types.push(self.convert_ast_type(&ty.value)?);
                }
                Ok(Type::Union(union_types))
            }
            zaco_ast::Type::Intersection(types) => {
                let mut inter_types = Vec::new();
                for ty in types {
                    inter_types.push(self.convert_ast_type(&ty.value)?);
                }
                Ok(Type::Intersection(inter_types))
            }
            zaco_ast::Type::Function(func_ty) => {
                let mut params = Vec::new();
                for param in &func_ty.params {
                    params.push(self.convert_ast_type(&param.ty.value)?);
                }
                let return_type = self.convert_ast_type(&func_ty.return_type.value)?;
                Ok(Type::Function {
                    params,
                    return_type: Box::new(return_type),
                })
            }
            zaco_ast::Type::TypeRef { name, type_args } => {
                let type_name = name.value.name.clone();
                let converted_args = if let Some(args) = type_args {
                    let mut result = Vec::new();
                    for arg in args {
                        result.push(self.convert_ast_type(&arg.value)?);
                    }
                    result
                } else {
                    vec![]
                };

                // Special-case: Promise<T> → Type::Promise(T)
                if type_name == "Promise" && converted_args.len() == 1 {
                    return Ok(Type::Promise(Box::new(converted_args.into_iter().next().unwrap())));
                }

                Ok(Type::TypeRef { name: type_name, type_args: converted_args })
            }
            zaco_ast::Type::Object(obj_ty) => {
                let mut properties = Vec::new();
                for member in &obj_ty.members {
                    match member {
                        zaco_ast::ObjectTypeMember::Property {
                            name,
                            ty,
                            optional,
                            ..
                        } => {
                            let prop_name = TypeHelpers::property_name_to_string(name);
                            let prop_ty = self.convert_ast_type(&ty.value)?;
                            properties.push((prop_name, prop_ty, *optional));
                        }
                        _ => {}
                    }
                }
                Ok(Type::Object { properties })
            }
            zaco_ast::Type::Literal(lit) => Ok(Type::Literal(TypeHelpers::convert_literal_type(lit))),
            zaco_ast::Type::Paren(ty) => self.convert_ast_type(&ty.value),
            zaco_ast::Type::WithOwnership { base, .. } => self.convert_ast_type(&base.value),
            zaco_ast::Type::Generic { base, .. } => self.convert_ast_type(&base.value),
            zaco_ast::Type::Conditional { true_type, false_type, .. } => {
                // Conditional type: T extends U ? X : Y
                // For now, return union of both branches
                let true_ty = self.convert_ast_type(&true_type.value)?;
                let false_ty = self.convert_ast_type(&false_type.value)?;
                Ok(Type::Union(vec![true_ty, false_ty]))
            }
            zaco_ast::Type::Mapped { value_type, .. } => {
                // Mapped type: { [K in keyof T]: V }
                // For now, return object with unknown properties
                self.convert_ast_type(&value_type.value)
            }
            zaco_ast::Type::TemplateLiteral { .. } => {
                // Template literal type: `hello ${string}`
                Ok(Type::String)
            }
            zaco_ast::Type::IndexedAccess { object_type, .. } => {
                // Indexed access type: T[K]
                // For now, return unknown - proper implementation would look up the property
                let _obj_ty = self.convert_ast_type(&object_type.value)?;
                Ok(Type::Unknown)
            }
            zaco_ast::Type::Keyof(ty) => {
                // keyof type: keyof T
                // For now, return string | number | symbol
                let _inner_ty = self.convert_ast_type(&ty.value)?;
                Ok(Type::Union(vec![Type::String, Type::Number]))
            }
            zaco_ast::Type::TypeofType(ty) => {
                // typeof type: typeof expr
                self.convert_ast_type(&ty.value)
            }
            zaco_ast::Type::Infer(_ident) => {
                // infer type: infer T (used in conditional types)
                Ok(Type::Unknown)
            }
            zaco_ast::Type::ImportType { .. } => {
                // Import type: import("module").Type
                Ok(Type::Unknown)
            }
        }
    }
}
