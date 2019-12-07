
#[derive(Debug)]
pub enum BinaryOp {
    Plus,
    Minus,
    Times,
    Div,
    Mod,
    LTH,
    LE,
    GTH,
    GE,
    EQU,
    NE,
    And,
    Or,
}

#[derive(Debug)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug)]
pub enum Expression {
    Var { ident: String },
    LitInt { val: u64 },
    LitBool { val: bool },
    LitStr { val: String },
    LitNull,  // extension: struct, class
    App { ident: String, args: Vec<Box<Expression>> },
    Unary { op: UnaryOp, arg: Box<Expression> },
    Binary { left: Box<Expression>, op: BinaryOp, right: Box<Expression> },
    InitDefault { class_name: String }, // extension: struct, class
    // InitFields { }  // TODO (not necessary, but nice)
    InitArr { t: Type, size: Box<Expression> }, // extension: array
    Member { obj: String, field: String }, // extension: struct, class
    Index { arr: String, idx: Box<Expression> }, // extension: array
    MethodApp { obj:String, method: String, args: Vec<Box<Expression>> }, // extension: class
    Cast { t: Type, expr: Box<Expression> },
}

#[derive(Debug)]
pub enum Type {
    Int,
    Str,
    Bool,
    Void,
    // extension: class
    Class { ident: String },
    // extension: array
    Array { item_t: Box<Type> },
}

#[derive(Debug)]
pub enum DeclItem {
    NoInit { ident: String },
    Init { ident: String, val: Box<Expression> }
}

#[derive(Debug)]
pub struct Block {
    pub stmts: Vec<Box<Stmt>>
}

#[derive(Debug)]
pub enum Stmt {
    Block { block: Block },
    Empty,
    Decl { t: Type, items: Vec<DeclItem> },
    Ass { ident: String, expr: Box<Expression> },
    Mut { ident: String, op: StmtOp },
    Return { expr: Option<Box<Expression>> },
    Cond { expr: Box<Expression>, stmt: Box<Stmt> },
    CondElse { expr: Box<Expression>, stmt_true: Box<Stmt>, stmt_false: Box<Stmt> },
    While { expr: Box<Expression>, stmt: Box<Stmt> },
    For { t: Type, ident: String, arr: Box<Expression>, stmt: Box<Stmt> },
    Expr { expr: Box<Expression> },
}

#[derive(Debug)]
pub enum StmtOp {
    Increment,
    Decrement,
}

#[derive(Debug)]
pub struct Arg { pub t: Type, pub ident: String }

#[derive(Debug)]
pub struct Function {
    pub ret: Type, 
    pub ident: String, 
    pub args: Vec<Arg>, 
    pub block: Block
}

// extension: class, struct
#[derive(Debug)]
pub struct ClassVar {
    pub t: Type,
    pub ident: String,
    pub default: Option<Box<Expression>>,
}

#[derive(Debug)]
pub enum TopDef {
    Function { func: Function },
    // extension: class, struct
    Class { 
        ident: String, 
        vars: Vec<ClassVar>,
        methods: Vec<Function>,
        parent: Option<String>,  // no need to hold entire class
    },
}

#[derive(Debug)]
pub struct Program { pub topdefs: Vec<TopDef> }
