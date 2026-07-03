//! The **Throw Grenade** contextual act's input-layer descriptor (GTW-546 / GTW-571).

use bevy::prelude::Entity;
use gdtf_battle_sim::{CellLevel, acts::ThrowGrenadeRequested};

use super::seam::ContextualAct;

/// The **Throw Grenade** contextual act (GTW-546) — lobs a grenade at a target cell at
/// range.
///
/// A compile-time act token (never instantiated): the [`ContextualAct`] impl is the
/// input layer's whole per-act surface. The carried target is the hovered target
/// [`CellLevel`] the panel offered (a cell AT RANGE — the throw is BLIND, no
/// LOS / facing / arc gate; a lob need not see its target); the drain emits
/// [`ThrowGrenadeRequested`] and the sim's `dispatch_throw_grenade` gate (wields a
/// `TrajectoryStyle::Arc` weapon with a loaded round + affords the `ThrowTu` leaf) is
/// the authoritative check — the SIM spends the TU + magazine round (one-way
/// `input -> sim` boundary).
#[derive(Debug, Clone, Copy)]
pub struct ThrowGrenadeAct;

impl ContextualAct for ThrowGrenadeAct {
    type Requested = ThrowGrenadeRequested;
    type Target = CellLevel;

    fn request(actor: Entity, target: CellLevel) -> ThrowGrenadeRequested {
        ThrowGrenadeRequested::new(actor, target)
    }
}
