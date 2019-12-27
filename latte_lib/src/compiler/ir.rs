/// internal type representation (as argument or object member):
/// string: pointer to an array of characters, passed by value
/// int, bool: passed by value
/// array: pointer to array, passed by value
/// object: pointer to struct, passed by value

use crate::util::env::Env;
use crate::parser::ast::{ClassVar, Type, UnaryOperator, BinaryOperator};

pub enum BasicValue {
    Bool { v: bool },
    Int { v: i32 },
}

pub enum Entity {
    /// special pointer to null value
    NullPtr,

    /// pointer names represent local variables
    Ptr { ident: String },

    /// registers store intermediate computation results
    Register { n: usize },

    /// constant (literal) values are preserved whenever types are compatible with LLVM
    /// and represented as constant pointers when they are not (ie. for objects)
    Const { val: BasicValue },
}

pub enum ConstComplexValue {
    /// constant string value
    Str { s: String },

    /// blank object of the given class
    Obj { cls: Class },

// TODO: Arrays - cannot find a nice way of fitting them into this representation :c
//    /// basic value array
//    BasicArr { item_val: BasicValue, len: usize },
//    /// complex value array
//    ComplexArr { item_val: Box<ConstComplexValue>, len: usize },
}

pub enum Instruction {
    /// complex operation: allocate complex type with a constant value
    ConstAlloc {
        /// identifier, such that `Entity::Register { n: register }` points to the allocated memory
        result_reg: usize,

        /// constant value which will be stored in memory
        val: ConstComplexValue,
    },
    /// %result_reg = op op_type arg_ent
    ApplyUnaryOp {
        result_reg: usize,
        op: UnaryOperator,
        arg_ent: Entity,
    },
    /// %result_reg = op op_type left_ent, right_ent
    ApplyBinaryOp {
        result_reg: usize,
        op: BinaryOperator,  // TODO: Handling string operations will be *VERY TRICKY*
        left_ent: Entity,
        right_ent: Entity,
    }
}

// TODO: Add lifetime specifiers to IR structures to make compilation more memory-efficient
pub struct Class {
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

pub struct Function {
    /// when arguments are passed during function call, structs will have to be cast to
    /// appropriate objects (ie, subclass to superclass)
    pub vars: Env<Type>,
}
