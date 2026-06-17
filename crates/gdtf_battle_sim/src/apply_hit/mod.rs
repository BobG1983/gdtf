//! Wound application + the terminal gates — the E3.6 slice (GTW-188).
//!
//! [`apply_hit`] folds **one already-resolved** hit onto a ganger
//! (`docs/combat/resolution.md` §9; `docs/combat/wounds-and-roster.md` §"From
//! damage to injury" / §"Severity tiers" / §"Downed → death … state machine"). It
//! is the per-hit application primitive the E3.9 capstone (`resolve_and_apply` /
//! `fire`) calls **after** the E3.3 damage formula ([`crate::resolve_hit`]) and the
//! E3.4 severity roll ([`crate::severity`]) have run — it takes the frozen
//! [`HitResult`](crate::resolve_hit::HitResult) and the rolled
//! [`Severity`](crate::severity::Severity) as **inputs** and recomputes neither.
//!
//! ## What it folds, in order
//!
//! 1. **Corpse-skip** — a ganger already at [`LifeState::Dead`](crate::ganger::LifeState::Dead)
//!    is **skipped entirely**: nothing mutates and no
//!    [`ArmorBroken`](crate::armor_wear::ArmorBroken) fires (the "corpse-skip
//!    discipline" of resolution.md §9's `resolve_and_apply`). Death is final; a
//!    later round in a burst cannot re-kill a corpse.
//! 2. **HP loss — ALWAYS** — the [`HitResult`](crate::resolve_hit::HitResult)'s
//!    [`HpDamage`](crate::resolve_hit::HpDamage) is subtracted from
//!    [`Hp`](crate::ganger::Hp), even on a [`Severity::None`](crate::severity::Severity::None)
//!    graze (a graze still bruises HP; wounds-and-roster.md §"From damage to
//!    injury": "A roll under the first bucket edge is a **graze**: HP loss only, no
//!    Wound"). The subtraction **saturates** at `0` — [`Hp`](crate::ganger::Hp) is
//!    unsigned, so a lethal HP hit depletes the pool to `0`, never underflowing.
//! 3. **Wounds by tier** — the [`Severity`](crate::severity::Severity) bucket spends
//!    the life pool ([`Wounds`](crate::ganger::Wounds)):
//!    [`Severity::None`](crate::severity::Severity::None) costs `0`,
//!    [`Severity::Minor`](crate::severity::Severity::Minor) /
//!    [`Severity::Major`](crate::severity::Severity::Major) /
//!    [`Severity::Critical`](crate::severity::Severity::Critical) cost the tunable
//!    [`crate::tuning::WoundCosts`], and [`Severity::Fatal`](crate::severity::Severity::Fatal)
//!    **empties** the pool (`Wounds → 0`) outright (wounds-and-roster.md §"Severity
//!    tiers": "**Fatal** … **dead** — outright, skips Downed"). The cost
//!    subtraction saturates at `0`.
//! 4. **Armor wear** — the [`HitResult`](crate::resolve_hit::HitResult)'s
//!    [`crate::resolve_hit::IntegrityWear`] is persisted onto the struck location of
//!    the battle-local [`WornArmor`](crate::armor::WornArmor) via the E3.5
//!    [`wear_armor`](crate::armor_wear::wear_armor) path, surfacing the
//!    `Some(`[`ArmorBroken`](crate::armor_wear::ArmorBroken)`)` on the single
//!    protecting→broken crossing for the caller to write to a message buffer.
//! 5. **Terminal gates, in order** (resolution.md §9: "`Wounds ≤ 0` → **Dead**
//!    (trumps Downed …), else `HP ≤ 0` → **Downed**"). Because the pools are
//!    unsigned and the subtractions saturate, "≤ 0" is reached as **depleted to
//!    `0`**: if [`Wounds`](crate::ganger::Wounds) is now `0` the ganger is
//!    [`LifeState::Dead`](crate::ganger::LifeState::Dead) (this **trumps** Downed —
//!    checked first); else if [`Hp`](crate::ganger::Hp) is now `0` the ganger is
//!    [`LifeState::Downed`](crate::ganger::LifeState::Downed). A single hit that
//!    depletes **both** pools yields **Dead, not Downed** — the trump.
//!
//! Pure, render-free model logic: no renderer, no pixel. [`apply_hit`] mutates the
//! ganger state in place through the borrowed [`GangerHitTarget`] bundle and
//! returns the armor-broken signal; the caller writes that `Some` to a
//! [`bevy::prelude::MessageWriter<ArmorBroken>`](crate::armor_wear::ArmorBroken) at
//! the system boundary (the same pure-helper / message-at-the-boundary split as
//! [`wear_armor`](crate::armor_wear::wear_armor)).

mod fold;

pub use fold::{GangerHitTarget, apply_hit};

#[cfg(test)]
mod test;
