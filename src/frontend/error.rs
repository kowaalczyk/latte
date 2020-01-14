use std::fmt;

use lalrpop_util::{ErrorRecovery, ParseError as LalrpopError};

use crate::frontend::ast;
use crate::meta::LocationMeta;
use crate::meta::Meta;

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
    ArgumentError {
        message: String,
    },
    SystemError {
        message: String,
    },
}

impl fmt::Display for FrontendErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FrontendErrorKind::ParseError { message } => {
                write!(f, "ParseError: {}", message)
            }
            FrontendErrorKind::EnvError { message } => {
                write!(f, "EnvironmentError: {}", message)
            }
            FrontendErrorKind::TypeError { expected, actual } => {
                write!(f, "TypeError: expected `{:?}`, got `{:?}`", expected, actual)
            }
            FrontendErrorKind::ArgumentError { message } => {
                write!(f, "ArgumentError: {}", message)
            }
            FrontendErrorKind::SystemError { message } => {
                write!(f, "SystemError: {}", message)
            }
        }
    }
}

/// standardized type to remember all frontend errors
pub type FrontendError<LocationT> = Meta<FrontendErrorKind, LocationT>;

impl<T: fmt::Debug, E: fmt::Debug> From<(ErrorRecovery<usize, T, E>)> for FrontendError<LocationMeta> {
    fn from(err: ErrorRecovery<usize, T, E>) -> Self {
        let (location, message) = match &err.error {
            LalrpopError::InvalidToken { location } => {
                (LocationMeta::from(*location), String::from("InvalidToken"))
            }
            LalrpopError::UnrecognizedEOF { location, expected: _ } => {
                (LocationMeta::from(*location), String::from("Unexpected end of file"))
            }
            LalrpopError::ExtraToken { token } => {
                (LocationMeta::from(token.0), format!("ExtraToken: {:?}", token.1))
            }
            LalrpopError::UnrecognizedToken { token, expected: _ } => {
                (LocationMeta::from(token.0), format!("UnrecognizedToken: {:?}", token.1))
            }
            LalrpopError::User { error } => {
                panic!("Impossible: Undefined lalrpop user error: {:#?}", error)
            }
        };
        FrontendError::new(FrontendErrorKind::ParseError { message }, location)
    }
}
