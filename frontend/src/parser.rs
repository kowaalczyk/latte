use crate::ast::Program;
use crate::latte::ProgramParser;
use crate::error::FrontendError;


pub fn parse_program(source_code: String) -> Result<Program, Vec<FrontendError>> {
    let mut errors = Vec::new();
    let parser = ProgramParser::new();
    match parser.parse(&mut errors, &source_code) {
        Ok(program) => {
            if errors.is_empty() {
                Ok(program)
            } else {
                Err(errors)
            }
        },
        Err(_) => {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_program_fails() -> Result<(), String> {
        let result = parse_program(String::from(""));
        match result {
            Ok(_) => Err(String::from("Empty program should not be parsed")),
            Err(errors) => {
                match errors.get(0) {
                    Some(FrontendError::ParseError { message: _, location }) => {
                        if *location == 0usize {
                            Ok(())
                        } else {
                            Err(format!("Invalid error location, expected {} got P{}", 0, location))
                        }
                    },
                    Some(_) => Err(String::from("Invalid error type, expected ParseError")),
                    None => Err(String::from("Missing ParseError in parsing results"))
                }
            }
        }
    }
}
