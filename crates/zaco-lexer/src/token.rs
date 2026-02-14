use zaco_ast::Span;

/// Represents the different kinds of tokens in TypeScript/Zaco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords - TypeScript
    Let,
    Const,
    Var,
    Function,
    Return,
    If,
    Else,
    For,
    While,
    Do,
    Break,
    Continue,
    Switch,
    Case,
    Default,
    Class,
    Extends,
    Implements,
    Interface,
    Type,
    Enum,
    Import,
    Export,
    From,
    As,
    New,
    This,
    Super,
    Typeof,
    Instanceof,
    In,
    Of,
    Void,
    Null,
    Undefined,
    True,
    False,
    Async,
    Await,
    Yield,
    Try,
    Catch,
    Finally,
    Throw,
    Static,
    Public,
    Private,
    Protected,
    Readonly,
    Abstract,
    Declare,
    Module,
    Namespace,
    Require,
    Keyof,
    Infer,
    Never,
    Unknown,
    Any,
    Satisfies,
    Override,
    Is,
    Asserts,
    Out,
    Accessor,
    Using,
    Debugger,
    With,

    // Keywords - Zaco ownership
    Owned,
    Ref,
    Clone,
    Mut,

    // Literals
    NumberLiteral,
    StringLiteral,
    TemplateLiteral,
    RegexLiteral,
    BigIntLiteral,

    // Identifier
    Identifier,

    // Operators
    Plus,              // +
    Minus,             // -
    Star,              // *
    Slash,             // /
    Percent,           // %
    StarStar,          // **
    Eq,                // =
    EqEq,              // ==
    EqEqEq,            // ===
    BangEq,            // !=
    BangEqEq,          // !==
    Lt,                // <
    Gt,                // >
    LtEq,              // <=
    GtEq,              // >=
    AmpAmp,            // &&
    PipePipe,          // ||
    Bang,              // !
    Amp,               // &
    Pipe,              // |
    Caret,             // ^
    Tilde,             // ~
    LtLt,              // <<
    GtGt,              // >>
    GtGtGt,            // >>>
    PlusEq,            // +=
    MinusEq,           // -=
    StarEq,            // *=
    SlashEq,           // /=
    PercentEq,         // %=
    StarStarEq,        // **=
    AmpAmpEq,          // &&=
    PipePipeEq,        // ||=
    QuestionQuestionEq,// ??=
    LtLtEq,               // <<=
    GtGtEq,               // >>=
    GtGtGtEq,             // >>>=
    AmpEq,                // &=
    PipeEq,               // |=
    CaretEq,              // ^=
    QuestionQuestion,  // ??
    QuestionDot,       // ?.
    PlusPlus,          // ++
    MinusMinus,        // --
    FatArrow,          // =>
    DotDotDot,         // ...

    // Delimiters
    LParen,            // (
    RParen,            // )
    LBrace,            // {
    RBrace,            // }
    LBracket,          // [
    RBracket,          // ]
    Semicolon,         // ;
    Comma,             // ,
    Dot,               // .
    Colon,             // :
    Question,          // ?
    At,                // @

    // Special
    Eof,
    Error,
}

/// Represents a token with its kind, span, and value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub value: String,
}

impl Token {
    pub(crate) fn new(kind: TokenKind, span: Span, value: String) -> Self {
        Self { kind, span, value }
    }
}
