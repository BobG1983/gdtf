//! The frozen value types of the E3.9 fold — the [`TargetGanger`] borrow-view the
//! fold mutates, and the `Copy` [`HitReport`] / [`AppliedDamage`] records it
//! returns. No-bare-types, no pixel: every field is a named domain newtype.

use crate::{
    armor::{BodyPart, WornArmor},
    armor_wear::ArmorBroken,
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    matchup::Matchup,
    resolve_coarse::ShotKind,
    resolve_hit::HitResult,
    severity::Severity,
};

/// The **bundle of one target ganger's battle state** [`resolve_and_apply`](super::resolve_and_apply)
/// folds a hit onto — the four `&mut` battle surfaces a hit can change, plus the
/// two read attribute stats the severity roll needs.
///
/// Grouping these into one named struct keeps
/// [`resolve_and_apply`](super::resolve_and_apply) under clippy's argument-count
/// gate (the [`GangerHitTarget`](crate::apply_hit::GangerHitTarget) /
/// [`SeverityInputs`](crate::severity::SeverityInputs) precedent). The mutable
/// borrows are exactly the [`GangerHitTarget`](crate::apply_hit::GangerHitTarget)
/// set (assembled from this bundle when
/// [`apply_hit`](crate::apply_hit::apply_hit) runs); the two reads
/// ([`Toughness`] / [`Luck`]) are the E3.0 defender attribute components fed to the
/// severity roll. Every field is an existing named domain component (no-bare-types).
/// The caller (a Bevy system, or the E4 `fire()` act) assembles this from the
/// target entity's components.
pub struct TargetGanger<'a> {
    /// The target's hit-points pool — the HP loss subtracts from it (always).
    pub hp:        &'a mut Hp,
    /// The target's Wounds (life) pool — the severity tier spends from it.
    pub wounds:    &'a mut Wounds,
    /// The target's terminal life state — the gates set it; corpse-skip reads it.
    pub life:      &'a mut LifeState,
    /// The target's battle-local worn armor — the struck piece wears in place; its
    /// [`protects`](WornArmor::protects) decides the armored-vs-bare-flesh branch.
    pub worn:      &'a mut WornArmor,
    /// The target's inflicted-wound record (GTW-279) — each registered wound appends
    /// its tier + struck part here (the additive presentation record for GTW-278).
    pub inflicted: &'a mut InflictedWounds,
    /// The target's Toughness — the defender's severity-mitigation term (E3.0, read).
    pub toughness: Toughness,
    /// The target's Luck — extends the severity roll's floor down (E3.0, read).
    pub luck:      Luck,
}

/// The **applied-damage block** of a [`HitReport`] — the resolved damage of a hit
/// that landed on a ganger (`docs/combat/resolution.md` §5 / §6).
///
/// A frozen `Copy` record of named newtypes (no bare primitive, no pixel): the
/// resolved [`Matchup`], the per-hit [`HitResult`], the rolled [`Severity`], the
/// ganger's [`LifeState`] **after** application, and the `Some(`[`ArmorBroken`]`)`
/// iff this hit broke the struck piece. The presenter reads it for FX; it is never
/// mutated after [`resolve_and_apply`](super::resolve_and_apply) returns. Present
/// only when the hit actually landed on a ganger — a non-ganger / corpse-skip /
/// no-part report carries `None` in [`HitReport::applied`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedDamage {
    /// The resolved weapon×armor matchup (E3.2) — [`Matchup::Neutral`] on bare flesh.
    pub matchup:    Matchup,
    /// The resolved per-hit damage / penetration / wear (E3.3).
    pub hit:        HitResult,
    /// The rolled wound severity bucket (E3.4) — the ONE RNG draw's outcome.
    pub severity:   Severity,
    /// The target's [`LifeState`] **after** the hit was applied (E3.6's terminal gates).
    pub life_after: LifeState,
    /// The armor-broken signal iff this hit broke the struck piece (E3.6) — else `None`.
    pub broken:     Option<ArmorBroken>,
}

/// The **frozen per-hit report** [`resolve_and_apply`](super::resolve_and_apply)
/// returns — the entire E3.9 fold's verdict (the frozen per-round report the
/// authoritative model hands the view; ADR-0001,
/// `docs/decisions/0001-rust-bevy-rewrite.md`).
///
/// A `Copy` value object of named domain types (no bare primitive, **no pixel** —
/// it carries only damage / wound math, never a screen coordinate). The presenter
/// reads it for FX staging; [`resolve_and_apply`](super::resolve_and_apply) owns no
/// mutation after it is returned.
///
/// - [`kind`](HitReport::kind) — what the shot struck (the
///   [`ShotOutcome`](crate::resolve_coarse::ShotOutcome)'s [`ShotKind`], carrying
///   the struck ganger / cover / surface-cell payload).
/// - [`part`](HitReport::part) — the struck [`BodyPart`], `Some` only for a hit
///   that landed on a ganger.
/// - [`applied`](HitReport::applied) — the [`AppliedDamage`] block, `Some` only
///   for a hit that landed on a ganger; `None` for a non-ganger outcome, a
///   corpse-skip, or a defensively-missing part (a **no-effect** report).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitReport {
    /// What the shot struck — the [`ShotOutcome`](crate::resolve_coarse::ShotOutcome)'s [`ShotKind`].
    pub kind:    ShotKind,
    /// The struck [`BodyPart`] — `Some` only when the hit landed on a ganger.
    pub part:    Option<BodyPart>,
    /// The applied-damage block — `Some` only when the hit landed on a ganger;
    /// `None` is a no-effect report (non-ganger / corpse-skip / no-part).
    pub applied: Option<AppliedDamage>,
}

impl HitReport {
    /// Build a **no-effect** report for `kind` — no part struck and no damage
    /// applied (the non-ganger, corpse-skip, and defensive-no-part folds).
    ///
    /// `pub` so the E4.5 `fire()` act (GTW-198) can fold a non-ganger / corpse-skip
    /// round to a no-effect report cross-module without re-deriving the shape.
    #[must_use]
    pub const fn no_effect(kind: ShotKind) -> Self {
        Self {
            kind,
            part: None,
            applied: None,
        }
    }
}
