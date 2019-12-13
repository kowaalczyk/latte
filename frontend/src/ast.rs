use crate::env::{Env, FromMemberVec};
use crate::location::Located;
use crate::error::FrontendError;

/// trait for marking ast items that can searched by key (in an environment)
pub trait Keyed {
    fn get_key(&self) -> &String;
}

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

impl Keyed for Arg {
    fn get_key(&self) -> &String {
        &self.ident
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Function {
    pub ret: Type, 
    pub ident: String, 
    pub args: Env<Arg>,
    pub block: Block
}

impl Function {
    pub fn new(ret: Type, ident: String, arg_vec: Vec<Located<Arg, usize>>, block: Block) 
    -> Result<Self, FrontendError<usize>> {
        let args = Env::from_vec(arg_vec)?;
        Ok(Self { ret, ident, args, block })
    }
}

impl Keyed for Function {
    fn get_key(&self) -> &String {
        &self.ident
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ClassVar {
    pub t: Type,
    pub ident: String,
    pub default: Option<Box<Expression>>,
}

impl Keyed for ClassVar {
    fn get_key(&self) -> &String {
        &self.ident
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Class {
    ident: String,
    vars: Env<ClassVar>,
    methods: Env<Function>,
    parent: Option<String>,
}

impl Class {
    pub fn new(ident: String, var_vec: Vec<Located<ClassVar, usize>>, method_vec: Vec<Located<Function, usize>>) 
    -> Result<Self, FrontendError<usize>> {
        let vars = Env::from_vec(var_vec)?;
        let methods = Env::from_vec(method_vec)?;
        let cls = Self { ident, vars, methods, parent: Option::None };
        Ok(cls)
    }

    pub fn with_parent(&mut self, parent: &String) -> Self {
        self.parent = Option::Some(parent.clone());
        self.clone()
    }
}

impl Keyed for Class {
    fn get_key(&self) -> &String {
        &self.ident
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TopDef {
    Function { func: Function },
    Class { cls: Class },
    Error,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Program {
    pub classes: Env<Class>,
    pub functions: Env<Function>,
}
