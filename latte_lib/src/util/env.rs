use crate::parser::ast::{Keyed, Type};
use crate::location::Located;
use crate::error::{FrontendError, FrontendErrorKind};

use std::collections::HashMap;
use std::fmt::Debug;


/// alias, we use String as key everywhere in the project
pub type Env<T> = HashMap<String, T>;

/// alias, useful for creating environment where all values remember their location
pub type LocEnv<ItemT, LocT> = HashMap<String, Located<ItemT, LocT>>;

pub trait UniqueLocEnv<ItemT, LocT> {
    fn insert_unique(&mut self, k: String, v: Located<ItemT, LocT>) -> Result<(), FrontendError<LocT>>;
}

impl<ItemT: Debug+Clone, LocT: Clone> UniqueLocEnv<ItemT, LocT> for LocEnv<ItemT, LocT> {
    fn insert_unique(&mut self, k: String, v: Located<ItemT, LocT>) -> Result<(), Located<FrontendErrorKind, LocT>> {
        let loc = v.get_location().clone();
        match self.insert(k, v) {
            Some(previous_val) => {
                let kind = FrontendErrorKind::EnvError {
                    message: format!("Duplicate declaration of {:?}", previous_val.item)
                };
                Err(FrontendError::new(kind, loc))
            },
            None => {
                Ok(())
            }
        }
    }
}

pub trait FromMemberVec<MemberT, LocT> {
    fn from_vec(member_vec: Vec<Located<MemberT, LocT>>)
        -> Result<Env<Located<MemberT, LocT>>, FrontendError<LocT>>;
}

impl<MemberT: Debug+Keyed+Clone, LocT: Clone> FromMemberVec<MemberT, LocT> for LocEnv<MemberT, LocT> {
    fn from_vec(member_vec: Vec<Located<MemberT, LocT>>) -> Result<Self, FrontendError<LocT>> {
        let mut env = Self::new();
        for member in member_vec {
            env.insert_unique(member.item.get_key().clone(), member.clone())?;
        }
        Ok(env)
    }
}

pub trait ToTypeEnv {
    /// convert env with complex values to only store type information
    fn to_type_env(&self) -> Env<Type>;
}
