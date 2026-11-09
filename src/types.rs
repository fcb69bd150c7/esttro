use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Double,
    Boolean,
    String,
    Void,
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Error,
}

impl Type {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Int | Type::Float | Type::Double)
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Int)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    pub fn is_assignable_from(&self, other: &Type) -> bool {
        if self.is_error() || other.is_error() {
            return true;
        }
        self == other
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Double => write!(f, "double"),
            Type::Boolean => write!(f, "boolean"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Function { params, ret } => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Error => write!(f, "<error>"),
        }
    }
}

