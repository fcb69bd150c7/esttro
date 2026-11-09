use std::fmt;

use crate::span::Span;

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_COMPILE_ERROR: i32 = 1;
pub const EXIT_RUNTIME_ERROR: i32 = 2;
pub const EXIT_INTERNAL_ERROR: i32 = 3;
pub const EXIT_IO_ERROR: i32 = 4;

pub const MAX_DIAGNOSTICS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Lexer,
    Parser,
    Typecheck,
    Runtime,
    Internal,
    Io,
}

impl Stage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Stage::Lexer => "lexer",
            Stage::Parser => "parser",
            Stage::Typecheck => "typecheck",
            Stage::Runtime => "runtime",
            Stage::Internal => "internal",
            Stage::Io => "io",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub file: String,
    pub span: Span,
    pub stage: Stage,
    pub message: String,
}

impl Diagnostic {
    pub fn new(file: &str, span: Span, stage: Stage, message: impl Into<String>) -> Self {
        Self {
            file: file.to_string(),
            span,
            stage,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {} error: {}",
            self.file,
            self.span.line,
            self.span.col,
            self.stage.as_str(),
            self.message
        )
    }
}

