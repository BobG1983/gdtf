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

const SEED: u64 = 0x0523_FA11_DEAD_BEEF;

fn cell() -> Cell {
    Cell::new(3, 4)
}

#[test]
fn drop_lands_on_ground_when_no_lower_slab() {
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

#[test]
fn drop_lands_on_first_present_slab_below() {
    let mut surface = SurfaceGrid::new();
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

#[test]
fn drop_falls_through_absent_and_destroyed_to_first_support() {
    let mut surface = SurfaceGrid::new();
    surface.destroy_slab(CellLevel::new(cell(), Level::new(3)));
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

#[test]
fn drop_from_ground_returns_none() {
    let surface = SurfaceGrid::new();
    assert!(
        resolve_drop(cell(), Level::new(0), &surface).is_none(),
        "a ganger already on the ground cannot fall (no lower storey)"
    );
}

#[test]
fn drop_distance_is_always_at_least_one() {
    let mut surface = SurfaceGrid::new();
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

struct Faller {
    hp:        Hp,
    wounds:    Wounds,
    life:      LifeState,
    inflicted: InflictedWounds,
    toughness: Toughness,
    luck:      Luck,
}

impl Faller {
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
    assert!(
        three > one,
        "a 3-storey fall must hurt strictly more than a 1-storey fall"
    );
}

#[test]
fn fall_damage_is_deterministic_under_same_seed() {
    let per_storey = PerStoreyDamage::new(8);
    let a = hp_lost_from_fall(per_storey, 2, SEED);
    let b = hp_lost_from_fall(per_storey, 2, SEED);
    assert_eq!(a, b, "same seed + inputs must yield identical fall damage");
}

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
