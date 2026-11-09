use crate::diag::{Diagnostic, Stage, MAX_DIAGNOSTICS};
use crate::span::Span;
use crate::token::{StringChunk, Token, TokenKind};

const MAX_SOURCE_SIZE: usize = 10 * 1024 * 1024;
const MAX_IDENT_LENGTH: usize = 128;
const MAX_STRING_LENGTH: usize = 65536;
const MAX_COMMENT_DEPTH: usize = 32;

pub struct Lexer<'a> {
    file: &'a str,
    source: &'a str,
    chars: Vec<(usize, char)>,
    cursor: usize,
    line: usize,
    col: usize,
    bracket_depth: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    pub fn new(file: &'a str, source: &'a str) -> Self {
        let chars: Vec<(usize, char)> = source.char_indices().collect();
        Self {
            file,
            source,
            chars,
            cursor: 0,
            line: 1,
            col: 1,
            bracket_depth: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, Vec<Diagnostic>> {
        if self.source.len() > MAX_SOURCE_SIZE {
            return Err(vec![Diagnostic::new(
                self.file,
                Span::new(0, 0, 1, 1),
                Stage::Lexer,
                "source file exceeds maximum size of 10MB",
            )]);
        }

        let mut tokens: Vec<Token> = Vec::new();
        while !self.is_at_end() {
            if self.diagnostics.len() >= MAX_DIAGNOSTICS {
                break;
            }
            self.lex_token(&mut tokens);
        }

        let eof_byte = self.source.len();
        tokens.push(Token::new(
            TokenKind::Eof,
            Span::new(eof_byte, eof_byte, self.line, self.col),
        ));

        if self.diagnostics.is_empty() {
            Ok(tokens)
        } else {
            Err(self.diagnostics)
        }
    }

    #[allow(clippy::too_many_lines)]
    fn lex_token(&mut self, tokens: &mut Vec<Token>) {
        let start_cursor = self.cursor;
        let start_byte = self.current_byte_pos();
        let start_line = self.line;
        let start_col = self.col;
        let start = (start_byte, start_line, start_col);

        let ch = self.advance();

        match ch {
            ' ' | '\t' | '\r' => {}
            '\n' => self.lex_newline(tokens, start),
            '/' => self.lex_slash(tokens, start),
            '-' => self.lex_minus(tokens, start),
            '+' => self.push_eq_op(tokens, TokenKind::PlusEq, TokenKind::Plus, start),
            '*' => self.push_eq_op(tokens, TokenKind::StarEq, TokenKind::Star, start),
            '%' => self.push_eq_op(tokens, TokenKind::PercentEq, TokenKind::Percent, start),
            '=' => self.push_eq_op(tokens, TokenKind::EqEq, TokenKind::Eq, start),
            '!' => self.push_eq_op(tokens, TokenKind::NotEq, TokenKind::Bang, start),
            '<' => self.push_eq_op(tokens, TokenKind::LtEq, TokenKind::Lt, start),
            '>' => self.push_eq_op(tokens, TokenKind::GtEq, TokenKind::Gt, start),
            '&' => self.lex_and(tokens, start),
            '|' => self.lex_or(tokens, start),
            '(' => {
                self.bracket_depth += 1;
                self.push_op(tokens, TokenKind::LParen, start);
            }
            ')' => {
                if self.bracket_depth > 0 {
                    self.bracket_depth -= 1;
                }
                self.push_op(tokens, TokenKind::RParen, start);
            }
            '{' => self.push_op(tokens, TokenKind::LBrace, start),
            '}' => self.push_op(tokens, TokenKind::RBrace, start),
            '[' => {
                self.bracket_depth += 1;
                self.push_op(tokens, TokenKind::LBracket, start);
            }
            ']' => {
                if self.bracket_depth > 0 {
                    self.bracket_depth -= 1;
                }
                self.push_op(tokens, TokenKind::RBracket, start);
            }
            ',' => self.push_op(tokens, TokenKind::Comma, start),
            ':' => self.push_op(tokens, TokenKind::Colon, start),
            ';' => self.push_op(tokens, TokenKind::Semicolon, start),
            '.' => {
                self.emit_error(
                    Span::new(start_byte, start_byte + 1, start_line, start_col),
                    "unexpected character '.'",
                );
            }
            '"' => {
                if let Some(tok) = self.lex_string(start_byte, start_line, start_col) {
                    tokens.push(tok);
                }
            }
            c if c.is_ascii_digit() => {
                self.cursor = start_cursor;
                self.col = start_col;
                if let Some(tok) = self.lex_number(start_byte, start_line, start_col) {
                    tokens.push(tok);
                }
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                self.cursor = start_cursor;
                self.col = start_col;
                if let Some(tok) = self.lex_identifier(start_byte, start_line, start_col) {
                    tokens.push(tok);
                }
            }
            other => {
                let span = Span::new(start_byte, start_byte + other.len_utf8(), start_line, start_col);
                self.emit_error(span, format!("unexpected character '{}'", other));
            }
        }
    }

    fn lex_newline(&mut self, tokens: &mut Vec<Token>, start: (usize, usize, usize)) {
        self.line += 1;
        self.col = 1;
        if self.bracket_depth == 0 {
            if let Some(last) = tokens.last() {
                if last.kind.is_statement_ender() {
                    tokens.push(Token::new(
                        TokenKind::Newline,
                        Span::new(start.0, start.0 + 1, start.1, start.2),
                    ));
                }
            }
        }
    }

    fn lex_slash(&mut self, tokens: &mut Vec<Token>, start: (usize, usize, usize)) {
        if self.peek() == Some('/') {
            while !self.is_at_end() && self.peek() != Some('\n') {
                self.advance();
            }
        } else if self.peek() == Some('*') {
            self.advance();
            self.skip_block_comment(start.0, start.1, start.2);
        } else {
            self.push_eq_op(tokens, TokenKind::SlashEq, TokenKind::Slash, start);
        }
    }

    fn lex_minus(&mut self, tokens: &mut Vec<Token>, start: (usize, usize, usize)) {
        if self.peek() == Some('-') {
            while !self.is_at_end() && self.peek() != Some('\n') {
                self.advance();
            }
        } else {
            let kind = if self.match_char('>') {
                TokenKind::Arrow
            } else if self.match_char('=') {
                TokenKind::MinusEq
            } else {
                TokenKind::Minus
            };
            self.push_op(tokens, kind, start);
        }
    }

    fn lex_and(&mut self, tokens: &mut Vec<Token>, start: (usize, usize, usize)) {
        if self.match_char('&') {
            self.push_op(tokens, TokenKind::AndAnd, start);
        } else {
            self.emit_error(
                Span::new(start.0, start.0 + 1, start.1, start.2),
                "unexpected character '&', expected '&&'",
            );
        }
    }

    fn lex_or(&mut self, tokens: &mut Vec<Token>, start: (usize, usize, usize)) {
        if self.match_char('|') {
            self.push_op(tokens, TokenKind::OrOr, start);
        } else {
            self.emit_error(
                Span::new(start.0, start.0 + 1, start.1, start.2),
                "unexpected character '|', expected '||'",
            );
        }
    }

    fn skip_block_comment(&mut self, start_byte: usize, start_line: usize, start_col: usize) {
        let mut depth = 1usize;
        while !self.is_at_end() && depth > 0 {
            if self.peek() == Some('/') && self.peek_next() == Some('*') {
                self.advance();
                self.advance();
                depth += 1;
                if depth > MAX_COMMENT_DEPTH {
                    let end_byte = self.current_byte_pos();
                    self.emit_error(
                        Span::new(start_byte, end_byte, start_line, start_col),
                        "comment nesting limit exceeded",
                    );
                    return;
                }
            } else if self.peek() == Some('*') && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                depth -= 1;
            } else {
                let ch = self.advance();
                if ch == '\n' {
                    self.line += 1;
                    self.col = 1;
                }
            }
        }

        if depth > 0 {
            let end_byte = self.current_byte_pos();
            self.emit_error(
                Span::new(start_byte, end_byte, start_line, start_col),
                "unterminated block comment",
            );
        }
    }

    fn lex_string(&mut self, start_byte: usize, start_line: usize, start_col: usize) -> Option<Token> {
        let mut chunks: Vec<StringChunk> = Vec::new();
        let mut current_literal = String::new();

        while !self.is_at_end() {
            let ch = self.advance();
            match ch {
                '"' => return self.finish_string(chunks, current_literal, start_byte, start_line, start_col),
                '{' => {
                    self.lex_interp_brace(&mut chunks, &mut current_literal, start_byte, start_line, start_col)?;
                }
                '\n' => {
                    self.line += 1;
                    self.col = 1;
                    let end_byte = self.current_byte_pos();
                    self.emit_error(
                        Span::new(start_byte, end_byte, start_line, start_col),
                        "unterminated string literal",
                    );
                    return None;
                }
                '\\' => {
                    let esc = self.lex_escape()?;
                    current_literal.push(esc);
                }
                other => {
                    self.append_string_char(&mut current_literal, other, start_byte, start_line, start_col)?;
                }
            }
        }

        let end_byte = self.current_byte_pos();
        self.emit_error(
            Span::new(start_byte, end_byte, start_line, start_col),
            "unterminated string literal",
        );
        None
    }

    fn lex_interp_brace(
        &mut self,
        chunks: &mut Vec<StringChunk>,
        current_literal: &mut String,
        start_byte: usize,
        start_line: usize,
        start_col: usize,
    ) -> Option<()> {
        if !current_literal.is_empty() {
            chunks.push(StringChunk::Literal(std::mem::take(current_literal)));
        }
        let chunk = self.lex_interp_expr(start_byte, start_line, start_col)?;
        chunks.push(chunk);
        Some(())
    }

    fn append_string_char(
        &mut self,
        current_literal: &mut String,
        ch: char,
        start_byte: usize,
        start_line: usize,
        start_col: usize,
    ) -> Option<()> {
        current_literal.push(ch);
        if current_literal.len() > MAX_STRING_LENGTH {
            let end_byte = self.current_byte_pos();
            self.emit_error(
                Span::new(start_byte, end_byte, start_line, start_col),
                "string literal exceeds maximum allowed size",
            );
            None
        } else {
            Some(())
        }
    }

    fn finish_string(
        &self,
        mut chunks: Vec<StringChunk>,
        current_literal: String,
        start_byte: usize,
        start_line: usize,
        start_col: usize,
    ) -> Option<Token> {
        let end_byte = self.current_byte_pos();
        let span = Span::new(start_byte, end_byte, start_line, start_col);
        if chunks.is_empty() {
            Some(Token::new(TokenKind::StringLit(current_literal), span))
        } else {
            if !current_literal.is_empty() {
                chunks.push(StringChunk::Literal(current_literal));
            }
            Some(Token::new(TokenKind::InterpolatedString(chunks), span))
        }
    }

    fn lex_interp_expr(
        &mut self,
        start_byte: usize,
        start_line: usize,
        start_col: usize,
    ) -> Option<StringChunk> {
        let expr_start_byte = self.current_byte_pos();
        let expr_start_line = self.line;
        let expr_start_col = self.col;
        let mut brace_depth = 1usize;
        let mut expr_str = String::new();

        while !self.is_at_end() && brace_depth > 0 {
            let cur = self.peek().unwrap();
            if cur == '{' {
                brace_depth += 1;
                expr_str.push(self.advance());
            } else if cur == '}' {
                brace_depth -= 1;
                if brace_depth == 0 {
                    break;
                }
                expr_str.push(self.advance());
            } else if cur == '"' {
                self.scan_nested_string(&mut expr_str);
            } else if cur == '\n' {
                self.line += 1;
                self.col = 1;
                expr_str.push(self.advance());
            } else {
                expr_str.push(self.advance());
            }
        }

        if brace_depth > 0 {
            let end_byte = self.current_byte_pos();
            self.emit_error(
                Span::new(start_byte, end_byte, start_line, start_col),
                "unterminated expression in string interpolation",
            );
            return None;
        }

        let expr_end_byte = self.current_byte_pos();
        self.advance();
        let expr_span = Span::new(expr_start_byte, expr_end_byte, expr_start_line, expr_start_col);
        Some(StringChunk::Expr(expr_str, expr_span))
    }

    fn scan_nested_string(&mut self, expr_str: &mut String) {
        expr_str.push(self.advance());
        while !self.is_at_end() && self.peek() != Some('"') {
            if self.peek() == Some('\\') {
                expr_str.push(self.advance());
            }
            if !self.is_at_end() {
                expr_str.push(self.advance());
            }
        }
        if !self.is_at_end() {
            expr_str.push(self.advance());
        }
    }

    fn lex_escape(&mut self) -> Option<char> {
        if self.is_at_end() {
            let end_byte = self.current_byte_pos();
            self.emit_error(
                Span::new(end_byte - 1, end_byte, self.line, self.col),
                "unterminated string escape sequence",
            );
            return None;
        }
        let esc = self.advance();
        match esc {
            'n' => Some('\n'),
            'r' => Some('\r'),
            't' => Some('\t'),
            '\\' => Some('\\'),
            '"' => Some('"'),
            '{' => Some('{'),
            '}' => Some('}'),
            '0' => Some('\0'),
            other => {
                let esc_byte = self.current_byte_pos();
                self.emit_error(
                    Span::new(esc_byte - 2, esc_byte, self.line, self.col - 2),
                    format!("unknown escape sequence '\\{}'", other),
                );
                None
            }
        }
    }

    fn lex_number(&mut self, start_byte: usize, start_line: usize, start_col: usize) -> Option<Token> {
        let mut digits = self.scan_digits();
        if self.has_fraction_dot() {
            self.lex_float(&mut digits, start_byte, start_line, start_col)
        } else {
            let end_byte = self.current_byte_pos();
            let span = Span::new(start_byte, end_byte, start_line, start_col);
            match digits.parse::<i64>() {
                Ok(val) => Some(Token::new(TokenKind::IntLit(val), span)),
                Err(_) => {
                    self.emit_error(span, "integer literal out of range");
                    None
                }
            }
        }
    }

    fn scan_digits(&mut self) -> String {
        let mut digits = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                digits.push(self.advance());
            } else {
                break;
            }
        }
        digits
    }

    fn has_fraction_dot(&self) -> bool {
        if self.peek() == Some('.') {
            self.peek_next().map(|c| c.is_ascii_digit()).unwrap_or(false)
        } else {
            false
        }
    }

    fn lex_float(
        &mut self,
        digits: &mut String,
        start_byte: usize,
        start_line: usize,
        start_col: usize,
    ) -> Option<Token> {
        digits.push(self.advance());
        digits.push_str(&self.scan_digits());

        if self.peek() == Some('e') || self.peek() == Some('E') {
            digits.push(self.advance());
            if self.peek() == Some('+') || self.peek() == Some('-') {
                digits.push(self.advance());
            }
            digits.push_str(&self.scan_digits());
        }

        let has_f_suffix = if self.peek() == Some('f') || self.peek() == Some('F') {
            self.advance();
            true
        } else {
            false
        };

        let end_byte = self.current_byte_pos();
        let span = Span::new(start_byte, end_byte, start_line, start_col);

        if has_f_suffix {
            match digits.parse::<f32>() {
                Ok(val) => Some(Token::new(TokenKind::FloatLit(val), span)),
                Err(_) => {
                    self.emit_error(span, "invalid float literal");
                    None
                }
            }
        } else {
            match digits.parse::<f64>() {
                Ok(val) => Some(Token::new(TokenKind::DoubleLit(val), span)),
                Err(_) => {
                    self.emit_error(span, "invalid double literal");
                    None
                }
            }
        }
    }

    fn lex_identifier(&mut self, start_byte: usize, start_line: usize, start_col: usize) -> Option<Token> {
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                text.push(self.advance());
                if text.len() > MAX_IDENT_LENGTH {
                    let end_byte = self.current_byte_pos();
                    self.emit_error(
                        Span::new(start_byte, end_byte, start_line, start_col),
                        "identifier exceeds maximum length of 128 characters",
                    );
                    return None;
                }
            } else {
                break;
            }
        }

        let end_byte = self.current_byte_pos();
        let span = Span::new(start_byte, end_byte, start_line, start_col);

        let kind = match text.as_str() {
            "function" => TokenKind::Function,
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "int" => TokenKind::TypeInt,
            "float" => TokenKind::TypeFloat,
            "double" => TokenKind::TypeDouble,
            "boolean" => TokenKind::TypeBoolean,
            "string" => TokenKind::TypeString,
            "void" => TokenKind::TypeVoid,
            _ => TokenKind::Ident(text),
        };

        Some(Token::new(kind, span))
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.chars.len()
    }

    fn advance(&mut self) -> char {
        let (_, ch) = self.chars[self.cursor];
        self.cursor += 1;
        self.col += 1;
        ch
    }

    fn peek(&self) -> Option<char> {
        if self.cursor < self.chars.len() {
            Some(self.chars[self.cursor].1)
        } else {
            None
        }
    }

    fn peek_next(&self) -> Option<char> {
        if self.cursor + 1 < self.chars.len() {
            Some(self.chars[self.cursor + 1].1)
        } else {
            None
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn current_byte_pos(&self) -> usize {
        if self.cursor < self.chars.len() {
            self.chars[self.cursor].0
        } else {
            self.source.len()
        }
    }

    fn make_span(&self, start_byte: usize, start_line: usize, start_col: usize) -> Span {
        let end_byte = self.current_byte_pos();
        Span::new(start_byte, end_byte, start_line, start_col)
    }

    fn push_op(
        &self,
        tokens: &mut Vec<Token>,
        kind: TokenKind,
        start: (usize, usize, usize),
    ) {
        tokens.push(Token::new(kind, self.make_span(start.0, start.1, start.2)));
    }

    fn push_eq_op(
        &mut self,
        tokens: &mut Vec<Token>,
        if_eq: TokenKind,
        if_not: TokenKind,
        start: (usize, usize, usize),
    ) {
        let kind = if self.match_char('=') { if_eq } else { if_not };
        self.push_op(tokens, kind, start);
    }

    fn emit_error(&mut self, span: Span, message: impl Into<String>) {
        if self.diagnostics.len() < MAX_DIAGNOSTICS {
            self.diagnostics.push(Diagnostic::new(
                self.file,
                span,
                Stage::Lexer,
                message,
            ));
        }
    }
}
