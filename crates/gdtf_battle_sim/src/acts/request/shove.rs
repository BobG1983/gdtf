//! The shove act request — [`ShoveRequested`] and its [`ShoveSource`] trigger.

use bevy::prelude::{Entity, Message};

/// A **shove** act was requested — `shover` knocks `target` back one cell (GTW-525).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the
/// shoving ganger [`Entity`] + the shoved target [`Entity`]. The deliberate SHOVE act (any
/// ganger, adjacent to an opposing alive target) writes this from the input seam
/// (the GTW-571 per-act contextual seam: `PendingContextualIntents<ShoveAct>` ->
/// `ShoveRequested`, GTW-525 C4); the WEAPON-TAG auto-shove hooks
/// write it internally on a connecting attack (a melee strike OR a ranged shot connect,
/// GTW-525 C3). [`dispatch_shove`](crate::acts::shove::dispatch_shove) drains it, and — per the
/// [`ShoveSource`] — either gates the deliberate act (8-adjacency + opposing + alive) and
/// spends the [`ShoveTu`](crate::tuning::ShoveTu) leaf, or trusts the connect that already
/// earned the auto-shove (no gate, no TU). Both route the SHARED shove verb
/// ([`resolve_shove`](crate::acts::shove::resolve_shove)): pure one-cell displacement away from the
/// shover, with the unsupported=>fall arm through the shared GTW-523 fall path. A shove deals
/// NO wound of its own — the fall, if any, does the harm.
///
/// The `shover` / `target` are Bevy [`Entity`] handles — framework plumbing, the only bare
/// type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShoveRequested {
    /// The shoving ganger — the target is knocked back one cell directly AWAY from it.
    pub shover: Entity,
    /// The shoved target ganger.
    pub target: Entity,
    /// Whether this is the DELIBERATE shove act (gate + TU) or a WEAPON-TAG auto-shove
    /// (already earned by a connecting attack — no gate re-check, no TU).
    pub source: ShoveSource,
}

/// What triggered a [`ShoveRequested`] (GTW-525) — the DELIBERATE shove act, or a WEAPON-TAG
/// auto-shove bundled into a connecting attack.
///
/// A named domain enum (no-bare-types: the trigger is a domain value, not a bare `bool`). The
/// single [`dispatch_shove`](crate::acts::shove::dispatch_shove) system resolves both through the
/// SAME shared shove verb, but the source decides the GATE + COST: a [`Deliberate`](Self::Deliberate)
/// shove re-checks 8-adjacency + opposing + alive and spends the [`ShoveTu`](crate::tuning::ShoveTu)
/// leaf (a fresh act); a [`Weapon`](Self::Weapon) auto-shove was already earned by the
/// connecting attack (the connect gates ran; the attack's own TU was spent), so it skips the
/// gate and the charge — it only displaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShoveSource {
    /// The DELIBERATE shove act — any ganger's context-sensitive melee shove. Gated
    /// (8-adjacency + opposing faction + alive) and TU-costed
    /// ([`ShoveTu`](crate::tuning::ShoveTu)).
    Deliberate,
    /// A WEAPON-TAG (`shove`) auto-shove — bundled into a CONNECTING melee strike OR ranged
    /// shot. Pre-earned by the connect (no gate re-check, no TU); pure displacement.
    Weapon,
}

impl ShoveRequested {
    /// Build a DELIBERATE shove request for `shover` knocking `target` back (the input-seam
    /// contextual-act form) — the gated, TU-costed act.
    #[must_use]
    pub const fn new(shover: Entity, target: Entity) -> Self {
        Self {
            shover,
            target,
            source: ShoveSource::Deliberate,
        }
    }

    /// Build a WEAPON-TAG auto-shove request for `shover` knocking `target` back on a
    /// connecting attack (GTW-525 C3) — the un-gated, TU-free form the melee / fire connect
    /// hooks write internally.
    #[must_use]
    pub const fn new_weapon(shover: Entity, target: Entity) -> Self {
        Self {
            shover,
            target,
            source: ShoveSource::Weapon,
        }
    }
}
