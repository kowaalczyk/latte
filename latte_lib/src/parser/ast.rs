use crate::util::env::{Env, FromKeyedVec};
use crate::error::FrontendError;
use crate::util::visitor::AstVisitor;
use crate::meta::Meta;


/// trait for marking ast items that can searched by key (in an environment)
pub trait Keyed {
    fn get_key(&self) -> &String;
}

/// alias for all metadata containers attached to ast items
pub type AstItem<ItemT, MetaT> = Meta<ItemT, MetaT>;

impl<ItemT: Keyed, MetaT> Keyed for AstItem<ItemT, MetaT> {
    /// if item is keyed, the whole wrapper can also be keyed using same key
    fn get_key(&self) -> &String {
        self.item.get_key()
    }
}

/// metadata type for location data (used by generated parser)
#[derive(Debug, PartialEq, Clone)]
pub struct LocationMeta {
    /// byte offset from the beginning of source code
    pub offset: usize,
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
pub enum ExpressionKind<MetaT> {
    LitInt { val: i32 },
    LitBool { val: bool },
    LitStr { val: String },
    LitNull,
    App { r: Reference<MetaT>, args: Vec<Box<Expression<MetaT>>> },
    Unary { op: UnaryOperator, arg: Box<Expression<MetaT>> },
    Binary {
        left: Box<Expression<MetaT>>,
        op: BinaryOperator,
        right: Box<Expression<MetaT>>
    },
    InitDefault { t: Type },
    // InitFields { }  // TODO (not necessary, but nice)
    InitArr { t: Type, size: Box<Expression<MetaT>> },
    Reference { r: Reference<MetaT> },
    // edge case: expr in Cast is always null so we don't need to locate it:
    Cast { t: Type, expr: Box<Expression<MetaT>> },
    Error,
}
pub type Expression<MetaT> = AstItem<ExpressionKind<MetaT>, MetaT>;

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

    /// used for checking types during function call
    Function { args: Vec<Box<Type>>, ret: Box<Type> },

    /// error type for smooth propagation of errors during parsing
    Error,

    /// represents any type, used to report errors like: 'expected Array { Any } got Int'
    Any,

    /// represents any object (instance of any class), used to report member access errors
    Object,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ReferenceKind<MetaT> {
    Ident { ident: String },
    Object { obj: String, field: String },
    ObjectSelf { field: String },
    Array { arr: String, idx: Box<Expression<MetaT>> },
    ArrayLen { ident: String },
}
pub type Reference<MetaT> = AstItem<ReferenceKind<MetaT>, MetaT>;

#[derive(Debug, PartialEq, Clone)]
pub enum DeclItemKind<MetaT> {
    NoInit { ident: String },
    Init { ident: String, val: Box<Expression<MetaT>> }
}
pub type DeclItem<MetaT> = AstItem<DeclItemKind<MetaT>, MetaT>;

#[derive(Debug, PartialEq, Clone)]
pub struct BlockItem<MetaT> {
    pub stmts: Vec<Box<Statement<MetaT>>>
}
pub type Block<MetaT> = AstItem<Block<MetaT>, MetaT>;

#[derive(Debug, PartialEq, Clone)]
pub enum StatementKind<MetaT> {
    Block { block: Block<MetaT> },
    Empty,
    Decl { t: Type, items: Vec<DeclItem<MetaT>> },
    Ass { r: Reference<MetaT>, expr: Box<Expression<MetaT>> },
    Mut { r: Reference<MetaT>, op: StatementOp },
    Return { expr: Option<Box<Expression<MetaT>>> },
    Cond { expr: Box<Expression<MetaT>>, stmt: Box<Statement<MetaT>> },
    CondElse {
        expr: Box<Expression<MetaT>>,
        stmt_true: Box<Statement<MetaT>>,
        stmt_false: Box<Statement<MetaT>>
    },
    While { expr: Box<Expression<MetaT>>, stmt: Box<Statement<MetaT>> },
    For {
        t: Type,
        ident: String,
        arr: Box<Expression<MetaT>>,
        stmt: Box<Statement<MetaT>>
    },
    Expr { expr: Box<Expression<MetaT>> },
    Error,
}
pub type Statement<MetaT> = AstItem<StatementKind<MetaT>, MetaT>;

#[derive(Debug, PartialEq, Clone)]
pub enum StatementOp {
    Increment,
    Decrement,
}

#[derive(Debug, PartialEq, Hash, Clone)]
pub struct ArgItem { pub t: Type, pub ident: String }
pub type Arg<MetaT> = AstItem<ArgItem, MetaT>;

impl Keyed for ArgItem {
    fn get_key(&self) -> &String {
        &self.ident
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct FunctionItem<MetaT> {
    pub ret: Type,
    pub ident: String,
    pub args: Vec<Arg<MetaT>>,
    pub block: Block<MetaT>
}
pub type Function<MetaT> = AstItem<FunctionItem<MetaT>, MetaT>;

impl FunctionItem<LocationMeta> {
    pub fn new(
        ret: Type, ident: String, args: Vec<Arg<LocationMeta>>, block: Block<LocationMeta>
    ) -> Result<Self, FrontendError<LocationMeta>> {
        // check if there are no duplicate arguments
        Env::from_vec(args.clone())?;

        // insert actual vector into the Function to preserve order
        Ok(Self { ret, ident, args, block })
    }

    pub fn get_type(&self) -> Type {
        let arg_types: Vec<_> = self.args.iter()
            .map(|arg| Box::new(arg.item.t.clone()))
            .collect();
        Type::Function { args: arg_types, ret: Box::new(self.ret.clone()) }
    }
}

impl<T> Keyed for FunctionItem<T> {
    fn get_key(&self) -> &String {
        &self.ident
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ClassVarItem<MetaT> {
    pub t: Type,
    pub ident: String,
    pub default: Option<Box<Expression<MetaT>>>, // TODO: is this even used?
}
pub type ClassVar<MetaT> = AstItem<ClassVarItem<MetaT>, MetaT>;

impl<T> Keyed for ClassVarItem<T> {
    fn get_key(&self) -> &String {
        &self.ident
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ClassItem<MetaT> {
    ident: String,
    pub vars: Env<ClassVar<MetaT>>,
    pub methods: Env<Function<MetaT>>,
    pub parent: Option<String>,
}
pub type Class<MetaT> = AstItem<ClassItem<MetaT>, MetaT>;

impl ClassItem<LocationMeta> {
    pub fn new(
        ident: String, var_vec: Vec<ClassVar<LocationMeta>>, method_vec: Vec<Function<LocationMeta>>
    ) -> Result<Self, Vec<FrontendError<LocationMeta>>> {
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

impl<T> Keyed for ClassItem<T> {
    fn get_key(&self) -> &String {
        &self.ident
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum TopDefKind<MetaT> {
    Function { func: Function<MetaT> },
    Class { cls: Class<MetaT> },
    Error,
}
pub type TopDef<MetaT> = AstItem<TopDefKind<MetaT>, MetaT>;

/// the result of parsing and all subsequent operations (ast root)
#[derive(Debug, PartialEq, Clone)]
pub struct Program<MetaT> {
    pub classes: Env<Class<MetaT>>,
    pub functions: Env<Function<MetaT>>,
}
