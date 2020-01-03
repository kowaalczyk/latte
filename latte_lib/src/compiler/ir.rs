/// internal type representation (as argument or object member):
/// string: pointer to an array of characters, passed by value
/// int, bool: passed by value
/// array: pointer to array, passed by value
/// object: pointer to struct, passed by value

use crate::util::env::Env;
use crate::parser::ast::{ClassVarItem, Type, UnaryOperator, BinaryOperator};
use crate::meta::Meta;

#[derive(Debug, Clone, PartialEq)]
pub enum Entity {
    /// special pointer to null value
    Null,

    /// registers store intermediate computation results
    Register { n: usize, t: Type },

    /// constant (literal) values are preserved whenever types are compatible with LLVM
    /// and represented as constant pointers when they are not (ie. for objects)
    Const { val: BasicValue },
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum BasicValue {
    Bool { v: bool },
    Int { v: i32 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum InstructionKind {
    /// allocate constant string value on heap memory
    StrAlloc {
        /// constant value which will be stored in memory
        val: String,
    },

    /// %result_reg = op op_type arg_ent
    ApplyUnaryOp {
        op: UnaryOperator,
        arg_ent: Entity,
    },

    /// %result_reg = op op_type left_ent, right_ent
    ApplyBinaryOp {
        op: BinaryOperator,
        left_ent: Entity,
        right_ent: Entity,  // TODO: Remember about lazy evaluation
    },

    /// load value from memory to register
    Load {
        ptr: String,
    },

    /// store value to memory / variable
    Store {
        ptr: String,
        val: Entity,
    },

    /// call a function or method
    Call {
        func_name: String,
        args: Vec<Entity>,
    },

    /// return an entity
    ReturnEnt {
        val: Entity,
    },

    /// return void
    ReturnVoid,

    /// conditional jump to true / false label
    JumpCond {
        cond: Entity,
        true_label: String,
        false_label: String,
    },

    /// unconditional jump to label
    Jump {
        label: String,
    },

    /// label definition
    Label {
        val: String,
    },

}

#[derive(Debug, Clone, PartialEq)]
pub struct InstructionMeta {
    /// registry where result is stored
    pub reg: usize,

    /// type of result
    pub t: Type,
}
pub type Instruction = Meta<InstructionKind, Option<InstructionMeta>>;

impl From<InstructionMeta> for Entity {
    fn from(meta: InstructionMeta) -> Self {
        Entity::Register {
            n: meta.reg,
            t: meta.t
        }
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
