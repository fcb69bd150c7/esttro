use crate::ast::*;
use crate::diag::{Diagnostic, Stage, MAX_DIAGNOSTICS};
use crate::lexer::Lexer;
use crate::span::Span;
use crate::token::{StringChunk, Token, TokenKind};

const MAX_PARSE_DEPTH: usize = 256;

#[allow(clippy::result_unit_err)]
pub struct Parser<'a> {
    file: &'a str,
    tokens: Vec<Token>,
    cursor: usize,
    depth: usize,
    diagnostics: Vec<Diagnostic>,
}

#[allow(clippy::result_unit_err)]
impl<'a> Parser<'a> {
    pub fn new(file: &'a str, tokens: Vec<Token>) -> Self {
        Self {
            file,
            tokens,
            cursor: 0,
            depth: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn parse(mut self) -> Result<Program, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        self.skip_terminators();

        let start_span = self.current_span();

        while !self.is_at_end() {
            if self.diagnostics.len() >= MAX_DIAGNOSTICS {
                break;
            }

            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(_) => {
                    self.synchronize();
                }
            }

            self.skip_terminators();
        }

        let end_span = if let Some(last) = statements.last() {
            last.span
        } else {
            start_span
        };

        if self.diagnostics.is_empty() {
            Ok(Program {
                statements,
                span: start_span.merge(end_span),
            })
        } else {
            Err(self.diagnostics)
        }
    }

