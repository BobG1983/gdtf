//! Execute-downed contextual act.

use bevy::prelude::Entity;
use gdtf_battle_sim::acts::ExecuteDownedRequested;

use super::seam::ContextualAct;

/// Marker for the execute-downed act family.
#[derive(Debug, Clone, Copy)]
pub struct ExecuteAct;

impl ContextualAct for ExecuteAct {
    type Requested = ExecuteDownedRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> ExecuteDownedRequested {
        ExecuteDownedRequested::new(actor, target)
    }
}
