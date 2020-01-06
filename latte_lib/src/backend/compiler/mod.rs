mod ir;
mod compiler;
mod visitor;
mod display;


use std::string::ToString;

use itertools::Itertools;

use crate::meta::TypeMeta;
use crate::util::visitor::AstVisitor;
use crate::frontend::ast::Program;
use crate::backend::compiler::compiler::Compiler;
use crate::backend::compiler::visitor::CompilationResult;
use crate::backend::compiler::ir::LLVM;


pub fn compile(program: &Program<TypeMeta>) -> String {
    let builtins = vec![
        LLVM::FuncDecl { decl: String::from("declare i8* @__builtin_method__str__init__(i32)") },
        LLVM::FuncDecl { decl: String::from("declare i8* @__builtin_method__str__concat__(i8*, i8*)") },
        LLVM::FuncDecl { decl: String::from("declare void @__func__printInt(i32)") },
        LLVM::FuncDecl { decl: String::from("declare void @__func__printString(i8*)") },
        LLVM::FuncDecl { decl: String::from("declare void @__func__error()") },
        LLVM::FuncDecl { decl: String::from("declare i32 @__func__readInt()") },
        LLVM::FuncDecl { decl: String::from("declare i8* @__func__readString()") },
    ];
    let mut compiler = Compiler::with_declarations(builtins);

    let compiled_program = program.functions.values()
        .map(|func| {
            compiler.visit_function(func)
        })
        .filter_map(CompilationResult::llvm)
        .flatten()
        .map(|i| i.to_string())
        .join("\n");
    let compiled_declarations = compiler.get_declarations().iter()
        .map(|i| i.to_string())
        .join("\n");
    compiled_declarations + "\n" + &compiled_program + "\n"
}
