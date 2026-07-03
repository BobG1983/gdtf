//! The **Stabilize** contextual act's input-layer descriptor (GTW-294 / GTW-571).

use bevy::prelude::Entity;
use gdtf_battle_sim::acts::StabilizeDownedRequested;

use super::seam::ContextualAct;

/// The **Stabilize** contextual act (GTW-294) — arrests an 8-adjacent downed ALLY's
/// bleed-out.
///
/// A compile-time act token (never instantiated): the [`ContextualAct`] impl is the
/// input layer's whole per-act surface. The carried target is the not-yet-stabilized
/// downed ally [`Entity`] the panel offered; the drain emits
/// [`StabilizeDownedRequested`] and the sim's `stabilize_downed` faction gate (an
/// 8-adjacent alive ALLY over a DOWNED target) is the authoritative check.
#[derive(Debug, Clone, Copy)]
pub struct StabilizeAct;

impl ContextualAct for StabilizeAct {
    type Requested = StabilizeDownedRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> StabilizeDownedRequested {
        StabilizeDownedRequested::new(actor, target)
    }
}
