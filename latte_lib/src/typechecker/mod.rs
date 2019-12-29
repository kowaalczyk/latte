mod util;
mod mapper;
mod env;
mod typechecker;

use crate::parser::ast::{Program, Type};
use crate::typechecker::typechecker::TypeChecker;
use crate::typechecker::mapper::TypeCheckResult;
use crate::typechecker::util::{get_builtins};
use crate::typechecker::env::{check_builtin_conflicts, check_main};
use crate::util::mapper::AstMapper;
use crate::meta::{Meta, LocationMeta, TypeMeta};

/// main typechecker function: checks types of the entire program
pub fn check_types(program: &Program<LocationMeta>) -> TypeCheckResult<Program<TypeMeta>> {
    // get builtin functions and check for duplicate declarations
    let buitlins = get_builtins();
    check_builtin_conflicts(&program, &buitlins)?;  // TODO: Move to preprocessor

    // create typechecker and iterate over entire program (classes & functions)
    let mut typechecker = TypeChecker::new(program, &buitlins);
    typechecker.map_program(program)
}
