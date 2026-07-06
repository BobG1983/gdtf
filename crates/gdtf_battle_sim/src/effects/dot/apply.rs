//! The [`DotApplied`] boundary message + the [`apply_dot`] applier — the GTW-544 seam that
//! turns a frozen [`GangerVerdict::dot_applied`](crate::resolve_and_apply::GangerVerdict::dot_applied)
//! attach decision into a persistent [`Dot`](crate::weapon::Dot) on the struck ganger.
//!
//! The DECISION is PURE + in-fold ([`resolve_and_apply`](crate::resolve_and_apply::resolve_and_apply),
//! frozen onto the report); the side effects live HERE, at the message boundary (mirroring
//! the [`InjuryInflicted`](crate::acts::InjuryInflicted) / `CoverDestroyed` bridge precedent,
//! `bevy-traps.md` #7 — no `&mut World`):
//!
//! 1. [`dispatch_fire`](crate::acts::dispatch_fire) emits one [`DotApplied`] per fired round
//!    whose [`GangerVerdict::dot_applied`](crate::resolve_and_apply::GangerVerdict::dot_applied) is
//!    `Some`, carrying the struck target [`Entity`] + the [`Dot`](crate::weapon::Dot) to
//!    attach.
//! 2. [`apply_dot`] drains that buffer and, for each message, ATTACHES the
//!    [`Dot`](crate::weapon::Dot) if absent, or REFRESHES it if already present
//!    ([`Dot::refresh_from`](crate::weapon::Dot::refresh_from) — refresh-not-stack: a second
//!    penetrating DOT hit RESETS the affliction's turns + per-turn damage rather than
//!    stacking).

use bevy::prelude::{Commands, Entity, Message, MessageReader, MessageWriter, Query};

use crate::weapon::{Dot, DotDamage};

/// One **DOT was applied** — the GTW-544 boundary message bridging a frozen
/// [`GangerVerdict::dot_applied`](crate::resolve_and_apply::GangerVerdict::dot_applied) attach
/// decision into the [`apply_dot`] applier.
///
/// Emitted once per fired round whose report attached a DOT (a penetrating hit from a DOT
/// weapon), by [`dispatch_fire`](crate::acts::dispatch_fire) (the
/// [`InjuryInflicted`](crate::acts::InjuryInflicted) / cover-destroyed bridge precedent). It
/// carries the struck [`target`](DotApplied::target) [`Entity`] and the
/// [`dot`](DotApplied::dot) to attach or REFRESH.
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), written with
/// [`MessageWriter`](bevy::prelude::MessageWriter) and read with [`MessageReader`], mirroring
/// [`Bleeding`](crate::effects::bleed::Bleeding) / [`InjuryInflicted`](crate::acts::InjuryInflicted).
/// The [`target`](DotApplied::target) is a Bevy [`Entity`] handle (framework plumbing, the
/// only bare type the no-bare-types rule permits in a payload); [`dot`](DotApplied::dot) is
/// the domain [`Dot`](crate::weapon::Dot) value.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DotApplied {
    /// The struck ganger — the entity the [`Dot`](crate::weapon::Dot) attaches (or refreshes) on.
    pub target: Entity,
    /// The DOT affliction to attach (built from the firing weapon's
    /// [`DotProfile`](crate::weapon::DotProfile) at the penetrating hit).
    pub dot:    Dot,
}

impl DotApplied {
    /// Build a [`DotApplied`] for `target` carrying the `dot` to attach — the fire-act
    /// bridge (GTW-544).
    #[must_use]
    pub const fn new(target: Entity, dot: Dot) -> Self {
        Self { target, dot }
    }
}

