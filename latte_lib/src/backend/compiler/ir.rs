/// internal type representation (as argument or object member):
/// string: pointer to an array of characters, passed by value
/// int, bool: passed by value
/// array: pointer to array, passed by value
/// object: pointer to struct, passed by value

use crate::frontend::ast::{Type, UnaryOperator, BinaryOperator};
use crate::meta::{Meta, GetType};
use crate::util::env::Env;
use std::fmt::{Display, Formatter, Error};
use regex::internal::Inst;


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
    LoadConst { name: String },
    UnaryOp { op: UnaryOperator, arg: Entity },
    BinaryOp { op: BinaryOperator, l: Entity, r: Entity },
    Call { func: String, args: Vec<Entity> },
    RetVal { val: Entity },
    RetVoid,
    JumpCond { cond: Entity, true_label: String, false_label: String },
    Jump { label: String },
}

impl InstructionKind {
    pub fn without_result(self) -> LLVM {
        LLVM::from(Instruction::from(self))
    }

    pub fn with_result(self, result: Entity) -> LLVM {
        LLVM::from(Instruction::new(self, Some(result)))
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
pub enum LLVM {
    Instruction { instruction: Instruction },
//    Entity { entity: Entity },
    Label { name: String },
    Function { name: String, ret_type: Type, arg_types: Vec<Type>, llvm: Vec<Box<LLVM>> },
}

impl GetEntity for LLVM {
    fn get_entity(&self) -> Entity {
        if let LLVM::Instruction { instruction } = self {
            instruction.get_entity()
        } else {
            panic!("no entity value for LLVM structure: {:?}", self)
        }
    }

    fn has_result_entity(&self) -> bool {
        if let LLVM::Instruction { instruction } = self {
            instruction.has_result_entity()
        } else {
            false
        }
    }
}

impl GetType for LLVM {
    fn get_type(&self) -> Type {
        self.get_entity().get_type()
    }
}

impl From<Instruction> for LLVM {
    fn from(instruction: Instruction) -> Self {
        LLVM::Instruction { instruction }
    }
}


// TODO: Add lifetime specifiers to IR structures to make compilation more memory-efficient
#[derive(Debug, Clone)]
pub struct Struct {
    /// to know which function to call, we have to remember the mapping method_name -> vtable entry
    /// each subclass preserves order of superclass methods in vtable
    /// when an existing method is overwritten in subclass, we use the vtable hashmap to update appropriate value
    pub vtable: Env<usize>,

    /// type casts will be necessary, to know how to cast during assignment to a class variable
    pub vars: Env<Type>,
}

// object initialization:
// 1. copy entire vtable (only contains array of pointers, should be quite lightweight)
// 2. allocate memory for all member variables
// 3. assign default values to all member variables
// TODO: Add reference counting to the objects, check reference counts after block exits

// method call:
// 1. get vtable entry by name
// 2. cast arguments to void pointer type - TODO: is this really necessary? possibly not

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// when arguments are passed during function call, structs will have to be cast to
    /// appropriate objects (ie, subclass to superclass)
    pub vars: Env<Type>,
}
