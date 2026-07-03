//! The **Exit Emplacement** contextual act's input-layer descriptor (GTW-543 / GTW-571).

use bevy::prelude::Entity;
use gdtf_battle_sim::acts::ExitEmplacementRequested;

use super::seam::ContextualAct;

/// The **Exit Emplacement** contextual act (GTW-543) — dismounts the emplacement the
/// selection is manning.
///
/// A compile-time act token (never instantiated): the [`ContextualAct`] impl is the
/// input layer's whole per-act surface. The carried target is the emplacement
/// [`Entity`] the panel offered (ONLY the one whose recorded `EmplacementOccupant` IS
/// the selection — there is NO force-eject; exit is a SEPARATE TU-costed act); the
/// drain emits [`ExitEmplacementRequested`] and the sim's `dispatch_exit_emplacement`
/// gate (the recorded occupant IS the actor + affords the `ExitEmplacementTu` leaf) is
/// the authoritative check — the SIM spends the TU (one-way `input -> sim` boundary).
#[derive(Debug, Clone, Copy)]
pub struct ExitEmplacementAct;

impl ContextualAct for ExitEmplacementAct {
    type Requested = ExitEmplacementRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> ExitEmplacementRequested {
        ExitEmplacementRequested::new(actor, target)
    }
}
