use zaco_ast::Span;
use crate::token::{Token, TokenKind};

/// The lexer/tokenizer for TypeScript/Zaco.
pub struct Lexer<'a> {
    source: &'a str,
    chars: std::str::CharIndices<'a>,
    current_pos: usize,
    current_char: Option<char>,
    file_id: usize,
}

impl<'a> Lexer<'a> {
    /// Creates a new lexer from source code.
    pub fn new(source: &'a str) -> Self {
        Self::with_file_id(source, 0)
    }

    /// Creates a new lexer with a specific file ID.
    pub fn with_file_id(source: &'a str, file_id: usize) -> Self {
        let mut chars = source.char_indices();
        let current_char = chars.next().map(|(_, c)| c);
        Self {
            source,
            chars,
            current_pos: 0,
            current_char,
            file_id,
        }
    }

    /// Tokenizes the entire source code and returns all tokens.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    /// Gets the next token from the source.
    pub fn next_token(&mut self) -> Token {
        if let Some(error_token) = self.skip_whitespace_and_comments() {
            return error_token;
        }

        let start = self.current_pos;

        match self.current_char {
            None => Token::new(TokenKind::Eof, Span::new(start, start, self.file_id), String::new()),
            Some(ch) => {
                let token = match ch {
                    // String literals
                    '"' | '\'' => self.read_string_literal(ch),
                    '`' => self.read_template_literal(),

                    // Numbers
                    '0'..='9' => self.read_number(),

                    // Identifiers and keywords
                    'a'..='z' | 'A'..='Z' | '_' | '$' => self.read_identifier_or_keyword(),

                    // Operators and delimiters
                    '+' => self.read_plus(),
                    '-' => self.read_minus(),
                    '*' => self.read_star(),
                    '/' => self.read_slash_or_regex(),
                    '%' => self.read_percent(),
                    '=' => self.read_eq(),
                    '!' => self.read_bang(),
                    '<' => self.read_lt(),
                    '>' => self.read_gt(),
                    '&' => self.read_amp(),
                    '|' => self.read_pipe(),
                    '^' => {
                        self.advance();
                        if self.current_char == Some('=') {
                            self.advance();
                            Token::new(TokenKind::CaretEq, Span::new(start, self.current_pos, self.file_id), "^=".to_string())
                        } else {
                            Token::new(TokenKind::Caret, Span::new(start, self.current_pos, self.file_id), "^".to_string())
                        }
                    }
                    '~' => {
                        self.advance();
                        Token::new(TokenKind::Tilde, Span::new(start, self.current_pos, self.file_id), "~".to_string())
                    }
                    '?' => self.read_question(),
                    '.' => self.read_dot(),

                    // Delimiters
                    '(' => {
                        self.advance();
                        Token::new(TokenKind::LParen, Span::new(start, self.current_pos, self.file_id), "(".to_string())
                    }
                    ')' => {
                        self.advance();
                        Token::new(TokenKind::RParen, Span::new(start, self.current_pos, self.file_id), ")".to_string())
                    }
                    '{' => {
                        self.advance();
                        Token::new(TokenKind::LBrace, Span::new(start, self.current_pos, self.file_id), "{".to_string())
                    }
                    '}' => {
                        self.advance();
                        Token::new(TokenKind::RBrace, Span::new(start, self.current_pos, self.file_id), "}".to_string())
                    }
                    '[' => {
                        self.advance();
                        Token::new(TokenKind::LBracket, Span::new(start, self.current_pos, self.file_id), "[".to_string())
                    }
                    ']' => {
                        self.advance();
                        Token::new(TokenKind::RBracket, Span::new(start, self.current_pos, self.file_id), "]".to_string())
                    }
                    ';' => {
                        self.advance();
                        Token::new(TokenKind::Semicolon, Span::new(start, self.current_pos, self.file_id), ";".to_string())
                    }
                    ',' => {
                        self.advance();
                        Token::new(TokenKind::Comma, Span::new(start, self.current_pos, self.file_id), ",".to_string())
                    }
                    ':' => {
                        self.advance();
                        Token::new(TokenKind::Colon, Span::new(start, self.current_pos, self.file_id), ":".to_string())
                    }
                    '@' => {
                        self.advance();
                        Token::new(TokenKind::At, Span::new(start, self.current_pos, self.file_id), "@".to_string())
                    }

                    // Unicode identifiers
                    _ if ch.is_alphabetic() || ch == '_' => self.read_identifier_or_keyword(),

                    // Error
                    _ => {
                        self.advance();
                        Token::new(
                            TokenKind::Error,
                            Span::new(start, self.current_pos, self.file_id),
                            format!("Unexpected character: {}", ch),
                        )
                    }
                };
                token
            }
        }
    }

