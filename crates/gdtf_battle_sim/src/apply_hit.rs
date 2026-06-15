//! Wound application + the terminal gates — the E3.6 slice (GTW-188).
//!
//! [`apply_hit`] folds **one already-resolved** hit onto a ganger
//! (`docs/combat/resolution.md` §9; `docs/combat/wounds-and-roster.md` §"From
//! damage to injury" / §"Severity tiers" / §"Downed → death … state machine"). It
//! is the per-hit application primitive the E3.9 capstone (`resolve_and_apply` /
//! `fire`) calls **after** the E3.3 damage formula ([`crate::resolve_hit`]) and the
//! E3.4 severity roll ([`crate::severity`]) have run — it takes the frozen
//! [`HitResult`] and the rolled [`Severity`] as **inputs** and recomputes neither.
//!
//! ## What it folds, in order
//!
//! 1. **Corpse-skip** — a ganger already at [`LifeState::Dead`] is **skipped
//!    entirely**: nothing mutates and no [`ArmorBroken`] fires (the
//!    "corpse-skip discipline" of resolution.md §9's `resolve_and_apply`). Death
//!    is final; a later round in a burst cannot re-kill a corpse.
//! 2. **HP loss — ALWAYS** — the [`HitResult`]'s [`HpDamage`] is subtracted from
//!    [`Hp`], even on a [`Severity::None`] graze (a graze still bruises HP;
//!    wounds-and-roster.md §"From damage to injury": "A roll under the first bucket
//!    edge is a **graze**: HP loss only, no Wound"). The subtraction
//!    **saturates** at `0` — [`Hp`] is unsigned, so a lethal HP hit depletes the
//!    pool to `0`, never underflowing.
//! 3. **Wounds by tier** — the [`Severity`] bucket spends the life pool
//!    ([`Wounds`]): [`Severity::None`] costs `0`, [`Severity::Minor`] /
//!    [`Severity::Major`] / [`Severity::Critical`] cost the tunable
//!    [`crate::tuning::WoundCosts`], and [`Severity::Fatal`] **empties** the pool
//!    (`Wounds → 0`) outright (wounds-and-roster.md §"Severity tiers": "**Fatal**
//!    … **dead** — outright, skips Downed"). The cost subtraction saturates at `0`.
//! 4. **Armor wear** — the [`HitResult`]'s [`crate::resolve_hit::IntegrityWear`] is
//!    persisted onto the struck location of the battle-local [`WornArmor`] via the
//!    E3.5 [`wear_armor`] path, surfacing the `Some(`[`ArmorBroken`]`)` on the
//!    single protecting→broken crossing for the caller to write to a message buffer.
//! 5. **Terminal gates, in order** (resolution.md §9: "`Wounds ≤ 0` → **Dead**
//!    (trumps Downed …), else `HP ≤ 0` → **Downed**"). Because the pools are
//!    unsigned and the subtractions saturate, "≤ 0" is reached as **depleted to
//!    `0`**: if [`Wounds`] is now `0` the ganger is [`LifeState::Dead`] (this
//!    **trumps** Downed — checked first); else if [`Hp`] is now `0` the ganger is
//!    [`LifeState::Downed`]. A single hit that depletes **both** pools yields
//!    **Dead, not Downed** — the trump.
//!
//! Pure, render-free model logic: no renderer, no pixel. [`apply_hit`] mutates the
//! ganger state in place through the borrowed [`GangerHitTarget`] bundle and
//! returns the armor-broken signal; the caller writes that `Some` to a
//! [`bevy::prelude::MessageWriter<ArmorBroken>`] at the system boundary (the same
//! pure-helper / message-at-the-boundary split as [`wear_armor`]).

use bevy::prelude::Entity;

use crate::{
    armor::{BodyPart, WornArmor},
    armor_wear::{ArmorBroken, wear_armor},
    ganger::{Hp, LifeState, Wounds},
    resolve_hit::HitResult,
    severity::Severity,
    tuning::{CombatTuning, WoundCost, WoundCosts},
};

