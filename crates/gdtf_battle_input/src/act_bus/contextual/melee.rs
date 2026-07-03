//! The **Melee** contextual act's input-layer descriptor (GTW-507 / GTW-508 / GTW-571).

use bevy::prelude::Entity;
use gdtf_battle_sim::acts::{MeleeRequested, MeleeTarget};

use super::seam::ContextualAct;

/// The **Melee** contextual act (GTW-507; the structure smash added by GTW-508) — the
/// close-combat strike on an 8-adjacent, alive, in-LOS enemy OR the uncontested smash
/// of an 8-adjacent intact Cover / Wall cell.
///
/// A compile-time act token (never instantiated): the [`ContextualAct`] impl is the
/// input layer's whole per-act surface. The carried target is the sim's own
/// [`MeleeTarget`] domain enum — a [`Ganger`](MeleeTarget::Ganger) (the GTW-507
/// contested path) or a [`Structure`](MeleeTarget::Structure) cell (the GTW-508
/// cover-smash) — so the ONE Melee button routes two target kinds through one act. The
/// drain emits [`MeleeRequested`] verbatim; the sim's `dispatch_melee` gate
/// (8-adjacency, + LOS / alive / opposing on the ganger arm) is the authoritative
/// check.
#[derive(Debug, Clone, Copy)]
pub struct MeleeAct;

impl ContextualAct for MeleeAct {
    type Requested = MeleeRequested;
    type Target = MeleeTarget;

    fn request(actor: Entity, target: MeleeTarget) -> MeleeRequested {
        MeleeRequested {
            attacker: actor,
            target,
        }
    }
}
