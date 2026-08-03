use bevy::prelude::Entity;
use gdtf_battle_sim::acts::StabilizeDownedRequested;

use super::seam::ContextualAct;

#[derive(Debug, Clone, Copy)]
pub struct StabilizeAct;

impl ContextualAct for StabilizeAct {
    type Requested = StabilizeDownedRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> StabilizeDownedRequested {
        StabilizeDownedRequested::new(actor, target)
    }
}
