//! The **shared apply contract** for the field-consequence palette — the
//! [`ApplyFieldEffect`] trait whose verbs ARE a consequence's behaviour, and the two
//! borrowed occupant surfaces ([`OccupantArmor`] / [`OccupantDrain`]) the
//! [`tick_fields`](crate::effects::fields::tick_fields) clock lends it for one fielded cell
//! (GTW-553, the field sibling of the GTW-558 attachment / GTW-550 injury / GTW-552
//! on-death palettes).
//!
//! Every isolated per-consequence `ApplyX` type (one per file in this module) impls this
//! trait; the closed [`FieldEffect`](super::FieldEffect) vocabulary enum impls it too, by
//! THIN mechanical delegation to each variant's isolated type. The mechanics never match
//! on the enum — [`tick_fields`](crate::effects::fields::tick_fields) and the
//! [`PlacedField`](crate::effects::fields::PlacedField) lifetime invoke this trait generically,
//! DIRECTLY on the drain path (synchronous — never a deferred command — so a lethal drain
//! flips [`LifeState::Dead`](crate::ganger::LifeState::Dead) and emits its
//! [`OnDeathOccurred`](crate::effects::on_death::OnDeathOccurred) at exactly the same point of the
//! round the pre-palette inline tick did).

use bevy::prelude::{Entity, MessageWriter, Mut, Query, With};

use super::FieldTurns;
use crate::{
    armor::{ArmorType, Wears, WornBy},
    effects::{fields::FieldTicked, on_death::OnDeathOccurred},
    ganger::{Hp, LifeState},
    metric::CellLevel,
};

/// The occupant's **worn-armor read surface** the exemption verb inspects — the borrowed
/// view over the occupant's [`Wears`] relationship plus the worn-piece [`ArmorType`]
/// lookup [`tick_fields`](crate::effects::fields::tick_fields) lends out for one fielded cell
/// (GTW-553).
///
/// A borrowed VIEW over the clock's own `SystemParam`s, never consequence-owned state: the
/// clock stays the one owner of the world access, and a consequence that needs a genuinely
/// NEW read surface adds a field here (one new view field), the
/// [`DeathFanOut`](crate::effects::on_death::DeathFanOut) precedent.
pub struct OccupantArmor<'a, 'w, 's> {
    /// The occupant's worn-piece relationship — which armor entities it wears.
    pub wears: &'a Wears,
    /// The worn-piece [`ArmorType`] lookup the exemption check resolves each piece through.
    pub worn:  &'a Query<'w, 's, &'static ArmorType, With<WornBy>>,
}

/// The occupant's mutable **drain surface** the per-turn drain verb writes through — the
/// borrowed view over ONE occupant's vitals plus the round's two signal writers,
/// lent out by [`tick_fields`](crate::effects::fields::tick_fields) for one fielded cell (GTW-553).
///
/// The vitals are lent as `&mut `[`Mut`]`<…>` (NOT bare `&mut`) so a consequence that
/// drains writes through [`Mut`] ONLY when it actually mutates — preserving the
/// pre-palette change-detection timing exactly (a bare re-borrow would dirty
/// `Changed<`[`LifeState`]`>` on every drained tick, not just the lethal one). Each field
/// carries its OWN inner lifetime (`&mut` is invariant — the clock's params live under
/// distinct anonymous lifetimes that must not be forced to unify).
pub struct OccupantDrain<'a, 'hp, 'life, 'wt, 'wd> {
    /// The occupant's HP pool the drain eats (saturating — floors at `0`).
    pub hp:     &'a mut Mut<'hp, Hp>,
    /// The occupant's life state — flipped to [`LifeState::Dead`] by a lethal drain.
    pub life:   &'a mut Mut<'life, LifeState>,
    /// The round's [`FieldTicked`] writer — one signal per draining tick (the FCT anchor).
    pub ticks:  &'a mut MessageWriter<'wt, FieldTicked>,
    /// The terminal-death writer — a lethal drain emits one
    /// [`OnDeathOccurred`] at the field cell so the GTW-547 resolver fans the dead
    /// ganger's authored on-death effect (a field-kill must not silently skip it).
    pub deaths: &'a mut MessageWriter<'wd, OnDeathOccurred>,
}

