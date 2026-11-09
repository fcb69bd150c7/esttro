use std::collections::HashMap;

use crate::ast::*;
use crate::diag::{Diagnostic, Stage, MAX_DIAGNOSTICS};
use crate::span::Span;
use crate::types::Type;

#[derive(Debug, Clone)]
struct Symbol {
    ty: Type,
    is_const: bool,
    is_initialized: bool,
}

#[derive(Debug, Clone)]
struct Scope {
    symbols: HashMap<String, Symbol>,
    is_loop: bool,
    fn_ret_ty: Option<Type>,
}

impl Scope {
    fn new(is_loop: bool, fn_ret_ty: Option<Type>) -> Self {
        Self {
            symbols: HashMap::new(),
            is_loop,
            fn_ret_ty,
        }
    }
}

pub struct TypeChecker {
    file: String,
    scopes: Vec<Scope>,
    diagnostics: Vec<Diagnostic>,
}

impl TypeChecker {
    pub fn new(file: &str) -> Self {
        let mut checker = Self {
            file: file.to_string(),
            scopes: Vec::new(),
            diagnostics: Vec::new(),
        };
        checker.init_builtins();
        checker
    }

    fn init_builtins(&mut self) {
        let mut global_scope = Scope::new(false, None);

        let print_type = Type::Function {
            params: vec![Type::String],
            ret: Box::new(Type::Void),
        };
        global_scope.symbols.insert(
            "print".to_string(),
            Symbol {
                ty: print_type,
                is_const: true,
                is_initialized: true,
            },
        );
        global_scope.symbols.insert(
            "assert".to_string(),
            Symbol {
                ty: Type::Function {
                    params: vec![Type::Boolean, Type::String],
                    ret: Box::new(Type::Void),
                },
                is_const: true,
                is_initialized: true,
            },
        );

        self.scopes.push(global_scope);
    }

    pub fn check(mut self, program: Program) -> Result<CheckedProgram, Vec<Diagnostic>> {
        for stmt in &program.statements {
            if let StatementKind::Function {
                name,
                params,
                ret_ty,
                ..
            } = &stmt.kind
            {
                let param_types = params.iter().map(|p| self.resolve_type(&p.ty)).collect();
                let ret_type = ret_ty.as_ref().map(|r| self.resolve_type(r)).unwrap_or(Type::Void);
                let fn_type = Type::Function {
                    params: param_types,
                    ret: Box::new(ret_type),
                };

                if self.scopes.last().unwrap().symbols.contains_key(name) {
                    self.emit_error(stmt.span, format!("duplicate definition of function '{}'", name));
                } else {
                    self.scopes.last_mut().unwrap().symbols.insert(
                        name.clone(),
                        Symbol {
                            ty: fn_type,
                            is_const: true,
                            is_initialized: true,
                        },
                    );
                }
            }
        }

        for stmt in &program.statements {
            if self.diagnostics.len() >= MAX_DIAGNOSTICS {
                break;
            }
            self.check_statement(stmt);
        }

        if self.diagnostics.is_empty() {
            Ok(CheckedProgram(program))
        } else {
            Err(self.diagnostics)
        }
    }

    fn check_statement(&mut self, stmt: &Statement) {
        match &stmt.kind {
            StatementKind::Let { name, ty, init } => self.check_let(stmt.span, name, ty, init),
            StatementKind::Const { name, ty, init } => self.check_const(stmt.span, name, ty, init),
            StatementKind::Function {
                params,
                ret_ty,
                body,
                ..
            } => self.check_fn_decl(stmt.span, params, ret_ty, body),
            StatementKind::If {
                cond,
                then_branch,
                else_branch,
            } => self.check_if(cond, then_branch, else_branch),
            StatementKind::While { cond, body } => self.check_while(cond, body),
            StatementKind::For {
                var,
                start,
                end,
                step,
                body,
            } => self.check_for(var, start, end, step, body),
            StatementKind::Return(val) => self.check_return(stmt.span, val),
            StatementKind::Break => self.check_break(stmt.span),
            StatementKind::Continue => self.check_continue(stmt.span),
            StatementKind::Expr(expr) => {
                self.check_expr(expr);
            }
            StatementKind::Block(block) => self.check_scoped_block(block),
        }
    }

