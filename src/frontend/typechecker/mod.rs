use crate::frontend::ast::Program;
use crate::meta::{LocationMeta, TypeMeta};
use crate::util::mapper::AstMapper;

use self::env::check_builtin_conflicts;
use self::mapper::TypeCheckResult;
use self::typechecker::TypeChecker;
use self::util::get_builtins;

mod util;
mod mapper;
mod env;
mod typechecker;


/// main typechecker function: checks types of the entire program,
/// converts implicit self-references to current object into explicit ones,
/// converts all references to object members to the typed ones,
/// converts array.length to the dedicated ArrayLength reference
pub fn check_types(program: Program<LocationMeta>) -> TypeCheckResult<Program<TypeMeta>> {
    // get builtin functions and check for duplicate declarations
    let buitlins = get_builtins();
    check_builtin_conflicts(&program, &buitlins)?;

    // create typechecker and iterate over entire program (classes & functions)
    let mut typechecker = TypeChecker::new(&program, &buitlins);
    typechecker.map_program(&program)
}
