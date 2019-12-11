use std::fmt;

use lalrpop_util::{ErrorRecovery, ParseError as LalrpopError};

use crate::ast;


#[derive(Debug)]
pub enum FrontendError {
    ParseError {
        message: String,
        location: usize,
    },
    TypeError {
        expected: ast::Type,
        actual: ast::Type,
        location: usize,
    },
}

// TODO: Use CodeMap in application layer to print file, row and column isntead of usize offset
impl fmt::Display for FrontendError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FrontendError::ParseError { message, location } => {
                write!(f, "{} ParseError: {}", location, message)
            },
            FrontendError::TypeError { expected, actual, location } => {
                write!(f, "{} TypeError: expected `{:?}`, got `{:?}`", location, expected, actual)
            },
        }
    }
}

impl<T: fmt::Debug, E: fmt::Debug> From<(ErrorRecovery<usize, T, E>)> for FrontendError {
    fn from(err: ErrorRecovery<usize,T,E>) -> Self {
        let (location, message) = match &err.error {
            LalrpopError::InvalidToken { location } => {
                (location.clone(), String::from("InvalidToken"))
            },
            LalrpopError::UnrecognizedEOF { location, expected: _ } => {
                (location.clone(), String::from("Unexpected end of file"))
            },
            LalrpopError::ExtraToken { token } => {
                (token.0.clone(), format!("ExtraToken: {:?}", token.1))
            },
            LalrpopError::UnrecognizedToken { token, expected: _ } => {
                (token.0.clone(), format!("UnrecognizedToken: {:?}", token.1))
            },
            LalrpopError::User { error } => {
                panic!("Impossible: Undefined lalrpop user error: {:#?}", error)
            }
        };
        FrontendError::ParseError {
            message,
            location
        }
    }
}
