use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeNodeKind {
    Int,
    Float,
    Double,
    Boolean,
    String,
    Void,
    Function {
        params: Vec<TypeNode>,
        ret: Box<TypeNode>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeNode {
    pub kind: TypeNodeKind,
    pub span: Span,
}

impl TypeNode {
    pub fn new(kind: TypeNodeKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    Pos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    IntLit(i64),
    FloatLit(f32),
    DoubleLit(f64),
    BoolLit(bool),
    StringLit(String),
    InterpolatedString(Vec<Expr>),
    Ident(String),
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Assign {
        target: String,
        op: AssignOp,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Group(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: TypeNode,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    If(Box<Statement>),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    Let {
        name: String,
        ty: Option<TypeNode>,
        init: Option<Expr>,
    },
    Const {
        name: String,
        ty: Option<TypeNode>,
        init: Expr,
    },
    Function {
        name: String,
        params: Vec<Param>,
        ret_ty: Option<TypeNode>,
        body: Block,
    },
    If {
        cond: Expr,
        then_branch: Block,
        else_branch: Option<ElseBranch>,
    },
    While {
        cond: Expr,
        body: Block,
    },
    For {
        var: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        body: Block,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Expr(Expr),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

impl Statement {
    pub fn new(kind: StatementKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckedProgram(pub Program);
