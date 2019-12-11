use std::fmt;

use lalrpop_util::{ErrorRecovery, ParseError as LalrpopError};

use crate::ast;


#[derive(Debug, PartialEq, Clone)]
pub enum FrontendErrorKind {
    ParseError {
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
            FrontendErrorKind::ParseError { message} => {
                write!(f, "ParseError: {}", message)
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

#[derive(Debug, PartialEq, Clone)]
pub struct FrontendError<LocationT> {
    pub location: LocationT,
    kind: FrontendErrorKind,
}

/// for file preprocessors that alter code layout (ie. comment removal)
pub trait LocationMapper<LocationT1, LocationT2> {
    fn map_location(&self, loc: &LocationT1) -> LocationT2;
}

/// using Mappers, we can correct the original location from lalrpop to the actual file location
impl<LocationT1> FrontendError<LocationT1> {
    pub fn new(kind: FrontendErrorKind, location: LocationT1) -> Self {
        Self {
            kind,
            location
        }
    }

    pub fn map_location<LocationT2>(&self, mapper: &dyn LocationMapper<LocationT1, LocationT2>) -> FrontendError<LocationT2> {
        FrontendError::<LocationT2> {
            location: mapper.map_location(&self.location),
            kind: self.kind.clone(),
        }
    }
}

impl<LocationT: fmt::Display> fmt::Display for FrontendError<LocationT> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.location, self.kind)
    }
}

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
        FrontendError::<usize> {
            kind: FrontendErrorKind::ParseError { message },
            location
        }
    }
}
