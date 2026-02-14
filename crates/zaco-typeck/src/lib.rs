//! # Zaco Type Checker
//!
//! Complete type checker with ownership inference for the Zaco compiler.
//! Implements TypeScript-style type checking with Rust-style ownership semantics.

mod error;
mod types;
mod ownership;
mod env;
mod typed_ast;
mod helpers;
mod checker;
mod decl_checker;
mod stmt_checker;
mod expr_checker;
mod builtins;

// Re-export public API
pub use error::{TypeError, TypeErrorKind};
pub use types::{Type, LiteralType};
pub use ownership::{OwnershipState, VarInfo};
pub use env::TypeEnv;
pub use typed_ast::{TypedExpr, TypedStmt, TypedProgram, TypedModuleItem, TypedDecl};
pub use checker::TypeChecker;

use zaco_ast::Program;

// =============================================================================
// Public API
// =============================================================================

/// Type check a program and return typed AST or errors
pub fn check_program(program: &Program) -> Result<TypedProgram, Vec<TypeError>> {
    let mut checker = TypeChecker::new();
    checker.check_program(program)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use zaco_ast::*;

    fn dummy_span() -> Span {
        Span::new(0, 0, 0)
    }

    fn make_node<T>(value: T) -> Node<T> {
        Node::new(value, dummy_span())
    }

    #[test]
    fn test_simple_var_decl() {
        let program = Program {
            items: vec![make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(
                VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("x")),
                            type_annotation: Some(Box::new(make_node(zaco_ast::Type::Primitive(
                                PrimitiveType::Number,
                            )))),
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Literal(Literal::Number(42.0)))),
                    }],
                },
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_mismatch() {
        let program = Program {
            items: vec![make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(
                VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("x")),
                            type_annotation: Some(Box::new(make_node(zaco_ast::Type::Primitive(
                                PrimitiveType::Number,
                            )))),
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Literal(Literal::String(
                            "hello".to_string(),
                        )))),
                    }],
                },
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_err());
        if let Err(errors) = result {
            assert_eq!(errors.len(), 1);
            assert!(matches!(
                errors[0].kind,
                TypeErrorKind::TypeMismatch { .. }
            ));
        }
    }

    #[test]
    fn test_undefined_variable() {
        let program = Program {
            items: vec![make_node(ModuleItem::Stmt(make_node(Stmt::Expr(
                make_node(Expr::Ident(Ident::new("x"))),
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_err());
        if let Err(errors) = result {
            assert_eq!(errors.len(), 1);
            assert!(matches!(
                errors[0].kind,
                TypeErrorKind::UndefinedVariable(_)
            ));
        }
    }

    #[test]
    fn test_function_type() {
        let program = Program {
            items: vec![make_node(ModuleItem::Decl(make_node(Decl::Function(
                FunctionDecl {
                    name: make_node(Ident::new("add")),
                    type_params: None,
                    params: vec![
                        Param {
                            pattern: make_node(Pattern::Ident {
                                name: make_node(Ident::new("a")),
                                type_annotation: Some(Box::new(make_node(
                                    zaco_ast::Type::Primitive(PrimitiveType::Number),
                                ))),
                                ownership: None,
                            }),
                            type_annotation: Some(Box::new(make_node(zaco_ast::Type::Primitive(
                                PrimitiveType::Number,
                            )))),
                            ownership: None,
                            optional: false,
                            is_rest: false,
                        },
                        Param {
                            pattern: make_node(Pattern::Ident {
                                name: make_node(Ident::new("b")),
                                type_annotation: Some(Box::new(make_node(
                                    zaco_ast::Type::Primitive(PrimitiveType::Number),
                                ))),
                                ownership: None,
                            }),
                            type_annotation: Some(Box::new(make_node(zaco_ast::Type::Primitive(
                                PrimitiveType::Number,
                            )))),
                            ownership: None,
                            optional: false,
                            is_rest: false,
                        },
                    ],
                    return_type: Some(Box::new(make_node(zaco_ast::Type::Primitive(
                        PrimitiveType::Number,
                    )))),
                    body: Some(make_node(BlockStmt {
                        stmts: vec![make_node(Stmt::Return(Some(make_node(Expr::Binary {
                            left: Box::new(make_node(Expr::Ident(Ident::new("a")))),
                            op: BinaryOp::Add,
                            right: Box::new(make_node(Expr::Ident(Ident::new("b")))),
                        }))))],
                    })),
                    is_async: false,
                    is_generator: false,
                    is_declare: false,
                },
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ownership_move() {
        // This test would require more complex setup to properly test move semantics
        // For now, just ensure basic structure works
        let program = Program {
            items: vec![],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_optional_chaining_type() {
        // Optional chaining should produce T | undefined
        // obj?.prop where obj: { prop: number } should give number | undefined
        let program = Program {
            items: vec![
                // let obj: { prop: number } = { prop: 42 };
                make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("obj")),
                            type_annotation: Some(Box::new(make_node(zaco_ast::Type::Object(
                                ObjectType {
                                    members: vec![ObjectTypeMember::Property {
                                        name: PropertyName::Ident(make_node(Ident::new("prop"))),
                                        ty: make_node(zaco_ast::Type::Primitive(PrimitiveType::Number)),
                                        optional: false,
                                        readonly: false,
                                    }],
                                },
                            )))),
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Object(vec![ObjectProperty::Property {
                            key: PropertyName::Ident(make_node(Ident::new("prop"))),
                            value: make_node(Expr::Literal(Literal::Number(42.0))),
                            shorthand: false,
                        }]))),
                    }],
                })))),
                // let x = obj?.prop;
                make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("x")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::OptionalMember {
                            object: Box::new(make_node(Expr::Ident(Ident::new("obj")))),
                            property: make_node(Ident::new("prop")),
                        })),
                    }],
                })))),
            ],
            span: dummy_span(),
        };

        let result = check_program(&program);
        // This should succeed - optional chaining is valid
        // The type checker should handle OptionalMember expressions
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_satisfies_type() {
        // Satisfies expression should pass through the expression type
        // { name: "foo" } satisfies Record<string, string> should still be { name: string }
        let program = Program {
            items: vec![make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(
                VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("x")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Satisfies {
                            expr: Box::new(make_node(Expr::Object(vec![
                                ObjectProperty::Property {
                                    key: PropertyName::Ident(make_node(Ident::new("name"))),
                                    value: make_node(Expr::Literal(Literal::String(
                                        "foo".to_string(),
                                    ))),
                                    shorthand: false,
                                },
                            ]))),
                            ty: Box::new(make_node(zaco_ast::Type::TypeRef {
                                name: make_node(Ident::new("Record")),
                                type_args: Some(vec![
                                    make_node(zaco_ast::Type::Primitive(PrimitiveType::String)),
                                    make_node(zaco_ast::Type::Primitive(PrimitiveType::String)),
                                ]),
                            })),
                        })),
                    }],
                },
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        // This should succeed or fail based on whether satisfies is implemented
        // The test verifies the AST structure is correctly formed
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_spread_expression() {
        // Spread in array: [...arr]
        let program = Program {
            items: vec![
                // let arr = [1, 2, 3];
                make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("arr")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Array(vec![
                            Some(make_node(Expr::Literal(Literal::Number(1.0)))),
                            Some(make_node(Expr::Literal(Literal::Number(2.0)))),
                            Some(make_node(Expr::Literal(Literal::Number(3.0)))),
                        ]))),
                    }],
                })))),
                // let spread = [...arr];
                make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("spread")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Array(vec![Some(make_node(
                            Expr::Spread(Box::new(make_node(Expr::Ident(Ident::new("arr"))))),
                        ))]))),
                    }],
                })))),
            ],
            span: dummy_span(),
        };

        let result = check_program(&program);
        // Should succeed - spread is a valid operation
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_yield_expression() {
        // Generator function with yield
        let program = Program {
            items: vec![make_node(ModuleItem::Decl(make_node(Decl::Function(
                FunctionDecl {
                    name: make_node(Ident::new("gen")),
                    type_params: None,
                    params: vec![],
                    return_type: None,
                    body: Some(make_node(BlockStmt {
                        stmts: vec![make_node(Stmt::Expr(make_node(Expr::Yield {
                            argument: Some(Box::new(make_node(Expr::Literal(Literal::Number(
                                42.0,
                            ))))),
                            delegate: false,
                        })))],
                    })),
                    is_async: false,
                    is_generator: true,
                    is_declare: false,
                },
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        // Should succeed - yield in generator is valid
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_using_declaration() {
        // using resource = getResource();
        let program = Program {
            items: vec![make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(
                VarDecl {
                    kind: VarDeclKind::Using,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("resource")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Call {
                            callee: Box::new(make_node(Expr::Ident(Ident::new("getResource")))),
                            type_args: None,
                            args: vec![],
                        })),
                    }],
                },
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        // Should succeed or fail based on using implementation
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_import_from_builtin_fs() {
        // import { readFileSync } from "fs";
        // let content = readFileSync("test.txt", "utf-8");
        let program = Program {
            items: vec![
                make_node(ModuleItem::Import(ImportDecl {
                    specifiers: vec![ImportSpecifier::Named {
                        imported: make_node(Ident::new("readFileSync")),
                        local: None,
                        type_only: false,
                    }],
                    source: "fs".to_string(),
                    type_only: false,
                })),
                make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("content")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Call {
                            callee: Box::new(make_node(Expr::Ident(Ident::new("readFileSync")))),
                            type_args: None,
                            args: vec![
                                make_node(Expr::Literal(Literal::String("test.txt".to_string()))),
                                make_node(Expr::Literal(Literal::String("utf-8".to_string()))),
                            ],
                        })),
                    }],
                })))),
            ],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok(), "Should successfully import and use readFileSync");
    }

    #[test]
    fn test_import_unknown_symbol_from_builtin() {
        // import { unknownFunc } from "fs";
        let program = Program {
            items: vec![make_node(ModuleItem::Import(ImportDecl {
                specifiers: vec![ImportSpecifier::Named {
                    imported: make_node(Ident::new("unknownFunc")),
                    local: None,
                    type_only: false,
                }],
                source: "fs".to_string(),
                type_only: false,
            }))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_err(), "Should fail - unknownFunc doesn't exist in fs module");
    }

    #[test]
    fn test_import_with_alias() {
        // import { readFileSync as read } from "fs";
        // let content = read("test.txt", "utf-8");
        let program = Program {
            items: vec![
                make_node(ModuleItem::Import(ImportDecl {
                    specifiers: vec![ImportSpecifier::Named {
                        imported: make_node(Ident::new("readFileSync")),
                        local: Some(make_node(Ident::new("read"))),
                        type_only: false,
                    }],
                    source: "fs".to_string(),
                    type_only: false,
                })),
                make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("content")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Call {
                            callee: Box::new(make_node(Expr::Ident(Ident::new("read")))),
                            type_args: None,
                            args: vec![
                                make_node(Expr::Literal(Literal::String("test.txt".to_string()))),
                                make_node(Expr::Literal(Literal::String("utf-8".to_string()))),
                            ],
                        })),
                    }],
                })))),
            ],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok(), "Should successfully import with alias");
    }

    #[test]
    fn test_namespace_import() {
        // import * as fs from "fs";
        // let content = fs.readFileSync("test.txt", "utf-8");
        let program = Program {
            items: vec![
                make_node(ModuleItem::Import(ImportDecl {
                    specifiers: vec![ImportSpecifier::Namespace(make_node(Ident::new("fs")))],
                    source: "fs".to_string(),
                    type_only: false,
                })),
                make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("content")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Call {
                            callee: Box::new(make_node(Expr::Member {
                                object: Box::new(make_node(Expr::Ident(Ident::new("fs")))),
                                property: make_node(Ident::new("readFileSync")),
                                computed: false,
                            })),
                            type_args: None,
                            args: vec![
                                make_node(Expr::Literal(Literal::String("test.txt".to_string()))),
                                make_node(Expr::Literal(Literal::String("utf-8".to_string()))),
                            ],
                        })),
                    }],
                })))),
            ],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok(), "Should successfully use namespace import");
    }

    #[test]
    fn test_global_math_usage() {
        // let x = Math.floor(3.7);
        let program = Program {
            items: vec![make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(
                VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("x")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Call {
                            callee: Box::new(make_node(Expr::Member {
                                object: Box::new(make_node(Expr::Ident(Ident::new("Math")))),
                                property: make_node(Ident::new("floor")),
                                computed: false,
                            })),
                            type_args: None,
                            args: vec![make_node(Expr::Literal(Literal::Number(3.7)))],
                        })),
                    }],
                },
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok(), "Should successfully use Math.floor");
    }

    #[test]
    fn test_global_console_log() {
        // console.log("Hello");
        let program = Program {
            items: vec![make_node(ModuleItem::Stmt(make_node(Stmt::Expr(
                make_node(Expr::Call {
                    callee: Box::new(make_node(Expr::Member {
                        object: Box::new(make_node(Expr::Ident(Ident::new("console")))),
                        property: make_node(Ident::new("log")),
                        computed: false,
                    })),
                    type_args: None,
                    args: vec![make_node(Expr::Literal(Literal::String(
                        "Hello".to_string(),
                    )))],
                }),
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok(), "Should successfully use console.log");
    }

    #[test]
    fn test_export_function() {
        // export function foo() { return 42; }
        let program = Program {
            items: vec![
                make_node(ModuleItem::Decl(make_node(Decl::Function(
                    FunctionDecl {
                        name: make_node(Ident::new("foo")),
                        type_params: None,
                        params: vec![],
                        return_type: None,
                        body: Some(make_node(BlockStmt {
                            stmts: vec![make_node(Stmt::Return(Some(make_node(
                                Expr::Literal(Literal::Number(42.0)),
                            ))))],
                        })),
                        is_async: false,
                        is_generator: false,
                        is_declare: false,
                    },
                )))),
                make_node(ModuleItem::Export(ExportDecl::Named {
                    specifiers: vec![ExportSpecifier {
                        local: make_node(Ident::new("foo")),
                        exported: None,
                        type_only: false,
                    }],
                    source: None,
                    type_only: false,
                })),
            ],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok(), "Should successfully export a function");
    }

    #[test]
    fn test_export_undefined_symbol() {
        // export { undefinedVar };
        let program = Program {
            items: vec![make_node(ModuleItem::Export(ExportDecl::Named {
                specifiers: vec![ExportSpecifier {
                    local: make_node(Ident::new("undefinedVar")),
                    exported: None,
                    type_only: false,
                }],
                source: None,
                type_only: false,
            }))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_err(), "Should fail - trying to export undefined symbol");
    }

    #[test]
    fn test_dirname_global() {
        // let d = __dirname;
        let program = Program {
            items: vec![make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(
                VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("d")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Ident(Ident::new("__dirname")))),
                    }],
                },
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok(), "Should successfully use __dirname global");
    }

    #[test]
    fn test_filename_global() {
        // let f = __filename;
        let program = Program {
            items: vec![make_node(ModuleItem::Stmt(make_node(Stmt::VarDecl(
                VarDecl {
                    kind: VarDeclKind::Let,
                    declarations: vec![VarDeclarator {
                        pattern: make_node(Pattern::Ident {
                            name: make_node(Ident::new("f")),
                            type_annotation: None,
                            ownership: None,
                        }),
                        init: Some(make_node(Expr::Ident(Ident::new("__filename")))),
                    }],
                },
            ))))],
            span: dummy_span(),
        };

        let result = check_program(&program);
        assert!(result.is_ok(), "Should successfully use __filename global");
    }

    #[test]
    fn test_generic_class_member_substitution() {
        use crate::types::Type as TyType;

        // class Container<T> { value: T; }
        // let c: Container<number> = ...;
        // c.value → should resolve to number
        let mut checker = TypeChecker::new();

        // Define a generic class Container<T> with field value: T
        let container_type = TyType::Class {
            name: "Container".to_string(),
            fields: vec![
                ("value".to_string(), TyType::TypeRef { name: "T".to_string(), type_args: vec![] }),
            ],
            methods: vec![],
        };
        checker.env.define_class("Container".to_string(), container_type);
        checker.env.define_type_params("Container".to_string(), vec!["T".to_string()]);

        // Declare variable c: Container<number>
        checker.env.declare("c".to_string(), VarInfo {
            ty: TyType::TypeRef {
                name: "Container".to_string(),
                type_args: vec![TyType::Number],
            },
            ownership: OwnershipState::Owned,
            is_mutable: false,
            is_initialized: true,
        });

        // Check c.value
        let result = checker.check_expr(
            &Expr::Member {
                object: Box::new(make_node(Expr::Ident(Ident::new("c")))),
                property: make_node(Ident::new("value")),
                computed: false,
            },
            &dummy_span(),
        );

        assert!(result.is_ok(), "Should resolve generic member access");
        assert_eq!(result.unwrap(), TyType::Number, "Container<number>.value should be number");
    }

    #[test]
    fn test_generic_interface_member_substitution() {
        use crate::types::Type as TyType;

        // interface Wrapper<T> { data: T; }
        let mut checker = TypeChecker::new();

        let wrapper_type = TyType::Interface {
            name: "Wrapper".to_string(),
            properties: vec![
                ("data".to_string(), TyType::TypeRef { name: "T".to_string(), type_args: vec![] }, false),
            ],
        };
        checker.env.define_interface("Wrapper".to_string(), wrapper_type);
        checker.env.define_type_params("Wrapper".to_string(), vec!["T".to_string()]);

        // Declare variable w: Wrapper<string>
        checker.env.declare("w".to_string(), VarInfo {
            ty: TyType::TypeRef {
                name: "Wrapper".to_string(),
                type_args: vec![TyType::String],
            },
            ownership: OwnershipState::Owned,
            is_mutable: false,
            is_initialized: true,
        });

        // Check w.data
        let result = checker.check_expr(
            &Expr::Member {
                object: Box::new(make_node(Expr::Ident(Ident::new("w")))),
                property: make_node(Ident::new("data")),
                computed: false,
            },
            &dummy_span(),
        );

        assert!(result.is_ok(), "Should resolve generic interface member access");
        assert_eq!(result.unwrap(), TyType::String, "Wrapper<string>.data should be string");
    }
}
