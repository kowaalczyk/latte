use crate::parser::ast;
use crate::compiler::ir::{Entity, Struct, Function};
use crate::util::env::Env;

#[derive(Clone)]
pub struct Compiler {
    /// next available register
    available_reg: usize,

    /// unique identifier for a next set of labels (if or loop branches)
    available_label: usize,

    /// local variable environment
    local_env: Env<Entity>,

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
            local_env: Env::new(),
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

    /// matches available register from the other compiler
    pub fn match_available_reg(&mut self, other: &Self) {
        self.available_reg = other.available_reg;
    }

    /// get new unique suffix for a label
    pub fn get_label_suffix(&mut self) -> usize {
        let label = self.available_label;
        self.available_label += 1;
        label
    }

    /// get entity representing a pointer to a variable with given identifier
    pub fn get_ptr(&self, ident: &String) -> &Entity {
        self.local_env.get(ident).unwrap()
    }

    /// set pointer to a variable in a local environment, given an entity that represents it
    pub fn set_ptr(&mut self, ident: &String, ent: &Entity) {
        self.local_env.insert(ident.clone(), ent.clone());
    }

    /// get name of the function that creates a class instance by class name
    pub fn get_init(&self, class_name: &String) -> String {
        format!("__init__{}", class_name)
    }

    /// get mangled function name from its source code identifier
    pub fn get_function(&self, func_name: &String) -> String {
        format!("__func__{}", func_name)
    }
}
