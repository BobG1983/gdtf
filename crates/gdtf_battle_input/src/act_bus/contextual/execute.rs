//! The **Execute** contextual act's input-layer descriptor (GTW-294 / GTW-571).

use bevy::prelude::Entity;
use gdtf_battle_sim::acts::ExecuteDownedRequested;

use super::seam::ContextualAct;

/// The **Execute** contextual act (GTW-294) — the coup-de-grâce on an 8-adjacent DOWNED
/// enemy.
///
/// A compile-time act token (never instantiated): the [`ContextualAct`] impl is the
/// input layer's whole per-act surface. The carried target is the downed enemy
/// [`Entity`] the panel offered; the drain emits [`ExecuteDownedRequested`] and the
/// sim's `execute_downed` faction gate (an 8-adjacent alive ENEMY over a DOWNED target)
/// is the authoritative check.
#[derive(Debug, Clone, Copy)]
pub struct ExecuteAct;

impl ContextualAct for ExecuteAct {
    type Requested = ExecuteDownedRequested;
    type Target = Entity;

    fn request(actor: Entity, target: Entity) -> ExecuteDownedRequested {
        ExecuteDownedRequested::new(actor, target)
    }
}
