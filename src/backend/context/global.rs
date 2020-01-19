use crate::backend::ir::{LLVM, StringDecl};
use crate::util::env::Env;

#[derive(Debug, Clone)]
pub struct GlobalContext {
    /// string value to string declaration mapping
    string_declarations: Env<StringDecl>,

    /// LLVM IR String representation of function declarations
    function_declarations: Vec<String>,

    /// number for next available global constant
    available_const: usize,

    /// unique suffix that can be used for new label (or set of labels)
    available_label_suffix: usize,
}

impl GlobalContext {
    pub fn new() -> Self {
        Self {
            string_declarations: Env::new(),
            function_declarations: vec![],
            available_const: 1,
            available_label_suffix: 1,
        }
    }

    /// add new constant string to declarations, get a name of declared constant
    pub fn declare_string(&mut self, val: String) -> StringDecl {
        if let Some(existing_decl) = self.string_declarations.get(&val) {
            existing_decl.clone()
        } else {
            let const_name = self.new_const_name();
            let new_decl = StringDecl {
                name: const_name.clone(),
                val: val.clone(),
                len: val.len() - 1,  // -2 for quotes +1 for trailing zero
            };
            self.string_declarations.insert(val, new_decl.clone());
            new_decl
        }
    }

    /// append vector of function declarations to the existing ones
    pub fn append_function_declarations(&mut self, declarations: &mut Vec<String>) {
        self.function_declarations.append(declarations);
    }

    /// get new unique name for a global constant
    fn new_const_name(&mut self) -> String {
        let ord = self.available_const;
        self.available_const += 1;
        format!(".str.{}", ord)  // same convention as clang uses for C strings
    }

    /// get new unique suffix for labels
    pub fn new_label_suffix(&mut self) -> usize {
        let suffix = self.available_label_suffix;
        self.available_label_suffix += 1;
        suffix
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

    /// get all global declarations
    pub fn get_declarations(&self) -> Vec<LLVM> {
        let llvm_func_decl = self.function_declarations.iter()
            .map(|decl| LLVM::DeclFunction { decl: decl.clone() });
        let llvm_str_decl = self.string_declarations.values()
            .map(|decl| LLVM::DeclString { decl: decl.clone() });
        llvm_func_decl.chain(llvm_str_decl).collect()
    }
}
