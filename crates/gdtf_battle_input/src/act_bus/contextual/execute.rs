use bevy::prelude::Entity;
use gdtf_battle_sim::acts::ExecuteDownedRequested;

use super::seam::ContextualAct;

#[derive(Debug, Clone, Copy)]
pub struct ExecuteAct;

impl ContextualAct for ExecuteAct {
    type Requested = ExecuteDownedRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> ExecuteDownedRequested {
        ExecuteDownedRequested::new(actor, target)
    }
}
