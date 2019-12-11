use crate::sourcemap::clean_comments;
use crate::parser::parse_program;
use crate::ast::Program;
use crate::error::FrontendError;


pub fn process_code(source_code: &String) -> Result<Program, Vec<FrontendError>> {
    let (clean_code, source_map) = clean_comments(source_code);
    let ast = parse_program(clean_code)?;
    Ok(ast)
    // TODO: build envs
    // TODO: check types
}
