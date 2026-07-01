//! Seeded, value-agnostic unit tests for the GTW-523 fall mechanic's PURE verbs —
//! [`resolve_drop`] (C2 drop resolution) and [`resolve_fall_hit`] (C4 damage / C5 injury
//! synthesis). The ECS system ([`apply_falls`](super::apply_falls)) — the C1 faller
//! predicate, the C3 stair brace, the C6/C7 signal + multi-faller, and the C8 hole
//! regression — is exercised end-to-end by the `GdtfTestAppBuilder` integration test
//! (`tests/gtw523_falls_sim.rs`).
//!
//! Every assert is on the FORMULA / relations (monotone in storeys, same-seed determinism,
//! a hot-edit shifts the blow), NEVER a pinned tunable magnitude (the GTW-506 melee /
//! GTW-466 reaction test style).

use bevy::prelude::Entity;

use super::{
    DropLanding, StoreysFallen,
    damage::{FallImpact, FallWoundEnv, resolve_fall_hit},
    message::FallOccurred,
    resolve::resolve_drop,
};
use crate::{
    armor::BodyPart,
    ganger::{Hp, LifeState, Luck, Toughness, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::{InjuryRegistry, InjuryTables},
    metric::{Cell, CellLevel, Level},
    resolve_and_apply::TargetGanger,
    rng::{BattleSeed, InjuryRng, SeverityRng},
    surface::{SlabState, SurfaceGrid},
    tuning::{CombatTuning, PerStoreyDamage},
};

/// An arbitrary fixed seed — determinism is the property, the value is irrelevant.
const SEED: u64 = 0x0523_FA11_DEAD_BEEF;

/// The test cell every drop scan runs in — the column x/y is irrelevant to the scan.
fn cell() -> Cell {
    Cell::new(3, 4)
}

// ── C2 — resolve_drop: land on the highest supported storey ───────────────────

/// C2 / QA(1): a 3-storey column with a ganger on level 2 whose slab is destroyed drops to
/// the ground (level 0) when no lower slab supports — distance == 2 storeys. The scan keys
/// off the START level (2), NOT level+1 (the caller's predicate guarantees start == the
/// destroyed slab's level).
#[test]
fn drop_lands_on_ground_when_no_lower_slab() {
    // Author no intermediate slabs: level-1 and level-2 slabs are Absent (open air); the
    // ground (level 0) is the only support.
    let surface = SurfaceGrid::new();

    let resolved = resolve_drop(cell(), Level::new(2), &surface);
    assert!(
        resolved.is_some(),
        "a faller starting at level 2 must resolve a landing"
    );
    let Some(landing) = resolved else { return };

    assert_eq!(
        landing.landing,
        Level::new(0),
        "with no lower slab, the faller lands on the ground (level 0)"
    );
    assert_eq!(
        landing.storeys,
        StoreysFallen::new(2),
        "start 2 → landing 0 is a 2-storey fall"
    );
}

/// C2 / QA(1): land on the FIRST intact `Present` slab below the start (not the ground) —
/// a floor at level 1 catches a faller dropping from level 2.
#[test]
fn drop_lands_on_first_present_slab_below() {
    let mut surface = SurfaceGrid::new();
    // An intact floor at level 1 supports; the faller stops there rather than reaching ground.
    surface.set_slab(CellLevel::new(cell(), Level::new(1)), SlabState::Present);

    let resolved = resolve_drop(cell(), Level::new(2), &surface);
    assert!(
        resolved.is_some(),
        "a faller at level 2 over a Present level-1 slab must land"
    );
    let Some(landing) = resolved else { return };

    assert_eq!(
        landing.landing,
        Level::new(1),
        "an intact Present slab at level 1 catches the fall"
    );
    assert_eq!(
        landing.storeys,
        StoreysFallen::new(1),
        "start 2 → landing 1 is one storey"
    );
}

/// C2 / QA(2): a multi-storey drop THROUGH an `Absent` intermediate (open air) lands on the
/// first support below it — a `Destroyed` slab is likewise NOT support (a hole), so a faller
/// falls through BOTH an Absent and a Destroyed slab to the first Present/ground.
#[test]
fn drop_falls_through_absent_and_destroyed_to_first_support() {
    let mut surface = SurfaceGrid::new();
    // level 3 slab: Destroyed (a hole — NOT support). level 2: Absent (open air — NOT
    // support). level 1: Present (support — the landing). Faller starts at level 4.
    surface.destroy_slab(CellLevel::new(cell(), Level::new(3)));
    // level 2 left Absent by default.
    surface.set_slab(CellLevel::new(cell(), Level::new(1)), SlabState::Present);

    let resolved = resolve_drop(cell(), Level::new(4), &surface);
    assert!(
        resolved.is_some(),
        "a faller at level 4 must land on the level-1 Present slab"
    );
    let Some(landing) = resolved else { return };

    assert_eq!(
        landing.landing,
        Level::new(1),
        "falls through Destroyed (3) + Absent (2), lands on Present (1)"
    );
    assert_eq!(
        landing.storeys,
        StoreysFallen::new(3),
        "start 4 → landing 1 is three storeys"
    );
}

/// C2: a start on the ground (level 0) cannot fall — there is no `k < 0` to land on, so the
/// scan returns `None` (the caller filters these; this is the defensive guard).
#[test]
fn drop_from_ground_returns_none() {
    let surface = SurfaceGrid::new();
    assert!(
        resolve_drop(cell(), Level::new(0), &surface).is_none(),
        "a ganger already on the ground cannot fall (no lower storey)"
    );
}

/// C2: every fall drops AT LEAST one storey — the destroyed slab the faller stood on is
/// never a landing candidate (the scan starts at start-1), so the distance is always ≥ 1.
#[test]
fn drop_distance_is_always_at_least_one() {
    let mut surface = SurfaceGrid::new();
    // A Present slab exactly at the start level must NOT catch the faller (that is the slab
    // it was standing on, now destroyed) — the scan begins BELOW start.
    surface.set_slab(CellLevel::new(cell(), Level::new(2)), SlabState::Present);
    let resolved = resolve_drop(cell(), Level::new(2), &surface);
    assert!(
        resolved.is_some(),
        "a faller at level 2 must still land somewhere below"
    );
    let Some(landing) = resolved else { return };
    assert!(
        *landing.storeys >= 1,
        "a fall always drops at least one storey (start's own slab is not support)"
    );
    assert!(
        landing.landing < Level::new(2),
        "landing is strictly below the start"
    );
}

// ── C4 / C5 — resolve_fall_hit: damage + injury synthesis ─────────────────────

/// A minimal faller state bundle for the pure damage-synthesis tests. Owns the mutable
/// battle surfaces so a test can build a [`TargetGanger`] borrow-view over them.
struct Faller {
    hp:        Hp,
    wounds:    Wounds,
    life:      LifeState,
    inflicted: InflictedWounds,
    toughness: Toughness,
    luck:      Luck,
}

impl Faller {
    /// A fresh unhurt faller with a big HP/Wounds pool (so a fall wounds but rarely kills,
    /// keeping the HP-drop comparison clean) and zero Toughness/Luck (no mitigation noise).
    fn fresh() -> Self {
        Self {
            hp:        Hp::new(1000),
            wounds:    Wounds::new(200),
            life:      LifeState::Alive,
            inflicted: InflictedWounds::default(),
            toughness: Toughness::new(0.0),
            luck:      Luck::new(0.0),
        }
    }

    /// Borrow this faller's surfaces as a bare-flesh (no worn piece) [`TargetGanger`].
    fn target(&mut self) -> TargetGanger<'_> {
        TargetGanger {
            hp:        &mut self.hp,
            wounds:    &mut self.wounds,
            life:      &mut self.life,
            piece:     None,
            inflicted: &mut self.inflicted,
            toughness: self.toughness,
            luck:      self.luck,
        }
    }
}