    fn check_break(&mut self, span: Span) {
        if !self.is_inside_loop() {
            self.emit_error(span, "'break' outside of loop");
        }
    }

    fn check_continue(&mut self, span: Span) {
        if !self.is_inside_loop() {
            self.emit_error(span, "'continue' outside of loop");
        }
    }

    fn check_scoped_block(&mut self, block: &Block) {
        self.push_scope(false);
        self.check_block(block);
        self.pop_scope();
    }

    fn check_let(
        &mut self,
        span: Span,
        name: &str,
        ty: &Option<TypeNode>,
        init: &Option<Expr>,
    ) {
        let explicit_ty = ty.as_ref().map(|t| self.resolve_type(t));
        let init_ty = init.as_ref().map(|e| (self.check_expr(e), e.span));
        let resolved_ty = self.resolve_let_type(span, explicit_ty, &init_ty);

        let is_init = init_ty.is_some();
        if self.scopes.last().unwrap().symbols.contains_key(name) {
            self.emit_error(span, format!("duplicate definition of variable '{}'", name));
        } else {
            self.scopes.last_mut().unwrap().symbols.insert(
                name.to_string(),
                Symbol {
                    ty: resolved_ty,
                    is_const: false,
                    is_initialized: is_init,
                },
            );
        }
    }

    fn resolve_let_type(
        &mut self,
        span: Span,
        explicit_ty: Option<Type>,
        init_ty: &Option<(Type, Span)>,
    ) -> Type {
        match (explicit_ty, init_ty) {
            (Some(exp), Some((in_ty, in_span))) => {
                if !exp.is_assignable_from(in_ty) {
                    self.emit_error(
                        *in_span,
                        format!("type mismatch: expected '{}', found '{}'", exp, in_ty),
                    );
                    Type::Error
                } else {
                    exp
                }
            }
            (Some(exp), None) => exp,
            (None, Some((in_ty, _))) => {
                if *in_ty == Type::Void {
                    self.emit_error(span, "cannot initialize variable with type 'void'");
                    Type::Error
                } else {
                    in_ty.clone()
                }
            }
            (None, None) => {
                self.emit_error(span, "variable declaration must specify type or initializer");
                Type::Error
            }
        }
    }

    fn check_const(
        &mut self,
        span: Span,
        name: &str,
        ty: &Option<TypeNode>,
        init: &Expr,
    ) {
        let explicit_ty = ty.as_ref().map(|t| self.resolve_type(t));
        let in_ty = self.check_expr(init);

        let resolved_ty = match explicit_ty {
            Some(exp) => {
                if !exp.is_assignable_from(&in_ty) {
                    self.emit_error(
                        init.span,
                        format!("type mismatch: expected '{}', found '{}'", exp, in_ty),
                    );
                    Type::Error
                } else {
                    exp
                }
            }
            None => in_ty,
        };

        if self.scopes.last().unwrap().symbols.contains_key(name) {
            self.emit_error(span, format!("duplicate definition of variable '{}'", name));
        } else {
            self.scopes.last_mut().unwrap().symbols.insert(
                name.to_string(),
                Symbol {
                    ty: resolved_ty,
                    is_const: true,
                    is_initialized: true,
                },
            );
        }
    }

    fn check_fn_decl(
        &mut self,
        span: Span,
        params: &[Param],
        ret_ty: &Option<TypeNode>,
        body: &Block,
    ) {
        let ret_type = ret_ty.as_ref().map(|r| self.resolve_type(r)).unwrap_or(Type::Void);
        let mut param_scope = Scope::new(false, Some(ret_type.clone()));

        for p in params {
            let p_ty = self.resolve_type(&p.ty);
            if param_scope.symbols.contains_key(&p.name) {
                self.emit_error(p.span, format!("duplicate parameter name '{}'", p.name));
            } else {
                param_scope.symbols.insert(
                    p.name.clone(),
                    Symbol {
                        ty: p_ty,
                        is_const: false,
                        is_initialized: true,
                    },
                );
            }
        }

        self.scopes.push(param_scope);
        self.check_block(body);
        self.scopes.pop();

        if ret_type != Type::Void && !self.block_returns_all_paths(body) {
            self.emit_error(span, "missing return statement in non-void function");
        }
    }

