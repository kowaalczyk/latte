use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;

use crate::frontend::ast::{Keyed, Type};
use crate::frontend::error::{FrontendError, FrontendErrorKind};
use crate::meta::{GetLocation, Meta};

/// alias, we use String as key everywhere in the project
pub type Env<T> = HashMap<String, T>;

pub trait UniqueEnv<ItemT, LocationT> {
    /// associates k with v, making sure v is unique
    fn insert_unique(&mut self, k: String, v: Meta<ItemT, LocationT>) -> Result<(), FrontendError<LocationT>>;
}

pub trait FromKeyedVec<ItemT, LocationT> {
    /// create an env mapping element.get_key() to element, making sure keys are unique
    fn from_vec(member_vec: &mut Vec<Meta<ItemT, LocationT>>) -> Result<Self, Vec<FrontendError<LocationT>>>
        where Self: std::marker::Sized;
}

impl<ItemT: Debug + Clone, MetaT: Clone> UniqueEnv<ItemT, MetaT> for Env<Meta<ItemT, MetaT>> {
    /// insert items with all metadata
    fn insert_unique(&mut self, k: String, v: Meta<ItemT, MetaT>) -> Result<(), FrontendError<MetaT>> {
        match self.insert(k, v.clone()) {
            Some(previous_val) => {
                let kind = FrontendErrorKind::EnvError {
                    message: format!("Duplicate declaration of {:?}", previous_val.item)
                };
                Err(FrontendError::new(kind, v.get_meta().clone()))
            }
            None => {
                Ok(())
            }
        }
    }
}

impl<ItemT: Debug + Clone, MetaT: Clone> UniqueEnv<ItemT, MetaT> for Env<ItemT> {
    /// insert only items
    fn insert_unique(&mut self, k: String, v: Meta<ItemT, MetaT>) -> Result<(), FrontendError<MetaT>> {
        match self.insert(k, v.item.clone()) {
            Some(previous_val) => {
                let kind = FrontendErrorKind::EnvError {
                    message: format!("Duplicate declaration of {:?}", previous_val)
                };
                Err(FrontendError::new(kind, v.get_meta().clone()))
            }
            None => {
                Ok(())
            }
        }
    }
}

impl<ItemT: Debug + Keyed + Clone, MetaT: Clone> FromKeyedVec<ItemT, MetaT> for Env<Meta<ItemT, MetaT>> {
    /// insert items with all metadata
    fn from_vec(member_vec: &mut Vec<Meta<ItemT, MetaT>>) -> Result<Self, Vec<FrontendError<MetaT>>> {
        let mut env = Self::new();
        let errors: Vec<_> = member_vec.into_iter()
            .map(|element| env.insert_unique(element.get_key().clone(), element.clone()))
            .filter_map(Result::err)
            .collect();
        if errors.is_empty() {
            Ok(env)
        } else {
            Err(errors)
        }
    }
}

impl<ItemT: Debug + Keyed + Clone, MetaT: Clone> FromKeyedVec<ItemT, MetaT> for Env<ItemT> {
    /// insert only items
    fn from_vec(member_vec: &mut Vec<Meta<ItemT, MetaT>>) -> Result<Self, Vec<FrontendError<MetaT>>> {
        let mut env = Self::new();
        let errors: Vec<_> = member_vec.into_iter()
            .map(|element| env.insert_unique(element.get_key().clone(), element.clone()))
            .filter_map(Result::err)
            .collect();
        if errors.is_empty() {
            Ok(env)
        } else {
            Err(errors)
        }
    }
}
