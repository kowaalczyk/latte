use std::collections::HashMap;

use crate::backend::builder::MapEntities;
use crate::backend::ir::Entity;
use crate::util::env::Env;

#[derive(Debug, Clone)]
pub struct BlockContext {
    /// current depth
    current_depth: usize,

    /// local variable environment stack
    env_stack: Vec<Env<Entity>>,
}

impl BlockContext {
    pub fn new() -> Self {
        Self { current_depth: 0, env_stack: vec![Env::new()] }
    }

    pub fn increase_depth(&mut self) {
        self.current_depth += 1;
        self.env_stack.push(Env::new());
    }

    pub fn decrease_depth(&mut self) {
        self.current_depth -= 1;
        self.env_stack.pop();
    }

    /// get entity representing a pointer to a variable with given identifier
    pub fn get_variable(&self, ident: &String) -> Entity {
        for env in self.env_stack.iter().rev() {
            if let Some(ent) = env.get(ident) {
                return ent.clone()
            }
        }
        panic!("Identifier not found: {}", ident)
    }

    /// update existing variable value from the current depth
    /// (the related entity with closest depth to current one will be updated)
    pub fn update_variable(&mut self, ident: String, ent: Entity) {
        for env in self.env_stack.iter_mut().rev() {
            if let Some(_) = env.get(&ident) {
                env.insert(ident, ent);
                return
            }
        }
        panic!("Identifier not found: {}", ident)
    }

    /// set pointer to a new variable in environment (at current depth)
    pub fn set_new_variable(&mut self, ident: String, ent: Entity) {
        self.env_stack[self.current_depth].insert(ident, ent);
    }

    /// get view of the environment from the current depth
    /// (each variable will be mapped to related entity with the closest depth)
    pub fn get_env_view(&self) -> Env<Entity> {
        let mut combined_env = Env::new();
        for env in self.env_stack.iter() {
            combined_env.extend(env.into_iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        combined_env
    }

    /// substitute entities in entire environment stack using provided mapping
    pub fn map_env(&mut self, mapping: &HashMap<Entity, Entity>) {
        self.env_stack = self.env_stack.iter()
            .map(|env| {
                env.iter()
                    .map(|(k, v)| (k.clone(), v.map_entities(0, &mapping)))
                    .collect()
            })
            .collect();
    }
}