    fn check_if(
        &mut self,
        cond: &Expr,
        then_branch: &Block,
        else_branch: &Option<ElseBranch>,
    ) {
        let cond_ty = self.check_expr(cond);
        if cond_ty != Type::Boolean && cond_ty != Type::Error {
            self.emit_error(
                cond.span,
                format!("condition must be of type boolean, found '{}'", cond_ty),
            );
        }

        self.push_scope(false);
        self.check_block(then_branch);
        self.pop_scope();

        match else_branch {
            Some(ElseBranch::If(nested_if)) => self.check_statement(nested_if),
            Some(ElseBranch::Block(b)) => {
                self.push_scope(false);
                self.check_block(b);
                self.pop_scope();
            }
            None => {}
        }
    }

    fn check_while(&mut self, cond: &Expr, body: &Block) {
        let cond_ty = self.check_expr(cond);
        if cond_ty != Type::Boolean && cond_ty != Type::Error {
            self.emit_error(
                cond.span,
                format!("condition must be of type boolean, found '{}'", cond_ty),
            );
        }

        self.push_scope(true);
        self.check_block(body);
        self.pop_scope();
    }

    fn check_for(
        &mut self,
        var: &str,
        start: &Expr,
        end: &Expr,
        step: &Option<Expr>,
        body: &Block,
    ) {
        let start_ty = self.check_expr(start);
        if start_ty != Type::Int && start_ty != Type::Error {
            self.emit_error(
                start.span,
                format!("loop start must be of type int, found '{}'", start_ty),
            );
        }

        let end_ty = self.check_expr(end);
        if end_ty != Type::Int && end_ty != Type::Error {
            self.emit_error(
                end.span,
                format!("loop end must be of type int, found '{}'", end_ty),
            );
        }

        if let Some(st) = step {
            let step_ty = self.check_expr(st);
            if step_ty != Type::Int && step_ty != Type::Error {
                self.emit_error(
                    st.span,
                    format!("loop step must be of type int, found '{}'", step_ty),
                );
            }
        }

        self.push_scope(true);
        self.scopes.last_mut().unwrap().symbols.insert(
            var.to_string(),
            Symbol {
                ty: Type::Int,
                is_const: true,
                is_initialized: true,
            },
        );
        self.check_block(body);
        self.pop_scope();
    }

    fn check_return(&mut self, span: Span, val: &Option<Expr>) {
        let current_ret_ty = self.current_fn_ret_ty();
        let val_ty = val.as_ref().map(|e| (self.check_expr(e), e.span));

        match (current_ret_ty, &val_ty) {
            (None, _) => {
                self.emit_error(span, "'return' outside of function");
            }
            (Some(expected), Some((v_ty, v_span))) => {
                if !expected.is_assignable_from(v_ty) {
                    self.emit_error(
                        *v_span,
                        format!("type mismatch: expected '{}', found '{}'", expected, v_ty),
                    );
                }
            }
            (Some(expected), None) => {
                if expected != Type::Void {
                    self.emit_error(
                        span,
                        format!("type mismatch: expected '{}', found 'void'", expected),
                    );
                }
            }
        }
    }

