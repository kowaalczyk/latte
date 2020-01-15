use crate::backend::ir::{BasicBlock, Instruction, InstructionKind};

#[derive(Debug, Clone)]
pub struct FunctionContext {
    /// basic block to which we're actively writing instructions
    current_block: Option<BasicBlock>,

    /// number of next available register
    available_register: usize,

    /// already compiled basic blocks
    compiled_blocks: Vec<BasicBlock>,
}

impl FunctionContext {
    pub fn new(starting_reg: usize) -> Self {
        Self {
            current_block: Some(BasicBlock { label: None, instructions: vec![] }),
            available_register: starting_reg,
            compiled_blocks: vec![]
        }
    }

    /// get new numbered register identifier
    pub fn new_register(&mut self) -> usize {
        let register = self.available_register;
        self.available_register += 1;
        register
    }

    /// add new instruction to the current basic block
    pub fn push_instruction(&mut self, instr: Instruction) {
        self.current_block.as_mut().unwrap().instructions.push(instr)
    }

    /// check if current block always returns
    pub fn current_block_always_returns(&self) -> bool {
        if let Some(block) = &self.current_block {
            if let Some(last_instr) = block.instructions.last() {
                match last_instr.item {
                    InstructionKind::RetVoid => true,
                    InstructionKind::RetVal { .. } => true,
                    _ => false,
                }
            } else {
                false
            }
        } else {
            panic!("Cannot check return value without current_block")
        }
    }

    /// concludes the current block and creates a new one, with specified label
    pub fn next_block(&mut self, label: String) {
        if let Some(block) = &mut self.current_block {
            self.compiled_blocks.push(block.clone());

            let new_block = BasicBlock {
                label: Some(label),
                instructions: vec![]
            };
            self.current_block = Some(new_block);
        } else {
            panic!("Cannot conclude current block: it doesn't exist")
        }
    }

    /// concludes the current block and returns all compiled blocks
    pub fn conclude(&mut self) -> Vec<BasicBlock> {
        if let Some(block) = &mut self.current_block {
            self.compiled_blocks.push(block.clone());
            self.current_block = None;
        } else {
            panic!("Cannot conclude current block: it doesn't exist")
        }
        self.compiled_blocks.drain(..).collect()
    }
}
