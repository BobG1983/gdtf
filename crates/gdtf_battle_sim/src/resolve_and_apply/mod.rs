//! The E3.9 **capstone integrator** — [`resolve_and_apply`] folds one
//! [`ShotOutcome`](crate::resolve_coarse::ShotOutcome) → damage → severity →
//! application into ONE model-side act and returns a FROZEN per-hit report.
//!
//! This is the last E3 slice (`docs/combat/resolution.md` §5 / §6 / §"What's pure
//! math vs sim"; `docs/combat/weapons-and-armor.md` §"Per-hit resolution"). It
//! applies the result as ONE model-side act (armor → severity → application, with
//! corpse-skip draw discipline) and returns the frozen per-round reports — the
//! authoritative-model role this crate plays in the model/view split (ADR-0001,
//! `docs/decisions/0001-rust-bevy-rewrite.md`). It **composes the already-built E3
//! verbs** — it rebuilds none of them:
//!
//! 1. **Kind gate** — only a [`ShotKind::Ganger`](crate::resolve_coarse::ShotKind::Ganger)
//!    outcome can WOUND. A [`ShotKind::Cover`](crate::resolve_coarse::ShotKind::Cover)
//!    (GTW-364) or [`ShotKind::Slab`](crate::resolve_coarse::ShotKind::Slab) (GTW-365)
//!    outcome instead spends the struck structural surface's OWN HP through the SAME
//!    [`resolve_hit`](crate::resolve_hit::resolve_hit) damage formula (against its own
//!    armor) and records a destroyed `(cell, level)` on depletion to zero — **no wound
//!    and no RNG draw**. A [`ShotKind::Ground`](crate::resolve_coarse::ShotKind::Ground)
//!    / [`ShotKind::Miss`](crate::resolve_coarse::ShotKind::Miss) outcome folds to a
//!    **no-effect** report (no wound, no HP spent, no draw).
//! 2. **Corpse-skip — BEFORE any draw** — a target already at
//!    [`LifeState::Dead`](crate::ganger::LifeState::Dead) yields a **no-effect**
//!    report: nothing mutates and **no draw is taken**, so a corpse can never
//!    consume an RNG draw (determinism is preserved) and a later round in a burst
//!    cannot re-wound a corpse (resolution.md §9's corpse-skip discipline).
//! 3. **Part** — the struck [`BodyPart`](crate::armor::BodyPart) is the outcome's
//!    [`ShotOutcome::body_part`](crate::resolve_coarse::ShotOutcome::body_part)
//!    (the §4 location roll already drawn upstream). A `Ganger` outcome carries
//!    `Some`; a defensive `None` folds to a no-damage report.
//! 4. **Armor lookup (or bare flesh)** — if the struck part's worn piece entity
//!    still protects (its [`ArmorIntegrity`](crate::armor::ArmorIntegrity) `> 0`), the
//!    hit resolves against that [`ArmorPiece`](crate::armor::ArmorPiece) under the E3.2
//!    [`matchup`](crate::matchup::matchup) of the weapon's
//!    [`DamageType`](crate::weapon::DamageType) vs the piece's
//!    [`ArmorType`](crate::armor::ArmorType). Otherwise it resolves as **bare
//!    flesh**: a zeroed soak (floor / protection / hardness all `0`) under
//!    [`Matchup::Neutral`](crate::matchup::Matchup::Neutral) (there is no armor
//!    type to match against, so no wheel advantage —
//!    `weapons-and-armor.md` §"Per-hit resolution": "later hits on that location
//!    resolve as bare flesh").
//! 5. **Damage** — E3.3 [`resolve_hit`](crate::resolve_hit::resolve_hit) →
//!    [`HitResult`](crate::resolve_hit::HitResult) against that piece + matchup.
//! 6. **Severity — the ONE draw** — E3.4
//!    [`roll_severity`](crate::severity::roll_severity) over the
//!    [`SeverityInputs`](crate::severity::SeverityInputs) assembled from the hit's
//!    penetrating damage, the defender's [`Toughness`](crate::ganger::Toughness),
//!    the struck part's [`part_severity_mod`](crate::severity::part_severity_mod),
//!    the weapon's [`FatalBias`](crate::weapon::FatalBias), and **both** gangers'
//!    [`Luck`](crate::ganger::Luck). The injected [`SimRng`](crate::rng::SimRng) is
//!    the **single draw point** — no `thread_rng`, no ad-hoc entropy.
//! 7. **Apply** — E3.6 [`apply_hit`](crate::apply_hit::apply_hit) folds HP loss +
//!    Wounds-by-tier + the GTW-279 [`InflictedWounds`](crate::inflicted_wound::InflictedWounds)
//!    record + armor wear + the terminal gates onto the target in place, surfacing
//!    the `Some(`[`ArmorBroken`](crate::armor_wear::ArmorBroken)`)` on a
//!    protecting→broken crossing.
//! 8. **Freeze** — the returned [`HitReport`] is a `Copy` record of named newtypes
//!    (the matchup, the [`HitResult`](crate::resolve_hit::HitResult), the
//!    [`Severity`](crate::severity::Severity), the resulting
//!    [`LifeState`](crate::ganger::LifeState), the optional
//!    [`ArmorBroken`](crate::armor_wear::ArmorBroken), the struck
//!    [`BodyPart`](crate::armor::BodyPart)) for the presenter's FX.
//!    [`resolve_and_apply`] owns **no** mutation after return.
//!
//! [`resolve_and_apply`] is the MODEL-side integrator. It does **not** charge TU or
//! loop the burst — that is the E4 `fire()` act (resolution.md §"What's pure math
//! vs sim"). Pure, render-free model logic: it carries **no pixel** — the report
//! holds only damage / wound math, never a screen coordinate.

mod fold;
mod report;

#[cfg(test)]
mod test;

pub use fold::resolve_and_apply;
pub use report::{AppliedDamage, HitReport, StruckPiece, StruckSurfaces, TargetGanger};
