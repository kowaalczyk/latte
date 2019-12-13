use std::fmt;

use lalrpop_util::{ErrorRecovery, ParseError as LalrpopError};

use crate::ast;
use crate::location::{Located};


#[derive(Debug, PartialEq, Clone)]
pub enum FrontendErrorKind {
    ParseError {
        message: String,
    },
    EnvError {
        message: String,
    },
    TypeError {
        expected: ast::Type,
        actual: ast::Type,
    },
    SystemError {
        message: String
    }
}

impl fmt::Display for FrontendErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FrontendErrorKind::ParseError { message } => {
                write!(f, "ParseError: {}", message)
            },
            FrontendErrorKind::EnvError { message } => {
                write!(f, "EnvironmentError: {}", message)
            },
            FrontendErrorKind::TypeError { expected, actual} => {
                write!(f, "TypeError: expected `{:?}`, got `{:?}`", expected, actual)
            },
            FrontendErrorKind::SystemError { message } => {
                write!(f, "SystemError: {}", message)
            }
        }
    }
}

/// standardized type to remember all frontend errors
pub type FrontendError<LocationT> = Located<FrontendErrorKind, LocationT>;

impl<T: fmt::Debug, E: fmt::Debug> From<(ErrorRecovery<usize, T, E>)> for FrontendError<usize> {
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
        FrontendError::new(FrontendErrorKind::ParseError { message }, location)
    }
}