    // Helper methods

    fn advance(&mut self) {
        if let Some((pos, ch)) = self.chars.next() {
            self.current_pos = pos;
            self.current_char = Some(ch);
        } else {
            self.current_pos = self.source.len();
            self.current_char = None;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.clone().next().map(|(_, c)| c)
    }

    #[allow(dead_code)]
    fn peek_nth(&self, n: usize) -> Option<char> {
        self.chars.clone().nth(n).map(|(_, c)| c)
    }

    fn skip_whitespace_and_comments(&mut self) -> Option<Token> {
        loop {
            match self.current_char {
                Some(ch) if ch.is_whitespace() => {
                    self.advance();
                }
                Some('/') => {
                    if self.peek() == Some('/') {
                        // Single-line comment
                        self.skip_single_line_comment();
                    } else if self.peek() == Some('*') {
                        // Multi-line comment
                        let start = self.current_pos;
                        if !self.skip_multi_line_comment() {
                            return Some(Token::new(
                                TokenKind::Error,
                                Span::new(start, self.current_pos, self.file_id),
                                "Unterminated multi-line comment".to_string(),
                            ));
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        None
    }

    fn skip_single_line_comment(&mut self) {
        // Skip //
        self.advance();
        self.advance();

        while let Some(ch) = self.current_char {
            if ch == '\n' {
                self.advance();
                break;
            }
            self.advance();
        }
    }

    fn skip_multi_line_comment(&mut self) -> bool {
        // Skip /*
        self.advance();
        self.advance();

        while let Some(ch) = self.current_char {
            if ch == '*' && self.peek() == Some('/') {
                self.advance(); // *
                self.advance(); // /
                return true;
            }
            self.advance();
        }
        false // Unterminated
    }

    fn read_string_literal(&mut self, quote: char) -> Token {
        let start = self.current_pos;
        self.advance(); // Skip opening quote

        let mut value = String::new();

        while let Some(ch) = self.current_char {
            if ch == quote {
                self.advance(); // Skip closing quote
                return Token::new(
                    TokenKind::StringLiteral,
                    Span::new(start, self.current_pos, self.file_id),
                    value,
                );
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.current_char {
                    match escaped {
                        'u' => {
                            self.advance();
                            value.push(self.read_unicode_escape());
                            // read_unicode_escape already advanced past all hex digits
                        }
                        'x' => {
                            self.advance();
                            value.push(self.read_hex_escape());
                            // read_hex_escape already advanced past all hex digits
                        }
                        _ => {
                            let unescaped = match escaped {
                                'n' => '\n',
                                'r' => '\r',
                                't' => '\t',
                                '\\' => '\\',
                                '\'' => '\'',
                                '"' => '"',
                                '0' => '\0',
                                _ => escaped,
                            };
                            value.push(unescaped);
                            self.advance();
                        }
                    }
                }
            } else if ch == '\n' {
                // Unterminated string
                return Token::new(
                    TokenKind::Error,
                    Span::new(start, self.current_pos, self.file_id),
                    "Unterminated string literal".to_string(),
                );
            } else {
                value.push(ch);
                self.advance();
            }
        }

        Token::new(
            TokenKind::Error,
            Span::new(start, self.current_pos, self.file_id),
            "Unterminated string literal".to_string(),
        )
    }

    fn read_unicode_escape(&mut self) -> char {
        let mut code = 0u32;
        for _ in 0..4 {
            if let Some(ch) = self.current_char {
                if let Some(digit) = ch.to_digit(16) {
                    code = code * 16 + digit;
                    self.advance();
                } else {
                    break;
                }
            }
        }
        char::from_u32(code).unwrap_or('\u{FFFD}')
    }

    fn read_hex_escape(&mut self) -> char {
        let mut code = 0u32;
        for _ in 0..2 {
            if let Some(ch) = self.current_char {
                if let Some(digit) = ch.to_digit(16) {
                    code = code * 16 + digit;
                    self.advance();
                } else {
                    break;
                }
            }
        }
        char::from_u32(code).unwrap_or('\u{FFFD}')
    }

    fn read_template_literal(&mut self) -> Token {
        let start = self.current_pos;
        self.advance(); // Skip opening backtick

        let mut value = String::new();

        while let Some(ch) = self.current_char {
            if ch == '`' {
                self.advance(); // Skip closing backtick
                return Token::new(
                    TokenKind::TemplateLiteral,
                    Span::new(start, self.current_pos, self.file_id),
                    value,
                );
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.current_char {
                    let unescaped = match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '\\' => '\\',
                        '`' => '`',
                        _ => escaped,
                    };
                    value.push(unescaped);
                    self.advance();
                }
            } else {
                value.push(ch);
                self.advance();
            }
        }

        Token::new(
            TokenKind::Error,
            Span::new(start, self.current_pos, self.file_id),
            "Unterminated template literal".to_string(),
        )
    }

    fn read_number(&mut self) -> Token {
        let start = self.current_pos;

        // Check for special number formats
        if self.current_char == Some('0') {
            match self.peek() {
                Some('x') | Some('X') => return self.read_hex_number(start),
                Some('o') | Some('O') => return self.read_octal_number(start),
                Some('b') | Some('B') => return self.read_binary_number(start),
                _ => {}
            }
        }

        let mut value = String::new();

        // Read integer part
        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() || ch == '_' {
                if ch != '_' {
                    value.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }

        // Check for decimal point
        if self.current_char == Some('.') && self.peek().map_or(false, |c| c.is_ascii_digit()) {
            value.push('.');
            self.advance();

            while let Some(ch) = self.current_char {
                if ch.is_ascii_digit() || ch == '_' {
                    if ch != '_' {
                        value.push(ch);
                    }
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Check for exponent
        if matches!(self.current_char, Some('e') | Some('E')) {
            value.push('e');
            self.advance();

            if matches!(self.current_char, Some('+') | Some('-')) {
                value.push(self.current_char.unwrap());
                self.advance();
            }

            while let Some(ch) = self.current_char {
                if ch.is_ascii_digit() || ch == '_' {
                    if ch != '_' {
                        value.push(ch);
                    }
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Check for BigInt suffix
        if self.current_char == Some('n') {
            self.advance();
            return Token::new(TokenKind::BigIntLiteral, Span::new(start, self.current_pos, self.file_id), value);
        }

        Token::new(TokenKind::NumberLiteral, Span::new(start, self.current_pos, self.file_id), value)
    }

    fn read_hex_number(&mut self, start: usize) -> Token {
        let mut value = String::from("0x");
        self.advance(); // 0
        self.advance(); // x

        while let Some(ch) = self.current_char {
            if ch.is_ascii_hexdigit() || ch == '_' {
                if ch != '_' {
                    value.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }

        Token::new(TokenKind::NumberLiteral, Span::new(start, self.current_pos, self.file_id), value)
    }

    fn read_octal_number(&mut self, start: usize) -> Token {
        let mut value = String::from("0o");
        self.advance(); // 0
        self.advance(); // o

        while let Some(ch) = self.current_char {
            if ('0'..='7').contains(&ch) || ch == '_' {
                if ch != '_' {
                    value.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }

        Token::new(TokenKind::NumberLiteral, Span::new(start, self.current_pos, self.file_id), value)
    }

    fn read_binary_number(&mut self, start: usize) -> Token {
        let mut value = String::from("0b");
        self.advance(); // 0
        self.advance(); // b

        while let Some(ch) = self.current_char {
            if ch == '0' || ch == '1' || ch == '_' {
                if ch != '_' {
                    value.push(ch);
                }
                self.advance();
            } else {
                break;
            }
        }

        Token::new(TokenKind::NumberLiteral, Span::new(start, self.current_pos, self.file_id), value)
    }

    fn read_identifier_or_keyword(&mut self) -> Token {
        let start = self.current_pos;
        let mut value = String::new();

        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                value.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match value.as_str() {
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "var" => TokenKind::Var,
            "function" => TokenKind::Function,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "do" => TokenKind::Do,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "class" => TokenKind::Class,
            "extends" => TokenKind::Extends,
            "implements" => TokenKind::Implements,
            "interface" => TokenKind::Interface,
            "type" => TokenKind::Type,
            "enum" => TokenKind::Enum,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "from" => TokenKind::From,
            "as" => TokenKind::As,
            "new" => TokenKind::New,
            "this" => TokenKind::This,
            "super" => TokenKind::Super,
            "typeof" => TokenKind::Typeof,
            "instanceof" => TokenKind::Instanceof,
            "in" => TokenKind::In,
            "of" => TokenKind::Of,
            "void" => TokenKind::Void,
            "null" => TokenKind::Null,
            "undefined" => TokenKind::Undefined,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "yield" => TokenKind::Yield,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "throw" => TokenKind::Throw,
            "static" => TokenKind::Static,
            "public" => TokenKind::Public,
            "private" => TokenKind::Private,
            "protected" => TokenKind::Protected,
            "readonly" => TokenKind::Readonly,
            "abstract" => TokenKind::Abstract,
            "declare" => TokenKind::Declare,
            "module" => TokenKind::Module,
            "namespace" => TokenKind::Namespace,
            "require" => TokenKind::Require,
            "keyof" => TokenKind::Keyof,
            "infer" => TokenKind::Infer,
            "never" => TokenKind::Never,
            "unknown" => TokenKind::Unknown,
            "any" => TokenKind::Any,
            "satisfies" => TokenKind::Satisfies,
            "override" => TokenKind::Override,
            "is" => TokenKind::Is,
            "asserts" => TokenKind::Asserts,
            "out" => TokenKind::Out,
            "accessor" => TokenKind::Accessor,
            "using" => TokenKind::Using,
            "debugger" => TokenKind::Debugger,
            "with" => TokenKind::With,
            "owned" => TokenKind::Owned,
            "ref" => TokenKind::Ref,
            "clone" => TokenKind::Clone,
            "mut" => TokenKind::Mut,
            _ => TokenKind::Identifier,
        };

        Token::new(kind, Span::new(start, self.current_pos, self.file_id), value)
    }

    fn read_plus(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('+') => {
                self.advance();
                Token::new(TokenKind::PlusPlus, Span::new(start, self.current_pos, self.file_id), "++".to_string())
            }
            Some('=') => {
                self.advance();
                Token::new(TokenKind::PlusEq, Span::new(start, self.current_pos, self.file_id), "+=".to_string())
            }
            _ => Token::new(TokenKind::Plus, Span::new(start, self.current_pos, self.file_id), "+".to_string()),
        }
    }

    fn read_minus(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('-') => {
                self.advance();
                Token::new(TokenKind::MinusMinus, Span::new(start, self.current_pos, self.file_id), "--".to_string())
            }
            Some('=') => {
                self.advance();
                Token::new(TokenKind::MinusEq, Span::new(start, self.current_pos, self.file_id), "-=".to_string())
            }
            _ => Token::new(TokenKind::Minus, Span::new(start, self.current_pos, self.file_id), "-".to_string()),
        }
    }

    fn read_star(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('*') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::new(TokenKind::StarStarEq, Span::new(start, self.current_pos, self.file_id), "**=".to_string())
                } else {
                    Token::new(TokenKind::StarStar, Span::new(start, self.current_pos, self.file_id), "**".to_string())
                }
            }
            Some('=') => {
                self.advance();
                Token::new(TokenKind::StarEq, Span::new(start, self.current_pos, self.file_id), "*=".to_string())
            }
            _ => Token::new(TokenKind::Star, Span::new(start, self.current_pos, self.file_id), "*".to_string()),
        }
    }

    fn read_slash_or_regex(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('=') => {
                self.advance();
                Token::new(TokenKind::SlashEq, Span::new(start, self.current_pos, self.file_id), "/=".to_string())
            }
            _ => Token::new(TokenKind::Slash, Span::new(start, self.current_pos, self.file_id), "/".to_string()),
        }
    }

    fn read_percent(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        if self.current_char == Some('=') {
            self.advance();
            Token::new(TokenKind::PercentEq, Span::new(start, self.current_pos, self.file_id), "%=".to_string())
        } else {
            Token::new(TokenKind::Percent, Span::new(start, self.current_pos, self.file_id), "%".to_string())
        }
    }

    fn read_eq(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('=') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::new(TokenKind::EqEqEq, Span::new(start, self.current_pos, self.file_id), "===".to_string())
                } else {
                    Token::new(TokenKind::EqEq, Span::new(start, self.current_pos, self.file_id), "==".to_string())
                }
            }
            Some('>') => {
                self.advance();
                Token::new(TokenKind::FatArrow, Span::new(start, self.current_pos, self.file_id), "=>".to_string())
            }
            _ => Token::new(TokenKind::Eq, Span::new(start, self.current_pos, self.file_id), "=".to_string()),
        }
    }

    fn read_bang(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('=') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::new(TokenKind::BangEqEq, Span::new(start, self.current_pos, self.file_id), "!==".to_string())
                } else {
                    Token::new(TokenKind::BangEq, Span::new(start, self.current_pos, self.file_id), "!=".to_string())
                }
            }
            _ => Token::new(TokenKind::Bang, Span::new(start, self.current_pos, self.file_id), "!".to_string()),
        }
    }

    fn read_lt(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('<') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::new(TokenKind::LtLtEq, Span::new(start, self.current_pos, self.file_id), "<<=".to_string())
                } else {
                    Token::new(TokenKind::LtLt, Span::new(start, self.current_pos, self.file_id), "<<".to_string())
                }
            }
            Some('=') => {
                self.advance();
                Token::new(TokenKind::LtEq, Span::new(start, self.current_pos, self.file_id), "<=".to_string())
            }
            _ => Token::new(TokenKind::Lt, Span::new(start, self.current_pos, self.file_id), "<".to_string()),
        }
    }

    fn read_gt(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('>') => {
                self.advance();
                match self.current_char {
                    Some('>') => {
                        self.advance();
                        if self.current_char == Some('=') {
                            self.advance();
                            Token::new(TokenKind::GtGtGtEq, Span::new(start, self.current_pos, self.file_id), ">>>=".to_string())
                        } else {
                            Token::new(TokenKind::GtGtGt, Span::new(start, self.current_pos, self.file_id), ">>>".to_string())
                        }
                    }
                    Some('=') => {
                        self.advance();
                        Token::new(TokenKind::GtGtEq, Span::new(start, self.current_pos, self.file_id), ">>=".to_string())
                    }
                    _ => Token::new(TokenKind::GtGt, Span::new(start, self.current_pos, self.file_id), ">>".to_string()),
                }
            }
            Some('=') => {
                self.advance();
                Token::new(TokenKind::GtEq, Span::new(start, self.current_pos, self.file_id), ">=".to_string())
            }
            _ => Token::new(TokenKind::Gt, Span::new(start, self.current_pos, self.file_id), ">".to_string()),
        }
    }

