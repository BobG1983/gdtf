//! The **Open Door** contextual act's input-layer descriptor (GTW-315 / GTW-571).

use bevy::prelude::Entity;
use gdtf_battle_sim::acts::OpenDoorRequested;

use super::seam::ContextualAct;

/// The **Open Door** contextual act (GTW-315) — opens an 8-adjacent CLOSED door.
///
/// A compile-time act token (never instantiated): the [`ContextualAct`] impl is the
/// input layer's whole per-act surface. The carried target is the openable door
/// [`Entity`] the panel offered (a CLOSED door only — the button always OPENS; closing
/// is not a contextual act, and F4 is PLAYER-ONLY); the drain emits
/// [`OpenDoorRequested`] and the sim's `dispatch_open_door` gate (CLOSED `OpenState` +
/// 8-adjacent + affords the `OpenDoorTu` leaf) is the authoritative check — the SIM
/// spends the TU (one-way `input -> sim` boundary).
#[derive(Debug, Clone, Copy)]
pub struct OpenDoorAct;

impl ContextualAct for OpenDoorAct {
    type Requested = OpenDoorRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> OpenDoorRequested {
        OpenDoorRequested::new(actor, target)
    }
}
