use std::collections::{HashMap, HashSet};
use std::collections::hash_map::RandomState;

use crate::backend::context::FunctionContext;
use crate::backend::ir::{BasicBlock, Entity, GetEntity, Instruction, InstructionKind};
use crate::frontend::ast::Type;
use crate::meta::{GetType, Meta};
use crate::util::env::Env;

#[derive(Debug, Clone)]
pub struct BlockBuilder {
    /// currently built basic block
    block: BasicBlock,

    /// entity mapping, available to re-use after initial block build
    entity_mapping: Option<HashMap<Entity, Entity>>,

    /// predecessor label => predecessor variable ident => entity in that predecessor
    predecessors: Env<Env<Entity>>,

    /// variables to which values were assigned in the block
    variables_gen: Env<Entity>,
}

impl BlockBuilder {
    pub fn without_label() -> Self {
        Self {
            predecessors: Env::new(),
            block: BasicBlock {
                label: None,
                instructions: vec![]
            },
            entity_mapping: None,
            variables_gen: Env::new(),
        }
    }

    pub fn with_label(label: String) -> Self {
        Self {
            predecessors: Env::new(),
            block: BasicBlock {
                label: Some(label),
                instructions: vec![]
            },
            entity_mapping: None,
            variables_gen: Env::new(),
        }
    }

    pub fn get_block_label(&self) -> String {
        self.block.get_label()
    }

    /// register environment from a compiled predecessor
    pub fn add_predecessor(&mut self, label: String, env: &Env<Entity>) {
        self.predecessors.insert(label, env.clone());
    }

    /// add new instruction to the current basic block
    pub fn push_instruction(&mut self, instr: Instruction) {
        self.block.instructions.push(instr)
    }

    /// checks if current block always returns
    pub fn block_always_returns(&self) -> bool {
        self.block.always_returns()
    }

//    /// update gen set for the current block
//    pub fn set_gen(&mut self, ident: String, val: Entity) {
//        self.variables_gen.insert(ident, val);  // TODO: Gen sets seem to no longer be necessary
//    }
//
//    /// shorthand for push_instruction(); set_gen() on the instruction result
//    pub fn push_gen_instruction(&mut self, instr: Instruction, gen_var_ident: String) {
//        let entity = instr.get_entity();
//        self.push_instruction(instr);
//        self.set_gen(gen_var_ident, entity);
//    }

//    /// get gen set for the current block
//    pub fn get_gen(&self) -> &Env<Entity> {
//        &self.variables_gen
//    }

    /// build the block, adding all necessary phi instructions at the beginning
    pub fn build(&mut self, function_ctx: &mut FunctionContext) -> BasicBlock {
        // create mapping: variable => (vec![(predecessor_label, entity_in_predecessor)], set of unique entities)
        // TODO: Refactor needed, this code is highly unreadable
        // TODO: Tests needed
        if self.predecessors.len() < 2 {
            return self.block.clone();
        }

        let mut predecessor_key_locations: Env<(Vec<(Entity, String)>, HashSet<Entity>)> = Env::new();

        for (predecessor_name, predecessor_env) in self.predecessors.iter() {
            for (key, entity) in predecessor_env.iter() {
                let location = (entity.clone(), predecessor_name.clone());
                if let Some(predecessor_data) = &mut predecessor_key_locations.get(key) {
                    // variable already existed in one of the previous predecessors' envs
                    let mut locations_vec = predecessor_data.0.clone();
                    let mut unique_entities = predecessor_data.1.clone();
                    locations_vec.push(location);
                    unique_entities.insert(entity.clone());
                    predecessor_key_locations.insert(key.clone(), (locations_vec, unique_entities));
                } else {
                    // variable appears for the first time
                    let mut new_set = HashSet::new();
                    new_set.insert(entity.clone());
                    predecessor_key_locations.insert(key.clone(), (vec![location], new_set));
                }
            }
        }

        // variable identifiers for which name conflicts are possible
        let phi_keys: HashSet<_> = predecessor_key_locations.iter()
            .filter(|(_, (_, unique_ents))| unique_ents.len() >= 2)
            .map(|(key, _)| key.clone())
            .collect();

        // mutable block is necessary for swapping instruction entities
        let mut cyclic_shift = 0 as usize;
        let mut phi_instructions = Vec::new();
        let mut entity_mapping = HashMap::new();

        for key in phi_keys {
            // create phi instruction
            let variable_t = predecessor_key_locations.get(&key).unwrap().0[0].0.get_type();
            let phi_instr = InstructionKind::Phi {
                args: predecessor_key_locations.get(&key).unwrap().0.clone()
            };

            let available_register = function_ctx.new_register(variable_t.clone());

            let actual_phi_register = if let Some(Entity::Register { n, t: _ }) = self.block.get_first_register() {
                // actual phi register will have the number of the first original register in the block,
                // and we'll shift the original registers' numbers later to prevent conflicts
                Entity::Register {
                    n: n + cyclic_shift,
                    t: variable_t
                }
            } else {
                // instructions in the don't use result registers so we use the available register
                available_register
            };
            phi_instructions.push(phi_instr.with_result(actual_phi_register.clone()));

            // the change is stored in mapping, that will later allow for re-numbering of arguments
            for original_env_location_data in &predecessor_key_locations.get(&key).unwrap().0 {
                let original_env_location = original_env_location_data.0.clone();
                entity_mapping.insert(original_env_location, actual_phi_register.clone());
            }
            cyclic_shift += 1;
        }

        // re-number original block statements and append them after newly created phi instructions
        let mut block_instructions = self.block.instructions.iter()
            .map(|i| i.map_entities(cyclic_shift, &entity_mapping))
            .collect();
        phi_instructions.append(&mut block_instructions);
        self.block.instructions = phi_instructions;

        // save entity mapping for easy re-use
        self.entity_mapping = Some(entity_mapping);

        self.block.clone()
    }

