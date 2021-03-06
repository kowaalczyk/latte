use itertools::Itertools;

use crate::frontend::ast::Program;
use crate::meta::TypeMeta;

use crate::backend::compiler::ProgramCompiler;

mod compiler;
mod builder;
mod context;
mod ir;

/// compiles the given program, assuming it meets all the necessary criteria (is checked by frontend)
/// into a string containing its LLVM intermediate representation
pub fn compile(program: Program<TypeMeta>) -> String {
    let mut builtins = vec![
        String::from("declare i8* @__builtin_method__str__init__(i32)"),
        String::from("declare i8* @__builtin_method__array__init__(i32)"),
        String::from("declare i8* @__builtin_method__str__concat__(i8*, i8*)"),
        String::from("declare void @__func__printInt(i32)"),
        String::from("declare void @__func__printString(i8*)"),
        String::from("declare void @__func__error()"),
        String::from("declare i32 @__func__readInt()"),
        String::from("declare i8* @__func__readString()"),
    ];
    let mut compiler = ProgramCompiler::with_builtin_functions(&mut builtins);
    compiler.compile_program(program).iter()
        .map(|llvm| llvm.to_string())
        .join("\n")
}
