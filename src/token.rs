use std::fmt;

use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum StringChunk {
    Literal(String),
    Expr(String, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Function,
    Let,
    Const,
    If,
    Else,
    While,
    For,
    Return,
    Break,
    Continue,
    True,
    False,

    TypeInt,
    TypeFloat,
    TypeDouble,
    TypeBoolean,
    TypeString,
    TypeVoid,

    Ident(String),
    IntLit(i64),
    FloatLit(f32),
    DoubleLit(f64),
    StringLit(String),
    InterpolatedString(Vec<StringChunk>),

    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,

    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    AndAnd,
    OrOr,
    Bang,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Comma,
    Colon,
    Semicolon,
    Arrow,

    Newline,
    Eof,
}

impl TokenKind {
    pub fn is_statement_ender(&self) -> bool {
        matches!(
            self,
            TokenKind::Ident(_)
                | TokenKind::IntLit(_)
                | TokenKind::FloatLit(_)
                | TokenKind::DoubleLit(_)
                | TokenKind::StringLit(_)
                | TokenKind::InterpolatedString(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::RParen
                | TokenKind::RBrace
                | TokenKind::RBracket
                | TokenKind::Return
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::TypeInt
                | TokenKind::TypeFloat
                | TokenKind::TypeDouble
                | TokenKind::TypeBoolean
                | TokenKind::TypeString
                | TokenKind::TypeVoid
        )
    }

    pub fn display_name(&self) -> &'static str {
        if let Some(name) = self.keyword_name() {
            return name;
        }
        if let Some(name) = self.symbol_name() {
            return name;
        }
        self.literal_name()
    }

    fn keyword_name(&self) -> Option<&'static str> {
        Some(match self {
            TokenKind::Function => "'function'",
            TokenKind::Let => "'let'",
            TokenKind::Const => "'const'",
            TokenKind::If => "'if'",
            TokenKind::Else => "'else'",
            TokenKind::While => "'while'",
            TokenKind::For => "'for'",
            TokenKind::Return => "'return'",
            TokenKind::Break => "'break'",
            TokenKind::Continue => "'continue'",
            TokenKind::True => "'true'",
            TokenKind::False => "'false'",
            TokenKind::TypeInt => "'int'",
            TokenKind::TypeFloat => "'float'",
            TokenKind::TypeDouble => "'double'",
            TokenKind::TypeBoolean => "'boolean'",
            TokenKind::TypeString => "'string'",
            TokenKind::TypeVoid => "'void'",
            _ => return None,
        })
    }

    fn symbol_name(&self) -> Option<&'static str> {
        Some(match self {
            TokenKind::Plus => "'+'",
            TokenKind::Minus => "'-'",
            TokenKind::Star => "'*'",
            TokenKind::Slash => "'/'",
            TokenKind::Percent => "'%'",
            TokenKind::Eq => "'='",
            TokenKind::PlusEq => "'+='",
            TokenKind::MinusEq => "'-='",
            TokenKind::StarEq => "'*='",
            TokenKind::SlashEq => "'/='",
            TokenKind::PercentEq => "'%='",
            TokenKind::EqEq => "'=='",
            TokenKind::NotEq => "'!='",
            TokenKind::Lt => "'<'",
            TokenKind::LtEq => "'<='",
            TokenKind::Gt => "'>'",
            TokenKind::GtEq => "'>='",
            TokenKind::AndAnd => "'&&'",
            TokenKind::OrOr => "'||'",
            TokenKind::Bang => "'!'",
            TokenKind::LParen => "'('",
            TokenKind::RParen => "')'",
            TokenKind::LBrace => "'{'",
            TokenKind::RBrace => "'}'",
            TokenKind::LBracket => "'['",
            TokenKind::RBracket => "']'",
            TokenKind::Comma => "','",
            TokenKind::Colon => "':'",
            TokenKind::Semicolon => "';'",
            TokenKind::Arrow => "'->'",
            _ => return None,
        })
    }

    fn literal_name(&self) -> &'static str {
        match self {
            TokenKind::Ident(_) => "identifier",
            TokenKind::IntLit(_) => "integer literal",
            TokenKind::FloatLit(_) => "float literal",
            TokenKind::DoubleLit(_) => "double literal",
            TokenKind::StringLit(_) => "string literal",
            TokenKind::InterpolatedString(_) => "interpolated string",
            TokenKind::Newline => "newline",
            TokenKind::Eof => "end of file",
            _ => "token",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
