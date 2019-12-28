mod util;
mod mapper;
mod env;
mod typechecker;

use crate::parser::ast::{Program, Type};
use crate::util::visitor::AstVisitor;
use crate::typechecker::typechecker::TypeChecker;
use crate::typechecker::mapper::TypeCheckResult;
use crate::typechecker::util::get_builtins;
use crate::typechecker::env::{check_builtin_conflicts, check_main};

/// main typechecker function: checks types of the entire program
pub fn check_types(program: &Program) -> TypeCheckResult {
    // get builtin functions and check for duplicate declarations
    let buitlins = get_builtins();
    check_builtin_conflicts(&program, &buitlins)?;

    // create typechecker and iterate over entire program (classes & functions)
    let mut typechecker = TypeChecker::new(program, &buitlins);
    let mut errors: Vec<_> = program.classes
        .values()
        .map(|cls| {
            typechecker.with_clean_env().map_class(&cls.item)
        })
        .filter_map(Result::err)
        .map(Vec::into_iter)
        .flatten()
        .collect();
    let mut func_errors: Vec<_> = program.functions
        .values()
        .map(|func| {
            typechecker.with_clean_env().map_function(&func.item)
        })
        .filter_map(Result::err)
        .map(Vec::into_iter)
        .flatten()
        .collect();
    errors.append(&mut func_errors);

    // check main function (required program entrypoint)
    if let Err(mut main_errors) = check_main(&program) {
        errors.append(&mut main_errors);
    }

    // accept the entire program or summarize all errors
    if errors.is_empty() {
        Ok(Type::Void)
    } else {
        Err(errors)
    }
}
