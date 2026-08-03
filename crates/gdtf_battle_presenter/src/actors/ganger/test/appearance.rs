use bevy::prelude::Color;
use gdtf_battle_sim::{
    ganger::{Aiming, Facing},
    prelude::{Direction, Faction, LifeState, Stance, StanceKind},
};

use super::super::{
    appearance::{GangerAppearance, ganger_sprite_appearance},
    roles::CharacterRoles,
    tint::faction_tint,
};
use crate::TileIndex;

fn roles() -> CharacterRoles {
    CharacterRoles {
        faction_0: TileIndex::new(10),
        faction_1: TileIndex::new(40),
    }
}

fn classify(
    faction: u8,
    facing: Direction,
    stance: StanceKind,
    aiming: bool,
    life: LifeState,
    suppressed: bool,
) -> GangerAppearance {
    ganger_sprite_appearance(
        Faction::new(faction),
        Facing::new(facing),
        Stance::new(stance),
        Aiming::new(aiming),
        life,
        suppressed,
        &roles(),
    )
}

fn posture(
    stance: StanceKind,
    aiming: bool,
    life: LifeState,
    suppressed: bool,
) -> GangerAppearance {
    classify(0, Direction::East, stance, aiming, life, suppressed)
}

fn baseline() -> GangerAppearance {
    posture(StanceKind::Standing, false, LifeState::Alive, false)
}

fn luminance(color: Color) -> f32 {
    let lin = color.to_linear();
    (lin.red + lin.green + lin.blue) / 3.0
}

#[test]
fn index_is_base_plus_frame_and_ignores_posture_life_and_suppression() {
    let base = baseline();
    assert_eq!(base.atlas_index, 13, "faction 0 + East = base + RIGHT");
    let f1 = classify(
        1,
        Direction::North,
        StanceKind::Standing,
        false,
        LifeState::Alive,
        false,
    );
    assert_eq!(f1.atlas_index, 42, "faction 1 + North = base + UP");
    for variant in [
        posture(StanceKind::Prone, false, LifeState::Alive, false),
        posture(StanceKind::Standing, true, LifeState::Alive, false),
        posture(StanceKind::Standing, false, LifeState::Downed, false),
        posture(StanceKind::Standing, false, LifeState::Alive, true),
    ] {
        assert_eq!(
            variant.atlas_index, base.atlas_index,
            "stance / aim / life / suppression must not move the atlas index",
        );
    }
}

#[test]
fn live_baseline_tint_is_the_faction_tint_and_factions_differ() {
    let f0 = baseline();
    let f1 = classify(
        1,
        Direction::East,
        StanceKind::Standing,
        false,
        LifeState::Alive,
        false,
    );
    assert_eq!(
        f0.tint.to_linear(),
        faction_tint(Faction::new(0)).to_linear(),
        "a quiet live ganger draws its plain faction tint",
    );
    assert_ne!(
        f0.tint, f1.tint,
        "the two factions must read as two colours"
    );
}

#[test]
fn live_posture_deltas_keep_their_directions() {
    let standing = baseline();
    let prone = posture(StanceKind::Prone, false, LifeState::Alive, false);
    let aiming = posture(StanceKind::Standing, true, LifeState::Alive, false);
    let suppressed = posture(StanceKind::Standing, false, LifeState::Alive, true);
    assert!(
        luminance(prone.tint) < luminance(standing.tint),
        "Prone dims a live ganger",
    );
    assert!(
        luminance(aiming.tint) > luminance(standing.tint),
        "aiming brightens a live ganger",
    );
    assert!(
        luminance(suppressed.tint) < luminance(standing.tint),
        "suppression darkens a live ganger",
    );
    assert_ne!(
        suppressed.tint, standing.tint,
        "a suppressed live ganger reads distinctly",
    );
}

#[test]
fn downed_pins_one_canonical_answer_for_stance_and_aim() {
    let downed = posture(StanceKind::Standing, false, LifeState::Downed, false);
    let downed_prone = posture(StanceKind::Prone, false, LifeState::Downed, false);
    let downed_aiming = posture(StanceKind::Standing, true, LifeState::Downed, false);
    assert_eq!(
        downed_prone, downed,
        "CANONICAL: Downed + Prone draws the plain Downed grey-out (stance ignored)",
    );
    assert_eq!(
        downed_aiming, downed,
        "CANONICAL: Downed + aiming draws the plain Downed grey-out (aim ignored)",
    );
    assert_ne!(
        downed.tint,
        baseline().tint,
        "Downed reads distinctly from Alive"
    );
}

#[test]
fn downed_pins_one_canonical_answer_for_suppression() {
    let downed = posture(StanceKind::Standing, false, LifeState::Downed, false);
    let downed_suppressed = posture(StanceKind::Standing, false, LifeState::Downed, true);
    assert_eq!(
        downed_suppressed, downed,
        "CANONICAL: Downed + suppressed draws the plain Downed grey-out (no extra drain)",
    );
}

#[test]
fn dead_classifies_as_the_live_tint_for_completeness() {
    let dead = posture(StanceKind::Standing, false, LifeState::Dead, false);
    assert_eq!(
        dead,
        baseline(),
        "Dead is classified like Alive (never drawn)"
    );
}
