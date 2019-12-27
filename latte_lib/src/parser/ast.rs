use crate::util::env::{Env, FromMemberVec};
use crate::location::Located;
use crate::error::FrontendError;


/// trait for marking ast items that can searched by key (in an environment)
pub trait Keyed {
    fn get_key(&self) -> &String;
}

/// initially, all ast items are located by usize (byte offset)
pub type Loc<AstT> = Located<AstT, usize>;

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
    LitInt { val: u64 },
    LitBool { val: bool },
    LitStr { val: String },
    LitNull,
    App { r: Loc<Reference>, args: Vec<Loc<Box<Expression>>> },
    Unary { op: UnaryOperator, arg: Loc<Box<Expression>> },
    Binary {
        left: Loc<Box<Expression>>,
        op: BinaryOperator,
        right: Loc<Box<Expression>>
    },
    InitDefault { t: Type },
    // InitFields { }  // TODO (not necessary, but nice)
    InitArr { t: Type, size: Loc<Box<Expression>> },
    Reference { r: Loc<Reference> },
    // edge case: expr in Cast is always null so we don't need to locate it:
    Cast { t: Type, expr: Loc<Box<Expression>> },
    Error,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Type {
    /// basic types
    Int,
    Str,
    Bool,
    Void,
    Null,

    /// complex types (extensions)
    Class { ident: String },
    Array { item_t: Box<Type> },

    /// extra types for reporting / propagating errors
    Error,
    Any,
    Object,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Reference {
    Ident { ident: String },
    Object { obj: String, field: String },
    Array { arr: String, idx: Loc<Box<Expression>> },
}

#[derive(Debug, PartialEq, Clone)]
pub enum DeclItem {
    NoInit { ident: String },
    Init { ident: String, val: Loc<Box<Expression>> }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Block {
    pub stmts: Vec<Loc<Box<Statement>>>
}

#[derive(Debug, PartialEq, Clone)]
pub enum Statement {
    Block { block: Loc<Block> },
    Empty,
    Decl { t: Type, items: Vec<DeclItem> },
    Ass { r: Loc<Reference>, expr: Loc<Box<Expression>> },
    Mut { r: Loc<Reference>, op: StatementOp },
    Return { expr: Option<Loc<Box<Expression>>> },
    Cond { expr: Loc<Box<Expression>>, stmt: Loc<Box<Statement>> },
    CondElse {
        expr: Loc<Box<Expression>>,
        stmt_true: Loc<Box<Statement>>,
        stmt_false: Loc<Box<Statement>>
    },
    While { expr: Loc<Box<Expression>>, stmt: Loc<Box<Statement>> },
    For {
        t: Type,
        ident: String,
        arr: Loc<Box<Expression>>,
        stmt: Loc<Box<Statement>>
    },
    Expr { expr: Loc<Box<Expression>> },
    Error,
}

#[derive(Debug, PartialEq, Clone)]
pub enum StatementOp {
    Increment,
    Decrement,
}

#[derive(Debug, PartialEq, Hash, Clone)]
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
    pub args: Vec<Loc<Arg>>,
    pub block: Loc<Block>
}

impl Function {
    pub fn new(
        ret: Type, ident: String, args: Vec<Located<Arg, usize>>, block: Loc<Block>
    ) -> Result<Self, FrontendError<usize>> {
        // check if there are no duplicate arguments
        Env::from_vec(args.clone())?;

        // insert actual vector into the Function to preserve order
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
    pub default: Option<Loc<Box<Expression>>>, // TODO: is this even used?
}

impl Keyed for ClassVar {
    fn get_key(&self) -> &String {
        &self.ident
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Class {
    ident: String,
    pub vars: Env<Located<ClassVar, usize>>,
    pub methods: Env<Located<Function, usize>>,
    pub parent: Option<String>,
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
    pub classes: Env<Located<Class, usize>>,
    pub functions: Env<Located<Function, usize>>,
}