    fn check_block(&mut self, block: &Block) {
        for s in &block.statements {
            if self.diagnostics.len() >= MAX_DIAGNOSTICS {
                break;
            }
            self.check_statement(s);
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Type {
        let span = expr.span;
        match &expr.kind {
            ExprKind::IntLit(_) => Type::Int,
            ExprKind::FloatLit(_) => Type::Float,
            ExprKind::DoubleLit(_) => Type::Double,
            ExprKind::BoolLit(_) => Type::Boolean,
            ExprKind::StringLit(_) => Type::String,
            ExprKind::InterpolatedString(parts) => {
                for p in parts {
                    self.check_expr(p);
                }
                Type::String
            }
            ExprKind::Ident(name) => self.check_ident(name, span),
            ExprKind::Group(inner) => self.check_expr(inner),
            ExprKind::Unary { op, expr } => self.check_unary(*op, expr, span),
            ExprKind::Binary { left, op, right } => self.check_binary(left, *op, right, span),
            ExprKind::Assign { target, op, value } => self.check_assign(target, *op, value, span),
            ExprKind::Call { callee, args } => self.check_call(callee, args, span),
        }
    }

    fn check_ident(&mut self, name: &str, span: Span) -> Type {
        if let Some(sym) = self.lookup_symbol(name) {
            if !sym.is_initialized {
                self.emit_error(span, format!("variable '{}' used before initialization", name));
                Type::Error
            } else {
                sym.ty.clone()
            }
        } else {
            self.emit_error(span, format!("undefined variable '{}'", name));
            Type::Error
        }
    }

    fn check_unary(&mut self, op: UnaryOp, expr: &Expr, span: Span) -> Type {
        let inner_ty = self.check_expr(expr);
        match op {
            UnaryOp::Neg | UnaryOp::Pos => {
                if !inner_ty.is_numeric() && !inner_ty.is_error() {
                    self.emit_error(
                        span,
                        format!("unary operator requires numeric type, found '{}'", inner_ty),
                    );
                    Type::Error
                } else {
                    inner_ty
                }
            }
            UnaryOp::Not => {
                if inner_ty != Type::Boolean && !inner_ty.is_error() {
                    self.emit_error(
                        span,
                        format!("unary '!' requires boolean type, found '{}'", inner_ty),
                    );
                    Type::Error
                } else {
                    Type::Boolean
                }
            }
        }
    }

    fn check_binary(&mut self, left: &Expr, op: BinOp, right: &Expr, span: Span) -> Type {
        let l = self.check_expr(left);
        let r = self.check_expr(right);

        match op {
            BinOp::Add => self.check_binary_add(&l, &r, span),
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                self.check_binary_arith(&l, op, &r, span)
            }
            BinOp::Eq | BinOp::NotEq => self.check_binary_eq(&l, &r, span),
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                self.check_binary_rel(&l, &r, span)
            }
            BinOp::And | BinOp::Or => self.check_binary_logical(&l, left.span, &r, right.span),
        }
    }

    fn check_binary_add(&mut self, l: &Type, r: &Type, span: Span) -> Type {
        if *l == Type::String || *r == Type::String {
            self.emit_error(
                span,
                "string concatenation with '+' is not supported; use string interpolation instead",
            );
            Type::Error
        } else if l.is_numeric() && r.is_numeric() {
            if l == r {
                l.clone()
            } else {
                self.emit_error(
                    span,
                    format!("mismatched types in addition: '{}' and '{}'", l, r),
                );
                Type::Error
            }
        } else if l.is_error() || r.is_error() {
            Type::Error
        } else {
            self.emit_error(span, format!("cannot add '{}' and '{}'", l, r));
            Type::Error
        }
    }

    fn check_binary_arith(&mut self, l: &Type, op: BinOp, r: &Type, span: Span) -> Type {
        if l.is_numeric() && r.is_numeric() {
            if l == r {
                if op == BinOp::Mod && !l.is_integer() {
                    self.emit_error(span, "modulo operator '%' is only valid on integers");
                    Type::Error
                } else {
                    l.clone()
                }
            } else {
                self.emit_error(
                    span,
                    format!("mismatched types in arithmetic: '{}' and '{}'", l, r),
                );
                Type::Error
            }
        } else if l.is_error() || r.is_error() {
            Type::Error
        } else {
            self.emit_error(
                span,
                format!("arithmetic requires numeric operands, found '{}' and '{}'", l, r),
            );
            Type::Error
        }
    }

    fn check_binary_eq(&mut self, l: &Type, r: &Type, span: Span) -> Type {
        if l != r && !l.is_error() && !r.is_error() {
            self.emit_error(
                span,
                format!("cannot compare equality between '{}' and '{}'", l, r),
            );
        }
        Type::Boolean
    }

