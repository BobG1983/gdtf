//! The per-hit damage / penetration formula — the pure math that turns a landed
//! hit (weapon stats vs one [`ArmorPiece`], under a resolved [`Matchup`]) into a
//! frozen [`HitResult`].
//!
//! This is the E3.3 slice (`docs/combat/weapons-and-armor.md` §"Per-hit
//! resolution" / §"How the matchup wheel plugs in"; `docs/combat/resolution.md`
//! §5). It composes the E1 armor newtypes ([`crate::armor`]), the E3.1 weapon
//! damage newtypes ([`crate::weapon`]: [`WeaponDamage`] / [`WeaponPunch`] /
//! [`WeaponShred`]), and the E3.2 matchup wheel ([`crate::matchup`]:
//! [`matchup_multiplier`]) into one function. It is **pure math** — it computes
//! the [`HitResult`] and mutates nothing: no HP, no Wounds, no armor wear is
//! applied here (that application is E3.6), and it carries no pixel.
//!
//! ## The formula (verbatim, `weapons-and-armor.md` §"Per-hit resolution")
//!
//! The matchup multiplier scales **punch & shred ONLY** — never `damage`,
//! `floor`, `protection`, `integrity`, or `hardness` (`weapons-and-armor.md`
//! §"How the matchup wheel plugs in", resolution.md §5). With `mult` the scalar
//! [`matchup_multiplier`] yields and `punch_scaled`/`shred_scaled` the rounded
//! products:
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

use bevy::prelude::Deref;

use crate::{
    armor::ArmorPiece,
    matchup::{Matchup, matchup_multiplier},
    tuning::CombatTuning,
    weapon::{WeaponDamage, WeaponPunch, WeaponShred},
};

/// The **penetrating damage** a hit drives into the target — the **pre-floor**
/// `max(0, inner)` of the per-hit formula (`docs/combat/resolution.md` §6
/// `pen_damage`; `weapons-and-armor.md` §"Per-hit resolution").
///
/// This is the value the §6 wound-severity roll is **gated by** (`roll_severity`,
/// E3.4) — "severity is penetration-gated", so a weak hit cannot reach the severe
/// buckets. It is **distinct from** [`HpDamage`]: a fully-soaked hit still bruises
/// HP (the floor) while penetrating `0`, which is exactly the resolution.md §6
/// graze (`< e0 → None`: HP loss only, no wound). A named domain newtype, not a
/// bare `i32` (no-bare-types); private inner + derived [`Deref`]. An `i32` to share
/// the formula's signed arithmetic (it is the clamp of a possibly-negative inner).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PenetratingDamage(i32);

impl PenetratingDamage {
    /// Build a penetrating-damage value from its computed magnitude.
    #[must_use]
    pub const fn new(pen: i32) -> Self {
        Self(pen)
    }
}

/// The **HP-loss damage** a hit deals — the `dmg = max(floor, inner)` of the
/// per-hit formula (`docs/combat/weapons-and-armor.md` §"Per-hit resolution" step
/// 2: "Damage to the ganger").
///
/// This is the HP reduction the (later E3.6) application spends against the
/// ganger's HP. It is floor-clamped: "a hit that lands on armor always deals at
/// least `floor`" (a vest still bruises). It is **distinct from**
/// [`PenetratingDamage`] — that one is pre-floor and gates wound severity, this
/// one is the post-floor HP loss. A named domain newtype (no-bare-types); private
/// inner + derived [`Deref`]; an `i32` matching the armor/weapon signed stats.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HpDamage(i32);

impl HpDamage {
    /// Build an HP-loss-damage value from its computed magnitude.
    #[must_use]
    pub const fn new(dmg: i32) -> Self {
        Self(dmg)
    }
}

/// The **integrity wear** a hit inflicts on the struck armor piece — the
/// `integrity_wear = min(protection, damage) + effPen + shred·mult` of the per-hit
/// formula (`docs/combat/weapons-and-armor.md` §"Per-hit resolution" step 3).
///
/// This is the amount the (later E3.6) application subtracts from the worn piece's
/// [`crate::armor::ArmorIntegrity`] — the soak/penetration wear **plus** the
/// weapon's shred (extra durability damage). This slice only **computes** the
/// wear; the actual `integrity −= …` mutation and the "useless at `≤ 0`" gate are
/// E3.6. A named domain newtype (no-bare-types); private inner + derived [`Deref`];
/// an `i32` matching [`crate::armor::ArmorIntegrity`]'s signed inner.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntegrityWear(i32);