/// The bundle of **mutable** ganger-state borrows [`apply_hit`] folds a hit onto —
/// the four battle-state surfaces a hit can change.
///
/// Grouping the four `&mut` borrows into one named struct keeps [`apply_hit`] under
/// clippy's argument-count gate (the same precedent as
/// [`crate::severity::SeverityInputs`] / [`crate::resolve_coarse::ShotInputs`]).
/// Every field is an existing named domain component ([`Hp`] / [`Wounds`] /
/// [`LifeState`] / [`WornArmor`] — no-bare-types), exclusively borrowed so the
/// application mutates them in place. The caller (a Bevy system, or the E3.9
/// capstone) assembles this from the ganger entity's components.
pub struct GangerHitTarget<'a> {
    /// The ganger's hit-points pool — the HP loss subtracts from it (always).
    pub hp:     &'a mut Hp,
    /// The ganger's Wounds (life) pool — the severity tier spends from it.
    pub wounds: &'a mut Wounds,
    /// The ganger's terminal life state — the gates set it (Dead trumps Downed).
    pub life:   &'a mut LifeState,
    /// The ganger's battle-local worn armor — the struck piece wears in place.
    pub worn:   &'a mut WornArmor,
}

/// The [`Wounds`]-budget cost a non-`Fatal` [`Severity`] tier spends from the life
/// pool (`docs/combat/wounds-and-roster.md` §"Severity tiers").
///
/// [`Severity::None`] is a graze and costs **nothing** (`0`); the three middle
/// tiers read their tunable cost off `costs` ([`crate::tuning::WoundCosts`]).
/// [`Severity::Fatal`] is **not** a fixed cost — it empties the whole pool — so it
/// is handled **structurally** at the [`apply_hit`] call site (it returns the
/// floor `0` here, but `apply_hit` never reaches this branch for Fatal). Returns a
/// named [`WoundCost`] (no bare `u8`). `costs` is taken by value — it is a small
/// `Copy` tuning bundle.
#[must_use]
const fn wound_cost(severity: Severity, costs: WoundCosts) -> WoundCost {
    match severity {
        // A graze (None) and the structurally-handled Fatal cost nothing *here*:
        // None is a true zero spend; Fatal empties the pool at the call site, so it
        // never reaches this helper (it falls through to the floor for totality).
        Severity::None | Severity::Fatal => WoundCost::new(0),
        Severity::Minor => costs.minor,
        Severity::Major => costs.major,
        Severity::Critical => costs.critical,
    }
}

/// Saturating-cast a non-negative [`HpDamage`](crate::resolve_hit::HpDamage)
/// magnitude (`i32 ≥ 0`) into the unsigned [`Hp`] inner type (`u16`), clamping into
/// the `u16` range so a wild value can never wrap or lose its sign.
///
/// The per-hit formula is signed `i32`, but HP-loss damage is `max(floor, inner)`
/// against a non-negative floor and reaches the HP pool as an unsigned count. A
/// value below `0` clamps to `0` and a value above `u16::MAX` clamps to `u16::MAX`,
/// so the subsequent [`u16::saturating_sub`] never underflows. The clamp +
/// localized `#[expect]` is the crate's guarded-cast idiom (see
/// [`crate::resolve_hit`]'s `round_to_i32` / [`crate::metric`]'s `floor_to_i32`),
/// so no `unwrap`/`expect` is needed. (Not a `const fn`: `i32::clamp` comes from
/// `Ord`, which is not yet const-callable.)
fn hp_damage_to_u16(damage: i32) -> u16 {
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "clamped into [0, u16::MAX] first, so the cast can neither wrap nor lose a sign"
    )]
    let clamped = damage.clamp(0, i32::from(u16::MAX)) as u16;
    clamped
}

