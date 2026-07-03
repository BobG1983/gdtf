//! The **Enter Emplacement** contextual act's input-layer descriptor (GTW-543 / GTW-571).

use bevy::prelude::Entity;
use gdtf_battle_sim::acts::EnterEmplacementRequested;

use super::seam::ContextualAct;

/// The **Enter Emplacement** contextual act (GTW-543) — mans an 8-adjacent VACANT
/// weapon emplacement.
///
/// A compile-time act token (never instantiated): the [`ContextualAct`] impl is the
/// input layer's whole per-act surface. The carried target is the emplacement
/// [`Entity`] the panel offered (a VACANT one only); the drain emits
/// [`EnterEmplacementRequested`] and the sim's `dispatch_enter_emplacement` gate
/// (VACANT `EmplacementState` + 8-adjacent + affords the `EnterEmplacementTu` leaf) is
/// the authoritative check — the SIM spends the TU (one-way `input -> sim` boundary).
#[derive(Debug, Clone, Copy)]
pub struct EnterEmplacementAct;

impl ContextualAct for EnterEmplacementAct {
    type Requested = EnterEmplacementRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> EnterEmplacementRequested {
        EnterEmplacementRequested::new(actor, target)
    }
}
