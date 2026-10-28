use std::io::{self, Write};

use crate::ast::*;
use crate::diag::{Diagnostic, Stage};
use crate::env::Environment;
use crate::span::Span;
use crate::value::Value;

const MAX_CALL_DEPTH: usize = 128;

enum Signal {
    None,
    Return(Value),
    Break,
    Continue,
}

pub struct Interpreter {
    file: String,
    env: Environment,
    call_depth: usize,
}

impl Interpreter {
    pub fn new(file: &str) -> Self {
        let mut interp = Self {
            file: file.to_string(),
            env: Environment::new(),
            call_depth: 0,
        };
        interp.init_builtins();
        interp
    }

    fn init_builtins(&mut self) {
        let builtins = [
            "print",
            "assert",
            "toInt",
            "toFloat",
            "toDouble",
            "toString",
            "toBoolean",
        ];
        for b in builtins {
            self.env.define(b.to_string(), Value::Builtin(b.to_string()));
        }
    }

    pub fn run(&mut self, program: CheckedProgram) -> Result<Value, Diagnostic> {
        let program = program.0;
        for stmt in &program.statements {
            if let StatementKind::Function {
                name,
                params,
                body,
                ..
            } = &stmt.kind
            {
                let param_names = params.iter().map(|p| p.name.clone()).collect();
                let fn_val = Value::Function {
                    name: name.clone(),
                    params: param_names,
                    body: body.clone(),
                };
                self.env.define(name.clone(), fn_val);
            }
        }

        let last_val = Value::Void;
        for stmt in program.statements {
            if matches!(stmt.kind, StatementKind::Function { .. }) {
                continue;
            }
            match self.execute_statement(&stmt)? {
                Signal::None => {}
                Signal::Return(val) => return Ok(val),
                Signal::Break => {
                    return Err(self.runtime_error(stmt.span, "unexpected break outside loop"));
                }
                Signal::Continue => {
                    return Err(self.runtime_error(stmt.span, "unexpected continue outside loop"));
                }
            }
        }
        Ok(last_val)
    }

    fn execute_statement(&mut self, stmt: &Statement) -> Result<Signal, Diagnostic> {
        match &stmt.kind {
            StatementKind::Let { name, init, .. } => self.exec_let(name, init.as_ref()),
            StatementKind::Const { name, init, .. } => self.exec_const(name, init),
            StatementKind::Function { .. } => Ok(Signal::None),
            StatementKind::If {
                cond,
                then_branch,
                else_branch,
            } => self.exec_if(cond, then_branch, else_branch.as_ref()),
            StatementKind::While { cond, body } => self.exec_while(cond, body),
            StatementKind::For {
                var,
                start,
                end,
                step,
                body,
            } => self.exec_for(var, start, end, step.as_ref(), body),
            StatementKind::Return(val) => {
                let return_val = match val {
                    Some(expr) => self.eval_expr(expr)?,
                    None => Value::Void,
                };
                Ok(Signal::Return(return_val))
            }
            StatementKind::Break => Ok(Signal::Break),
            StatementKind::Continue => Ok(Signal::Continue),
            StatementKind::Expr(expr) => {
                self.eval_expr(expr)?;
                Ok(Signal::None)
            }
            StatementKind::Block(b) => self.execute_block(b),
        }
    }

    fn exec_let(&mut self, name: &str, init: Option<&Expr>) -> Result<Signal, Diagnostic> {
        let val = match init {
            Some(expr) => self.eval_expr(expr)?,
            None => Value::Void,
        };
        self.env.define(name.to_string(), val);
        Ok(Signal::None)
    }

    fn exec_const(&mut self, name: &str, init: &Expr) -> Result<Signal, Diagnostic> {
        let val = self.eval_expr(init)?;
        self.env.define(name.to_string(), val);
        Ok(Signal::None)
    }

