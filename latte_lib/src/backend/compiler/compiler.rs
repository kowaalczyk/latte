use crate::util::env::Env;
use crate::backend::compiler::ir::{Entity, LLVM};


#[derive(Clone)]
pub struct Compiler {
    /// next available register
    available_reg: usize,

    /// unique identifier for a next set of labels (if or loop branches)
    available_label: usize,

    /// next free name for const string literal
    available_const: usize,

    /// local variable environment
    local_env: Env<Entity>,

    /// declarations of functions, global constants, etc
    declarations: Vec<LLVM>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            available_reg: 1,
            available_label: 1,
            available_const: 1,
            local_env: Env::new(),
            declarations: Vec::new(),
        }
    }

    /// construct a compiler with a pre-defined vector of declarations
    pub fn with_declarations(declarations: Vec<LLVM>) -> Self {
        let mut compiler = Self::new();
        compiler.declarations = declarations;
        compiler
    }

    /// creates a compiler with higher inital available_reg
    pub fn with_starting_reg(reg: usize) -> Self {
        let mut compiler = Self::new();
        compiler.available_reg = reg;
        compiler
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

    /// get new name for a const string literal
    pub fn new_const(&mut self) -> String {
        let ord = self.available_const;
        self.available_const += 1;
        format!(".str.{}", ord)  // same convention as clang uses for C strings
    }

    /// add new declaration to the list
    pub fn add_decl(&mut self, decl: LLVM) {
        self.declarations.push(decl);
    }

    /// combine constants with ones from the other compiler
    pub fn combine_declarations(&mut self, other: &mut Self) {
        self.declarations.append(&mut other.declarations);
    }

    /// get all declarations
    pub fn get_declarations(self) -> Vec<LLVM> {
        self.declarations
    }

    /// get new unique suffix for a label
    pub fn get_label_suffix(&mut self) -> usize {
        let label = self.available_label;
        self.available_label += 1;
        label
    }

    /// get entity representing a pointer to a variable with given identifier
    pub fn get_ptr(&self, ident: &String) -> Entity {
        self.local_env.get(ident).unwrap().clone()
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
        if func_name == "main" {
            // main is the only non-mangled function
            func_name.clone()
        } else {
            format!("__func__{}", func_name)
        }
    }
}
