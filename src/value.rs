use std::fmt;

use crate::ast::Block;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f32),
    Double(f64),
    Boolean(bool),
    String(String),
    Void,
    Function {
        name: String,
        params: Vec<String>,
        body: Block,
    },
    Builtin(String),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            _ => true,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Double(n) => write!(f, "{}", n),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Void => write!(f, "void"),
            Value::Function { name, .. } => write!(f, "<fn {}>", name),
            Value::Builtin(name) => write!(f, "<builtin {}>", name),
        }
    }
}

