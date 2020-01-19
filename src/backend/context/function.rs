use std::collections::HashMap;

use crate::backend::builder::MapEntities;
use crate::backend::ir::{BasicBlock, Entity, Instruction, InstructionKind};
use crate::frontend::ast::Type;
use crate::util::env::Env;

#[derive(Debug, Clone)]
pub struct FunctionContext {
    /// number of next available register
    available_register: usize,

    /// uuid for variables initialized to constant values
    available_uuid: usize,

    /// already compiled basic blocks
    compiled_blocks: Vec<BasicBlock>,
}

impl FunctionContext {
    pub fn new() -> Self {
        Self {
            available_register: 1,
            available_uuid: 1, // 0 is reserved for built-in constants
            compiled_blocks: vec![],
        }
    }

    /// get new numbered register identifier
    pub fn new_register(&mut self, t: Type) -> Entity {
        let register_n = self.available_register;
        self.available_register += 1;

        Entity::Register {
            n: register_n,
            t,
        }
    }

    /// get new unique identifier
    pub fn new_uuid(&mut self) -> usize {
        let uuid = self.available_uuid;
        self.available_uuid += 1;
        uuid
    }

    /// push new, compiled basic block to the function context
    pub fn push_block(&mut self, block: BasicBlock) {
        self.compiled_blocks.push(block);
    }

    /// map entities in instructions from the last compiled block using provided mapping
    pub fn map_entities_in_last_block(&mut self, mapping: &HashMap<Entity, Entity>) {
        let mut last_block = self.compiled_blocks.pop().unwrap();
        last_block = last_block.map_entities(0, mapping);
        self.compiled_blocks.push(last_block);
    }

    /// concludes the current block and returns all compiled blocks
    pub fn conclude(&mut self) -> Vec<BasicBlock> {
        self.compiled_blocks.drain(..).collect()
    }
}