    fn read_amp(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('&') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::new(TokenKind::AmpAmpEq, Span::new(start, self.current_pos, self.file_id), "&&=".to_string())
                } else {
                    Token::new(TokenKind::AmpAmp, Span::new(start, self.current_pos, self.file_id), "&&".to_string())
                }
            }
            Some('=') => {
                self.advance();
                Token::new(TokenKind::AmpEq, Span::new(start, self.current_pos, self.file_id), "&=".to_string())
            }
            _ => Token::new(TokenKind::Amp, Span::new(start, self.current_pos, self.file_id), "&".to_string()),
        }
    }

    fn read_pipe(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('|') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::new(TokenKind::PipePipeEq, Span::new(start, self.current_pos, self.file_id), "||=".to_string())
                } else {
                    Token::new(TokenKind::PipePipe, Span::new(start, self.current_pos, self.file_id), "||".to_string())
                }
            }
            Some('=') => {
                self.advance();
                Token::new(TokenKind::PipeEq, Span::new(start, self.current_pos, self.file_id), "|=".to_string())
            }
            _ => Token::new(TokenKind::Pipe, Span::new(start, self.current_pos, self.file_id), "|".to_string()),
        }
    }

    fn read_question(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        match self.current_char {
            Some('?') => {
                self.advance();
                if self.current_char == Some('=') {
                    self.advance();
                    Token::new(TokenKind::QuestionQuestionEq, Span::new(start, self.current_pos, self.file_id), "??=".to_string())
                } else {
                    Token::new(TokenKind::QuestionQuestion, Span::new(start, self.current_pos, self.file_id), "??".to_string())
                }
            }
            Some('.') => {
                self.advance();
                Token::new(TokenKind::QuestionDot, Span::new(start, self.current_pos, self.file_id), "?.".to_string())
            }
            _ => Token::new(TokenKind::Question, Span::new(start, self.current_pos, self.file_id), "?".to_string()),
        }
    }

    fn read_dot(&mut self) -> Token {
        let start = self.current_pos;
        self.advance();

        if self.current_char == Some('.') && self.peek() == Some('.') {
            self.advance();
            self.advance();
            Token::new(TokenKind::DotDotDot, Span::new(start, self.current_pos, self.file_id), "...".to_string())
        } else if self.current_char.map_or(false, |c| c.is_ascii_digit()) {
            // Number starting with dot (e.g., .5)
            // Read fractional part directly — don't rewind the iterator
            let mut value = String::from("0.");

            while let Some(ch) = self.current_char {
                if ch.is_ascii_digit() || ch == '_' {
                    if ch != '_' {
                        value.push(ch);
                    }
                    self.advance();
                } else {
                    break;
                }
            }

            // Check for exponent
            if matches!(self.current_char, Some('e') | Some('E')) {
                value.push('e');
                self.advance();

                if matches!(self.current_char, Some('+') | Some('-')) {
                    value.push(self.current_char.unwrap());
                    self.advance();
                }

                while let Some(ch) = self.current_char {
                    if ch.is_ascii_digit() || ch == '_' {
                        if ch != '_' {
                            value.push(ch);
                        }
                        self.advance();
                    } else {
                        break;
                    }
                }
            }

            Token::new(TokenKind::NumberLiteral, Span::new(start, self.current_pos, self.file_id), value)
        } else {
            Token::new(TokenKind::Dot, Span::new(start, self.current_pos, self.file_id), ".".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let source = "let const var function return if else owned ref mut";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert_eq!(tokens[1].kind, TokenKind::Const);
        assert_eq!(tokens[2].kind, TokenKind::Var);
        assert_eq!(tokens[3].kind, TokenKind::Function);
        assert_eq!(tokens[4].kind, TokenKind::Return);
        assert_eq!(tokens[5].kind, TokenKind::If);
        assert_eq!(tokens[6].kind, TokenKind::Else);
        assert_eq!(tokens[7].kind, TokenKind::Owned);
        assert_eq!(tokens[8].kind, TokenKind::Ref);
        assert_eq!(tokens[9].kind, TokenKind::Mut);
    }

    #[test]
    fn test_numbers() {
        let source = "123 45.67 0x1A 0o77 0b1010 1_000_000";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[0].value, "123");
        assert_eq!(tokens[1].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[1].value, "45.67");
        assert_eq!(tokens[2].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[2].value, "0x1A");
        assert_eq!(tokens[3].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[3].value, "0o77");
        assert_eq!(tokens[4].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[4].value, "0b1010");
        assert_eq!(tokens[5].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[5].value, "1000000");
    }

    #[test]
    fn test_strings() {
        let source = r#""hello" 'world' `template`"#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[0].value, "hello");
        assert_eq!(tokens[1].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[1].value, "world");
        assert_eq!(tokens[2].kind, TokenKind::TemplateLiteral);
        assert_eq!(tokens[2].value, "template");
    }

    #[test]
    fn test_operators() {
        let source = "+ - * / % ** += -= === !== <= >= && || ?? ?.";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[1].kind, TokenKind::Minus);
        assert_eq!(tokens[2].kind, TokenKind::Star);
        assert_eq!(tokens[3].kind, TokenKind::Slash);
        assert_eq!(tokens[4].kind, TokenKind::Percent);
        assert_eq!(tokens[5].kind, TokenKind::StarStar);
        assert_eq!(tokens[6].kind, TokenKind::PlusEq);
        assert_eq!(tokens[7].kind, TokenKind::MinusEq);
        assert_eq!(tokens[8].kind, TokenKind::EqEqEq);
        assert_eq!(tokens[9].kind, TokenKind::BangEqEq);
        assert_eq!(tokens[10].kind, TokenKind::LtEq);
        assert_eq!(tokens[11].kind, TokenKind::GtEq);
        assert_eq!(tokens[12].kind, TokenKind::AmpAmp);
        assert_eq!(tokens[13].kind, TokenKind::PipePipe);
        assert_eq!(tokens[14].kind, TokenKind::QuestionQuestion);
        assert_eq!(tokens[15].kind, TokenKind::QuestionDot);
    }

    #[test]
    fn test_comments() {
        let source = r#"
            // Single line comment
            let x = 5;
            /* Multi-line
               comment */
            const y = 10;
        "#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert_eq!(tokens[1].kind, TokenKind::Identifier);
        assert_eq!(tokens[2].kind, TokenKind::Eq);
        assert_eq!(tokens[3].kind, TokenKind::NumberLiteral);
        assert_eq!(tokens[4].kind, TokenKind::Semicolon);
        assert_eq!(tokens[5].kind, TokenKind::Const);
    }

    #[test]
    fn test_identifiers() {
        let source = "foo bar_123 _private $jquery";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].value, "foo");
        assert_eq!(tokens[1].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].value, "bar_123");
        assert_eq!(tokens[2].kind, TokenKind::Identifier);
        assert_eq!(tokens[2].value, "_private");
        assert_eq!(tokens[3].kind, TokenKind::Identifier);
        assert_eq!(tokens[3].value, "$jquery");
    }

    #[test]
    fn test_complex_expression() {
        let source = "const add = (a: number, b: number): number => a + b;";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Const);
        assert_eq!(tokens[1].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].value, "add");
        assert_eq!(tokens[2].kind, TokenKind::Eq);
        assert_eq!(tokens[3].kind, TokenKind::LParen);
        assert_eq!(tokens[4].kind, TokenKind::Identifier);
        assert_eq!(tokens[4].value, "a");
        assert_eq!(tokens[5].kind, TokenKind::Colon);
        assert_eq!(tokens[6].kind, TokenKind::Identifier);
        assert_eq!(tokens[6].value, "number");
    }

    #[test]
    fn test_ts5_tokens() {
        // Test new TS5 keywords
        let source = "satisfies override using debugger with";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Satisfies);
        assert_eq!(tokens[0].value, "satisfies");
        assert_eq!(tokens[1].kind, TokenKind::Override);
        assert_eq!(tokens[1].value, "override");
        assert_eq!(tokens[2].kind, TokenKind::Using);
        assert_eq!(tokens[2].value, "using");
        assert_eq!(tokens[3].kind, TokenKind::Debugger);
        assert_eq!(tokens[3].value, "debugger");
        assert_eq!(tokens[4].kind, TokenKind::With);
        assert_eq!(tokens[4].value, "with");

        // Test BigInt literals
        let source = "42n 0n 123456n";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::BigIntLiteral);
        assert_eq!(tokens[0].value, "42");
        assert_eq!(tokens[1].kind, TokenKind::BigIntLiteral);
        assert_eq!(tokens[1].value, "0");
        assert_eq!(tokens[2].kind, TokenKind::BigIntLiteral);
        assert_eq!(tokens[2].value, "123456");

        // Test compound assignment operators
        let source = "<<= >>= >>>= &= |= ^=";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::LtLtEq);
        assert_eq!(tokens[0].value, "<<=");
        assert_eq!(tokens[1].kind, TokenKind::GtGtEq);
        assert_eq!(tokens[1].value, ">>=");
        assert_eq!(tokens[2].kind, TokenKind::GtGtGtEq);
        assert_eq!(tokens[2].value, ">>>=");
        assert_eq!(tokens[3].kind, TokenKind::AmpEq);
        assert_eq!(tokens[3].value, "&=");
        assert_eq!(tokens[4].kind, TokenKind::PipeEq);
        assert_eq!(tokens[4].value, "|=");
        assert_eq!(tokens[5].kind, TokenKind::CaretEq);
        assert_eq!(tokens[5].value, "^=");
    }
}