/// Run one fall onto a fresh faller and return the HP LOST (a clean single-fall measurement).
fn hp_lost_from_fall(per_storey: PerStoreyDamage, storeys: u8, seed: u64) -> u16 {
    let tuning = CombatTuning::default();
    let tables = InjuryTables::default();
    let registry = InjuryRegistry::default();
    let mut severity = SeverityRng::from_root(BattleSeed::new(seed));
    let mut injury = InjuryRng::from_root(BattleSeed::new(seed));

    let mut faller = Faller::fresh();
    let before = *faller.hp;
    let _rolled = resolve_fall_hit(
        FallImpact {
            per_storey,
            storeys: StoreysFallen::new(storeys),
            part: BodyPart::Torso,
            target: faller.target(),
            target_entity: Entity::PLACEHOLDER,
        },
        FallWoundEnv {
            tuning:       &tuning,
            tables:       &tables,
            registry:     &registry,
            severity_rng: &mut severity,
            injury_rng:   &mut injury,
        },
    );
    before - *faller.hp
}

/// C4 / QA(5): fall damage is MONOTONE (non-decreasing) in storeys fallen — a bigger drop
/// hurts at least as much (the LINEAR `per_storey_damage × storeys`, before armor). A
/// value-agnostic relation, NOT a pinned magnitude.
#[test]
fn fall_damage_is_monotone_in_storeys() {
    let per_storey = PerStoreyDamage::new(10);
    let one = hp_lost_from_fall(per_storey, 1, SEED);
    let two = hp_lost_from_fall(per_storey, 2, SEED);
    let three = hp_lost_from_fall(per_storey, 3, SEED);
    assert!(
        one <= two && two <= three,
        "HP lost must be non-decreasing in storeys fallen (1:{one} 2:{two} 3:{three})"
    );
    // Strict growth somewhere across the range (the linear scaling is real, not flat).
    assert!(
        three > one,
        "a 3-storey fall must hurt strictly more than a 1-storey fall"
    );
}

