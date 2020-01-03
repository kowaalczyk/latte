use crate::parser::ast;
use crate::compiler::ir::{Entity, Struct, Function};
use crate::util::env::Env;

pub struct Compiler {
    /// next available register
    available_reg: usize,

    /// unique identifier for a next set of labels (if or loop branches)
    available_label: usize,

    /// compiled representation of structs and class properties
    structs: Env<Struct>,

    /// compiled representation of of functions and class methods
    functions: Env<Function>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            available_reg: 0,
            available_label: 0,
            structs: Env::new(),
            functions: Env::new()
        }
    }

    /// get new (unused) register name for a temporary variable
    pub fn new_reg(&mut self) -> usize {
        let reg = self.available_reg;
        self.available_reg += 1;
        reg
    }

    /// get new unique suffix for a label
    pub fn get_label_suffix(&mut self) -> usize {
        let label = self.available_label;
        self.available_label += 1;
        label
    }

    /// get mangled name of pointer from variable name
    pub fn get_ptr(&self, ident: &String) -> String {
        format!("__ptr__{}", ident)
    }

    /// get name of the function that creates a class instance by class name
    pub fn get_init(&self, class_name: &String) -> String {
        format!("__init__{}", class_name)
    }
}