    fn exec_if(
        &mut self,
        cond: &Expr,
        then_branch: &Block,
        else_branch: Option<&ElseBranch>,
    ) -> Result<Signal, Diagnostic> {
        if self.eval_expr(cond)?.is_truthy() {
            return self.execute_block(then_branch);
        }
        match else_branch {
            Some(ElseBranch::Block(b)) => self.execute_block(b),
            Some(ElseBranch::If(nested_if)) => self.execute_statement(nested_if),
            None => Ok(Signal::None),
        }
    }

    fn exec_while(&mut self, cond: &Expr, body: &Block) -> Result<Signal, Diagnostic> {
        while self.eval_expr(cond)?.is_truthy() {
            match self.execute_block(body)? {
                Signal::Break => break,
                Signal::Continue => continue,
                Signal::Return(val) => return Ok(Signal::Return(val)),
                Signal::None => {}
            }
        }
        Ok(Signal::None)
    }

    fn exec_for(
        &mut self,
        var: &str,
        start: &Expr,
        end: &Expr,
        step: Option<&Expr>,
        body: &Block,
    ) -> Result<Signal, Diagnostic> {
        let start_val = match self.eval_expr(start)? {
            Value::Int(n) => n,
            _ => return Err(self.runtime_error(start.span, "loop start must be an integer")),
        };
        let end_val = match self.eval_expr(end)? {
            Value::Int(n) => n,
            _ => return Err(self.runtime_error(end.span, "loop end must be an integer")),
        };
        let step_val = match step {
            Some(st) => match self.eval_expr(st)? {
                Value::Int(0) => return Err(self.runtime_error(st.span, "loop step cannot be zero")),
                Value::Int(n) => n,
                _ => return Err(self.runtime_error(st.span, "loop step must be an integer")),
            },
            None => 1,
        };

        let mut current = start_val;
        loop {
            if step_val > 0 && current > end_val {
                break;
            }
            if step_val < 0 && current < end_val {
                break;
            }

            self.env.push_scope();
            self.env.define(var.to_string(), Value::Int(current));
            let res = self.execute_block_without_scope(body);
            self.env.pop_scope();

            match res? {
                Signal::Break => break,
                Signal::Continue => {}
                Signal::Return(val) => return Ok(Signal::Return(val)),
                Signal::None => {}
            }

            match current.checked_add(step_val) {
                Some(next) => current = next,
                None => return Err(self.runtime_error(body.span, "integer overflow in loop counter")),
            }
        }
        Ok(Signal::None)
    }

    fn execute_block(&mut self, block: &Block) -> Result<Signal, Diagnostic> {
        self.env.push_scope();
        let result = self.execute_block_without_scope(block);
        self.env.pop_scope();
        result
    }