    fn check_binary_rel(&mut self, l: &Type, r: &Type, span: Span) -> Type {
        if l.is_numeric() && r.is_numeric() {
            if l != r {
                self.emit_error(
                    span,
                    format!("mismatched types in comparison: '{}' and '{}'", l, r),
                );
            }
        } else if !l.is_error() && !r.is_error() {
            self.emit_error(
                span,
                format!("relational operators require numeric operands, found '{}' and '{}'", l, r),
            );
        }
        Type::Boolean
    }

    fn check_binary_logical(
        &mut self,
        l: &Type,
        left_span: Span,
        r: &Type,
        right_span: Span,
    ) -> Type {
        if *l != Type::Boolean && !l.is_error() {
            self.emit_error(left_span, format!("expected boolean operand, found '{}'", l));
        }
        if *r != Type::Boolean && !r.is_error() {
            self.emit_error(right_span, format!("expected boolean operand, found '{}'", r));
        }
        Type::Boolean
    }

    fn check_assign(
        &mut self,
        target: &str,
        op: AssignOp,
        value: &Expr,
        span: Span,
    ) -> Type {
        let val_ty = self.check_expr(value);
        let s = match self.lookup_symbol(target).cloned() {
            Some(s) => s,
            None => {
                self.emit_error(span, format!("undefined variable '{}'", target));
                return Type::Error;
            }
        };

        if s.is_const {
            self.emit_error(span, format!("cannot assign to immutable binding '{}'", target));
            return Type::Error;
        }

        match op {
            AssignOp::Assign => self.check_assign_direct(target, &s.ty, &val_ty, value.span),
            _ => self.check_assign_compound(op, &s.ty, &val_ty, span),
        }
    }

    fn check_assign_direct(
        &mut self,
        target: &str,
        target_ty: &Type,
        val_ty: &Type,
        val_span: Span,
    ) -> Type {
        if !target_ty.is_assignable_from(val_ty) {
            self.emit_error(
                val_span,
                format!("type mismatch: expected '{}', found '{}'", target_ty, val_ty),
            );
            Type::Error
        } else {
            self.mark_initialized(target);
            target_ty.clone()
        }
    }

    fn check_assign_compound(
        &mut self,
        op: AssignOp,
        target_ty: &Type,
        val_ty: &Type,
        span: Span,
    ) -> Type {
        match op {
            AssignOp::AddAssign => self.check_assign_add(target_ty, val_ty, span),
            AssignOp::SubAssign | AssignOp::MulAssign | AssignOp::DivAssign => {
                if target_ty.is_numeric() && val_ty == target_ty {
                    target_ty.clone()
                } else {
                    self.emit_error(
                        span,
                        format!("invalid types for compound assignment: '{}' and '{}'", target_ty, val_ty),
                    );
                    Type::Error
                }
            }
            AssignOp::ModAssign => {
                if target_ty.is_integer() && val_ty == target_ty {
                    target_ty.clone()
                } else {
                    self.emit_error(
                        span,
                        format!("invalid types for '%=': '{}' and '{}'", target_ty, val_ty),
                    );
                    Type::Error
                }
            }
            AssignOp::Assign => unreachable!(),
        }
    }

    fn check_assign_add(&mut self, target_ty: &Type, val_ty: &Type, span: Span) -> Type {
        if *target_ty == Type::String || *val_ty == Type::String {
            self.emit_error(
                span,
                "string concatenation with '+=' is not supported; use string interpolation instead",
            );
            Type::Error
        } else if target_ty.is_numeric() && val_ty == target_ty {
            target_ty.clone()
        } else {
            self.emit_error(
                span,
                format!("invalid types for '+=': '{}' and '{}'", target_ty, val_ty),
            );
            Type::Error
        }
    }

    fn check_call(&mut self, callee: &Expr, args: &[Expr], span: Span) -> Type {
        if let ExprKind::Ident(ref name) = callee.kind {
            if let Some(cast_ty) = self.check_cast_builtin(name, args, span) {
                for a in args {
                    self.check_expr(a);
                }
                return cast_ty;
            }

            if name == "print" {
                for a in args {
                    self.check_expr(a);
                }
                return Type::Void;
            }
        }

        let callee_ty = self.check_expr(callee);
        let arg_types: Vec<(Type, Span)> = args.iter().map(|a| (self.check_expr(a), a.span)).collect();
        self.check_call_sig(&callee_ty, &arg_types, callee.span, span)
    }

