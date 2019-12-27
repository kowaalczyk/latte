mod util;
mod visitor;
mod typechecker;

use crate::parser::ast::{Program, Type};
use crate::util::visitor::AstVisitor;
use crate::typechecker::typechecker::TypeChecker;
use crate::typechecker::visitor::TypeCheckResult;
use crate::typechecker::util::get_builtins;

/// main typechecker function: checks types of the entire program
pub fn check_types(program: &Program) -> TypeCheckResult {
    let buitlins = get_builtins();
    // TODO: Check if there are no conflicts with builtin functions
    // TODO: Check if main function exists and has correct types
    let mut typechecker = TypeChecker::new(program, &buitlins);
    let mut errors: Vec<_> = program.classes
        .values()
        .map(|cls| {
            typechecker.with_clean_env().visit_class(&cls.item)
        })
        .filter_map(Result::err)
        .map(Vec::into_iter)
        .flatten()
        .collect();
    let mut func_errors: Vec<_> = program.functions
        .values()
        .map(|func| {
            typechecker.with_clean_env().visit_function(&func.item)
        })
        .filter_map(Result::err)
        .map(Vec::into_iter)
        .flatten()
        .collect();
//    for class in program.classes.values() {
//        typechecker.with_clean_env().visit_class(&class.item)?;
//    }
//    for function in program.functions.values() {
//        typechecker.with_clean_env().visit_function(&function.item)?;
//    }
    errors.append(&mut func_errors);
    if errors.is_empty() {
        Ok(Type::Void)
    } else {
        Err(errors)
    }
}