    fn execute_block_without_scope(&mut self, block: &Block) -> Result<Signal, Diagnostic> {
        for stmt in &block.statements {
            match self.execute_statement(stmt)? {
                Signal::None => {}
                other => return Ok(other),
            }
        }
        Ok(Signal::None)
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, Diagnostic> {
        let span = expr.span;
        match &expr.kind {
            ExprKind::IntLit(n) => Ok(Value::Int(*n)),
            ExprKind::FloatLit(n) => Ok(Value::Float(*n)),
            ExprKind::DoubleLit(n) => Ok(Value::Double(*n)),
            ExprKind::BoolLit(b) => Ok(Value::Boolean(*b)),
            ExprKind::StringLit(s) => Ok(Value::String(s.clone())),
            ExprKind::InterpolatedString(parts) => self.eval_interp_str(parts),
            ExprKind::Ident(name) => match self.env.get(name) {
                Some(val) => Ok(val.clone()),
                None => Err(self.runtime_error(span, format!("undefined variable '{}'", name))),
            },
            ExprKind::Group(inner) => self.eval_expr(inner),
            ExprKind::Unary { op, expr } => self.eval_unary(*op, expr, span),
            ExprKind::Binary { left, op, right } => self.eval_binary(left, *op, right, span),
            ExprKind::Assign { target, op, value } => self.eval_assign(target, *op, value, span),
            ExprKind::Call { callee, args } => self.eval_call(callee, args, span),
        }
    }

    fn eval_interp_str(&mut self, parts: &[Expr]) -> Result<Value, Diagnostic> {
        let mut buf = String::new();
        for p in parts {
            let val = self.eval_expr(p)?;
            buf.push_str(&val.to_string());
        }
        Ok(Value::String(buf))
    }

    fn eval_unary(&mut self, op: UnaryOp, expr: &Expr, span: Span) -> Result<Value, Diagnostic> {
        let inner = self.eval_expr(expr)?;
        match op {
            UnaryOp::Neg => match inner {
                Value::Int(n) => n
                    .checked_neg()
                    .map(Value::Int)
                    .ok_or_else(|| self.runtime_error(span, "integer overflow")),
                Value::Float(f) => Ok(Value::Float(-f)),
                Value::Double(d) => Ok(Value::Double(-d)),
                _ => Err(self.runtime_error(span, "invalid operand for negation")),
            },
            UnaryOp::Pos => match inner {
                Value::Int(n) => Ok(Value::Int(n)),
                Value::Float(f) => Ok(Value::Float(f)),
                Value::Double(d) => Ok(Value::Double(d)),
                _ => Err(self.runtime_error(span, "invalid operand for unary plus")),
            },
            UnaryOp::Not => match inner {
                Value::Boolean(b) => Ok(Value::Boolean(!b)),
                _ => Err(self.runtime_error(span, "invalid operand for logical not")),
            },
        }
    }

    fn eval_binary(
        &mut self,
        left: &Expr,
        op: BinOp,
        right: &Expr,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if op == BinOp::And {
            let left_val = self.eval_expr(left)?;
            if !left_val.is_truthy() {
                return Ok(Value::Boolean(false));
            }
            let right_val = self.eval_expr(right)?;
            return Ok(Value::Boolean(right_val.is_truthy()));
        }

        if op == BinOp::Or {
            let left_val = self.eval_expr(left)?;
            if left_val.is_truthy() {
                return Ok(Value::Boolean(true));
            }
            let right_val = self.eval_expr(right)?;
            return Ok(Value::Boolean(right_val.is_truthy()));
        }

        let l = self.eval_expr(left)?;
        let r = self.eval_expr(right)?;
        self.eval_binary_op(&l, op, &r, span)
    }

    fn eval_assign(
        &mut self,
        target: &str,
        op: AssignOp,
        value: &Expr,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let rval = self.eval_expr(value)?;
        let new_val = if op == AssignOp::Assign {
            rval
        } else {
            let cur = self
                .env
                .get(target)
                .cloned()
                .ok_or_else(|| self.runtime_error(span, format!("undefined variable '{}'", target)))?;
            let bin_op = match op {
                AssignOp::AddAssign => BinOp::Add,
                AssignOp::SubAssign => BinOp::Sub,
                AssignOp::MulAssign => BinOp::Mul,
                AssignOp::DivAssign => BinOp::Div,
                AssignOp::ModAssign => BinOp::Mod,
                AssignOp::Assign => unreachable!(),
            };
            self.eval_binary_op(&cur, bin_op, &rval, span)?
        };

        if self.env.assign(target, new_val.clone()) {
            Ok(new_val)
        } else {
            Err(self.runtime_error(span, format!("undefined variable '{}'", target)))
        }
    }

    fn eval_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let callee_val = self.eval_expr(callee)?;
        let mut evaluated_args = Vec::new();
        for a in args {
            evaluated_args.push(self.eval_expr(a)?);
        }

        match callee_val {
            Value::Builtin(ref name) => self.call_builtin(name, evaluated_args, span),
            Value::Function {
                params,
                body,
                ..
            } => {
                self.call_depth += 1;
                if self.call_depth > MAX_CALL_DEPTH {
                    return Err(self.runtime_error(span, "stack overflow"));
                }

                self.env.push_scope();
                for (param_name, arg_val) in params.iter().zip(evaluated_args) {
                    self.env.define(param_name.clone(), arg_val);
                }

                let res = self.execute_block_without_scope(&body);
                self.env.pop_scope();
                self.call_depth -= 1;

                match res? {
                    Signal::Return(val) => Ok(val),
                    _ => Ok(Value::Void),
                }
            }
            _ => Err(self.runtime_error(span, "cannot call non-function value")),
        }
    }

