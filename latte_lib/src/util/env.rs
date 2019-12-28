use crate::parser::ast::{Keyed, Type};
use crate::error::{FrontendError, FrontendErrorKind};

use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use crate::meta::Meta;


/// alias, we use String as key everywhere in the project
pub type Env<T> = HashMap<String, T>;

pub trait UniqueEnv<ItemT, LocationT> {
    /// associates k with v, making sure v is unique
    fn insert_unique(&mut self, k: String, v: Meta<ItemT, LocationT>) -> Result<(), FrontendError<LocationT>>;
}

pub trait FromKeyedVec<ItemT, LocationT> {
    /// create an env mapping element.get_key() to element, making sure keys are unique
    fn from_vec(member_vec: &mut Vec<Meta<ItemT, LocationT>>) -> Result<Self, Vec<FrontendError<LocationT>>>;
}

impl<ItemT: Debug, LocationT> UniqueEnv<ItemT, LocationT> for Env<Meta<ItemT, LocationT>> {
    /// insert items with all metadata
    fn insert_unique(&mut self, k: String, v: Meta<ItemT, LocationT>) -> Result<(), FrontendError<LocationT>> {
        match self.insert(k, v) {
            Some(previous_val) => {
                let kind = FrontendErrorKind::EnvError {
                    message: format!("Duplicate declaration of {:?}", previous_val.item)
                };
                Err(FrontendError::new(kind, v.get_meta()))
            },
            None => {
                Ok(())
            }
        }
    }
}
impl<ItemT: Debug, LocationT> UniqueEnv<ItemT, LocationT> for Env<ItemT> {
    /// insert only items
    fn insert_unique(&mut self, k: String, v: Meta<ItemT, LocationT>) -> Result<(), FrontendError<LocationT>> {
        match self.insert(k, v.item) {
            Some(previous_val) => {
                let kind = FrontendErrorKind::EnvError {
                    message: format!("Duplicate declaration of {:?}", previous_val)
                };
                Err(FrontendError::new(kind, v.get_meta()))
            },
            None => {
                Ok(())
            }
        }
    }
}

impl<ItemT: Debug+Keyed, LocationT> FromKeyedVec<ItemT, LocationT> for Env<Meta<ItemT, LocationT>> {
    /// insert items with all metadata
    fn from_vec(member_vec: &mut Vec<Meta<ItemT, LocationT>>) -> Result<Self, Vec<FrontendError<LocationT>>> {
        let mut env = Self::new();
        let errors: Vec<_> = member_vec.iter()
            .map(|element| env.insert_unique(element.get_key().clone(), element))
            .filter_map(Result::err)
            .collect();
        if errors.is_empty() {
            Ok(env)
        } else {
            Err(errors)
        }
    }
}

impl<ItemT: Debug+Keyed, LocationT> FromKeyedVec<ItemT, LocationT> for Env<ItemT> {
    /// insert only items
    fn from_vec(member_vec: &mut Vec<Meta<ItemT, LocationT>>) -> Result<Self, Vec<FrontendError<LocationT>>> {
        let mut env = Self::new();
        let errors: Vec<_> = member_vec.iter()
            .map(|element| env.insert_unique(element.get_key().clone(), element.item))
            .filter_map(Result::err)
            .collect();
        if errors.is_empty() {
            Ok(env)
        } else {
            Err(errors)
        }
    }
}
