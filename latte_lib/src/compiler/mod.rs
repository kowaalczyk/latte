use itertools::Itertools;

use crate::meta::TypeMeta;
use crate::parser::ast::Program;
use crate::compiler::compiler::Compiler;
use crate::util::visitor::AstVisitor;
use crate::compiler::llvm::ToLLVM;

mod ir;
mod compiler;
mod visitor;
mod llvm;

pub fn compile(program: Program<TypeMeta>) -> String {
    let mut compiler = Compiler::new();
    program.functions.values()
        .map(|func| {
            compiler.visit_function(func).to_llvm()
        })
        .join("\n")
}
