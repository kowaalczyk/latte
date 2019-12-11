
#[derive(Debug, PartialEq, Clone)]
pub enum BinaryOperator {
    Plus,
    Minus,
    Times,
    Divide,
    Modulo,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    And,
    Or,
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOperator {
    Neg,
    Not,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Var { ident: String },
    LitInt { val: u64 },
    LitBool { val: bool },
    LitStr { val: String },
    LitNull,
    App { r: Reference, args: Vec<Box<Expression>> },
    Unary { op: UnaryOperator, arg: Box<Expression> },
    Binary { left: Box<Expression>, op: BinaryOperator, right: Box<Expression> },
    InitDefault { t: Type },
    // InitFields { }  // TODO (not necessary, but nice)
    InitArr { t: Type, size: Box<Expression> },
    Reference { r: Reference },
    Cast { t: Type, expr: Box<Expression> },
    Error,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    Int,
    Str,
    Bool,
    Void,
    Class { ident: String },
    Array { item_t: Box<Type> },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Reference {
    Ident { ident: String },
    Object { obj: String, field: String },
    Array { arr: String, idx: Box<Expression> },
}

#[derive(Debug, PartialEq, Clone)]
pub enum DeclItem {
    NoInit { ident: String },
    Init { ident: String, val: Box<Expression> }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub stmts: Vec<Box<Statement>>
}

#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
    Block { block: Block },
    Empty,
    Decl { t: Type, items: Vec<DeclItem> },
    Ass { r: Reference, expr: Box<Expression> },
    Mut { r: Reference, op: StatementOp },
    Return { expr: Option<Box<Expression>> },
    Cond { expr: Box<Expression>, stmt: Box<Statement> },
    CondElse { expr: Box<Expression>, stmt_true: Box<Statement>, stmt_false: Box<Statement> },
    While { expr: Box<Expression>, stmt: Box<Statement> },
    For { t: Type, ident: String, arr: Box<Expression>, stmt: Box<Statement> },
    Expr { expr: Box<Expression> },
    Error,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StatementOp {
    Increment,
    Decrement,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Arg { pub t: Type, pub ident: String }

#[derive(Debug, PartialEq, Clone)]
pub struct Function {
    pub ret: Type, 
    pub ident: String, 
    pub args: Vec<Arg>, 
    pub block: Block
}

#[derive(Debug, PartialEq, Clone)]
pub struct ClassVar {
    pub t: Type,
    pub ident: String,
    pub default: Option<Box<Expression>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TopDef {
    Function { 
        ret: Type, 
        ident: String, 
        args: Vec<Arg>, 
        block: Block
    },
    Class { 
        t: Type, 
        vars: Vec<ClassVar>,
        methods: Vec<Function>,
        parent: Option<Type>,
    },
    Error,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Program { pub topdefs: Vec<TopDef> }
