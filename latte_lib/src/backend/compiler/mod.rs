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

const BUILTINS: &str = r"

declare i8* @__builtin_method__str__init__(i32)
declare i8* @__builtin_method__str_const__(i8*)
declare i8* @__builtin_method__str__concat__(i8*, i8*)

declare void @__func__printInt(i32)
declare void @__func__printString(i8*)
declare void @__func__error()
declare i32 @__func__readInt()
declare i8* @__func__readString()

";

pub fn compile(program: &Program<TypeMeta>) -> String {
    let mut compiler = Compiler::new();
    let compiled_program = program.functions.values()
        .map(|func| {
            compiler.visit_function(func)
        })
        .filter_map(CompilationResult::llvm)
        .flatten()
        .map(|i| i.to_string())
        .join("\n");
    String::from(BUILTINS) + &compiled_program + "\n"
}