    fn parse_statement(&mut self) -> Result<Statement, ()> {
        self.check_depth()?;

        match self.peek().kind {
            TokenKind::Let => self.parse_var_stmt(false),
            TokenKind::Const => self.parse_var_stmt(true),
            TokenKind::Function => self.parse_function_stmt(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Break => self.parse_simple_stmt(StatementKind::Break, "break"),
            TokenKind::Continue => self.parse_simple_stmt(StatementKind::Continue, "continue"),
            TokenKind::LBrace => {
                let block = self.parse_block()?;
                let span = block.span;
                Ok(Statement::new(StatementKind::Block(block), span))
            }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_var_stmt(&mut self, is_const: bool) -> Result<Statement, ()> {
        let kw_token = self.advance();
        let kw = if is_const { "const" } else { "let" };
        let name = self.expect_ident(&format!("expected identifier after '{}'", kw))?;

        let ty = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let init = if is_const {
            if !self.match_token(TokenKind::Eq) {
                let span = self.current_span();
                self.emit_error(span, "expected '=' in const declaration");
                return Err(());
            }
            Some(self.parse_expr()?)
        } else if self.match_token(TokenKind::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let term_span = self.expect_terminator(&format!(
            "expected ';' or newline after {} declaration",
            kw
        ))?;
        let span = kw_token.span.merge(term_span);

        let kind = if is_const {
            StatementKind::Const {
                name,
                ty,
                init: init.unwrap(),
            }
        } else {
            StatementKind::Let { name, ty, init }
        };

        Ok(Statement::new(kind, span))
    }

    fn parse_function_stmt(&mut self) -> Result<Statement, ()> {
        let fn_token = self.advance();
        let name = self.expect_ident("expected function name after 'function'")?;

        if !self.match_token(TokenKind::LParen) {
            let span = self.current_span();
            self.emit_error(span, "expected '(' after function name");
            return Err(());
        }

        let params = self.parse_fn_params()?;

        let ret_ty = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.skip_terminators();

        if self.peek().kind != TokenKind::LBrace {
            let span = self.current_span();
            self.emit_error(span, "expected '{' before function body");
            return Err(());
        }

        let body = self.parse_block()?;
        let span = fn_token.span.merge(body.span);

        Ok(Statement::new(
            StatementKind::Function {
                name,
                params,
                ret_ty,
                body,
            },
            span,
        ))
    }

    fn parse_fn_params(&mut self) -> Result<Vec<Param>, ()> {
        let mut params = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                let param_name = self.expect_ident("expected parameter name")?;
                if !self.match_token(TokenKind::Colon) {
                    let span = self.current_span();
                    self.emit_error(span, "expected ':' after parameter name");
                    return Err(());
                }
                let param_ty = self.parse_type()?;
                let param_span = param_ty.span;
                params.push(Param {
                    name: param_name,
                    ty: param_ty,
                    span: param_span,
                });

                if self.match_token(TokenKind::Comma) {
                    if self.peek().kind == TokenKind::RParen {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        if !self.match_token(TokenKind::RParen) {
            let span = self.current_span();
            self.emit_error(span, "expected ')' after parameter list");
            return Err(());
        }

        Ok(params)
    }

    fn parse_if_stmt(&mut self) -> Result<Statement, ()> {
        let if_token = self.advance();
        let cond = self.parse_expr()?;

        self.skip_terminators();
        if self.peek().kind != TokenKind::LBrace {
            let span = self.current_span();
            self.emit_error(span, "expected '{' after if condition");
            return Err(());
        }

        let then_branch = self.parse_block()?;
        let mut end_span = then_branch.span;

        self.skip_terminators();

        let else_branch = if self.match_token(TokenKind::Else) {
            self.skip_terminators();
            if self.peek().kind == TokenKind::If {
                let nested_if = self.parse_if_stmt()?;
                end_span = nested_if.span;
                Some(ElseBranch::If(Box::new(nested_if)))
            } else if self.peek().kind == TokenKind::LBrace {
                let else_block = self.parse_block()?;
                end_span = else_block.span;
                Some(ElseBranch::Block(else_block))
            } else {
                let span = self.current_span();
                self.emit_error(span, "expected '{' or 'if' after 'else'");
                return Err(());
            }
        } else {
            None
        };

        let span = if_token.span.merge(end_span);
        Ok(Statement::new(
            StatementKind::If {
                cond,
                then_branch,
                else_branch,
            },
            span,
        ))
    }

    fn parse_while_stmt(&mut self) -> Result<Statement, ()> {
        let while_token = self.advance();
        let cond = self.parse_expr()?;

        self.skip_terminators();
        if self.peek().kind != TokenKind::LBrace {
            let span = self.current_span();
            self.emit_error(span, "expected '{' after while condition");
            return Err(());
        }

        let body = self.parse_block()?;
        let span = while_token.span.merge(body.span);

        Ok(Statement::new(
            StatementKind::While { cond, body },
            span,
        ))
    }

    fn parse_for_stmt(&mut self) -> Result<Statement, ()> {
        let for_token = self.advance();
        let var = self.expect_ident("expected loop variable after 'for'")?;

        if !self.match_token(TokenKind::Eq) {
            let span = self.current_span();
            self.emit_error(span, "expected '=' after for loop variable");
            return Err(());
        }

        let start = self.parse_expr()?;

        if !self.match_token(TokenKind::Comma) {
            let span = self.current_span();
            self.emit_error(span, "expected ',' after for loop start expression");
            return Err(());
        }

        let end = self.parse_expr()?;

        let step = if self.match_token(TokenKind::Comma) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.skip_terminators();
        if self.peek().kind != TokenKind::LBrace {
            let span = self.current_span();
            self.emit_error(span, "expected '{' after for loop range");
            return Err(());
        }

        let body = self.parse_block()?;
        let span = for_token.span.merge(body.span);

        Ok(Statement::new(
            StatementKind::For {
                var,
                start,
                end,
                step,
                body,
            },
            span,
        ))
    }

    fn parse_return_stmt(&mut self) -> Result<Statement, ()> {
        let ret_token = self.advance();

        let value = if self.is_statement_terminator() {
            None
        } else {
            Some(self.parse_expr()?)
        };

        let term_span = self.expect_terminator("expected ';' or newline after return statement")?;
        let span = ret_token.span.merge(term_span);

        Ok(Statement::new(StatementKind::Return(value), span))
    }

    fn parse_simple_stmt(&mut self, kind: StatementKind, name: &str) -> Result<Statement, ()> {
        let token = self.advance();
        let term_span = self.expect_terminator(&format!("expected ';' or newline after '{}'", name))?;
        let span = token.span.merge(term_span);
        Ok(Statement::new(kind, span))
    }

    fn parse_expr_stmt(&mut self) -> Result<Statement, ()> {
        let expr = self.parse_expr()?;
        let term_span = self.expect_terminator("expected ';' or newline after expression")?;
        let span = expr.span.merge(term_span);
        Ok(Statement::new(StatementKind::Expr(expr), span))
    }

    fn parse_block(&mut self) -> Result<Block, ()> {
        let lbrace = self.advance();
        self.skip_terminators();

        let mut statements = Vec::new();
        while self.peek().kind != TokenKind::RBrace && !self.is_at_end() {
            if self.diagnostics.len() >= MAX_DIAGNOSTICS {
                break;
            }

            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(_) => {
                    self.synchronize();
                }
            }

            self.skip_terminators();
        }

        if !self.match_token(TokenKind::RBrace) {
            let span = self.current_span();
            self.emit_error(span, "expected '}' after block");
            return Err(());
        }

        let rbrace_span = self.previous().span;
        let span = lbrace.span.merge(rbrace_span);

        Ok(Block { statements, span })
    }

    pub fn parse_type(&mut self) -> Result<TypeNode, ()> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::TypeInt => Ok(TypeNode::new(TypeNodeKind::Int, tok.span)),
            TokenKind::TypeFloat => Ok(TypeNode::new(TypeNodeKind::Float, tok.span)),
            TokenKind::TypeDouble => Ok(TypeNode::new(TypeNodeKind::Double, tok.span)),
            TokenKind::TypeBoolean => Ok(TypeNode::new(TypeNodeKind::Boolean, tok.span)),
            TokenKind::TypeString => Ok(TypeNode::new(TypeNodeKind::String, tok.span)),
            TokenKind::TypeVoid => Ok(TypeNode::new(TypeNodeKind::Void, tok.span)),
            TokenKind::Function => self.parse_function_type(&tok),
            _ => {
                self.emit_error(tok.span, "expected type name");
                Err(())
            }
        }
    }

    fn parse_function_type(&mut self, tok: &Token) -> Result<TypeNode, ()> {
        if !self.match_token(TokenKind::LParen) {
            let span = self.current_span();
            self.emit_error(span, "expected '(' in function type");
            return Err(());
        }

        let mut params = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                params.push(self.parse_type()?);
                if self.match_token(TokenKind::Comma) {
                    if self.peek().kind == TokenKind::RParen {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        if !self.match_token(TokenKind::RParen) {
            let span = self.current_span();
            self.emit_error(span, "expected ')' after function type parameter list");
            return Err(());
        }

        if !self.match_token(TokenKind::Arrow) {
            let span = self.current_span();
            self.emit_error(span, "expected '->' in function type");
            return Err(());
        }

        let ret = Box::new(self.parse_type()?);
        let span = tok.span.merge(ret.span);
        Ok(TypeNode::new(TypeNodeKind::Function { params, ret }, span))
    }

    pub fn parse_expr(&mut self) -> Result<Expr, ()> {
        self.parse_precedence(1)
    }

    fn parse_precedence(&mut self, min_bp: u8) -> Result<Expr, ()> {
        self.check_depth()?;
        let mut lhs = self.parse_prefix()?;

        loop {
            let tok = self.peek();
            if let Some(op) = self.as_assign_op(&tok.kind) {
                if 1 < min_bp {
                    break;
                }
                self.advance();
                let rhs = self.parse_precedence(1)?;
                lhs = self.make_assignment(lhs, op, rhs)?;
                continue;
            }

            if tok.kind == TokenKind::LParen {
                if 9 < min_bp {
                    break;
                }
                lhs = self.parse_call_args(lhs)?;
                continue;
            }

            if let Some((l_bp, r_bp, op)) = self.infix_binding_power(&tok.kind) {
                if l_bp < min_bp {
                    break;
                }
                self.advance();
                let rhs = self.parse_precedence(r_bp)?;
                let span = lhs.span.merge(rhs.span);
                lhs = Expr::new(
                    ExprKind::Binary {
                        left: Box::new(lhs),
                        op,
                        right: Box::new(rhs),
                    },
                    span,
                );
                continue;
            }

            break;
        }

        Ok(lhs)
    }

    fn parse_call_args(&mut self, lhs: Expr) -> Result<Expr, ()> {
        self.advance();
        let mut args = Vec::new();
        if self.peek().kind != TokenKind::RParen {
            loop {
                args.push(self.parse_expr()?);
                if self.match_token(TokenKind::Comma) {
                    if self.peek().kind == TokenKind::RParen {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        if !self.match_token(TokenKind::RParen) {
            let span = self.current_span();
            self.emit_error(span, "expected ')' after argument list");
            return Err(());
        }
        let span = lhs.span.merge(self.previous().span);
        Ok(Expr::new(
            ExprKind::Call {
                callee: Box::new(lhs),
                args,
            },
            span,
        ))
    }

    #[allow(clippy::too_many_lines)]
    fn parse_prefix(&mut self) -> Result<Expr, ()> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::IntLit(val) => Ok(Expr::new(ExprKind::IntLit(val), tok.span)),
            TokenKind::FloatLit(val) => Ok(Expr::new(ExprKind::FloatLit(val), tok.span)),
            TokenKind::DoubleLit(val) => Ok(Expr::new(ExprKind::DoubleLit(val), tok.span)),
            TokenKind::StringLit(val) => Ok(Expr::new(ExprKind::StringLit(val), tok.span)),
            TokenKind::InterpolatedString(chunks) => {
                let mut parts = Vec::new();
                for chunk in chunks {
                    match chunk {
                        StringChunk::Literal(lit) => {
                            parts.push(Expr::new(ExprKind::StringLit(lit), tok.span));
                        }
                        StringChunk::Expr(expr_src, expr_span) => {
                            let sub_lexer = Lexer::new(self.file, &expr_src);
                            match sub_lexer.tokenize() {
                                Ok(mut sub_tokens) => {
                                    for t in &mut sub_tokens {
                                        t.span = Span::new(
                                            expr_span.start + t.span.start,
                                            expr_span.start + t.span.end,
                                            expr_span.line + t.span.line - 1,
                                            if t.span.line == 1 {
                                                expr_span.col + t.span.col - 1
                                            } else {
                                                t.span.col
                                            },
                                        );
                                    }
                                    let mut sub_parser = Parser::new(self.file, sub_tokens);
                                    match sub_parser.parse_expr() {
                                        Ok(parsed_e) => parts.push(parsed_e),
                                        Err(_) => {
                                            for d in sub_parser.diagnostics {
                                                if self.diagnostics.len() < MAX_DIAGNOSTICS {
                                                    self.diagnostics.push(d);
                                                }
                                            }
                                            return Err(());
                                        }
                                    }
                                }
                                Err(diags) => {
                                    for d in diags {
                                        if self.diagnostics.len() < MAX_DIAGNOSTICS {
                                            self.diagnostics.push(d);
                                        }
                                    }
                                    return Err(());
                                }
                            }
                        }
                    }
                }
                Ok(Expr::new(ExprKind::InterpolatedString(parts), tok.span))
            }
            TokenKind::True => Ok(Expr::new(ExprKind::BoolLit(true), tok.span)),
            TokenKind::False => Ok(Expr::new(ExprKind::BoolLit(false), tok.span)),
            TokenKind::Ident(name) => Ok(Expr::new(ExprKind::Ident(name), tok.span)),
            TokenKind::Minus => {
                let operand = self.parse_precedence(8)?;
                let span = tok.span.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Neg,
                        expr: Box::new(operand),
                    },
                    span,
                ))
            }
            TokenKind::Plus => {
                let operand = self.parse_precedence(8)?;
                let span = tok.span.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Pos,
                        expr: Box::new(operand),
                    },
                    span,
                ))
            }
            TokenKind::Bang => {
                let operand = self.parse_precedence(8)?;
                let span = tok.span.merge(operand.span);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(operand),
                    },
                    span,
                ))
            }
            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                if !self.match_token(TokenKind::RParen) {
                    let span = self.current_span();
                    self.emit_error(span, "expected ')' after expression");
                    return Err(());
                }
                let span = tok.span.merge(self.previous().span);
                Ok(Expr::new(ExprKind::Group(Box::new(expr)), span))
            }
            TokenKind::Eof => {
                self.emit_error(tok.span, "unexpected end of file");
                Err(())
            }
            _ => {
                self.emit_error(
                    tok.span,
                    format!("unexpected token {}", tok.kind.display_name()),
                );
                Err(())
            }
        }
    }

    fn make_assignment(&mut self, lhs: Expr, op: AssignOp, rhs: Expr) -> Result<Expr, ()> {
        let name = match lhs.kind {
            ExprKind::Ident(ident) => ident,
            _ => {
                self.emit_error(lhs.span, "invalid target for assignment");
                return Err(());
            }
        };
        let span = lhs.span.merge(rhs.span);
        Ok(Expr::new(
            ExprKind::Assign {
                target: name,
                op,
                value: Box::new(rhs),
            },
            span,
        ))
    }

    fn as_assign_op(&self, kind: &TokenKind) -> Option<AssignOp> {
        match kind {
            TokenKind::Eq => Some(AssignOp::Assign),
            TokenKind::PlusEq => Some(AssignOp::AddAssign),
            TokenKind::MinusEq => Some(AssignOp::SubAssign),
            TokenKind::StarEq => Some(AssignOp::MulAssign),
            TokenKind::SlashEq => Some(AssignOp::DivAssign),
            TokenKind::PercentEq => Some(AssignOp::ModAssign),
            _ => None,
        }
    }

    fn infix_binding_power(&self, kind: &TokenKind) -> Option<(u8, u8, BinOp)> {
        match kind {
            TokenKind::OrOr => Some((2, 3, BinOp::Or)),
            TokenKind::AndAnd => Some((3, 4, BinOp::And)),
            TokenKind::EqEq => Some((4, 5, BinOp::Eq)),
            TokenKind::NotEq => Some((4, 5, BinOp::NotEq)),
            TokenKind::Lt => Some((5, 6, BinOp::Lt)),
            TokenKind::LtEq => Some((5, 6, BinOp::LtEq)),
            TokenKind::Gt => Some((5, 6, BinOp::Gt)),
            TokenKind::GtEq => Some((5, 6, BinOp::GtEq)),
            TokenKind::Plus => Some((6, 7, BinOp::Add)),
            TokenKind::Minus => Some((6, 7, BinOp::Sub)),
            TokenKind::Star => Some((7, 8, BinOp::Mul)),
            TokenKind::Slash => Some((7, 8, BinOp::Div)),
            TokenKind::Percent => Some((7, 8, BinOp::Mod)),
            _ => None,
        }
    }

    fn is_statement_terminator(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Semicolon | TokenKind::Newline | TokenKind::RBrace | TokenKind::Eof
        )
    }

    fn expect_terminator(&mut self, err_msg: &str) -> Result<Span, ()> {
        let tok = self.peek();
        match tok.kind {
            TokenKind::Semicolon => {
                let span = tok.span;
                self.advance();
                Ok(span)
            }
            TokenKind::Newline => {
                let span = tok.span;
                self.advance();
                Ok(span)
            }
            TokenKind::RBrace | TokenKind::Eof => Ok(tok.span),
            _ => {
                self.emit_error(tok.span, err_msg);
                Err(())
            }
        }
    }

    fn skip_terminators(&mut self) {
        while matches!(
            self.peek().kind,
            TokenKind::Semicolon | TokenKind::Newline
        ) {
            self.advance();
        }
    }

    fn synchronize(&mut self) {
        while !self.is_at_end() {
            if matches!(
                self.peek().kind,
                TokenKind::Semicolon | TokenKind::Newline
            ) {
                self.advance();
                return;
            }

            match self.peek().kind {
                TokenKind::Let
                | TokenKind::Const
                | TokenKind::Function
                | TokenKind::If
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Return
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::RBrace => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn expect_ident(&mut self, err_msg: &str) -> Result<String, ()> {
        let tok = self.peek();
        if let TokenKind::Ident(name) = &tok.kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            self.emit_error(tok.span, err_msg);
            Err(())
        }
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.peek().kind == kind {
            self.advance();
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            let tok = self.tokens[self.cursor].clone();
            self.cursor += 1;
            tok
        } else {
            self.tokens[self.tokens.len() - 1].clone()
        }
    }

    fn peek(&self) -> &Token {
        if self.cursor < self.tokens.len() {
            &self.tokens[self.cursor]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.cursor - 1]
    }

    fn current_span(&self) -> Span {
        self.peek().span
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn check_depth(&mut self) -> Result<(), ()> {
        self.depth += 1;
        if self.depth > MAX_PARSE_DEPTH {
            let span = self.current_span();
            self.emit_error(span, "parser recursion depth limit exceeded");
            return Err(());
        }
        self.depth -= 1;
        Ok(())
    }

    fn emit_error(&mut self, span: Span, message: impl Into<String>) {
        if self.diagnostics.len() < MAX_DIAGNOSTICS {
            self.diagnostics.push(Diagnostic::new(
                self.file,
                span,
                Stage::Parser,
                message,
            ));
        }
    }
}
