# Zaco Lexer

A comprehensive lexer/tokenizer for TypeScript with Zaco ownership extensions, implemented from scratch in Rust.

## Features

- **Full TypeScript Support**: Tokenizes all TypeScript keywords, operators, and literals
- **Zaco Ownership Extensions**: Supports `owned`, `ref`, `clone`, and `mut` keywords
- **Unicode Identifiers**: Handles Unicode characters in identifiers
- **Number Literals**: Supports integer, float, hex (0x), octal (0o), binary (0b), and underscore separators
- **String Literals**: Single quotes, double quotes, and template literals (backticks)
- **Escape Sequences**: Handles `\n`, `\r`, `\t`, `\\`, `\'`, `\"`, `\0`, `\uXXXX`, `\xXX`
- **Comments**: Skips single-line (`//`) and multi-line (`/* */`) comments
- **Operators**: All TypeScript operators including `?.`, `??`, `===`, `!==`, `=>`, `...`, etc.
- **Span Tracking**: Accurate source position tracking for error reporting
- **Zero Dependencies**: No external lexer libraries, fully custom implementation

## Usage

### Basic Example

```rust
use zaco_lexer::{Lexer, TokenKind};

let source = r#"
    const greeting: string = "Hello, World!";
    let count: number = 42;
"#;

let mut lexer = Lexer::new(source);
let tokens = lexer.tokenize();

for token in tokens {
    println!("{:?} = '{}'", token.kind, token.value);
}
```

### Zaco Ownership Example

```rust
use zaco_lexer::Lexer;

let source = r#"
    function process(owned data: mut string): string {
        return clone data;
    }
"#;

let mut lexer = Lexer::new(source);
let tokens = lexer.tokenize();
```

### Incremental Tokenization

```rust
use zaco_lexer::{Lexer, TokenKind};

let source = "const x = 42;";
let mut lexer = Lexer::new(source);

loop {
    let token = lexer.next_token();
    println!("{:?}", token);
    if token.kind == TokenKind::Eof {
        break;
    }
}
```

## Token Types

### Keywords

#### TypeScript Keywords
`let`, `const`, `var`, `function`, `return`, `if`, `else`, `for`, `while`, `do`, `break`, `continue`, `switch`, `case`, `default`, `class`, `extends`, `implements`, `interface`, `type`, `enum`, `import`, `export`, `from`, `as`, `new`, `this`, `super`, `typeof`, `instanceof`, `in`, `of`, `void`, `null`, `undefined`, `true`, `false`, `async`, `await`, `yield`, `try`, `catch`, `finally`, `throw`, `static`, `public`, `private`, `protected`, `readonly`, `abstract`, `declare`, `module`, `namespace`, `require`, `keyof`, `infer`, `never`, `unknown`, `any`

#### Zaco Ownership Keywords
`owned`, `ref`, `clone`, `mut`

### Literals

- **NumberLiteral**: `123`, `45.67`, `0x1A`, `0o77`, `0b1010`, `1_000_000`, `3.14e-10`
- **StringLiteral**: `"hello"`, `'world'`
- **TemplateLiteral**: `` `template string` ``
- **RegexLiteral**: (Reserved for future use)

### Operators

- **Arithmetic**: `+`, `-`, `*`, `/`, `%`, `**`
- **Assignment**: `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `&&=`, `||=`, `??=`
- **Comparison**: `==`, `===`, `!=`, `!==`, `<`, `>`, `<=`, `>=`
- **Logical**: `&&`, `||`, `!`
- **Bitwise**: `&`, `|`, `^`, `~`, `<<`, `>>`, `>>>`
- **Other**: `++`, `--`, `=>`, `...`, `??`, `?.`

### Delimiters

`(`, `)`, `{`, `}`, `[`, `]`, `;`, `,`, `.`, `:`, `?`, `@`

## Token Structure

Each token contains:

```rust
pub struct Token {
    pub kind: TokenKind,  // Type of token
    pub span: Span,       // Source position (start, end)
    pub value: String,    // Actual text value
}
```

## Span Tracking

The lexer tracks source positions using the `Span` type from `zaco-ast`:

```rust
pub struct Span {
    pub start: usize,  // Byte offset start
    pub end: usize,    // Byte offset end
}
```

This enables accurate error reporting and source mapping.

## Comments

Comments are automatically skipped during tokenization:

```rust
let source = r#"
    // Single-line comment
    let x = 5;

    /* Multi-line
       comment */
    const y = 10;
"#;

let mut lexer = Lexer::new(source);
let tokens = lexer.tokenize();
// Only `let`, `x`, `=`, `5`, `;`, `const`, `y`, `=`, `10`, `;` are tokenized
```

## Number Formats

### Decimal
- Integer: `123`, `1_000_000`
- Float: `45.67`, `3.14`
- Scientific: `1e10`, `2.5e-3`

### Hexadecimal
`0x1A`, `0xFF`, `0x00`

### Octal
`0o77`, `0o123`

### Binary
`0b1010`, `0b11110000`

## String Escapes

Supported escape sequences:

- `\n` - Newline
- `\r` - Carriage return
- `\t` - Tab
- `\\` - Backslash
- `\'` - Single quote
- `\"` - Double quote
- `\0` - Null character
- `\uXXXX` - Unicode escape (4 hex digits)
- `\xXX` - Hex escape (2 hex digits)

Example:

```rust
let source = r#""Hello\nWorld\t!" 'It\'s' "Quote: \"Hi\"""#;
let mut lexer = Lexer::new(source);
let tokens = lexer.tokenize();
```

## Error Handling

The lexer produces error tokens for invalid input:

```rust
let source = "let x = @invalid;";
let mut lexer = Lexer::new(source);
let tokens = lexer.tokenize();

// tokens[3].kind == TokenKind::Error
// tokens[3].value == "Unexpected character: @"
```

Common errors:
- Unterminated strings
- Invalid characters
- Malformed numbers

## Performance

The lexer is designed for speed and efficiency:

- Single-pass tokenization
- Zero-copy where possible (uses `&str` references)
- Minimal allocations
- Direct character iteration using `CharIndices`

## Testing

Run the test suite:

```bash
cargo test --package zaco-lexer
```

Run examples:

```bash
cargo run --package zaco-lexer --example basic_usage
```

## Implementation Details

### Architecture

The lexer uses a simple state machine approach:

1. **Whitespace/Comment Skipping**: Automatically skip whitespace and comments
2. **Character Dispatch**: Match current character to appropriate tokenization method
3. **Lookahead**: Use `peek()` for multi-character operators
4. **Position Tracking**: Update span information as characters are consumed

### Key Methods

- `new(source: &str)` - Create a new lexer
- `tokenize(&mut self) -> Vec<Token>` - Tokenize entire source
- `next_token(&mut self) -> Token` - Get next single token
- `advance(&mut self)` - Move to next character
- `peek(&self) -> Option<char>` - Look at next character without consuming

## License

Part of the Zaco compiler project.
