//! The GTW-631 appearance-classifier unit matrix (C4): the (atlas index, tint) verdict
//! over the input space, including the formerly-divergent stance+Downed and
//! suppressed+Downed combinations — each pinned to ONE canonical answer.

use bevy::prelude::Color;
use gdtf_battle_sim::{Aiming, Direction, Facing, Faction, LifeState, Stance, StanceKind};

use super::super::{
    appearance::{GangerAppearance, ganger_sprite_appearance},
    roles::CharacterRoles,
    tint::faction_tint,
};
use crate::TileIndex;

/// An arbitrary in-test roles table (never the shipped data — the matrix pins the
/// classifier MECHANISM, not magnitudes).
fn roles() -> CharacterRoles {
    CharacterRoles {
        faction_0: TileIndex::new(10),
        faction_1: TileIndex::new(40),
    }
}

/// Classify with every input explicit (the full-matrix probe).
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

/// Classify a FIXED faction-0, East-facing ganger, varying only the posture / life /
/// suppression axes most cases probe.
fn posture(
    stance: StanceKind,
    aiming: bool,
    life: LifeState,
    suppressed: bool,
) -> GangerAppearance {
    classify(0, Direction::East, stance, aiming, life, suppressed)
}

/// The quiet live baseline: standing, unaimed, alive, unsuppressed.
fn baseline() -> GangerAppearance {
    posture(StanceKind::Standing, false, LifeState::Alive, false)
}

/// The relative luminance proxy (mean linear channel) — enough to assert the
/// DIRECTION of a tint delta without pinning channel values.
fn luminance(color: Color) -> f32 {
    let lin = color.to_linear();
    (lin.red + lin.green + lin.blue) / 3.0
}

/// The atlas-index half is the structural `faction_base + facing_frame` sum, and it is
/// decided by faction + facing + the table ALONE: stance, aim, life, and suppression
/// never move the index.
#[test]
fn index_is_base_plus_frame_and_ignores_posture_life_and_suppression() {
    let base = baseline();
    // Faction 0 East -> base 10 + RIGHT (3) = 13; faction 1 North -> base 40 + UP (2) = 42.
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
    // The index-invariant inputs: every posture/life/suppression variation keeps 13.
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

/// The live baseline tint is the faction tint (standing, unaimed, unsuppressed), and the
/// two factions read as two distinct colours.
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
    // Compare on linear channels: the classifier composes through the linear pipeline.
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

/// The live posture deltas keep their documented directions: Prone dims, aiming
/// brightens, and suppression both darkens AND differs from every unsuppressed variant.
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

/// CANONICAL PIN (GTW-631 C4) — stance/aim + Downed: the Downed grey-out WINS. A downed
/// body is out of the fight; Prone does not dim it further and aim does not brighten it.
/// (Formerly divergent: the life-state re-tint ignored stance/aim while the reframe
/// re-tint composed them — one derivation could disagree with the other on the same
/// state.)
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

/// CANONICAL PIN (GTW-631 C4) — suppressed + Downed: the Downed grey-out WINS. The
/// suppression colour-drain applies only to a LIVE ganger; a downed body keeps the one
/// out-of-fight grey.
#[test]
fn downed_pins_one_canonical_answer_for_suppression() {
    let downed = posture(StanceKind::Standing, false, LifeState::Downed, false);
    let downed_suppressed = posture(StanceKind::Standing, false, LifeState::Downed, true);
    assert_eq!(
        downed_suppressed, downed,
        "CANONICAL: Downed + suppressed draws the plain Downed grey-out (no extra drain)",
    );
}

/// Dead classifies as the live tint — the documented completeness answer (a Dead
/// ganger's sprite is despawned, or a deferred corpse the writer never re-styles, so
/// this verdict is never drawn).
#[test]
fn dead_classifies_as_the_live_tint_for_completeness() {
    let dead = posture(StanceKind::Standing, false, LifeState::Dead, false);
    assert_eq!(
        dead,
        baseline(),
        "Dead is classified like Alive (never drawn)"
    );
}
