use crate::ast::Program;
use crate::latte::ProgramParser;
use crate::error::FrontendError;


pub fn parse_program(source_code: String) -> Result<Program, Vec<FrontendError<usize>>> {
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
    use crate::ast;

    #[test]
    fn empty_program_fails() -> Result<(), String> {
        let result = parse_program(String::from(""));
        match result {
            Ok(_) => Err(String::from("Empty program should not be parsed")),
            Err(errors) => {
                match errors.get(0) {
                    Some(e) => {
                        if e.location == 0usize {
                            Ok(())
                        } else {
                            Err(format!("Invalid error location, expected {} got P{}", 0, e.location))
                        }
                    },
                    None => Err(String::from("Missing ParseError in parsing results"))
                }
            }
        }
    }

    #[test]
    fn simple_main_parses() -> Result<(), Vec<FrontendError<usize>>> {
        let code = r#"
        int main() {
            return 0;
        }
        "#;
        let code_str = String::from(code);
        let main_block = ast::Block {
            stmts: vec![
                Box::new(ast::Statement::Return { 
                    expr: Some(Box::new(ast::Expression::LitInt { val: 0 }))
                })
            ]
        };
        let main_fn = ast::TopDef::Function {
            ret: ast::Type::Int,
            ident: String::from("main"),
            args: vec![],
            block: main_block,
        };
        let expected_ast = ast::Program {
            topdefs: vec![main_fn]
        };
        let actual_ast = parse_program(code_str)?;
        assert_eq!(expected_ast, actual_ast);
        Ok(())
    }
}