/// A DOT **affliction span STARTED** — a fresh [`Dot`](crate::weapon::Dot) was ATTACHED to
/// `ganger` (GTW-572).
///
/// The once-at-affliction-start fact the combat log's DOT line reads (the Q2 ruling:
/// DOT logs ONCE at affliction start, never per tick). Emitted by [`apply_dot`] ONLY on the
/// fresh-ATTACH branch: a refresh of an already-burning ganger (refresh-not-stack, the
/// GTW-544 locked design) is mid-affliction and emits nothing, and the per-round
/// [`DotTicked`](super::tick::DotTicked) drain never emits it. The sim emits the FACT — the
/// afflicted ganger + the per-turn drain magnitude — never a rendered line (ADR-0001).
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`DotApplied`]. The [`ganger`](DotAfflicted::ganger) is a Bevy [`Entity`] handle
/// (framework plumbing, the only bare type the no-bare-types rule permits in a payload).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DotAfflicted {
    /// The freshly-afflicted ganger — resolved to a display name at the presenter boundary.
    pub ganger:   Entity,
    /// The affliction's flat per-turn HP drain (the attached DOT's profile magnitude).
    pub per_turn: DotDamage,
}

impl DotAfflicted {
    /// Build a DOT-affliction-started fact for `ganger` draining `per_turn` HP each round.
    #[must_use]
    pub const fn new(ganger: Entity, per_turn: DotDamage) -> Self {
        Self { ganger, per_turn }
    }
}

/// **Apply** every buffered [`DotApplied`] to its target ganger — the GTW-544 boundary
/// system (the [`apply_injury`](crate::acts::apply_injury) / cover-destroyed bridge
/// precedent, `bevy-traps.md` #7 — query / [`Commands`] / [`MessageReader`], no `&mut
/// World`).
///
/// For each drained message: if the target already carries a [`Dot`], REFRESH it in place
/// ([`Dot::refresh_from`](crate::weapon::Dot::refresh_from) — refresh-not-stack: reset both
/// the remaining turns AND the per-turn damage to the new attach, so a second penetrating DOT
/// hit REPLACES rather than stacks); otherwise ATTACH the [`Dot`] via [`Commands`]. A message
/// whose target is despawned or absent is a no-op (fail-closed). Takes NO RNG draw — the
/// attach decision already happened in-fold (the determinism property). Param-only
/// (`bevy-traps.md` #7): a [`MessageReader`] (drain), a [`Query`] for the target's existing
/// [`Dot`], and [`Commands`] (the attach) — no `&mut World`.
pub fn apply_dot(
    mut applied: MessageReader<DotApplied>,
    mut existing: Query<&mut Dot>,
    mut commands: Commands,
    // GTW-572: the once-at-affliction-start fact — written ONLY on the fresh-ATTACH branch
    // (a refresh is mid-affliction; the combat log's DOT line logs once per span, never per
    // tick — the Q2 ruling). `bevy-traps.md` #4: the buffer is registered by the acts plugin.
    mut afflicted: MessageWriter<DotAfflicted>,
) {
    // Track targets freshly attached THIS drain: a second DotApplied for the same target in
    // one tick sees only the deferred Commands insert (not the live Query), so without this
    // set it would wrongly read as a second fresh attach and double-emit the start fact.
    let mut attached_this_tick: bevy::platform::collections::HashSet<Entity> =
        bevy::platform::collections::HashSet::default();
    for message in applied.read() {
        let target = message.target;
        // A despawned target — skip (fail-closed; the entity may have been removed between
        // fire resolution and this applier).
        let Ok(mut entity) = commands.get_entity(target) else {
            continue;
        };
        if let Ok(mut dot) = existing.get_mut(target) {
            // REFRESH-not-stack: reset the affliction to the fresh attach (turns + per-turn
            // damage), the GTW-544 locked design — a second DOT hit does not extend or add.
            dot.refresh_from(crate::weapon::DotProfile::new(
                message.dot.per_turn_damage,
                message.dot.damage_type,
                message.dot.remaining_turns,
            ));
        } else if attached_this_tick.contains(&target) {
            // Already freshly attached earlier THIS drain — treat the second hit as the
            // refresh it is (last write wins via the deferred insert), no second start fact.
            entity.insert(message.dot);
        } else {
            // No existing DOT — attach the fresh one and emit the affliction-start fact.
            entity.insert(message.dot);
            afflicted.write(DotAfflicted::new(target, message.dot.per_turn_damage));
            attached_this_tick.insert(target);
        }
    }
}
