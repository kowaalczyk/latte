use std::fmt::{Display, Error, Formatter};

use regex::internal::Inst;

/// internal type representation (as argument or object member):
/// string: pointer to an array of characters, passed by value
/// int, bool: passed by value
/// array: pointer to array, passed by value
/// object: pointer to struct, passed by value

use crate::frontend::ast::{BinaryOperator, Type, UnaryOperator};
use crate::meta::{GetType, Meta};
use crate::util::env::Env;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum Entity {
    Null,
    Int { v: i32 },
    Bool { v: bool },
    Register { n: usize, t: Type },
}

impl GetType for Entity {
    fn get_type(&self) -> Type {
        match self {
            Entity::Null => Type::Null,
            Entity::Int { .. } => Type::Int,
            Entity::Bool { .. } => Type::Bool,
            Entity::Register { n: _, t } => t.clone(),
        }
    }
}

impl From<i32> for Entity {
    fn from(i: i32) -> Self {
        Entity::Int { v: i }
    }
}

impl From<bool> for Entity {
    fn from(b: bool) -> Self {
        Entity::Bool { v: b }
    }
}

#[derive(Debug, Clone)]
pub enum InstructionKind {
    Alloc { t: Type },
    Load { ptr: Entity },
    Store { val: Entity, ptr: Entity },
    LoadConst { name: String, len: usize },
    UnaryOp { op: UnaryOperator, arg: Entity },
    BinaryOp { op: BinaryOperator, l: Entity, r: Entity },
    Call { func: String, args: Vec<Entity> },
    RetVal { val: Entity },
    RetVoid,
    JumpCond { cond: Entity, true_label: String, false_label: String },
    Jump { label: String },
    Phi { args: Vec<(Entity, String)> },
}

impl InstructionKind {
    pub fn without_result(self) -> Instruction {
        Instruction::from(self)
    }

    pub fn with_result(self, result: Entity) -> Instruction {
        Instruction::new(self, Some(result))
    }
}

pub type Instruction = Meta<InstructionKind, Option<Entity>>;

pub trait GetEntity {
    fn get_entity(&self) -> Entity;
    fn has_result_entity(&self) -> bool;
}

impl GetEntity for Instruction {
    fn get_entity(&self) -> Entity {
        if let Some(entity) = self.get_meta() {
            entity.clone()
        } else {
            panic!("missing result entity information for {:?}", self)
        }
    }

    fn has_result_entity(&self) -> bool {
        if let Option::Some(_) = self.get_meta() {
            true
        } else {
            false
        }
    }
}

impl GetType for Instruction {
    fn get_type(&self) -> Type {
        self.get_entity().get_type()
    }
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: Option<String>,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StringDecl {
    pub name: String,
    pub val: String,
    pub len: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub ret_type: Type,
    pub arg_types: Vec<Type>,
    pub body: Vec<BasicBlock>,
}

#[derive(Debug, Clone)]
pub enum LLVM {
    DeclFunction { decl: String },
    DeclStruct { decl: StructDecl },
    DeclString { decl: StringDecl },
    Function { def: FunctionDef },
}
