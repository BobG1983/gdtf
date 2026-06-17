//! The pure per-hit arithmetic — [`resolve_hit`] and its rounding helpers — that
//! turns weapon stats vs one [`ArmorPiece`] under a resolved [`Matchup`] into a
//! frozen [`HitResult`]. The matchup multiplier scales punch & shred ONLY.

use crate::{
    armor::ArmorPiece,
    matchup::{Matchup, matchup_multiplier},
    resolve_hit::result::{HitResult, HpDamage, IntegrityWear, PenetratingDamage},
    tuning::CombatTuning,
    weapon::{WeaponDamage, WeaponPunch, WeaponShred},
};

/// Round an `f32` product to the nearest `i32`, clamped into the `i32` range so a
/// wild value can never wrap on the cast.
///
/// The matchup multiplier scales punch & shred as `f32`; the formula then works in
/// `i32`. Rounding mode is **not** pinned by the design (it is unspecified tuning
/// detail) — this uses round-half-away-from-zero ([`f32::round`]). The clamp +
/// localized `#[expect]` is the crate's guarded-cast idiom (see
/// [`crate::metric`]'s `floor_to_i32`), so no `unwrap`/`expect` is needed.
const fn round_to_i32(value: f32) -> i32 {
    let rounded = value.round();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to the i32 range below, so the cast cannot wrap; fractional part is gone after round"
    )]
    let clamped = rounded.clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    clamped
}

/// Scale a weapon stat (`punch` or `shred`) by the matchup multiplier and round to
/// `i32` — the **only** stats the matchup touches (`weapons-and-armor.md` §"How the
/// matchup wheel plugs in", resolution.md §5).
///
/// `stat` is the weapon stat's already-`Deref`'d `i32`; `mult` is the matchup
/// scalar. Computed in `f32` (`stat × mult`) then rounded back via
/// [`round_to_i32`].
#[expect(
    clippy::cast_precision_loss,
    reason = "i32 stat → f32 for the matchup multiply; weapon punch/shred magnitudes are far inside f32's exact-integer range"
)]
pub(super) fn scale_by_matchup(stat: i32, mult: f32) -> i32 {
    round_to_i32(stat as f32 * mult)
}

/// Resolve a single landed hit into a [`HitResult`] — the per-hit damage /
/// penetration formula (`docs/combat/weapons-and-armor.md` §"Per-hit resolution",
/// `docs/combat/resolution.md` §5).
///
/// Pure math: it reads the weapon damage stats, the struck [`ArmorPiece`]'s
/// `floor` / `protection` / `hardness`, the resolved [`Matchup`], and the tuning
/// multipliers, and returns the frozen result. It applies **nothing** — HP,
/// Wounds, and armor wear are mutated by E3.6.
///
/// The matchup multiplier scales **punch & shred only** (never `damage` / `floor`
/// / `protection` / `integrity` / `hardness`). Then, with `effPen = max(0,
/// punch·mult − hardness)` and `inner = damage − max(0, protection − effPen)`:
/// the [`HpDamage`] is `max(floor, inner)` (floor bruise), the
/// [`PenetratingDamage`] is the **pre-floor** `max(0, inner)` (gates severity),
/// and the [`IntegrityWear`] is `min(protection, damage) + effPen + shred·mult`.
///
/// `armor`'s `integrity` is **not** read here — the formula's output needs only
/// `floor` / `protection` / `hardness`; the integrity wear is *computed* (its
/// application and the `≤ 0` clamp are E3.6).
#[must_use]
pub fn resolve_hit(
    weapon_damage: WeaponDamage,
    weapon_punch: WeaponPunch,
    weapon_shred: WeaponShred,
    armor: &ArmorPiece,
    matchup: Matchup,
    tuning: &CombatTuning,
) -> HitResult {
    // (a) The matchup scalar — the single hook (E3.2). Touches punch & shred only.
    let mult = *matchup_multiplier(matchup, tuning);

    // (b) Apply the multiplier to PUNCH & SHRED ONLY (rounded back to i32).
    let punch_scaled = scale_by_matchup(*weapon_punch, mult);
    let shred_scaled = scale_by_matchup(*weapon_shred, mult);

    // The armor stats the formula reads (integrity is NOT needed for the output).
    let floor = *armor.floor;
    let protection = *armor.protection;
    let hardness = *armor.hardness;
    let damage = *weapon_damage;

    // (c) Effective penetration: floors at zero — hardness ≥ punch·mult ⇒ effPen 0.
    let eff_pen = (punch_scaled - hardness).max(0);

    // (d) Inner damage — may be negative when protection out-soaks the damage.
    let inner = damage - (protection - eff_pen).max(0);

    // (e) HP-loss damage: floor-clamped ("a vest still bruises").
    let hp_damage = HpDamage::new(floor.max(inner));

    // (f) Penetrating damage: PRE-floor max(0, inner) — gates wound severity (§6).
    let penetrating = PenetratingDamage::new(inner.max(0));

    // (g) Armor integrity wear: soak/pen wear + scaled shred (computed, not applied).
    let wear = IntegrityWear::new(protection.min(damage) + eff_pen + shred_scaled);

    HitResult {
        penetrating,
        hp_damage,
        wear,
    }
}
