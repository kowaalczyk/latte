mod latte;
pub mod ast;

use self::ast::Program;
use self::latte::ProgramParser;
use crate::error::FrontendError;
use crate::meta::LocationMeta;


pub fn parse_program(source_code: String) -> Result<Program<LocationMeta>, Vec<FrontendError<LocationMeta>>> {
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
                    },
                    None => Err(String::from("Missing ParseError in parsing results"))
                }
            }
        }
    }

    // TODO: Needs to be re-implemented to account for locations
    // #[test]
    // fn simple_main_parses() -> Result<(), Vec<FrontendError<usize>>> {
    //     let code = r#"
    //     int main() {
    //         return 0;
    //     }
    //     "#;
    //     let code_str = String::from(code);
    //     let main_block = ast::Block {
    //         stmts: vec![
    //             Box::new(ast::Statement::Return { 
    //                 expr: Some(Box::new(ast::Expression::LitInt { val: 0 }))
    //             })
    //         ]
    //     };
    //     let main_fn = ast::TopDef::Function { func: ast::Function {
    //         ret: ast::Type::Int,
    //         ident: String::from("main"),
    //         args: Env::new(),
    //         block: main_block,
    //     }};
    //     let fenv = Env::new();
    //     fenv.insert_unique(main_fn, )?;
    //     let expected_ast = ast::Program {
    //         classes: Env::new(),
    //         functions: fenv
    //     };
    //     let actual_ast = parse_program(code_str.trim())?;
    //     assert_eq!(expected_ast, actual_ast);
    //     Ok(())
    // }
}
