//! The per-hit damage / penetration formula — the pure math that turns a landed
//! hit (weapon stats vs one [`ArmorPiece`](crate::armor::ArmorPiece), under a
//! resolved [`Matchup`](crate::matchup::Matchup)) into a frozen [`HitResult`].
//!
//! This is the E3.3 slice (`docs/combat/weapons-and-armor.md` §"Per-hit
//! resolution" / §"How the matchup wheel plugs in"; `docs/combat/resolution.md`
//! §5). It composes the E1 armor newtypes ([`crate::armor`]), the E3.1 weapon
//! damage newtypes ([`crate::weapon`]: [`WeaponDamage`](crate::weapon::WeaponDamage)
//! / [`WeaponPunch`](crate::weapon::WeaponPunch) /
//! [`WeaponShred`](crate::weapon::WeaponShred)), and the E3.2 matchup wheel
//! ([`crate::matchup`]: [`matchup_multiplier`](crate::matchup::matchup_multiplier))
//! into one function. It is **pure math** — it computes the [`HitResult`] and
//! mutates nothing: no HP, no Wounds, no armor wear is applied here (that
//! application is E3.6), and it carries no pixel.
//!
//! ## The formula (verbatim, `weapons-and-armor.md` §"Per-hit resolution")
//!
//! The matchup multiplier scales **punch & shred ONLY** — never `damage`,
//! `floor`, `protection`, `integrity`, or `hardness` (`weapons-and-armor.md`
//! §"How the matchup wheel plugs in", resolution.md §5). With `mult` the scalar
//! [`matchup_multiplier`](crate::matchup::matchup_multiplier) yields and
//! `punch_scaled`/`shred_scaled` the rounded products:
//!
//! 1. **Effective penetration:** `effPen = max(0, punch·mult − hardness)`
//! 2. **Inner damage (may be negative):** `inner = damage − max(0, protection − effPen)`
//! 3. **HP-loss damage:** `dmg = max(floor, inner)` — protection soaks, but the
//!    result never drops below `floor` ("a vest still bruises"). This is the HP
//!    reduction, [`HpDamage`].
//! 4. **Penetrating damage:** `pen = max(0, inner)` — the **pre-floor**
//!    penetration. This is the value that gates the §6 severity roll
//!    (resolution.md §6 `pen_damage`): a fully-soaked hit still bruises HP (step
//!    3's floor) yet penetrates `0`, so it can lose HP but roll no wound (a
//!    graze). It is [`PenetratingDamage`], and is the reason HP-loss and
//!    penetrating damage are **two distinct fields**, not one.
//! 5. **Armor integrity wear:** `integrity_wear = min(protection, damage) +
//!    effPen + shred·mult` — the soak/penetration wear plus the weapon's shred.
//!    This is the wear amount E3.6 spends against the worn piece's integrity;
//!    [`IntegrityWear`] only *computes* it (application/clamp is E3.6).
//!
//! Every value is an `i32`, matching the honest-signed arithmetic the armor
//! ([`crate::armor`]) and weapon damage ([`crate::weapon`]) stats already use:
//! `inner` legitimately goes negative when protection out-soaks the damage, and
//! the formula clamps it.

mod formula;
mod result;

#[cfg(test)]
mod test;

pub use formula::resolve_hit;
pub use result::{HitResult, HpDamage, IntegrityWear, PenetratingDamage};
