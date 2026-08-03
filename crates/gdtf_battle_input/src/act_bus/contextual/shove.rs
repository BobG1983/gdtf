use bevy::prelude::Entity;
use gdtf_battle_sim::acts::ShoveRequested;

use super::seam::ContextualAct;

#[derive(Debug, Clone, Copy)]
pub struct ShoveAct;

impl ContextualAct for ShoveAct {
    type Requested = ShoveRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> ShoveRequested {
        ShoveRequested::new(actor, target)
    }
}
