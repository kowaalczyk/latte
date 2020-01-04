mod ir;
mod compiler;
mod visitor;
mod llvm;


use itertools::Itertools;

use crate::meta::TypeMeta;
use crate::util::visitor::AstVisitor;
use crate::frontend::ast::Program;
use crate::backend::compiler::compiler::Compiler;
use crate::backend::compiler::llvm::ToLLVM;


pub fn compile(program: &Program<TypeMeta>) -> String {
    let mut compiler = Compiler::new();
    program.functions.values()
        .map(|func| {
            compiler.visit_function(func).to_llvm()
        })
        // TODO: Remove print below (used only for debugging)
        .map(|i| {
            eprintln!("{}", i);
            i
        })
        .join("\n")
}
