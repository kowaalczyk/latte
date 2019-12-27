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
    for class in program.classes.values() {
        typechecker.with_clean_env().visit_class(&class.item)?;
    }
    for function in program.functions.values() {
        typechecker.with_clean_env().visit_function(&function.item)?;
    }
    Ok(Type::Void)
}
