use crate::backend::ir::{LLVM, StringDecl, StructDecl};
use crate::util::env::Env;
use crate::frontend::ast::{Class, Keyed, ClassVar, Type};
use crate::meta::{TypeMeta, GetType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GlobalContext {
    /// string value to string declaration mapping
    string_declarations: Env<StringDecl>,

    /// LLVM IR String representation of function declarations
    function_declarations: Vec<String>,

    /// struct name to struct declaration mapping
    struct_declarations: Env<StructDecl>,

    /// structs representing arrays have dedicated environment
    array_struct_definitions: HashMap<Type, StructDecl>,

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
            struct_declarations: Env::new(),
            array_struct_definitions: HashMap::new(),
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

    /// add new class declaration, get a StructDecl object corresponding to the new LLVM IR struct
    pub fn declare_class(&mut self, s: &Class<TypeMeta>) -> StructDecl {
        let struct_name = s.item.get_key();

        let mut fields = Vec::new();
        let mut field_env = Env::new();
        for (field_idx, (field_name, field_var)) in s.item.vars.iter().enumerate() {
            // objects are always stored as references
            let stored_field_t = field_var.get_type().clone();
            fields.push(stored_field_t);
            field_env.insert(field_name.clone(), field_idx as i32);
        }

        let new_struct = StructDecl {
            name: format!("__class__{}", struct_name.clone()),
            size_constant_name: format!("__sizeof__{}", struct_name.clone()),
            fields,
            field_env,
        };
        self.struct_declarations.insert(struct_name.clone(), new_struct.clone());
        new_struct
    }

    /// get declaration of a struct representing an array type
    pub fn get_or_declare_array_struct(&mut self, item_t: &Type) -> StructDecl {
        if let Some(s) = self.array_struct_definitions.get(item_t) {
            s.clone()
        } else {
            let mut field_env = Env::new();
            field_env.insert(String::from("length"), 0);
            field_env.insert(String::from("array"), 1);

            let array_struct = StructDecl {
                // TODO: Ensure if there are no special symbols in type name (!!!)
                name: self.get_array_struct_name(item_t),
                size_constant_name: self.get_array_struct_size_name(item_t),
                fields: vec![Type::Int, Type::Reference { t: Box::new(item_t.clone()) }],
                field_env
            };

            self.array_struct_definitions.insert(item_t.clone(), array_struct.clone());
            array_struct
        }
    }

    pub fn get_array_struct_name(&self, item_t: &Type) -> String {
        format!("__builtin_struct__array_{}", item_t).replace("*", "ptr")
    }

    pub fn get_array_struct_size_name(&self, item_t: &Type) -> String {
        format!("__builtin_sizeof__array_{}", item_t).replace("*", "ptr")
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

    /// get struct declaration from the original class identifier
    pub fn get_struct_decl(&self, class_ident: &String) -> StructDecl {
        self.struct_declarations.get(class_ident).unwrap().clone()
    }

    /// get all global declarations
    pub fn get_declarations(&self) -> Vec<LLVM> {
        let llvm_func_decl = self.function_declarations.iter()
            .map(|decl| LLVM::DeclFunction { decl: decl.clone() });
        let llvm_str_decl = self.string_declarations.values()
            .map(|decl| LLVM::DeclString { decl: decl.clone() });
        let llvm_struct_decl = self.struct_declarations.values()
            .map(|decl| LLVM::DeclStruct{ decl: decl.clone() });
        let array_struct_decl = self.array_struct_definitions.values()
            .map(|decl| LLVM::DeclStruct{ decl: decl.clone() });
        llvm_func_decl
            .chain(llvm_str_decl)
            .chain(llvm_struct_decl)
            .chain(array_struct_decl)
            .collect()
    }
}
