use zaco_lexer::{Lexer, TokenKind};

fn main() {
    // Example 1: Simple TypeScript code
    println!("=== Example 1: Simple TypeScript ===");
    let source1 = r#"
        const greeting: string = "Hello, World!";
        let count: number = 42;
    "#;

    let mut lexer1 = Lexer::new(source1);
    let tokens1 = lexer1.tokenize();

    for token in &tokens1 {
        if token.kind != TokenKind::Eof {
            println!("{:?} at {:?} = '{}'", token.kind, token.span, token.value);
        }
    }

    // Example 2: Function with Zaco ownership
    println!("\n=== Example 2: Zaco Ownership ===");
    let source2 = r#"
        function process(owned data: mut string): string {
            return clone data;
        }
    "#;

    let mut lexer2 = Lexer::new(source2);
    let tokens2 = lexer2.tokenize();

    for token in &tokens2 {
        if token.kind != TokenKind::Eof {
            println!("{:?} at {:?} = '{}'", token.kind, token.span, token.value);
        }
    }

    // Example 3: Complex operators
    println!("\n=== Example 3: Complex Operators ===");
    let source3 = "a === b && c !== d || e?.foo ?? defaultValue";

    let mut lexer3 = Lexer::new(source3);
    let tokens3 = lexer3.tokenize();

    for token in &tokens3 {
        if token.kind != TokenKind::Eof {
            println!("{:?} = '{}'", token.kind, token.value);
        }
    }

    // Example 4: Number literals
    println!("\n=== Example 4: Number Literals ===");
    let source4 = "123 45.67 0x1A 0o77 0b1010 1_000_000 3.14e-10";

    let mut lexer4 = Lexer::new(source4);
    let tokens4 = lexer4.tokenize();

    for token in &tokens4 {
        if token.kind != TokenKind::Eof {
            println!("{:?} = '{}'", token.kind, token.value);
        }
    }

    // Example 5: Template literal
    println!("\n=== Example 5: Template Literal ===");
    let source5 = r#"`Hello, ${name}! Welcome.`"#;

    let mut lexer5 = Lexer::new(source5);
    let tokens5 = lexer5.tokenize();

    for token in &tokens5 {
        if token.kind != TokenKind::Eof {
            println!("{:?} = '{}'", token.kind, token.value);
        }
    }

    // Example 6: Arrow function
    println!("\n=== Example 6: Arrow Function ===");
    let source6 = "const add = (a: number, b: number): number => a + b;";

    let mut lexer6 = Lexer::new(source6);
    let tokens6 = lexer6.tokenize();

    for token in &tokens6 {
        if token.kind != TokenKind::Eof {
            println!("{:?} = '{}'", token.kind, token.value);
        }
    }

    // Example 7: Class with modifiers
    println!("\n=== Example 7: Class with Modifiers ===");
    let source7 = r#"
        abstract class Animal {
            protected readonly name: string;
            private age: number;

            public async move(): void {
                await this.performMove();
            }
        }
    "#;

    let mut lexer7 = Lexer::new(source7);
    let tokens7 = lexer7.tokenize();

    for token in &tokens7 {
        if token.kind != TokenKind::Eof {
            println!("{:?} = '{}'", token.kind, token.value);
        }
    }

    // Example 8: Comments
    println!("\n=== Example 8: Comments (should be skipped) ===");
    let source8 = r#"
        // This is a single-line comment
        let x = 5; // Another comment
        /* This is a
           multi-line comment */
        const y = 10;
    "#;

    let mut lexer8 = Lexer::new(source8);
    let tokens8 = lexer8.tokenize();

    for token in &tokens8 {
        if token.kind != TokenKind::Eof {
            println!("{:?} = '{}'", token.kind, token.value);
        }
    }

    // Example 9: String escapes
    println!("\n=== Example 9: String Escapes ===");
    let source9 = r#""Hello\nWorld\t!" 'It\'s' "Quote: \"Hi\"""#;

    let mut lexer9 = Lexer::new(source9);
    let tokens9 = lexer9.tokenize();

    for token in &tokens9 {
        if token.kind != TokenKind::Eof {
            println!("{:?} = {:?}", token.kind, token.value);
        }
    }

    // Example 10: Type annotations
    println!("\n=== Example 10: Type Annotations ===");
    let source10 = "type Result<T> = T | null; interface User { name: string; }";

    let mut lexer10 = Lexer::new(source10);
    let tokens10 = lexer10.tokenize();

    for token in &tokens10 {
        if token.kind != TokenKind::Eof {
            println!("{:?} = '{}'", token.kind, token.value);
        }
    }
}