/// Fold **one resolved hit** onto a ganger — HP loss + Wounds-by-severity + armor
/// wear + the terminal gates (`docs/combat/resolution.md` §9;
/// `docs/combat/wounds-and-roster.md` §"From damage to injury" / §"Severity tiers"
/// / §"Downed → death … state machine").
///
/// Takes the **already-computed** [`HitResult`] (E3.3) and rolled [`Severity`]
/// (E3.4) as inputs — it applies them, it does not recompute them (the
/// resolve → severity → apply chain is the E3.9 capstone). `part` is the struck
/// [`BodyPart`] (E3.4's location roll), `ganger` is the owning [`Entity`] (carried
/// on the returned [`ArmorBroken`]), and `tuning` supplies the per-tier
/// [`crate::tuning::WoundCosts`].
///
/// Order (verbatim, see the module docs):
///
/// 1. **Corpse-skip** — if the ganger is already [`LifeState::Dead`], return
///    [`None`] and mutate nothing.
/// 2. **HP loss — always** — subtract the [`HitResult`]'s
///    [`HpDamage`](crate::resolve_hit::HpDamage) from [`Hp`] (saturating at `0`),
///    even on a graze.
/// 3. **Wounds by tier** — [`Severity::Fatal`] sets [`Wounds`] to `0` (empties the
///    pool); otherwise subtract [`wound_cost`] (saturating at `0`).
/// 4. **Armor wear** — persist the [`HitResult`]'s integrity wear onto the struck
///    [`WornArmor`] piece via [`wear_armor`], capturing the `Some(`[`ArmorBroken`]`)`
///    on the single protecting→broken crossing.
/// 5. **Terminal gates** — `Wounds == 0` → [`LifeState::Dead`] (**trumps**); else
///    `Hp == 0` → [`LifeState::Downed`].
///
/// Returns the [`ArmorBroken`] signal iff this hit broke the struck piece (for the
/// caller to write to a [`bevy::prelude::MessageWriter<ArmorBroken>`]), [`None`]
/// otherwise (including the corpse-skip). Pure, render-free, saturating arithmetic
/// — no underflow, no pixel.
#[must_use]
pub fn apply_hit(
    target: GangerHitTarget<'_>,
    hit: &HitResult,
    severity: Severity,
    part: BodyPart,
    ganger: Entity,
    tuning: &CombatTuning,
) -> Option<ArmorBroken> {
    // (a) Corpse-skip: a dead ganger is final — mutate nothing, emit nothing.
    if *target.life == LifeState::Dead {
        return None;
    }

    // (b) HP loss ALWAYS — saturating at 0 (Hp is unsigned; a lethal hit depletes
    // the pool to 0, never underflows). Applies even on a Severity::None graze.
    let hp_loss = hp_damage_to_u16(*hit.hp_damage);
    *target.hp = Hp::new(target.hp.saturating_sub(hp_loss));

    // (c) Wounds by severity tier. Fatal EMPTIES the pool (structural); every other
    // tier spends its tunable cost, saturating at 0.
    if severity == Severity::Fatal {
        *target.wounds = Wounds::new(0);
    } else {
        let cost = *wound_cost(severity, tuning.wound_costs);
        *target.wounds = Wounds::new(target.wounds.saturating_sub(cost));
    }

    // (d) Armor wear — persist this hit's integrity wear onto the struck worn piece
    // (E3.5), capturing the broken signal on the single protecting→broken crossing.
    let broken = wear_armor(target.worn, part, hit.wear, ganger);

    // (e) Terminal gates, in order — Wounds depleted to 0 → Dead (TRUMPS Downed,
    // checked first); else Hp depleted to 0 → Downed. (Pools are unsigned, so the
    // doc's "≤ 0" gate is "== 0 after the saturating spend".)
    if *target.wounds == Wounds::new(0) {
        *target.life = LifeState::Dead;
    } else if *target.hp == Hp::new(0) {
        *target.life = LifeState::Downed;
    }

    broken
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{App, MinimalPlugins, Update, World};

    use super::*;
    use crate::{
        armor::{
            ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorPiece, ArmorProtection, ArmorType,
            SourceArmor,
        },
        resolve_hit::{HpDamage, IntegrityWear, PenetratingDamage},
    };

    /// A real, valid [`Entity`] id to stand in for the owning ganger — spawned from
    /// a throwaway [`World`] so the tests never hand-craft a raw id (0.18's
    /// `from_raw_u32` is fallible; spawning yields a guaranteed-valid handle without
    /// any `unwrap`). Mirrors `armor_wear::tests::a_ganger`.
    fn a_ganger() -> Entity {
        World::new().spawn_empty().id()
    }

    /// A uniform worn suit whose every piece starts at `integrity` — an arbitrary
    /// (NOT-shipped-tuning) magnitude; the other three armor stats are irrelevant to
    /// these tests and set to `0`. So a hit's wear lands without first being soaked.
    fn worn_suit(integrity: i32) -> WornArmor {
        WornArmor::seed_from(&SourceArmor::uniform(ArmorPiece::new(
            ArmorFloor::new(0),
            ArmorProtection::new(0),
            ArmorIntegrity::new(integrity),
            ArmorHardness::new(0),
            ArmorType::DEFAULT,
        )))
    }

    /// Build a [`HitResult`] from arbitrary per-test magnitudes — mechanism inputs,
    /// never asserted as values. `hp_damage` drives the HP loss, `wear` the armor
    /// wear; `penetrating` is carried for completeness (`apply_hit` does not read it —
    /// the severity it would have gated is passed in directly).
    fn hit(hp_damage: i32, wear: i32) -> HitResult {
        HitResult {
            penetrating: PenetratingDamage::new(hp_damage.max(0)),
            hp_damage:   HpDamage::new(hp_damage),
            wear:        IntegrityWear::new(wear),
        }
    }

    /// AC1 — HP loss applies ALWAYS, even on a `Severity::None` graze, and no Wound
    /// is spent. Drives a graze with `hp_damage > 0` onto a healthy ganger and
    /// asserts HP fell by exactly the HP-loss while Wounds is unchanged.
    #[test]
    fn graze_subtracts_hp_and_spends_no_wound() {
        let mut hp = Hp::new(20);
        let mut wounds = Wounds::new(5);
        let mut life = LifeState::Alive;
        let mut worn = worn_suit(100);
        let tuning = CombatTuning::default();

        let target = GangerHitTarget {
            hp:     &mut hp,
            wounds: &mut wounds,
            life:   &mut life,
            worn:   &mut worn,
        };
        // A graze: Severity::None, HP-loss 7, low wear (no break).
        let broke = apply_hit(
            target,
            &hit(7, 1),
            Severity::None,
            BodyPart::Torso,
            a_ganger(),
            &tuning,
        );

        assert_eq!(*hp, 20 - 7, "a graze must subtract its HP-loss from Hp");
        assert_eq!(*wounds, 5, "a graze (Severity::None) must spend NO Wound");
        assert_eq!(
            life,
            LifeState::Alive,
            "a non-lethal graze must leave the ganger Alive"
        );
        assert_eq!(
            broke, None,
            "a low-wear hit on a fresh suit must not break armor"
        );
    }

    /// AC2 (Fatal empties) — a `Severity::Fatal` hit drives `Wounds == 0` regardless
    /// of the prior pool (the mechanism), independent of any tuned cost. Uses a large
    /// starting pool so a per-tier subtraction could never reach 0 — only the
    /// pool-emptying branch can.
    #[test]
    fn fatal_empties_the_wounds_pool_regardless_of_prior() {
        let tuning = CombatTuning::default();
        for prior in [1u8, 3, 7, 200, u8::MAX] {
            let mut hp = Hp::new(50);
            let mut wounds = Wounds::new(prior);
            let mut life = LifeState::Alive;
            let mut worn = worn_suit(100);

            let target = GangerHitTarget {
                hp:     &mut hp,
                wounds: &mut wounds,
                life:   &mut life,
                worn:   &mut worn,
            };
            let _broke = apply_hit(
                target,
                &hit(1, 0),
                Severity::Fatal,
                BodyPart::Head,
                a_ganger(),
                &tuning,
            );

            assert_eq!(
                *wounds, 0,
                "Fatal must empty the Wounds pool from prior {prior}"
            );
        }
    }

    /// AC2 (tier ordering) — Minor < Major < Critical in Wounds cost, as a RELATION
    /// (never a pinned split). Constructs three otherwise-identical hits differing
    /// only in their severity tier, applies each to an identical fresh ganger, and
    /// asserts the post-application Wounds order: more severe ⇒ fewer Wounds left.
    #[test]
    fn wound_cost_orders_minor_lt_major_lt_critical() {
        let tuning = CombatTuning::default();
        // A pool large enough that even Critical's cost can't reach 0 (so the gate
        // never collapses the ordering to a shared floor) — an arbitrary fixture.
        let start = Wounds::new(200);

        let post_wounds = |severity: Severity| -> u8 {
            let mut hp = Hp::new(50);
            let mut wounds = start;
            let mut life = LifeState::Alive;
            let mut worn = worn_suit(100);
            let target = GangerHitTarget {
                hp:     &mut hp,
                wounds: &mut wounds,
                life:   &mut life,
                worn:   &mut worn,
            };
            let _broke = apply_hit(
                target,
                &hit(1, 0),
                severity,
                BodyPart::Torso,
                a_ganger(),
                &tuning,
            );
            *wounds
        };

        let after_minor = post_wounds(Severity::Minor);
        let after_major = post_wounds(Severity::Major);
        let after_critical = post_wounds(Severity::Critical);

        // More severe ⇒ spends MORE Wounds ⇒ leaves FEWER — Minor < Major < Critical
        // cost, asserted as the relation, never the exact split.
        assert!(
            after_minor > after_major,
            "Major must spend more Wounds than Minor: {after_minor} (minor) <= {after_major} (major)",
        );
        assert!(
            after_major > after_critical,
            "Critical must spend more Wounds than Major: {after_major} (major) <= {after_critical} (critical)",
        );
    }

    /// AC3 — terminal gate Wounds depleted to 0 → Dead. Drains Wounds to 0 (here via
    /// a Fatal hit, the cleanest pool-emptying path) at full HP and asserts the
    /// ganger is `LifeState::Dead` — death even at full HP (the life pool is what
    /// kills).
    #[test]
    fn wounds_to_zero_is_dead() {
        let mut hp = Hp::new(50); // full HP — death comes from the life pool, not HP
        let mut wounds = Wounds::new(3);
        let mut life = LifeState::Alive;
        let mut worn = worn_suit(100);
        let tuning = CombatTuning::default();

        let target = GangerHitTarget {
            hp:     &mut hp,
            wounds: &mut wounds,
            life:   &mut life,
            worn:   &mut worn,
        };
        let _broke = apply_hit(
            target,
            &hit(1, 0),
            Severity::Fatal,
            BodyPart::Torso,
            a_ganger(),
            &tuning,
        );

        assert_eq!(*wounds, 0, "Fatal must empty the Wounds pool");
        assert_eq!(
            life,
            LifeState::Dead,
            "Wounds depleted to 0 must set LifeState::Dead"
        );
    }

    /// AC4 — terminal gate Hp depleted to 0 with Wounds remaining → Downed. Drives
    /// HP to 0 with a non-Fatal tier (so Wounds stays > 0) and asserts the ganger is
    /// `LifeState::Downed` (alive, incapacitated) — HP loss downs, never kills.
    #[test]
    fn hp_to_zero_with_wounds_left_is_downed() {
        let mut hp = Hp::new(8);
        let mut wounds = Wounds::new(5); // plenty left after a Minor spend
        let mut life = LifeState::Alive;
        let mut worn = worn_suit(100);
        let tuning = CombatTuning::default();

        let target = GangerHitTarget {
            hp:     &mut hp,
            wounds: &mut wounds,
            life:   &mut life,
            worn:   &mut worn,
        };
        // HP-loss (20) overshoots HP (8) — saturates to 0; Minor leaves Wounds > 0.
        let _broke = apply_hit(
            target,
            &hit(20, 0),
            Severity::Minor,
            BodyPart::LeftLeg,
            a_ganger(),
            &tuning,
        );

        assert_eq!(
            *hp, 0,
            "an overshooting HP-loss must saturate Hp to 0 (no underflow)"
        );
        assert!(*wounds > 0, "a Minor wound must leave Wounds > 0");
        assert_eq!(
            life,
            LifeState::Downed,
            "Hp depleted to 0 with Wounds left must be Downed"
        );
    }

    /// AC5 — Dead TRUMPS Downed: a single hit that depletes BOTH Hp → 0 AND
    /// Wounds → 0 yields `Dead`, not `Downed`. A Fatal hit (empties Wounds) whose
    /// HP-loss also overshoots HP — both gates trip, the Wounds gate must win.
    #[test]
    fn both_pools_depleted_is_dead_not_downed() {
        let mut hp = Hp::new(4);
        let mut wounds = Wounds::new(2);
        let mut life = LifeState::Alive;
        let mut worn = worn_suit(100);
        let tuning = CombatTuning::default();

        let target = GangerHitTarget {
            hp:     &mut hp,
            wounds: &mut wounds,
            life:   &mut life,
            worn:   &mut worn,
        };
        // HP-loss (99) overshoots HP (4) → 0, AND Fatal empties Wounds → 0.
        let _broke = apply_hit(
            target,
            &hit(99, 0),
            Severity::Fatal,
            BodyPart::Head,
            a_ganger(),
            &tuning,
        );

        assert_eq!(*hp, 0, "the lethal HP-loss must saturate Hp to 0");
        assert_eq!(*wounds, 0, "Fatal must empty the Wounds pool");
        assert_eq!(
            life,
            LifeState::Dead,
            "with BOTH pools depleted the ganger must be Dead (trumps Downed), not Downed",
        );
    }

    /// AC6 — corpse-skip: applying a hit to an already-`Dead` ganger is a NO-OP on
    /// every pool and on the worn armor (and emits no [`ArmorBroken`]). Snapshots all
    /// four surfaces before, applies a hit that WOULD wear/deplete on a live ganger,
    /// and asserts each is byte-for-byte unchanged.
    #[test]
    fn corpse_skip_changes_nothing() {
        let mut hp = Hp::new(12);
        let mut wounds = Wounds::new(4);
        let mut life = LifeState::Dead; // already a corpse
        let mut worn = worn_suit(1); // near-broken: a live hit here WOULD break it
        let tuning = CombatTuning::default();

        // Snapshot every surface by value before applying.
        let hp_before = hp;
        let wounds_before = wounds;
        let life_before = life;
        let worn_before = worn;

        let target = GangerHitTarget {
            hp:     &mut hp,
            wounds: &mut wounds,
            life:   &mut life,
            worn:   &mut worn,
        };
        // A hit that, on a live ganger, would subtract HP, spend Wounds, and break
        // the near-broken piece — proving the skip, not a harmless input.
        let broke = apply_hit(
            target,
            &hit(10, 50),
            Severity::Critical,
            BodyPart::Torso,
            a_ganger(),
            &tuning,
        );

        assert_eq!(broke, None, "a corpse-skip must emit no ArmorBroken");
        assert_eq!(hp, hp_before, "a corpse's Hp must not change");
        assert_eq!(wounds, wounds_before, "a corpse's Wounds must not change");
        assert_eq!(life, life_before, "a corpse's LifeState must stay Dead");
        assert_eq!(worn, worn_before, "a corpse's WornArmor must not wear");
    }

    /// AC7 (wear path) — `apply_hit` wears the struck piece as part of application: the
    /// struck location's integrity drops by exactly the hit's wear, and a high-wear
    /// hit on a near-broken piece returns `Some(ArmorBroken)`. A unit assertion on
    /// the worn copy through the real `apply_hit` entry point.
    #[test]
    fn apply_hit_wears_the_struck_piece_and_can_break_it() {
        let tuning = CombatTuning::default();
        let part = BodyPart::RightArm;

        // (1) Wears by exactly the hit's wear: a sturdy piece, sub-fatal wear.
        let mut hp = Hp::new(50);
        let mut wounds = Wounds::new(9);
        let mut life = LifeState::Alive;
        let mut worn = worn_suit(20);
        let before = *worn.at(part).integrity;
        {
            let target = GangerHitTarget {
                hp:     &mut hp,
                wounds: &mut wounds,
                life:   &mut life,
                worn:   &mut worn,
            };
            let broke = apply_hit(
                target,
                &hit(1, 6),
                Severity::Minor,
                part,
                a_ganger(),
                &tuning,
            );
            assert_eq!(
                broke, None,
                "a sub-fatal wear on a sturdy piece must not break it"
            );
        }
        assert_eq!(
            *worn.at(part).integrity,
            before - 6,
            "apply_hit must drop the struck piece's integrity by exactly the hit's wear",
        );

        // (2) A high-wear hit on a near-broken piece returns Some(ArmorBroken).
        let ganger = a_ganger();
        let mut hp2 = Hp::new(50);
        let mut wounds2 = Wounds::new(9);
        let mut life2 = LifeState::Alive;
        let mut worn2 = worn_suit(1); // protecting (1 > 0), one hit from broken
        let broke = {
            let target = GangerHitTarget {
                hp:     &mut hp2,
                wounds: &mut wounds2,
                life:   &mut life2,
                worn:   &mut worn2,
            };
            apply_hit(target, &hit(1, 5), Severity::Minor, part, ganger, &tuning)
        };
        assert_eq!(
            broke,
            Some(ArmorBroken::new(ganger, part)),
            "a high-wear hit crossing a near-broken piece to ≤ 0 must return Some(ArmorBroken)",
        );
        assert!(
            !worn2.protects(part),
            "the struck piece must be broken (≤ 0) after the crossing hit",
        );
    }

    /// AC7 (HEADLESS, `bevy-traps.md` #4) — the [`ArmorBroken`] `apply_hit` surfaces is
    /// a buffered `#[derive(Message)]`, written through a real `MessageWriter` at the
    /// system boundary. A `MinimalPlugins` app runs a one-shot system that builds a
    /// near-broken ganger, calls `apply_hit`, and writes the returned `Some` to the
    /// buffer; a reader drains it into a capture resource the test asserts on. The
    /// same bare-App + `add_message` pattern `armor_wear.rs` sanctions (the sim crate
    /// cannot depend on `gdtf_test_utils`).
    #[test]
    fn apply_hit_armor_broken_flows_through_a_message_buffer() {
        use bevy::prelude::{IntoScheduleConfigs, MessageReader, MessageWriter, ResMut, Resource};

        /// Captures the drained [`ArmorBroken`] messages for assertion after
        /// `update()` (no `unwrap` in the test body).
        #[derive(Resource, Default)]
        struct Captured(Vec<ArmorBroken>);

        let ganger = a_ganger();
        let part = BodyPart::Torso;

        // Producer: builds a near-broken ganger locally, applies a breaking hit
        // through the real apply_hit, and writes the Some to the buffer — the sim's
        // message boundary.
        let produce = move |mut writer: MessageWriter<ArmorBroken>| {
            let tuning = CombatTuning::default();
            let mut hp = Hp::new(30);
            let mut wounds = Wounds::new(6);
            let mut life = LifeState::Alive;
            let mut worn = WornArmor::seed_from(&SourceArmor::uniform(ArmorPiece::new(
                ArmorFloor::new(0),
                ArmorProtection::new(0),
                ArmorIntegrity::new(1),
                ArmorHardness::new(0),
                ArmorType::DEFAULT,
            )));
            let target = GangerHitTarget {
                hp:     &mut hp,
                wounds: &mut wounds,
                life:   &mut life,
                worn:   &mut worn,
            };
            if let Some(broke) =
                apply_hit(target, &hit(1, 5), Severity::Minor, part, ganger, &tuning)
            {
                writer.write(broke);
            }
        };

        // Consumer: drains the buffered messages (MessageReader, NOT an observer).
        let consume = |mut reader: MessageReader<ArmorBroken>, mut captured: ResMut<Captured>| {
            for broke in reader.read() {
                captured.0.push(*broke);
            }
        };

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<ArmorBroken>();
        app.init_resource::<Captured>();
        app.add_systems(Update, (produce, consume).chain());

        app.update();

        let captured = app
            .world()
            .get_resource::<Captured>()
            .map_or_else(Vec::new, |c| c.0.clone());

        assert_eq!(
            captured.len(),
            1,
            "apply_hit's ArmorBroken must reach the message buffer once"
        );
        assert_eq!(
            captured.first(),
            Some(&ArmorBroken::new(ganger, part)),
            "the buffered ArmorBroken must carry the struck ganger Entity + BodyPart",
        );
    }

    /// No-bare-types / mechanism — the per-tier [`WoundCost`] newtype derefs to its
    /// inner `u8`, and [`wound_cost`] maps each severity tier to its tunable cost:
    /// None (and the structurally-handled Fatal) cost 0; Minor/Major/Critical read
    /// the tuning. Asserts the ordering relation, never the shipped split.
    #[test]
    fn wound_cost_helper_maps_tiers_and_newtype_derefs() {
        let costs = WoundCosts::default();
        // None is a true zero spend; Fatal is handled structurally so its helper
        // value is the floor 0 (apply_hit never reaches it for Fatal).
        assert_eq!(*wound_cost(Severity::None, costs), 0);
        assert_eq!(*wound_cost(Severity::Fatal, costs), 0);
        // The three middle tiers read the tunable costs, ascending — a relation,
        // never a pinned magnitude.
        let minor = *wound_cost(Severity::Minor, costs);
        let major = *wound_cost(Severity::Major, costs);
        let critical = *wound_cost(Severity::Critical, costs);
        assert!(
            minor < major,
            "Minor cost must be < Major: {minor} >= {major}"
        );
        assert!(
            major < critical,
            "Major cost must be < Critical: {major} >= {critical}"
        );
        // The newtype derefs to its inner u8 (arbitrary value, mechanism not value).
        assert_eq!(*WoundCost::new(7), 7u8);
    }
}
