use std::collections::HashSet;
/// internal type representation (as argument or object member):
/// string: pointer to an array of characters, passed by value
/// int, bool: passed by value
/// array: pointer to array, passed by value
/// object: pointer to struct, passed by value

use std::fmt::{Display, Error, Formatter};

use crate::frontend::ast::{ArgItem, BinaryOperator, Type, UnaryOperator};
use crate::meta::{GetType, Meta};
use crate::util::env::Env;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// represents anything that can be passed as argument to LLVM operation
/// uuid fields are necessary for constants to guarantee that they are
/// differentiate between variables with same, constant value
pub enum Entity {
    Null { uuid: usize, t: Type },
    Int { v: i32, uuid: usize },
    Bool { v: bool, uuid: usize },
    Register { n: usize, t: Type },
    NamedRegister { name: String, t: Type },
    GlobalConstInt { name: String },
}

impl Entity {
    /// interprets entity as array, returns type of its item
    pub fn get_array_item_t(&self) -> Type {
        if let Type::Array { item_t } = self.get_type() {
            item_t.as_ref().clone()
        } else {
            panic!("expected array type, got {}", self.get_type())
        }
    }
}

impl GetType for Entity {
    fn get_type(&self) -> Type {
        match self {
            Entity::Null { uuid, t } => t.clone(),
            Entity::Int { .. } => Type::Int,
            Entity::Bool { .. } => Type::Bool,
            Entity::Register { n: _, t } => t.clone(),
            Entity::NamedRegister { name: _, t } => t.clone(),
            Entity::GlobalConstInt { .. } => Type::Reference { t: Box::new(Type::Int) },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionKind {
    Alloc { t: Type },
    Load { ptr: Entity },
    Store { val: Entity, ptr: Entity },
    GetStructElementPtr { container_type_name: String, var: Entity, idx: Entity },
    GetArrayElementPtr { item_t: Type, var: Entity, idx: Entity },
    LoadConst { name: String, len: usize },
    BitCast { ent: Entity, to: Type },
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
    /// convert to an instruction wihtout a result
    pub fn without_result(self) -> Instruction {
        Instruction::from(self)
    }

    /// convert to an instruction with provided result entity
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

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub label: Option<String>,
    pub instructions: Vec<Instruction>,
}

impl BasicBlock {
    /// checks if the block ends with a return instruction
    pub fn always_returns(&self) -> bool {
        if let Some(last_instr) = self.instructions.last() {
            match &last_instr.item {
                InstructionKind::RetVoid => true,
                InstructionKind::RetVal { .. } => true,
                _ => false,
            }
        } else {
            false
        }
    }

    /// get the lowest register number used within the block
    pub fn get_first_register(&self) -> Option<Entity> {
        self.instructions.iter()
            .filter_map(|i| i.get_meta().clone())
            .nth(0)
    }

    /// get block label name, or unnamed label 0
    pub fn get_label(&self) -> String {
        if let Some(label) = &self.label {
            label.clone()
        } else {
            // label is numbered, % will be automatically prepended
            // when used as a variable (ie. phi argument)
            String::from("0")
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    /// name of the structure
    pub name: String,

    // name of the constant representing the structure
    pub size_constant_name: String,

    /// field types in fixed order
    pub fields: Vec<Type>,

    /// mapping: field name => field index
    pub field_env: Env<i32>,
}

impl StructDecl {
    pub fn llvm_name(&self) -> String {
        format!("%{}", self.name)
    }
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
    pub args: Vec<ArgItem>,
    pub body: Vec<BasicBlock>,
}

#[derive(Debug, Clone)]
pub enum LLVM {
    DeclFunction { decl: String },
    DeclStruct { decl: StructDecl },
    DeclString { decl: StringDecl },
    Function { def: FunctionDef },
}
