//! The **Shove** contextual act's input-layer descriptor (GTW-525 / GTW-571).

use bevy::prelude::Entity;
use gdtf_battle_sim::acts::ShoveRequested;

use super::seam::ContextualAct;

/// The **Shove** contextual act (GTW-525) — the deliberate, universal knock-back on an
/// 8-adjacent, ALIVE, opposing ganger (no weapon, no LOS — a shove is contact).
///
/// A compile-time act token (never instantiated): the [`ContextualAct`] impl is the
/// input layer's whole per-act surface. The carried target is the opposing ganger
/// [`Entity`] the panel offered; the drain emits
/// [`ShoveRequested::new`](ShoveRequested::new) — the DELIBERATE, gated, TU-costed form
/// (the weapon-tag auto-shove is a sim-internal producer, never this queue's) — and the
/// sim's `dispatch_shove` gate (8-adjacent + opposing + alive) is the authoritative
/// check. Pure displacement: the fall, if any, does the harm.
#[derive(Debug, Clone, Copy)]
pub struct ShoveAct;

impl ContextualAct for ShoveAct {
    type Requested = ShoveRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> ShoveRequested {
        ShoveRequested::new(actor, target)
    }
}