    /// get entity mapping from already built block, that can later be applied to another block
    pub fn get_entity_mapping(&mut self, function_ctx: &mut FunctionContext) -> &HashMap<Entity, Entity> {
        if let Some(mapping) = &self.entity_mapping {
            mapping
        } else {
            panic!("to get mapping, build the block first")
        }
    }
}

pub trait MapEntities {
    fn map_entities(&self, cyclic_shift: usize, mapping: &HashMap<Entity, Entity>) -> Self;
}

impl MapEntities for Entity {
    /// renumber register entities, correct ONLY for NON-PHI instructions
    fn map_entities(&self, cyclic_shift: usize, mapping: &HashMap<Entity, Entity>) -> Self {
        if let Some(mapped_ent) = mapping.get(self) {
            mapped_ent.clone()
        } else if let Entity::Register { n, t } = self {
            Entity::Register { n: n + cyclic_shift, t: t.clone() }
        } else {
            self.clone()
        }
    }
}

impl MapEntities for Instruction {
    /// renumber argument and result entities, correct ONLY for NON-PHI instructions
    fn map_entities(&self, cyclic_shift: usize, mapping: &HashMap<Entity, Entity>) -> Self {
        let mapped_args_ent = match &self.item {
            InstructionKind::Load { ptr } => {
                InstructionKind::Load { ptr: ptr.map_entities(cyclic_shift, mapping) }
            }
            InstructionKind::Store { val, ptr } => {
                InstructionKind::Store {
                    val: val.map_entities(cyclic_shift, mapping),
                    ptr: ptr.map_entities(cyclic_shift, mapping),
                }
            }
            InstructionKind::BitCast { ent, to } => {
                InstructionKind::BitCast {
                    ent: ent.map_entities(cyclic_shift, mapping),
                    to: to.clone(),
                }
            }
            InstructionKind::UnaryOp { op, arg } => {
                InstructionKind::UnaryOp {
                    op: op.clone(),
                    arg: arg.map_entities(cyclic_shift, mapping),
                }
            }
            InstructionKind::BinaryOp { op, l, r } => {
                InstructionKind::BinaryOp {
                    op: op.clone(),
                    l: l.map_entities(cyclic_shift, mapping),
                    r: r.map_entities(cyclic_shift, mapping),
                }
            }
            InstructionKind::Call { func, args } => {
                InstructionKind::Call {
                    func: func.clone(),
                    args: args.iter().map(|arg| arg.map_entities(cyclic_shift, mapping)).collect(),
                }
            }
            InstructionKind::RetVal { val } => {
                InstructionKind::RetVal {
                    val: val.map_entities(cyclic_shift, mapping)
                }
            }
            InstructionKind::JumpCond { cond, true_label, false_label } => {
                InstructionKind::JumpCond {
                    cond: cond.map_entities(cyclic_shift, mapping),
                    true_label: true_label.clone(),
                    false_label: false_label.clone(),
                }
            }
            i => i.clone()
        };
        if let Some(ent) = self.get_meta() {
            if let Entity::Register { n, t } = ent {
                mapped_args_ent.with_result(Entity::Register {
                    n: n + cyclic_shift,
                    t: t.clone(),
                })
            } else {
                panic!("Expected register entity, found: {:?}", ent)
            }
        } else {
            mapped_args_ent.without_result()
        }
    }
}

impl MapEntities for BasicBlock {
    fn map_entities(&self, cyclic_shift: usize, mapping: &HashMap<Entity, Entity, RandomState>) -> Self {
        let mut mapping = mapping.clone();
        let mut instructions = Vec::new();
        for instr in &self.instructions {
            let mapped_instr = instr.map_entities(cyclic_shift, &mapping);
            if let Some(entity) = mapped_instr.get_meta() {
                // after assignment to a mapped variable, we remove it from mapping
                // to prevent it from being used in consecutive instructions
                mapping.remove(entity);
            }
            instructions.push(mapped_instr);
        }
        Self {
            label: self.label.clone(),
            instructions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::UnaryOperator;

    #[test]
    fn block_entities_are_mapped() {
        let block = BasicBlock {
            label: None,
            instructions: vec![
                InstructionKind::UnaryOp {
                    op: UnaryOperator::Neg,
                    arg: Entity::Bool { v: false, uuid: 1 }
                }.with_result(Entity::Register { n: 1, t: Type::Int }),
                InstructionKind::RetVal {
                    val: Entity::Register { n: 1, t: Type::Int }
                }.without_result(),
            ]
        };
        let expected_block = BasicBlock {
            label: None,
            instructions: vec![
                InstructionKind::UnaryOp {
                    op: UnaryOperator::Neg,
                    arg: Entity::Register { n: 2, t: Type::Int }
                }.with_result(Entity::Register { n: 3, t: Type::Int }),
                InstructionKind::RetVal {
                    val: Entity::Register { n: 3, t: Type::Int }
                }.without_result(),
            ]
        };

        let mut mapping = HashMap::new();
        mapping.insert(
            Entity::Bool { v: false, uuid: 1 },
            Entity::Register { n: 2, t: Type::Int }
        );
        let offset = 2 as usize;

        assert_eq!(block.map_entities(offset, &mapping), expected_block)
    }
}