    fn check_call_sig(
        &mut self,
        callee_ty: &Type,
        arg_types: &[(Type, Span)],
        callee_span: Span,
        call_span: Span,
    ) -> Type {
        match callee_ty {
            Type::Function { params, ret } => {
                if params.len() != arg_types.len() {
                    self.emit_error(
                        call_span,
                        format!("expected {} arguments, found {}", params.len(), arg_types.len()),
                    );
                } else {
                    for (idx, (p_ty, (a_ty, a_span))) in params.iter().zip(arg_types).enumerate() {
                        if !p_ty.is_assignable_from(a_ty) {
                            self.emit_error(
                                *a_span,
                                format!("type mismatch for argument {}: expected '{}', found '{}'", idx + 1, p_ty, a_ty),
                            );
                        }
                    }
                }
                *ret.clone()
            }
            Type::Error => Type::Error,
            other => {
                self.emit_error(callee_span, format!("cannot call non-function type '{}'", other));
                Type::Error
            }
        }
    }

    fn check_cast_builtin(&mut self, name: &str, args: &[Expr], span: Span) -> Option<Type> {
        let target_ty = match name {
            "toInt" => Type::Int,
            "toFloat" => Type::Float,
            "toDouble" => Type::Double,
            "toString" => Type::String,
            "toBoolean" => Type::Boolean,
            _ => return None,
        };

        if args.len() != 1 {
            self.emit_error(span, format!("expected 1 argument for '{}', found {}", name, args.len()));
        }

        Some(target_ty)
    }

    fn resolve_type(&mut self, node: &TypeNode) -> Type {
        match &node.kind {
            TypeNodeKind::Int => Type::Int,
            TypeNodeKind::Float => Type::Float,
            TypeNodeKind::Double => Type::Double,
            TypeNodeKind::Boolean => Type::Boolean,
            TypeNodeKind::String => Type::String,
            TypeNodeKind::Void => Type::Void,
            TypeNodeKind::Function { params, ret } => {
                let p_types = params.iter().map(|p| self.resolve_type(p)).collect();
                let r_type = Box::new(self.resolve_type(ret));
                Type::Function {
                    params: p_types,
                    ret: r_type,
                }
            }
        }
    }

    fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.symbols.get(name) {
                return Some(sym);
            }
        }
        None
    }

    fn mark_initialized(&mut self, name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(sym) = scope.symbols.get_mut(name) {
                sym.is_initialized = true;
                return;
            }
        }
    }

    fn push_scope(&mut self, is_loop: bool) {
        let current_ret = self.current_fn_ret_ty();
        self.scopes.push(Scope::new(is_loop || self.is_inside_loop(), current_ret));
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn is_inside_loop(&self) -> bool {
        self.scopes.iter().any(|s| s.is_loop)
    }

    fn current_fn_ret_ty(&self) -> Option<Type> {
        for s in self.scopes.iter().rev() {
            if s.fn_ret_ty.is_some() {
                return s.fn_ret_ty.clone();
            }
        }
        None
    }

    fn block_returns_all_paths(&self, block: &Block) -> bool {
        for stmt in &block.statements {
            if self.stmt_returns_all_paths(stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_returns_all_paths(&self, stmt: &Statement) -> bool {
        match &stmt.kind {
            StatementKind::Return(_) => true,
            StatementKind::Block(b) => self.block_returns_all_paths(b),
            StatementKind::If {
                then_branch,
                else_branch: Some(else_b),
                ..
            } => {
                let then_ret = self.block_returns_all_paths(then_branch);
                let else_ret = match else_b {
                    ElseBranch::Block(b) => self.block_returns_all_paths(b),
                    ElseBranch::If(nested) => self.stmt_returns_all_paths(nested),
                };
                then_ret && else_ret
            }
            _ => false,
        }
    }

    fn emit_error(&mut self, span: Span, message: impl Into<String>) {
        if self.diagnostics.len() < MAX_DIAGNOSTICS {
            self.diagnostics.push(Diagnostic::new(
                &self.file,
                span,
                Stage::Typecheck,
                message,
            ));
        }
    }
}
