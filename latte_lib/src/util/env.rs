use crate::parser::ast::Keyed;
use crate::location::Located;
use crate::error::{FrontendError, FrontendErrorKind};

use std::collections::HashMap;
use std::fmt::Debug;


pub type Env<T> = HashMap<String, T>;

pub trait UniqueEnv<EnvT> {
    /// insert or throw error if key already exists in the environment
    fn insert_unique(&mut self, k: String, v: EnvT, location: usize) -> Result<(), FrontendError<usize>>;
}

impl<T: Debug+Keyed> UniqueEnv<T> for Env<T> {
    fn insert_unique(&mut self, k: String, v: T, location: usize) -> Result<(), FrontendError<usize>> {
        match self.insert(k, v) {
            Some(previous_val) => {
                let kind = FrontendErrorKind::EnvError {
                    message: format!("Duplicate declaration of {:?}", previous_val)
                };
                Err(FrontendError::new(kind, location))
            },
            None => {
                Ok(())
            }
        }
    }
}

pub trait FromMemberVec<MemberT, LocationT> {
    fn from_vec(member_vec: Vec<Located<MemberT, LocationT>>) -> Result<Env<MemberT>, FrontendError<LocationT>>;
}

impl<T: Debug+Keyed+Clone> FromMemberVec<T, usize> for Env<T> {
    fn from_vec(member_vec: Vec<Located<T, usize>>) -> Result<Self, FrontendError<usize>> {
        let mut env = Self::new();
        for member in member_vec {
            env.insert_unique(member.item.get_key().clone(), member.item.clone(), member.get_location())?;
        }
        Ok(env)
    }
}
