use crate::backend::context::GlobalContext;
use crate::frontend::ast::{Program, Class, ClassItem};
use crate::meta::{TypeMeta, GetType};
use crate::backend::ir::LLVM;
use crate::backend::compiler::function::FunctionCompiler;
use crate::backend::compiler::class::ClassCompiler;
use crate::util::env::Env;
use std::collections::HashSet;

#[derive(Clone)]
pub struct ProgramCompiler {
    global_context: GlobalContext
}

impl ProgramCompiler {
    /// create a compiler with empty global context
    pub fn new() -> Self {
        Self {
            global_context: GlobalContext::new(),
        }
    }

    /// create a compiler with a pre-defined vector of function declarations
    pub fn with_builtin_functions(declarations: &mut Vec<String>) -> Self {
        let mut compiler = Self::new();
        compiler.global_context.append_function_declarations(declarations);
        compiler
    }

    /// merge with nested compiler that compiled a function
    fn merge_function_compiler(&mut self, function_compiler: FunctionCompiler) {
        self.global_context = function_compiler.get_global_context().clone();
    }

    /// merge with nested compiler that compiled a function
    fn merge_class_compiler(&mut self, class_compiler: ClassCompiler) {
        self.global_context = class_compiler.get_global_context().clone();
    }

    /// apply inheritance and declare all classes' structs
    fn declare_classes(&mut self, classes: &Env<Class<TypeMeta>>) {
        let mut processed_classes = HashSet::new();

        // insert all root tree nodes
        for (class_name, class) in classes {
            if class.item.parent.is_none() {
                self.global_context.declare_class(&class);
                processed_classes.insert(class_name.clone());
            }
        }

        // insert all children
        while classes.len() > processed_classes.len() {
            // this loop is really inefficient, but it's not a problem unless we get > 10k total classes
            for (class_name, class) in classes {
                if let Some(parent) = &class.item.parent {
                    // we apply inheritance only when parent already has all its inherited properties
                    if let Some(parent_cls) = processed_classes.get(parent) {
//                        // inherit all vars from parent, override when there are any conflicts
//                        let mut inherited_vars = parent_cls.item.vars.clone();
//                        for (var_ident, var_item) in class.item.vars {
//                            inherited_vars.insert(var_ident, var_item);
//                        }
//
//                        // do the same for methods
//                        let mut inherited_methods = parent_cls.item.methods.clone();
//                        for (method_name, method_item) in class.item.methods {
//                            inherited_methods.insert(method_name, method_item);
//                        }
//
//                        // build new class and insert to the results collection
//                        let new_class_item = ClassItem {
//                            ident: class_name.clone(),
//                            vars: inherited_vars,
//                            methods: inherited_methods,
//                            parent: None // we no longer need to store parent information
//                        };
//                        let new_class = Class::new(new_class_item, class.get_meta().clone());
                        self.global_context.declare_class(&class);
                        processed_classes.insert(class_name.clone());
                    }
                }
            }
        }
    }

    pub fn compile_program(&mut self, program: Program<TypeMeta>) -> Vec<LLVM> {
        // declare structures for all classes
        self.declare_classes(&program.classes);

        // compile all functions
        let mut compiled_functions: Vec<LLVM> = program.functions.values()
            .map(|func| {
                let mut function_compiler = FunctionCompiler::new(&self.global_context);
                let compiled_function = function_compiler.compile_function(func.clone());
                self.merge_function_compiler(function_compiler);
                compiled_function
            })
            .map(|def| LLVM::Function { def })
            .collect();

        // compile all class methods (incl. init)
        let mut compiled_methods: Vec<LLVM> = program.classes.values()
            .flat_map(|cls| {
                let mut class_compiler = ClassCompiler::new(&self.global_context);
                let compiled_class_functions = class_compiler.compile_class(cls.clone());
                self.merge_class_compiler(class_compiler);
                compiled_class_functions
            })
            .map(|def| LLVM::Function { def })
            .collect();

        // get all global declarations after compilation (so that they contain const string literals)
        let mut declarations_after_compilation = self.global_context.get_declarations();

        // return combined result
        let mut compiled = Vec::new();
        compiled.append(&mut declarations_after_compilation);
        compiled.append(&mut compiled_methods);
        compiled.append(&mut compiled_functions);
        compiled
    }
}
