use bevy::prelude::Entity;
use gdtf_battle_sim::{acts::ThrowGrenadeRequested, prelude::CellLevel};

use super::seam::ContextualAct;

#[derive(Debug, Clone, Copy)]
pub struct ThrowGrenadeAct;

impl ContextualAct for ThrowGrenadeAct {
    type Requested = ThrowGrenadeRequested;
    type Target = CellLevel;

    fn request(actor: Entity, target: CellLevel) -> ThrowGrenadeRequested {
        ThrowGrenadeRequested::new(actor, target)
    }
}
