use bevy::prelude::Entity;
use gdtf_battle_sim::acts::ExitEmplacementRequested;

use super::seam::ContextualAct;

#[derive(Debug, Clone, Copy)]
pub struct ExitEmplacementAct;

impl ContextualAct for ExitEmplacementAct {
    type Requested = ExitEmplacementRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> ExitEmplacementRequested {
        ExitEmplacementRequested::new(actor, target)
    }
}
