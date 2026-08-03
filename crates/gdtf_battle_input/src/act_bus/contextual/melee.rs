use bevy::prelude::Entity;
use gdtf_battle_sim::acts::{MeleeRequested, MeleeTarget};

use super::seam::ContextualAct;

#[derive(Debug, Clone, Copy)]
pub struct MeleeAct;

impl ContextualAct for MeleeAct {
    type Requested = MeleeRequested;
    type Target = MeleeTarget;

    fn request(actor: Entity, target: MeleeTarget) -> MeleeRequested {
        MeleeRequested {
            attacker: actor,
            target,
        }
    }
}
