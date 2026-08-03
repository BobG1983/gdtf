//! Open-door contextual act.

use bevy::prelude::Entity;
use gdtf_battle_sim::acts::OpenDoorRequested;

use super::seam::ContextualAct;

/// Marker for the open-door act family.
#[derive(Debug, Clone, Copy)]
pub struct OpenDoorAct;

impl ContextualAct for OpenDoorAct {
    type Requested = OpenDoorRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> OpenDoorRequested {
        OpenDoorRequested::new(actor, target)
    }
}
