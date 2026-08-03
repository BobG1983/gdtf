//! Enter-emplacement contextual act.

use bevy::prelude::Entity;
use gdtf_battle_sim::acts::EnterEmplacementRequested;

use super::seam::ContextualAct;

/// Marker for the enter-emplacement act family.
#[derive(Debug, Clone, Copy)]
pub struct EnterEmplacementAct;

impl ContextualAct for EnterEmplacementAct {
    type Requested = EnterEmplacementRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> EnterEmplacementRequested {
        EnterEmplacementRequested::new(actor, target)
    }
}