impl IntegrityWear {
    /// Build an integrity-wear value from its computed magnitude.
    #[must_use]
    pub const fn new(wear: i32) -> Self {
        Self(wear)
    }
}

/// The **frozen record** a single per-hit resolution produces — the output of
/// [`resolve_hit`] (`docs/combat/resolution.md` §5, `weapons-and-armor.md`
/// §"Per-hit resolution").
///
/// A `Copy` value object of named newtypes (no bare primitive), carrying the three
/// downstream-consumed results of the formula:
/// - [`PenetratingDamage`] — drives the E3.4 wound-severity roll (penetration-gated);
/// - [`HpDamage`] — the HP reduction E3.6 applies;
/// - [`IntegrityWear`] — the armor wear E3.6 spends.
///
/// It is **pure data** — `resolve_hit` mutates nothing to make it; HP / Wounds /
/// armor application is E3.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResult {
    /// Pre-floor penetrating damage — gates wound severity (E3.4).
    pub penetrating: PenetratingDamage,
    /// Post-floor HP-loss damage — the HP reduction (applied in E3.6).
    pub hp_damage:   HpDamage,
    /// Armor integrity wear — the durability damage (applied in E3.6).
    pub wear:        IntegrityWear,
}

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
fn scale_by_matchup(stat: i32, mult: f32) -> i32 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart,
        SourceArmor,
    };

    /// Build a single [`ArmorPiece`] from arbitrary (NOT shipped-tuning)
    /// magnitudes. The numbers are chosen per-test to put the formula in the regime
    /// the test exercises; they are mechanism inputs, never asserted as values.
    fn armor_piece(floor: i32, protection: i32, hardness: i32) -> ArmorPiece {
        ArmorPiece::new(
            ArmorFloor::new(floor),
            ArmorProtection::new(protection),
            // Integrity is irrelevant to the formula output; an arbitrary positive.
            ArmorIntegrity::new(100),
            ArmorHardness::new(hardness),
            ArmorType::DEFAULT,
        )
    }

    /// AC1 — the HP-loss damage never drops below the armor `floor`. With high
    /// protection and low damage, `inner` is driven below `floor`; assert the
    /// clamp holds (relation, not magnitude).
    #[test]
    fn hp_damage_clamps_up_to_floor() {
        let tuning = CombatTuning::default();
        // protection (20) ≫ damage (3): inner = 3 − 20 < 0 < floor (4).
        let armor = armor_piece(4, 20, 0);
        let result = resolve_hit(
            WeaponDamage::new(3),
            WeaponPunch::new(0),
            WeaponShred::new(0),
            &armor,
            Matchup::Neutral,
            &tuning,
        );

        assert_eq!(
            *result.hp_damage, *armor.floor,
            "HP-loss damage must clamp up to the armor floor when inner < floor",
        );
        assert!(
            *result.hp_damage >= *armor.floor,
            "HP-loss damage must never drop below the floor",
        );
        // And the fully-soaked hit penetrates nothing (pre-floor inner ≤ 0) — the
        // graze case: HP loss (the floor bruise) but no wound.
        assert_eq!(
            *result.penetrating, 0,
            "a fully-soaked hit must penetrate 0 (the graze: HP loss, no wound)",
        );
    }

    /// AC2 — effective penetration floors at zero. With `hardness ≥ punch·mult`,
    /// `effPen == 0`, so the soak is undiminished: the outcome equals the
    /// no-penetration baseline `dmg = max(floor, damage − protection)`. Asserting
    /// that equality surfaces `effPen == 0`.
    #[test]
    fn eff_pen_floors_at_zero_when_hardness_dominates() {
        let tuning = CombatTuning::default();
        // hardness (50) ≫ punch·mult (≈5·1=5): effPen = max(0, 5 − 50) = 0.
        let armor = armor_piece(1, 6, 50);
        let damage = 10;
        let protection = *armor.protection;
        let floor = *armor.floor;
        let result = resolve_hit(
            WeaponDamage::new(damage),
            WeaponPunch::new(5),
            WeaponShred::new(0),
            &armor,
            Matchup::Neutral,
            &tuning,
        );

        // effPen == 0 ⇒ soak undiminished ⇒ inner = damage − protection.
        let baseline_hp = floor.max(damage - protection);
        let baseline_pen = (damage - protection).max(0);
        assert_eq!(
            *result.hp_damage, baseline_hp,
            "with effPen 0 the soak is undiminished: dmg == max(floor, damage − protection)",
        );
        assert_eq!(
            *result.penetrating, baseline_pen,
            "with effPen 0 the penetrating damage equals the no-penetration baseline",
        );
    }

    /// AC3 — penetrating damage rises monotonically with punch (all else fixed):
    /// a higher punch yields `>=` penetrating damage (ordering relation).
    #[test]
    fn penetrating_damage_monotonic_in_punch() {
        let tuning = CombatTuning::default();
        let armor = armor_piece(0, 10, 2);
        let resolve = |punch: i32| {
            resolve_hit(
                WeaponDamage::new(12),
                WeaponPunch::new(punch),
                WeaponShred::new(0),
                &armor,
                Matchup::Neutral,
                &tuning,
            )
        };

        let low = resolve(3);
        let high = resolve(8);
        assert!(
            *high.penetrating >= *low.penetrating,
            "higher punch must yield >= penetrating damage (got high {} < low {})",
            *high.penetrating,
            *low.penetrating,
        );
    }

    /// AC4 — a Favorable matchup yields `>=` penetrating damage AND `>=` integrity
    /// wear than a Resisted one on the same weapon/armor — the hook is felt through
    /// punch & shred. The same test asserts the HP-loss difference traces to PUNCH
    /// (penetration eating soak), never to `damage`/`floor`/`protection` changing.
    #[test]
    fn favorable_beats_resisted_through_punch_and_shred() {
        let tuning = CombatTuning::default();
        // Soak is in play (protection > damage − full-pen) so punch scaling is felt
        // on the HP-loss side; shred > 0 so the wear difference is felt too.
        let armor = armor_piece(1, 12, 1);
        let resolve = |matchup: Matchup| {
            resolve_hit(
                WeaponDamage::new(14),
                WeaponPunch::new(10),
                WeaponShred::new(6),
                &armor,
                matchup,
                &tuning,
            )
        };

        let favorable = resolve(Matchup::Favorable);
        let resisted = resolve(Matchup::Resisted);

        assert!(
            *favorable.penetrating >= *resisted.penetrating,
            "Favorable must yield >= penetrating damage than Resisted",
        );
        assert!(
            *favorable.wear >= *resisted.wear,
            "Favorable must yield >= integrity wear than Resisted",
        );

        // The HP-loss difference traces to PUNCH only: recompute the expected
        // HP-loss from the SAME damage/floor/protection but the per-matchup scaled
        // punch — if it matches, nothing but punch moved.
        let mult_fav = *matchup_multiplier(Matchup::Favorable, &tuning);
        let mult_res = *matchup_multiplier(Matchup::Resisted, &tuning);
        let expected_hp = |mult: f32| {
            let punch_scaled = scale_by_matchup(10, mult);
            let eff_pen = (punch_scaled - *armor.hardness).max(0);
            let inner = 14 - (*armor.protection - eff_pen).max(0);
            (*armor.floor).max(inner)
        };
        assert_eq!(
            *favorable.hp_damage,
            expected_hp(mult_fav),
            "Favorable HP-loss must trace to the scaled PUNCH, not to damage/floor/protection",
        );
        assert_eq!(
            *resisted.hp_damage,
            expected_hp(mult_res),
            "Resisted HP-loss must trace to the scaled PUNCH, not to damage/floor/protection",
        );
    }

    /// AC5 — integrity wear mechanism: shred adds to wear ON TOP of the soak/pen
    /// term. Compare `shred = 0` vs `shred > 0`, all else equal, and assert the
    /// higher-shred wear is greater by exactly the scaled shred. Also assert the
    /// matchup multiplier leaves `damage`/`floor` untouched: in a fully-soaked
    /// regime the HP-loss stays pinned to `floor` across Neutral vs Favorable.
    #[test]
    fn shred_adds_to_wear_and_matchup_leaves_damage_floor() {
        let tuning = CombatTuning::default();
        let armor = armor_piece(2, 8, 1);
        let resolve = |shred: i32, matchup: Matchup| {
            resolve_hit(
                WeaponDamage::new(9),
                WeaponPunch::new(4),
                WeaponShred::new(shred),
                &armor,
                matchup,
                &tuning,
            )
        };

        // shred adds ON TOP of the soak/pen term: the only difference is the scaled
        // shred (mult == 1.0 at Neutral, so the difference is exactly the shred).
        let no_shred = resolve(0, Matchup::Neutral);
        let with_shred = resolve(5, Matchup::Neutral);
        let neutral_mult = *matchup_multiplier(Matchup::Neutral, &tuning);
        let scaled_shred = scale_by_matchup(5, neutral_mult);
        assert_eq!(
            *with_shred.wear - *no_shred.wear,
            scaled_shred,
            "shred must add to integrity wear on top of the soak/pen term",
        );
        assert!(
            *with_shred.wear > *no_shred.wear,
            "shred > 0 must raise integrity wear above the shred = 0 baseline",
        );

        // The matchup multiplier scales PUNCH & SHRED only — never damage/floor.
        // Fully-soaked regime (high protection, low punch): inner < floor, so the
        // HP-loss is pinned to `floor` and is UNCHANGED across Neutral vs Favorable.
        let soaked = armor_piece(3, 30, 40); // hardness 40 ⇒ effPen 0 even favorable
        let soak_resolve = |matchup: Matchup| {
            resolve_hit(
                WeaponDamage::new(2),
                WeaponPunch::new(5),
                WeaponShred::new(0),
                &soaked,
                matchup,
                &tuning,
            )
        };
        let neutral = soak_resolve(Matchup::Neutral);
        let favorable = soak_resolve(Matchup::Favorable);
        assert_eq!(
            *neutral.hp_damage, *soaked.floor,
            "fully-soaked HP-loss must pin to the armor floor",
        );
        assert_eq!(
            *favorable.hp_damage, *neutral.hp_damage,
            "the matchup multiplier must NOT change damage/floor — HP-loss stays at floor",
        );
    }

    /// AC6 (mechanism) — the three [`HitResult`] fields `Deref` to their inner
    /// `i32` (all named newtypes, no bare primitive escapes). A round-trip pin on
    /// the newtype Deref, built from arbitrary literals (mechanism, not magnitude).
    #[test]
    fn hit_result_fields_are_named_newtypes() {
        let result = HitResult {
            penetrating: PenetratingDamage::new(7),
            hp_damage:   HpDamage::new(8),
            wear:        IntegrityWear::new(9),
        };
        assert_eq!(*result.penetrating, 7i32);
        assert_eq!(*result.hp_damage, 8i32);
        assert_eq!(*result.wear, 9i32);
    }

    /// The HP-loss field is post-floor while the penetrating field is pre-floor —
    /// the doc-grounded **two-field split** (`weapons-and-armor.md` §"Per-hit
    /// resolution" `dmg` vs resolution.md §6 `pen_damage`). In a soaked-but-bruising
    /// regime they MUST differ: HP-loss == floor (> 0), penetrating == 0.
    #[test]
    fn pen_and_hp_damage_split_pre_and_post_floor() {
        let tuning = CombatTuning::default();
        // Fully soaked: inner < 0 < floor. HP-loss clamps to floor; pen clamps to 0.
        let armor = armor_piece(5, 25, 0);
        let result = resolve_hit(
            WeaponDamage::new(4),
            WeaponPunch::new(0),
            WeaponShred::new(0),
            &armor,
            Matchup::Neutral,
            &tuning,
        );
        assert_eq!(
            *result.hp_damage, 5,
            "HP-loss (post-floor) must be the floor bruise",
        );
        assert_eq!(
            *result.penetrating, 0,
            "penetrating (pre-floor) must be 0 — distinct from the floor-bruised HP loss",
        );
        assert_ne!(
            *result.hp_damage, *result.penetrating,
            "the two-field split: post-floor HP-loss and pre-floor penetration differ in the graze regime",
        );
    }

    /// A full uniform-suit round-trip through [`SourceArmor`] — the struck piece is
    /// read off an `ArmorPiece` exactly as a real call site would supply it
    /// (mechanism: the formula reads floor/protection/hardness off the piece).
    #[test]
    fn resolves_against_a_piece_from_a_source_suit() {
        let tuning = CombatTuning::default();
        let piece = armor_piece(1, 5, 2);
        let suit = SourceArmor::uniform(piece);
        let struck = suit.at(BodyPart::Torso);

        let result = resolve_hit(
            WeaponDamage::new(10),
            WeaponPunch::new(6),
            WeaponShred::new(3),
            &struck,
            Matchup::Neutral,
            &tuning,
        );

        // Hand-computed at Neutral (mult 1.0): effPen = max(0, 6 − 2) = 4;
        // inner = 10 − max(0, 5 − 4) = 10 − 1 = 9; hp = max(1, 9) = 9; pen = 9;
        // wear = min(5, 10) + 4 + 3 = 12.
        assert_eq!(*result.hp_damage, 9, "HP-loss from the soak/pen formula");
        assert_eq!(
            *result.penetrating, 9,
            "penetrating equals inner when inner > 0"
        );
        assert_eq!(
            *result.wear, 12,
            "wear = min(protection, damage) + effPen + shred"
        );
    }
}