/// C7 / QA(6): the synthesis is DETERMINISTIC — the SAME seed + inputs yields the identical
/// HP loss (the two injected streams are the only entropy; no `FightRng` draw).
#[test]
fn fall_damage_is_deterministic_under_same_seed() {
    let per_storey = PerStoreyDamage::new(8);
    let a = hp_lost_from_fall(per_storey, 2, SEED);
    let b = hp_lost_from_fall(per_storey, 2, SEED);
    assert_eq!(a, b, "same seed + inputs must yield identical fall damage");
}

/// C4 / QA(7): a hot-edit of `per_storey_damage` shifts the blow — a larger per-storey leaf
/// deals MORE HP loss for the same fall (the FORMULA asserted, NOT a shipped magnitude).
#[test]
fn hot_edit_per_storey_damage_changes_the_blow() {
    let small = hp_lost_from_fall(PerStoreyDamage::new(4), 2, SEED);
    let large = hp_lost_from_fall(PerStoreyDamage::new(40), 2, SEED);
    assert!(
        large > small,
        "a larger per_storey_damage leaf must deal more HP loss for the same fall \
         (small:{small} large:{large})"
    );
}

/// C5: a corpse faller is CORPSE-SKIPPED before any draw — no HP loss, no mutation (and no
/// stream advance). The synthesis returns `None` and the pools are untouched.
#[test]
fn corpse_faller_is_skipped_without_damage() {
    let tuning = CombatTuning::default();
    let tables = InjuryTables::default();
    let registry = InjuryRegistry::default();
    let mut severity = SeverityRng::from_root(BattleSeed::new(SEED));
    let mut injury = InjuryRng::from_root(BattleSeed::new(SEED));

    let mut faller = Faller::fresh();
    faller.life = LifeState::Dead;
    let hp_before = *faller.hp;
    let wounds_before = *faller.wounds;

    let rolled = resolve_fall_hit(
        FallImpact {
            per_storey:    PerStoreyDamage::new(100),
            storeys:       StoreysFallen::new(5),
            part:          BodyPart::Torso,
            target:        faller.target(),
            target_entity: Entity::PLACEHOLDER,
        },
        FallWoundEnv {
            tuning:       &tuning,
            tables:       &tables,
            registry:     &registry,
            severity_rng: &mut severity,
            injury_rng:   &mut injury,
        },
    );

    assert!(rolled.is_none(), "a corpse rolls no injury (no draw)");
    assert_eq!(*faller.hp, hp_before, "a corpse takes no HP loss");
    assert_eq!(*faller.wounds, wounds_before, "a corpse spends no Wounds");
}

// ── Message / value-object construction ───────────────────────────────────────

/// The [`FallOccurred`] + [`DropLanding`] value objects carry the storeys distance and
/// endpoints they were built with (no-bare-types round-trip through the constructors).
#[test]
fn fall_signal_and_landing_carry_their_fields() {
    let landing = DropLanding {
        landing: Level::new(1),
        storeys: StoreysFallen::new(3),
    };
    assert_eq!(landing.landing, Level::new(1));
    assert_eq!(landing.storeys, StoreysFallen::new(3));

    let signal = FallOccurred::new(
        Entity::PLACEHOLDER,
        Level::new(4),
        Level::new(1),
        StoreysFallen::new(3),
    );
    assert_eq!(signal.from_level, Level::new(4));
    assert_eq!(signal.to_level, Level::new(1));
    assert_eq!(signal.storeys, StoreysFallen::new(3));
    assert_eq!(
        *signal.from_level - *signal.to_level,
        *signal.storeys,
        "storeys == from_level − to_level (the linear damage scalar)"
    );
}
