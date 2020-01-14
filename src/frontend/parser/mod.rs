use crate::frontend::error::FrontendError;
use crate::meta::LocationMeta;

use self::ast::Program;
use self::latte::ProgramParser;

mod latte;

pub mod ast;


pub type ParsedProgram = Program<LocationMeta>;
pub type ParserErrors = Vec<FrontendError<LocationMeta>>;


pub fn parse_program(source_code: String) -> Result<ParsedProgram, ParserErrors> {
    let mut errors = Vec::new();
    let parser = ProgramParser::new();
    match parser.parse(&mut errors, &source_code) {
        Ok(program) => {
            if errors.is_empty() {
                Ok(program)
            } else {
                Err(errors)
            }
        }
        Err(_) => {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

// use crate::ast;

    #[test]
    fn empty_program_fails() -> Result<(), String> {
        let result = parse_program(String::from(""));
        match result {
            Ok(_) => Err(String::from("Empty program should not be parsed")),
            Err(errors) => {
                match errors.get(0) {
                    Some(e) => {
                        if e.get_meta().offset == 0usize {
                            Ok(())
                        } else {
                            Err(format!("Invalid error location, expected {} got {:?}", 0, e.get_meta()))
                        }
                    }
                    None => Err(String::from("Missing ParseError in parsing results"))
                }
            }
        }
    }
}
