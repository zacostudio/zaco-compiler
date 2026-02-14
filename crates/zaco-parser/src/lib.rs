//! # Zaco Parser
//!
//! Recursive descent parser for TypeScript with Zaco ownership extensions.
//! Uses Pratt parsing for expressions with proper operator precedence.

use zaco_ast::*;
use zaco_lexer::{Token, TokenKind};

// Module declarations
mod error;
mod parser;
mod expr;
mod stmt;
mod types;
mod decl;
mod pattern;
mod helpers;

// Re-export public types
pub use error::{ParseError, ParseResult};
pub use parser::Parser;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use zaco_lexer::Lexer;

    fn parse(source: &str) -> Result<Program, Vec<ParseError>> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    #[test]
    fn test_parse_variable_declaration() {
        let source = "let x: number = 42;";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_function_declaration() {
        let source = "function add(a: number, b: number): number { return a + b; }";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_ownership_annotation() {
        let source = "let x: owned string = 'hello';";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_arrow_function() {
        let source = "const add = (a: number, b: number): number => a + b;";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_class_declaration() {
        let source = r#"
            class Point {
                x: number;
                y: number;

                constructor(x: number, y: number) {
                    this.x = x;
                    this.y = y;
                }
            }
        "#;
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_interface_declaration() {
        let source = r#"
            interface Drawable {
                draw(): void;
            }
        "#;
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_import_export() {
        let source = r#"
            import { foo, bar } from "./module";
            export function baz() {}
        "#;
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 2);
    }

    #[test]
    fn test_parse_control_flow() {
        let source = r#"
            if (x > 0) {
                console.log("positive");
            } else {
                console.log("negative");
            }

            for (let i = 0; i < 10; i++) {
                console.log(i);
            }

            while (true) {
                break;
            }
        "#;
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 3);
    }

    #[test]
    fn test_parse_optional_chaining() {
        // Optional member access
        let source = "let x = obj?.prop;";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
        if let ModuleItem::Stmt(stmt) = &program.items[0].value {
            if let Stmt::VarDecl(decl) = &stmt.value {
                if let Some(init) = &decl.declarations[0].init {
                    assert!(matches!(init.value, Expr::OptionalMember { .. }));
                }
            }
        }

        // Optional index access
        let source = "let y = arr?.[0];";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);

        // Optional call
        let source = "let z = fn?.();";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_spread_and_yield() {
        // Spread in call
        let source = "foo(...args);";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);

        // Generator with yield
        let source = "function* gen() { yield 42; }";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
        if let ModuleItem::Decl(decl) = &program.items[0].value {
            if let Decl::Function(func) = &decl.value {
                assert!(func.is_generator);
            }
        }

        // Yield with delegate
        let source = "function* gen() { yield* other(); }";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_satisfies() {
        let source = r#"let x = { name: "foo" } satisfies Record<string, string>;"#;
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
        if let ModuleItem::Stmt(stmt) = &program.items[0].value {
            if let Stmt::VarDecl(decl) = &stmt.value {
                if let Some(init) = &decl.declarations[0].init {
                    assert!(matches!(init.value, Expr::Satisfies { .. }));
                }
            }
        }
    }

    #[test]
    fn test_parse_advanced_types() {
        // Keyof type
        let source = "type K = keyof Person;";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);

        // Typeof type
        let source = "type T = typeof myVar;";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);

        // Conditional type
        let source = "type C = string extends any ? true : false;";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);

        // Indexed access type
        let source = r#"type A = Person["name"];"#;
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);

        // Mapped type
        let source = "type M = { [K in keyof T]: T[K] };";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);

        // Infer type
        let source = "type I = T extends Array<infer U> ? U : never;";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_parse_using_declaration() {
        let source = "using resource = getResource();";
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
        if let ModuleItem::Stmt(stmt) = &program.items[0].value {
            if let Stmt::VarDecl(decl) = &stmt.value {
                assert!(matches!(decl.kind, VarDeclKind::Using));
            }
        }
    }

    #[test]
    fn test_parse_decorators() {
        // Class decorator
        let source = r#"
            @Component
            class MyClass {
                @Input
                name: string;
            }
        "#;
        let program = parse(source).unwrap();
        assert_eq!(program.items.len(), 1);
        if let ModuleItem::Decl(decl) = &program.items[0].value {
            if let Decl::Class(class_decl) = &decl.value {
                assert!(!class_decl.decorators.is_empty());
            }
        }
    }
}
