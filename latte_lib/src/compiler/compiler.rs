use crate::parser::ast;
use crate::compiler::ir::{Entity, Class};

pub struct Compiler {
    available_reg: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Self { available_reg: 0 }
    }

    /// get new (unused) register name for a temporary variable
    pub fn new_reg(&mut self) -> usize {
        let reg = self.available_reg;
        self.available_reg += 1;
        reg
    }

    /// get representation of class that can be compiled to llvm without context
    pub fn get_ir(&self, class_ident: &String) -> Class {
        unimplemented!()  // TODO
    }
}
