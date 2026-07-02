//! The [`OnDeathOccurred`] signal — the buffered message emitted from EVERY terminal
//! death / cover-destroyed gate (GTW-547, child GTW-41g).

use bevy::prelude::{Entity, Message};

use crate::metric::CellLevel;

/// A **terminal death happened** at a `(cell, level)` — a ganger reached
/// [`LifeState::Dead`](crate::ganger::LifeState) OR a piece of cover was destroyed
/// (GTW-547, child GTW-41g).
///
/// Emitted from EVERY terminal gate the sim reaches (the GTW-41 advanced-effects epic, child
/// GTW-41g — a ticket-defined feature not yet written into `docs/combat/resolution.md`): the
/// ranged (primary + splash) / melee / falls ganger-kill paths, the per-round bleed / DOT /
/// area-damage-field clocks, and both cover-destroy sites. The
/// [`resolve_on_death`](super::resolve_on_death) system drains this buffer, looks up the
/// dead source's authored [`OnDeathEffect`](super::OnDeathEffect) (from the ganger's wielded
/// weapon / gear, or the destroyed cover cell's authored def), and fans the matching
/// per-variant folder function at [`at`](OnDeathOccurred::at). A CHAIN reaction (an
/// [`Explode`](super::OnDeathEffect::Explode) that kills MORE) re-emits this message; the
/// resolver drains it to a visited-set fixpoint so every death is processed exactly once and
/// the cascade terminates same-frame (see [`resolve_on_death`](super::resolve_on_death)).
///
/// A buffered Bevy **message** (`bevy-traps.md` #4 — NOT the observer `Event`), written with
/// [`MessageWriter`](bevy::prelude::MessageWriter) and read with
/// [`MessageReader`](bevy::prelude::MessageReader), mirroring
/// [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) /
/// [`DotTicked`](crate::dot::DotTicked) / [`FieldTicked`](crate::fields::FieldTicked). The
/// [`entity`](OnDeathOccurred::entity) is a Bevy [`Entity`] handle (framework plumbing — the
/// only bare type the no-bare-types rule permits in a payload); it is
/// [`Entity::PLACEHOLDER`] for a COVER death (cover is not an entity — the resolver keys a
/// cover on-death off the [`at`](OnDeathOccurred::at) cell, never the entity).
/// [`at`](OnDeathOccurred::at) is the domain [`CellLevel`] the death / destruction landed at
/// (the ticket's `{ cell, level }`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OnDeathOccurred {
    /// The entity that died — a ganger's [`Entity`] for a ganger death, or
    /// [`Entity::PLACEHOLDER`] for a cover death (cover is not an entity; the resolver keys
    /// a cover on-death off [`at`](OnDeathOccurred::at)).
    pub entity: Entity,
    /// The `(cell, level)` the death / cover-destruction happened at — the fan-out origin the
    /// per-variant folder function centres its effect on.
    pub at:     CellLevel,
}

impl OnDeathOccurred {
    /// Build an on-death signal for `entity` that died at `at`.
    #[must_use]
    pub const fn new(entity: Entity, at: CellLevel) -> Self {
        Self { entity, at }
    }

    /// Build an on-death signal for a COVER destruction at `at` — the entity is
    /// [`Entity::PLACEHOLDER`] (cover is not an entity; the resolver keys a cover on-death
    /// off the cell, so the entity field is unused for a cover death).
    #[must_use]
    pub const fn cover(at: CellLevel) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            at,
        }
    }
}
