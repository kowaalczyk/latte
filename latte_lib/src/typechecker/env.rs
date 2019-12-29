use crate::parser::ast::{Program, Function, Type};
use crate::typechecker::mapper::TypeCheckResult;
use crate::error::{FrontendErrorKind, FrontendError};
use crate::meta::{Meta, LocationMeta};
use crate::util::env::Env;

/// checks if main function is defined and has a correct signature
pub fn check_main(program: &Program<LocationMeta>) -> Result<(), Vec<FrontendError<LocationMeta>>> {
    if let Some(func) = program.functions.get("main") {
        if func.item.args.is_empty() {
            if func.item.ret == Type::Int {
                Ok(())
            } else {
                let kind = FrontendErrorKind::TypeError {
                    expected: Type::Int,
                    actual: func.item.ret.clone()
                };
                Err(vec![FrontendError::new(kind, func.get_meta().clone())])
            }
        } else {
            let kind = FrontendErrorKind::EnvError {
                message: String::from("Function 'main' cannot take any arguments")
            };
            Err(vec![FrontendError::new(kind, func.get_meta().clone())])
        }
    } else {
        let kind = FrontendErrorKind::EnvError {
            message: String::from("Function 'main' not defined")
        };
        Err(vec![FrontendError::new(kind,  LocationMeta { offset: 0 })])
    }
}

/// checks if no builtin method is overwritten
pub fn check_builtin_conflicts(
    program: &Program<LocationMeta>, builtins: &Env<Type>
) -> Result<(), Vec<FrontendError<LocationMeta>>> {
    let overwritten_builtins: Vec<_> = program.functions
        .keys()
        .filter(|f| builtins.contains_key(f.clone()))
        .collect();
    if overwritten_builtins.is_empty() {
        Ok(())
    } else {
        let errors: Vec<_> = overwritten_builtins.iter()
            .map(|f| {
                let kind = FrontendErrorKind::EnvError {
                    message: format!("Function {} shadows built-in function", f.clone())
                };
                FrontendError::new(
                    kind,
                    program.functions.get(f.clone()).unwrap().get_meta().clone()
                )
            })
            .collect();
        Err(errors)
    }
}