/// One field consequence's **isolated behaviour** — the trait each concrete consequence
/// type impls, its verbs BEING the consequence (GTW-553; the effect-isolation contract,
/// the GTW-558 attachment-palette precedent).
///
/// The whole point: a consequence's logic lives in exactly ONE place — its own type's impl
/// in its own palette file — not in a central `match`, an inline tick branch, or a scatter
/// of authoring steps. The [`FieldEffect`](super::FieldEffect) vocabulary enum impls this
/// by DELEGATING each variant to its isolated type (the palette's ONE mechanical match),
/// and the fields MECHANICS ([`tick_fields`](crate::effects::fields::tick_fields) + the
/// [`PlacedField`](crate::effects::fields::PlacedField) lifetime) invoke it generically.
///
/// The verbs mirror what field consequences actually do — every verb is DEFAULTED to the
/// inert answer, so a consequence overrides exactly the verb(s) that ARE its behaviour
/// (the injuries-palette `disables_hand` precedent) and a new consequence file never has
/// to name the others:
/// - **exempt** ([`exempts_occupant`](Self::exempts_occupant)) — the pre-drain occupant
///   gate (whole-source armor immunity). ANY consequence answering `true` skips the
///   occupant's whole drain this round.
/// - **drain** ([`drain_occupant`](Self::drain_occupant)) — the per-turn occupant
///   mutation (the flat HP drain + its signals).
/// - **lifetime** ([`initial_countdown`](Self::initial_countdown) /
///   [`count_down_one_turn`](Self::count_down_one_turn)) — the placement-time countdown
///   seed and the per-round expiry step of the Turns/Permanent duration.
pub trait ApplyFieldEffect {
    /// Whether this consequence **exempts** the occupant from the field's drain this
    /// round — the whole-source-immunity gate, asked of every consequence BEFORE any
    /// drain verb runs (an exempt occupant takes zero damage and emits no signal).
    ///
    /// Defaulted to `false` so only the exempting consequence overrides it.
    fn exempts_occupant(&self, _armor: &OccupantArmor<'_, '_, '_>) -> bool {
        false
    }

    /// **Drain** the occupant standing at the field cell `at` this round — the per-turn
    /// occupant mutation, applied synchronously through the borrowed [`OccupantDrain`]
    /// surface (never a deferred command: the lethal flip + its
    /// [`OnDeathOccurred`](crate::effects::on_death::OnDeathOccurred) must land the same tick the
    /// clock runs, the pre-palette timing).
    ///
    /// Defaulted to the inert no-op so only the draining consequence overrides it.
    /// Deterministic and render-free: an implementation takes NO RNG (the flat GTW-545
    /// drain — a field tick must not perturb the seeded autobattle stream).
    fn drain_occupant(
        &self,
        _at: CellLevel,
        _occupant: Entity,
        _drain: &mut OccupantDrain<'_, '_, '_, '_, '_>,
    ) {
    }

    /// The **countdown this consequence seeds** at placement time — the remaining-turns
    /// value a fresh [`PlacedField`](crate::effects::fields::PlacedField) starts from.
    ///
    /// Defaulted to zero so only the lifetime consequence overrides it (every other
    /// consequence contributes nothing to the countdown).
    fn initial_countdown(&self) -> FieldTurns {
        FieldTurns::new(0)
    }

    /// **Count the placement's lifetime down one round**, mutating `remaining`, and
    /// report `true` iff the placement is now EXPIRED (and must be removed) — the
    /// per-round expiry step [`tick_down_and_expire`](crate::effects::fields::FieldRegistry::tick_down_and_expire)
    /// drives after the round's drain.
    ///
    /// Defaulted to `false` with `remaining` untouched, so only the lifetime consequence
    /// overrides it (a consequence with no lifetime opinion never expires the field).
    fn count_down_one_turn(&self, _remaining: &mut FieldTurns) -> bool {
        false
    }
}
