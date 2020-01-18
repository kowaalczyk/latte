use std::collections::HashMap;

use crate::backend::builder::MapEntities;
use crate::backend::ir::Entity;
use crate::util::env::Env;

#[derive(Debug, Clone)]
pub struct BlockContext {
    /// local variable environment
    env: Env<Entity>,
}

impl BlockContext {
    pub fn new() -> Self {
        Self { env: Env::new() }
    }

    /// get entity representing a pointer to a variable with given identifier
    pub fn get_ptr(&self, ident: &String) -> Entity {
        self.env.get(ident).unwrap().clone()
    }

    /// set pointer to a variable in a local environment, given an entity that represents it
    pub fn set_ptr(&mut self, ident: &String, ent: &Entity) {
        self.env.insert(ident.clone(), ent.clone());
    }

    pub fn get_env(&self) -> &Env<Entity> {
        &self.env
    }

    pub fn map_env(&mut self, mapping: &HashMap<Entity, Entity>) {
        self.env = self.env.iter()
            .map(|(k, v)| (k.clone(), v.map_entities(0, &mapping)))
            .collect()
    }
}
