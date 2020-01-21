use crate::backend::context::GlobalContext;
use crate::backend::ir::{FunctionDef, StructDecl, InstructionKind, Entity, BasicBlock, LLVM};
use crate::frontend::ast::{Type, Class, Keyed};
use crate::backend::compiler::function::FunctionCompiler;
use crate::meta::{TypeMeta, GetType};

pub struct ClassCompiler {
    global_context: GlobalContext
}

impl ClassCompiler {
    pub fn new(global_context: &GlobalContext) -> Self {
        // TODO: Store global_context as reference to avoid unnecessary copying
        Self {
            global_context: global_context.clone()
        }
    }

    pub fn get_global_context(&self) -> &GlobalContext {
        &self.global_context
    }

    /// creates and compiles init function, used to initialize the struct representing a class
    pub fn compile_init_function(
        &mut self, func_name: String, struct_decl: StructDecl, struct_t: Type
    ) -> FunctionDef {
        // prepare necessary entities
        let load_ptr = Entity::GlobalConstInt { name: struct_decl.size_constant_name };
        let load_ent = Entity::Register { n: 1, t: Type::Int };
        let array_init_ent = Entity::Register { n: 2, t: Type::Str };
        let return_ent = Entity::Register { n: 3, t: struct_t.clone() };

        let instructions = vec![
            // first, we load global constant representing the structure size
            InstructionKind::Load {
                ptr: load_ptr
            }.with_result(load_ent.clone()),
            // we use array init as a shorthand for malloc and memset
            InstructionKind::Call {
                func: String::from("__builtin_method__array__init__"),
                args: vec![load_ent]
            }.with_result(array_init_ent.clone()),
            // before returning, we cast the result to appropriate type
            InstructionKind::BitCast {
                ent: array_init_ent,
                to: struct_t.clone()
            }.with_result(return_ent.clone()),
            InstructionKind::RetVal {
                val: return_ent
            }.without_result()
        ];
        let func_def = FunctionDef {
            name: func_name,
            ret_type: struct_t,
            args: vec![],
            body: vec![BasicBlock {
                label: None,
                instructions
            }]
        };
        func_def
    }

    pub fn compile_class(&mut self, class: Class<TypeMeta>) -> Vec<FunctionDef> {
        let struct_decl = self.global_context.get_struct_decl(class.get_key());
        let mut compiled_functions = Vec::new();

        let init_func = self.compile_init_function(
            self.global_context.get_init_name(class.get_key()), struct_decl, class.get_type()
        );
        compiled_functions.push(init_func);

        // TODO: Compile user-defined methods

        compiled_functions
    }
}