    fn eval_binary_op(
        &self,
        l: &Value,
        op: BinOp,
        r: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                self.eval_arithmetic(l, op, r, span)
            }
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                self.eval_compare(l, op, r, span)
            }
            BinOp::And | BinOp::Or => unreachable!(),
        }
    }

    fn eval_arithmetic(
        &self,
        l: &Value,
        op: BinOp,
        r: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match (l, r) {
            (Value::Int(a), Value::Int(b)) => self.eval_int_arith(*a, op, *b, span),
            (Value::Float(a), Value::Float(b)) => self.eval_float_arith(*a, op, *b, span),
            (Value::Double(a), Value::Double(b)) => self.eval_double_arith(*a, op, *b, span),
            _ => Err(self.runtime_error(span, "invalid operands for arithmetic operation")),
        }
    }

    fn eval_int_arith(&self, a: i64, op: BinOp, b: i64, span: Span) -> Result<Value, Diagnostic> {
        let res = match op {
            BinOp::Add => a.checked_add(b),
            BinOp::Sub => a.checked_sub(b),
            BinOp::Mul => a.checked_mul(b),
            BinOp::Div => {
                if b == 0 {
                    return Err(self.runtime_error(span, "division by zero"));
                }
                a.checked_div(b)
            }
            BinOp::Mod => {
                if b == 0 {
                    return Err(self.runtime_error(span, "modulo by zero"));
                }
                a.checked_rem(b)
            }
            _ => unreachable!(),
        };
        res.map(Value::Int)
            .ok_or_else(|| self.runtime_error(span, "integer overflow"))
    }

    fn eval_float_arith(&self, a: f32, op: BinOp, b: f32, span: Span) -> Result<Value, Diagnostic> {
        match op {
            BinOp::Add => Ok(Value::Float(a + b)),
            BinOp::Sub => Ok(Value::Float(a - b)),
            BinOp::Mul => Ok(Value::Float(a * b)),
            BinOp::Div => {
                if b == 0.0 {
                    Err(self.runtime_error(span, "division by zero"))
                } else {
                    Ok(Value::Float(a / b))
                }
            }
            _ => Err(self.runtime_error(span, "invalid operands for float arithmetic")),
        }
    }

    fn eval_double_arith(&self, a: f64, op: BinOp, b: f64, span: Span) -> Result<Value, Diagnostic> {
        match op {
            BinOp::Add => Ok(Value::Double(a + b)),
            BinOp::Sub => Ok(Value::Double(a - b)),
            BinOp::Mul => Ok(Value::Double(a * b)),
            BinOp::Div => {
                if b == 0.0 {
                    Err(self.runtime_error(span, "division by zero"))
                } else {
                    Ok(Value::Double(a / b))
                }
            }
            _ => Err(self.runtime_error(span, "invalid operands for double arithmetic")),
        }
    }

    fn eval_compare(
        &self,
        l: &Value,
        op: BinOp,
        r: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match op {
            BinOp::Eq => Ok(Value::Boolean(l == r)),
            BinOp::NotEq => Ok(Value::Boolean(l != r)),
            BinOp::Lt => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Boolean(a < b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a < b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Boolean(a < b)),
                _ => Err(self.runtime_error(span, "invalid operands for '<'")),
            },
            BinOp::LtEq => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Boolean(a <= b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a <= b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Boolean(a <= b)),
                _ => Err(self.runtime_error(span, "invalid operands for '<='")),
            },
            BinOp::Gt => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Boolean(a > b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a > b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Boolean(a > b)),
                _ => Err(self.runtime_error(span, "invalid operands for '>'")),
            },
            BinOp::GtEq => match (l, r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Boolean(a >= b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a >= b)),
                (Value::Double(a), Value::Double(b)) => Ok(Value::Boolean(a >= b)),
                _ => Err(self.runtime_error(span, "invalid operands for '>='")),
            },
            _ => unreachable!(),
        }
    }

    fn call_builtin(
        &mut self,
        name: &str,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match name {
            "print" => self.builtin_print(&args),
            "assert" => self.builtin_assert(&args, span),
            "toString" => {
                let arg = self.arg1(&args, "toString", span)?;
                Ok(Value::String(arg.to_string()))
            }
            "toInt" | "toFloat" | "toDouble" | "toBoolean" => {
                let arg = self.arg1(&args, name, span)?;
                self.builtin_cast(name, arg, span)
            }
            _ => Err(self.runtime_error(span, format!("unknown builtin '{}'", name))),
        }
    }

    fn builtin_print(&self, args: &[Value]) -> Result<Value, Diagnostic> {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{}", arg);
        }
        println!();
        io::stdout().flush().ok();
        Ok(Value::Void)
    }

    fn builtin_assert(&self, args: &[Value], span: Span) -> Result<Value, Diagnostic> {
        let cond = args.first().map(|v| v.is_truthy()).unwrap_or(false);
        let msg = args
            .get(1)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "assertion failed".to_string());
        if !cond {
            Err(self.runtime_error(span, format!("assertion failed: {}", msg)))
        } else {
            Ok(Value::Void)
        }
    }

    fn builtin_cast(&self, name: &str, arg: &Value, span: Span) -> Result<Value, Diagnostic> {
        match name {
            "toInt" => self.cast_to_int(arg, span),
            "toFloat" => self.cast_to_float(arg, span),
            "toDouble" => self.cast_to_double(arg, span),
            "toBoolean" => Ok(Value::Boolean(self.cast_to_boolean(arg))),
            _ => unreachable!(),
        }
    }

    fn cast_to_int(&self, arg: &Value, span: Span) -> Result<Value, Diagnostic> {
        match arg {
            Value::Int(n) => Ok(Value::Int(*n)),
            Value::Float(f) => Ok(Value::Int(*f as i64)),
            Value::Double(d) => Ok(Value::Int(*d as i64)),
            Value::Boolean(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
            Value::String(s) => s
                .trim()
                .parse::<i64>()
                .map(Value::Int)
                .map_err(|_| self.runtime_error(span, format!("invalid integer string '{}'", s))),
            _ => Err(self.runtime_error(span, "cannot convert to int")),
        }
    }

    fn cast_to_float(&self, arg: &Value, span: Span) -> Result<Value, Diagnostic> {
        match arg {
            Value::Int(n) => Ok(Value::Float(*n as f32)),
            Value::Float(f) => Ok(Value::Float(*f)),
            Value::Double(d) => Ok(Value::Float(*d as f32)),
            Value::String(s) => s
                .trim()
                .parse::<f32>()
                .map(Value::Float)
                .map_err(|_| self.runtime_error(span, format!("invalid float string '{}'", s))),
            _ => Err(self.runtime_error(span, "cannot convert to float")),
        }
    }

    fn cast_to_double(&self, arg: &Value, span: Span) -> Result<Value, Diagnostic> {
        match arg {
            Value::Int(n) => Ok(Value::Double(*n as f64)),
            Value::Float(f) => Ok(Value::Double(*f as f64)),
            Value::Double(d) => Ok(Value::Double(*d)),
            Value::String(s) => s
                .trim()
                .parse::<f64>()
                .map(Value::Double)
                .map_err(|_| self.runtime_error(span, format!("invalid double string '{}'", s))),
            _ => Err(self.runtime_error(span, "cannot convert to double")),
        }
    }

    fn cast_to_boolean(&self, arg: &Value) -> bool {
        match arg {
            Value::Boolean(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Double(d) => *d != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Void => false,
            _ => true,
        }
    }

    fn arg1<'b>(&self, args: &'b [Value], name: &str, span: Span) -> Result<&'b Value, Diagnostic> {
        args.first()
            .ok_or_else(|| self.runtime_error(span, format!("{} requires 1 argument", name)))
    }

    fn runtime_error(&self, span: Span, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(&self.file, span, Stage::Runtime, message)
    }
}
