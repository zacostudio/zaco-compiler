//! Statement checking methods

use zaco_ast::{BlockStmt, ForInit, Pattern, Span, Stmt, VarDecl, VarDeclKind};
use crate::checker::TypeChecker;
use crate::error::{TypeError, TypeErrorKind};
use crate::types::Type;
use crate::ownership::{OwnershipState, VarInfo};
use crate::helpers::TypeHelpers;

impl TypeChecker {
    pub(crate) fn check_stmt(&mut self, stmt: &Stmt, span: &Span) -> Result<(), TypeError> {
        match stmt {
            Stmt::Expr(expr) => {
                self.check_expr(&expr.value, &expr.span)?;
                Ok(())
            }
            Stmt::VarDecl(var_decl) => self.check_var_decl(var_decl, span),
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    let return_ty = self.check_expr(&expr.value, &expr.span)?;
                    // Validate return type against declared function return type
                    if let Some(ref declared_ret) = self.current_return_type {
                        // In async functions the declared return type is Promise<T>,
                        // but the user returns T directly. Unwrap the Promise wrapper
                        // so we compare against the inner type.
                        let effective_ret = match declared_ret {
                            Type::Promise(inner) => inner.as_ref(),
                            other => other,
                        };
                        if !TypeHelpers::is_assignable_with_env(&return_ty, effective_ret, Some(&self.env)) {
                            return Err(TypeError::new(
                                TypeErrorKind::TypeMismatch {
                                    expected: effective_ret.clone(),
                                    found: return_ty,
                                },
                                expr.span.clone(),
                            ));
                        }
                    }
                }
                Ok(())
            }
            Stmt::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                let _cond_ty = self.check_expr(&condition.value, &condition.span)?;
                // Condition should be boolean-ish
                self.check_stmt(&then_stmt.value, &then_stmt.span)?;
                if let Some(else_stmt) = else_stmt {
                    self.check_stmt(&else_stmt.value, &else_stmt.span)?;
                }
                Ok(())
            }
            Stmt::For {
                init,
                condition,
                update,
                body,
            } => {
                self.env.push_scope();

                if let Some(init) = init {
                    match init {
                        ForInit::VarDecl(var_decl) => {
                            self.check_var_decl(var_decl, span)?;
                        }
                        ForInit::Expr(expr) => {
                            self.check_expr(&expr.value, &expr.span)?;
                        }
                    }
                }

                if let Some(condition) = condition {
                    self.check_expr(&condition.value, &condition.span)?;
                }

                if let Some(update) = update {
                    self.check_expr(&update.value, &update.span)?;
                }

                self.check_stmt(&body.value, &body.span)?;
                self.env.pop_scope();
                Ok(())
            }
            Stmt::ForIn { left: _, right, body } => {
                self.env.push_scope();
                self.check_expr(&right.value, &right.span)?;
                // Declare loop variable
                self.check_stmt(&body.value, &body.span)?;
                self.env.pop_scope();
                Ok(())
            }
            Stmt::ForOf {
                left: _,
                right,
                body,
                ..
            } => {
                self.env.push_scope();
                self.check_expr(&right.value, &right.span)?;
                self.check_stmt(&body.value, &body.span)?;
                self.env.pop_scope();
                Ok(())
            }
            Stmt::While { condition, body } => {
                self.check_expr(&condition.value, &condition.span)?;
                self.check_stmt(&body.value, &body.span)?;
                Ok(())
            }
            Stmt::DoWhile { body, condition } => {
                self.check_stmt(&body.value, &body.span)?;
                self.check_expr(&condition.value, &condition.span)?;
                Ok(())
            }
            Stmt::Block(block) => self.check_block_stmt(block, span),
            Stmt::Break(_) | Stmt::Continue(_) => Ok(()),
            Stmt::Throw(expr) => {
                self.check_expr(&expr.value, &expr.span)?;
                Ok(())
            }
            Stmt::Try {
                block,
                catch,
                finally,
            } => {
                self.check_block_stmt(&block.value, &block.span)?;
                if let Some(catch) = catch {
                    self.env.push_scope();
                    // Bind catch parameter as `unknown` type
                    if let Some(ref param) = catch.param {
                        if let Pattern::Ident { name, .. } = &param.value {
                            self.env.declare(
                                name.value.name.clone(),
                                VarInfo {
                                    ty: Type::Unknown,
                                    ownership: OwnershipState::Owned,
                                    is_mutable: true,
                                    is_initialized: true,
                                },
                            );
                        }
                    }
                    self.check_block_stmt(&catch.body.value, &catch.body.span)?;
                    self.env.pop_scope();
                }
                if let Some(finally) = finally {
                    self.check_block_stmt(&finally.value, &finally.span)?;
                }
                Ok(())
            }
            Stmt::Switch {
                discriminant,
                cases,
            } => {
                self.check_expr(&discriminant.value, &discriminant.span)?;
                for case in cases {
                    if let Some(test) = &case.test {
                        self.check_expr(&test.value, &test.span)?;
                    }
                    for stmt in &case.consequent {
                        self.check_stmt(&stmt.value, &stmt.span)?;
                    }
                }
                Ok(())
            }
            Stmt::Labeled { stmt, .. } => self.check_stmt(&stmt.value, &stmt.span),
            Stmt::Empty | Stmt::Debugger => Ok(()),
        }
    }

    pub(crate) fn check_block_stmt(&mut self, block: &BlockStmt, _span: &Span) -> Result<(), TypeError> {
        self.env.push_scope();
        for stmt in &block.stmts {
            self.check_stmt(&stmt.value, &stmt.span)?;
        }
        self.env.pop_scope();
        Ok(())
    }

    pub(crate) fn check_var_decl(&mut self, var_decl: &VarDecl, span: &Span) -> Result<(), TypeError> {
        let is_const = matches!(var_decl.kind, VarDeclKind::Const);

        for declarator in &var_decl.declarations {
            match &declarator.pattern.value {
                Pattern::Ident {
                    name,
                    type_annotation,
                    ownership,
                } => {
                    let var_name = &name.value.name;

                    // Infer or check type
                    let ty = if let Some(init) = &declarator.init {
                        let init_ty = self.check_expr(&init.value, &init.span)?;

                        // If type annotation exists, check compatibility
                        if let Some(type_ann) = type_annotation {
                            let annotated_ty = self.convert_ast_type(&type_ann.value)?;
                            if !TypeHelpers::is_assignable_with_env(&init_ty, &annotated_ty, Some(&self.env)) {
                                return Err(TypeError::new(
                                    TypeErrorKind::TypeMismatch {
                                        expected: annotated_ty,
                                        found: init_ty,
                                    },
                                    span.clone(),
                                ));
                            }
                            annotated_ty
                        } else {
                            init_ty
                        }
                    } else if let Some(type_ann) = type_annotation {
                        self.convert_ast_type(&type_ann.value)?
                    } else {
                        Type::Unknown
                    };

                    // Determine ownership
                    let ownership_state = if let Some(own) = ownership {
                        TypeHelpers::convert_ownership(&own.kind)
                    } else {
                        // Auto-inference: default to owned
                        OwnershipState::Owned
                    };

                    // Duplicate variable detection: let/const cannot redeclare in same scope
                    // (var redeclarations are allowed in JS/TS)
                    if !matches!(var_decl.kind, VarDeclKind::Var)
                        && self.env.has_in_current_scope(var_name)
                    {
                        return Err(TypeError::new(
                            TypeErrorKind::DuplicateDeclaration(var_name.clone()),
                            span.clone(),
                        ));
                    }

                    self.env.declare(
                        var_name.clone(),
                        VarInfo {
                            ty,
                            ownership: ownership_state,
                            is_mutable: !is_const,
                            is_initialized: declarator.init.is_some(),
                        },
                    );
                }
                Pattern::Array { elements: _, .. } => {
                    // Handle array destructuring
                    if let Some(init) = &declarator.init {
                        self.check_expr(&init.value, &init.span)?;
                    }
                }
                Pattern::Object { properties: _, .. } => {
                    // Handle object destructuring
                    if let Some(init) = &declarator.init {
                        self.check_expr(&init.value, &init.span)?;
                    }
                }
                Pattern::Assignment { pattern: _, default } => {
                    // Handle assignment pattern
                    self.check_expr(&default.value, &default.span)?;
                }
            }
        }

        Ok(())
    }
}
