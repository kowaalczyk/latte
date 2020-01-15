use crate::util::env::Env;
use crate::backend::ir::Entity;

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
}
