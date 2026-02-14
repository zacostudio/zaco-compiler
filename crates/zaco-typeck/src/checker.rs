//! Main type checker struct

use zaco_ast::{ModuleItem, Program, Span, ImportDecl, ImportSpecifier, ExportDecl};
use crate::env::TypeEnv;
use crate::error::{TypeError, TypeErrorKind};
use crate::types::Type;
use crate::ownership::{OwnershipState, VarInfo};
use crate::typed_ast::{TypedDecl, TypedModuleItem, TypedProgram, TypedStmt};
use crate::builtins::BuiltinRegistry;

/// Main type checker
pub struct TypeChecker {
    pub(crate) env: TypeEnv,
    pub(crate) errors: Vec<TypeError>,
    pub(crate) builtin_registry: BuiltinRegistry,
    /// The declared return type of the current function being checked (for return-type validation)
    pub(crate) current_return_type: Option<Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            env: TypeEnv::new(),
            errors: Vec::new(),
            builtin_registry: BuiltinRegistry::new(),
            current_return_type: None,
        };
        checker.register_builtins();
        checker
    }

    /// Register built-in global variables and functions
    fn register_builtins(&mut self) {
        // console object: console.log, console.error, console.warn, etc.
        let console_methods = vec![
            ("log".to_string(), Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Void),
            }, false),
            ("error".to_string(), Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Void),
            }, false),
            ("warn".to_string(), Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Void),
            }, false),
            ("info".to_string(), Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Void),
            }, false),
            ("debug".to_string(), Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Void),
            }, false),
        ];
        self.env.declare("console".to_string(), VarInfo {
            ty: Type::Object { properties: console_methods },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });

        // Math object - expanded with all methods
        let math_methods = vec![
            ("sqrt".to_string(), Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("abs".to_string(), Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("floor".to_string(), Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("ceil".to_string(), Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("round".to_string(), Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("min".to_string(), Type::Function {
                params: vec![Type::Number, Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("max".to_string(), Type::Function {
                params: vec![Type::Number, Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("random".to_string(), Type::Function {
                params: vec![],
                return_type: Box::new(Type::Number),
            }, false),
            ("pow".to_string(), Type::Function {
                params: vec![Type::Number, Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("sin".to_string(), Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("cos".to_string(), Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("tan".to_string(), Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("log".to_string(), Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Number),
            }, false),
            ("PI".to_string(), Type::Number, false),
            ("E".to_string(), Type::Number, false),
        ];
        self.env.declare("Math".to_string(), VarInfo {
            ty: Type::Object { properties: math_methods },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });

        // JSON object
        let json_methods = vec![
            ("stringify".to_string(), Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::String),
            }, false),
            ("parse".to_string(), Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::Any),
            }, false),
        ];
        self.env.declare("JSON".to_string(), VarInfo {
            ty: Type::Object { properties: json_methods },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });

        // process object (available globally without import, like in Node.js)
        let process_properties = vec![
            ("exit".to_string(), Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Void),
            }, false),
            ("cwd".to_string(), Type::Function {
                params: vec![],
                return_type: Box::new(Type::String),
            }, false),
            ("env".to_string(), Type::Any, false),
            ("pid".to_string(), Type::Number, false),
            ("platform".to_string(), Type::String, false),
            ("arch".to_string(), Type::String, false),
            ("argv".to_string(), Type::Array(Box::new(Type::String)), false),
        ];
        self.env.declare("process".to_string(), VarInfo {
            ty: Type::Object { properties: process_properties },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });

        // Global functions
        self.env.declare("parseInt".to_string(), VarInfo {
            ty: Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::Number),
            },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });
        self.env.declare("parseFloat".to_string(), VarInfo {
            ty: Type::Function {
                params: vec![Type::String],
                return_type: Box::new(Type::Number),
            },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });
        self.env.declare("isNaN".to_string(), VarInfo {
            ty: Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Boolean),
            },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });
        self.env.declare("isFinite".to_string(), VarInfo {
            ty: Type::Function {
                params: vec![Type::Any],
                return_type: Box::new(Type::Boolean),
            },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });

        // __dirname and __filename globals (Node.js-style)
        self.env.declare("__dirname".to_string(), VarInfo {
            ty: Type::String,
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });
        self.env.declare("__filename".to_string(), VarInfo {
            ty: Type::String,
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });

        // Timer functions
        self.env.declare("setTimeout".to_string(), VarInfo {
            ty: Type::Function {
                params: vec![Type::Any, Type::Number],
                return_type: Box::new(Type::Number),
            },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });
        self.env.declare("setInterval".to_string(), VarInfo {
            ty: Type::Function {
                params: vec![Type::Any, Type::Number],
                return_type: Box::new(Type::Number),
            },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });
        self.env.declare("clearTimeout".to_string(), VarInfo {
            ty: Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Void),
            },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });
        self.env.declare("clearInterval".to_string(), VarInfo {
            ty: Type::Function {
                params: vec![Type::Number],
                return_type: Box::new(Type::Void),
            },
            ownership: OwnershipState::Borrowed,
            is_mutable: false,
            is_initialized: true,
        });
    }

    /// Main entry point: type check a program
    pub fn check_program(&mut self, program: &Program) -> Result<TypedProgram, Vec<TypeError>> {
        let mut typed_items = Vec::new();

        for item in &program.items {
            match self.check_module_item(&item.value, &item.span) {
                Ok(typed_item) => typed_items.push(typed_item),
                Err(err) => self.errors.push(err),
            }
        }

        if self.errors.is_empty() {
            Ok(TypedProgram {
                items: typed_items,
                span: program.span.clone(),
            })
        } else {
            Err(self.errors.clone())
        }
    }

    fn check_module_item(
        &mut self,
        item: &ModuleItem,
        span: &Span,
    ) -> Result<TypedModuleItem, TypeError> {
        match item {
            ModuleItem::Import(import_decl) => {
                self.check_import(import_decl, span)?;
                Ok(TypedModuleItem::Import)
            }
            ModuleItem::Export(export_decl) => {
                self.check_export(export_decl, span)?;
                Ok(TypedModuleItem::Export)
            }
            ModuleItem::Stmt(stmt) => {
                self.check_stmt(&stmt.value, &stmt.span)?;
                Ok(TypedModuleItem::Stmt(TypedStmt {
                    stmt: stmt.value.clone(),
                    span: stmt.span.clone(),
                }))
            }
            ModuleItem::Decl(decl) => {
                self.check_decl(&decl.value, &decl.span)?;
                Ok(TypedModuleItem::Decl(TypedDecl {
                    decl: decl.value.clone(),
                    span: decl.span.clone(),
                }))
            }
        }
    }

    /// Check import declaration and register imported symbols in the type environment
    fn check_import(&mut self, import: &ImportDecl, span: &Span) -> Result<(), TypeError> {
        // Check if this is a built-in module
        if self.builtin_registry.is_builtin_module(&import.source) {
            // Validate and register each imported symbol
            for specifier in &import.specifiers {
                match specifier {
                    ImportSpecifier::Named { imported, local, .. } => {
                        let import_name = imported.value.name.as_str();

                        // Check if the symbol exists in the built-in module
                        if let Some(symbol_type) = self.builtin_registry.get_export_type(&import.source, import_name) {
                            // Register the symbol with its local name in the environment
                            // If no local alias is provided, use the imported name
                            let local_name = local
                                .as_ref()
                                .map(|n| n.value.name.clone())
                                .unwrap_or_else(|| imported.value.name.clone());

                            self.env.declare(local_name, VarInfo {
                                ty: symbol_type.clone(),
                                ownership: OwnershipState::Borrowed,
                                is_mutable: false,
                                is_initialized: true,
                            });
                        } else {
                            // Symbol not found in built-in module
                            return Err(TypeError {
                                kind: TypeErrorKind::UndefinedVariable(format!(
                                    "Module '{}' does not export '{}'",
                                    import.source, import_name
                                )),
                                span: span.clone(),
                            });
                        }
                    }
                    ImportSpecifier::Default(ident) => {
                        // For now, treat default imports from built-in modules as Any
                        // This could be improved with a default export registry
                        self.env.declare(ident.value.name.clone(), VarInfo {
                            ty: Type::Any,
                            ownership: OwnershipState::Borrowed,
                            is_mutable: false,
                            is_initialized: true,
                        });
                    }
                    ImportSpecifier::Namespace(ident) => {
                        // import * as name from "module"
                        // Create an object type with all exports from the module
                        if let Some(exports) = self.builtin_registry.get_module_exports(&import.source) {
                            let properties: Vec<(String, Type, bool)> = exports
                                .iter()
                                .map(|(name, ty)| (name.clone(), ty.clone(), false))
                                .collect();

                            self.env.declare(ident.value.name.clone(), VarInfo {
                                ty: Type::Object { properties },
                                ownership: OwnershipState::Borrowed,
                                is_mutable: false,
                                is_initialized: true,
                            });
                        }
                    }
                }
            }
        } else {
            // For local modules (relative paths), we can't validate the imports yet
            // This would require cross-module type information from the driver
            // For now, register imported symbols as Any to avoid false errors
            for specifier in &import.specifiers {
                match specifier {
                    ImportSpecifier::Named { imported, local, .. } => {
                        let local_name = local
                            .as_ref()
                            .map(|n| n.value.name.clone())
                            .unwrap_or_else(|| imported.value.name.clone());

                        self.env.declare(local_name, VarInfo {
                            ty: Type::Any,
                            ownership: OwnershipState::Borrowed,
                            is_mutable: false,
                            is_initialized: true,
                        });
                    }
                    ImportSpecifier::Default(ident) => {
                        self.env.declare(ident.value.name.clone(), VarInfo {
                            ty: Type::Any,
                            ownership: OwnershipState::Borrowed,
                            is_mutable: false,
                            is_initialized: true,
                        });
                    }
                    ImportSpecifier::Namespace(ident) => {
                        self.env.declare(ident.value.name.clone(), VarInfo {
                            ty: Type::Any,
                            ownership: OwnershipState::Borrowed,
                            is_mutable: false,
                            is_initialized: true,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Check export declaration and register exported symbols
    fn check_export(&mut self, export: &ExportDecl, span: &Span) -> Result<(), TypeError> {
        match export {
            ExportDecl::Named { specifiers, source, .. } => {
                if source.is_some() {
                    // Re-export from another module: export { x } from "module"
                    // For now, just pass through without validation
                    // This would require module resolution to properly type-check
                    return Ok(());
                }

                // export { name1, name2 }
                for spec in specifiers {
                    let local_name = &spec.local.value.name;

                    // Check if the local symbol exists in the current environment
                    if let Some(var_info) = self.env.lookup(local_name) {
                        let export_name = spec.exported
                            .as_ref()
                            .map(|n| n.value.name.clone())
                            .unwrap_or_else(|| local_name.clone());

                        // Register the export
                        self.env.export_symbol(export_name, var_info.ty.clone());
                    } else {
                        // Symbol being exported doesn't exist
                        return Err(TypeError {
                            kind: TypeErrorKind::UndefinedVariable(format!(
                                "Cannot export undefined symbol '{}'",
                                local_name
                            )),
                            span: span.clone(),
                        });
                    }
                }
            }
            ExportDecl::Default(_expr) => {
                // export default expr
                // For now, just mark that there's a default export
                // We could check the expression and store its type
                self.env.export_symbol("default".to_string(), Type::Any);
            }
            ExportDecl::DefaultDecl(_decl) => {
                // export default function foo() { ... }
                // The declaration should already be checked
                // Just mark it as the default export
                self.env.export_symbol("default".to_string(), Type::Any);
            }
            ExportDecl::Decl(decl) => {
                // export function foo() { ... } or export class Bar { ... }
                // Check the declaration first
                self.check_decl(&decl.value, &decl.span)?;

                // Extract the actual declaration name and register its type as an export
                let (name, ty) = match &decl.value {
                    zaco_ast::Decl::Function(f) => {
                        let n = f.name.value.name.clone();
                        let t = self.env.lookup(&n).map(|v| v.ty.clone()).unwrap_or(Type::Any);
                        (n, t)
                    }
                    zaco_ast::Decl::Class(c) => {
                        let n = c.name.value.name.clone();
                        let t = self.env.lookup(&n).map(|v| v.ty.clone()).unwrap_or(Type::Any);
                        (n, t)
                    }
                    zaco_ast::Decl::Interface(i) => (i.name.value.name.clone(), Type::Any),
                    zaco_ast::Decl::TypeAlias(a) => (a.name.value.name.clone(), Type::Any),
                    zaco_ast::Decl::Enum(e) => (e.name.value.name.clone(), Type::Any),
                    zaco_ast::Decl::Var(v) => {
                        // For var declarations, export each declared binding
                        for declarator in &v.declarations {
                            if let zaco_ast::Pattern::Ident { name: ident, .. } = &declarator.pattern.value {
                                let n = ident.value.name.clone();
                                let t = self.env.lookup(&n).map(|vi| vi.ty.clone()).unwrap_or(Type::Any);
                                self.env.export_symbol(n, t);
                            }
                        }
                        return Ok(());
                    }
                    zaco_ast::Decl::Module(_) => ("module".to_string(), Type::Any),
                };
                self.env.export_symbol(name, ty);
            }
            ExportDecl::All { .. } => {
                // export * from "module"
                // This re-exports everything from another module
                // Requires module resolution to properly handle
                // For now, just pass through
            }
        }

        Ok(())
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
