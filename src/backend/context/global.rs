use crate::backend::ir::{LLVM, StringDecl, StructDecl, VTableDecl};
use crate::util::env::Env;
use crate::frontend::ast::{Class, Keyed, ClassVar, Type};
use crate::meta::{TypeMeta, GetType};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct GlobalContext {
    /// string value to string declaration mapping
    string_declarations: Env<StringDecl>,

    /// LLVM IR String representation of function declarations
    function_declarations: Vec<String>,

    /// struct name to struct declaration mapping
    struct_declarations: Env<StructDecl>,

    /// struct name to struct vtable declaration mapping
    struct_vtable_declarations: Env<VTableDecl>,

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
            struct_vtable_declarations: Env::new(),
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

    fn get_parent_struct_decl(&self, cls: &Class<TypeMeta>) -> Option<StructDecl> {
        if let Some(parent_cls) = &cls.item.parent {
            self.struct_declarations.get(parent_cls).cloned()
        } else {
            None
        }
    }

    fn get_parent_struct_vtable(&self, cls: &Class<TypeMeta>) -> Option<VTableDecl> {
        if let Some(parent_cls) = &cls.item.parent {
            self.struct_vtable_declarations.get(parent_cls).cloned()
        } else {
            None
        }
    }

    /// add new class declaration, get a StructDecl object corresponding to the new LLVM IR struct,
    /// new vtable type for the class methods and the corresponding vtable declaration
    /// (function definitions for class methods are compiled separately)
    pub fn declare_class(&mut self, cls: &Class<TypeMeta>) -> StructDecl {
        let class_name = cls.item.get_key();

        // create a vtable for current class, unifying it with parent class to preserve order
        let mut method_types = Vec::new();
        let mut method_declarations = Vec::new();
        let mut method_env = Env::new();

        if let Some(parent_vtable) = self.get_parent_struct_vtable(cls) {
            for (method_type, method_name) in parent_vtable.methods {
                method_declarations.push((method_type.clone(), method_name));
                method_types.push(Box::new(method_type));
            }
            for (k, v) in parent_vtable.method_env {
                method_env.insert(k, v);
            }
        }

        for (method_name, method) in cls.item.methods.iter() {
            let actual_method_idx = if let Some(parent_method_idx) = method_env.get(method_name) {
                // replacing method with same name, defined in the parent class
                *parent_method_idx
            } else {
                // adding new method
                method_declarations.len() as i32
            };
            method_env.insert(method_name.clone(), actual_method_idx);

            let method_t = method.get_type();
            method_types.insert(actual_method_idx as usize, Box::new(method_t.clone()));

            let method_name = self.get_method_name(class_name, method_name);
            method_declarations.insert(actual_method_idx as usize, (method_t, method_name));
        }
        let vtable_decl = VTableDecl {
            name: self.get_vtable_struct_name(class_name),
            data_const_name: self.get_vtable_struct_const(class_name),
            methods: method_declarations,
            method_env
        };
        self.struct_vtable_declarations.insert(class_name.clone(), vtable_decl);

        // build LLVM representation of the structure
        let mut fields = Vec::new();
        fields.push(Type::BuiltinClass { ident: self.get_vtable_struct_name(class_name) });
        let mut field_env = Env::new();

        if let Some(parent_decl) = self.get_parent_struct_decl(cls) {
            for field in parent_decl.fields[1..].iter() {
                fields.push(field.clone())
            }
            for (field_name, field_idx) in parent_decl.field_env {
                field_env.insert(field_name, field_idx);
            }
        }

        for (field_name, field_var) in cls.item.vars.iter() {
            let field_idx = if let Some(_) = field_env.get(field_name) {
                // unlike methods, fields cannot be replaced (and this should've been caught by typechecker)
                panic!("subclass {} defined field {} which replaces same field in parent class", class_name, field_name)
            } else {
                fields.len() as i32
            };
            let stored_field_t = field_var.get_type().clone();
            fields.insert(field_idx as usize, stored_field_t);
            // we offset field_idx by 1 to account for vtable at the beginning of the array
            field_env.insert(field_name.clone(), field_idx);
        }

        let new_struct = StructDecl {
            name: self.get_struct_name(class_name),
            size_constant_name: self.get_size_constant_name(class_name),
            fields,
            field_env,
        };
        self.struct_declarations.insert(class_name.clone(), new_struct.clone());
        new_struct
    }

    pub fn get_struct_name(&self, class_name: &String) -> String {
        format!("__class__{}", class_name)
    }

    pub fn get_size_constant_name(&self, class_name: &String) -> String {
        format!("__sizeof__{}", class_name.clone())
    }

    pub fn get_vtable_struct_name(&self, class_name: &String) -> String {
        format!("__vtable_type__{}", class_name)
    }

    pub fn get_vtable_struct_const(&self, class_name: &String) -> String {
        format!("__vtable_const__{}", class_name)
    }

    /// get name of LLVM function that represents the given method for the given class
    fn get_method_name(&self, class_name: &String, method_name: &String) -> String {
        format!("__method__{}__{}", class_name, method_name)
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
    pub fn get_init_name(&self, class_name: &String) -> String {
        format!("__init__{}", class_name)
    }

    /// get mangled function name from its source code identifier
    pub fn get_function_name(&self, func_name: &String) -> String {
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
        let vtable_decl = self.struct_vtable_declarations.values()
            .map(|decl| LLVM::DeclVTable{ decl: decl.clone() });
        let array_struct_decl = self.array_struct_definitions.values()
            .map(|decl| LLVM::DeclStruct{ decl: decl.clone() });
        llvm_func_decl
            .chain(llvm_str_decl)
            .chain(llvm_struct_decl)
            .chain(vtable_decl)
            .chain(array_struct_decl)
            .collect()
    }
}
