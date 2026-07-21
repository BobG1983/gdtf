//! Facing reframe + stance/aim retint on the same sprite; the facing-frame API
//! surface.

use gdtf_battle_presenter::{FacingFrame, GangerSprites};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Level, StanceKind},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

/// Reframe / re-tint (the contract's "asserted to match" clause, identical phrasing to
/// the Death clause AC4 asserts) — drive the change path through the presenter's OWN
/// posture mirror ([`DrawnPose`](gdtf_battle_presenter::DrawnPose)), which GTW-727 C17
/// re-sourced the resolver onto (what the cursor has SHOWN, not the state the sim reached):
///
/// - a facing change recomputes the atlas index via the 8->4 map: the sprite re-indexes
///   to `base_for(faction) + facing_frame(new_facing)` read STRUCTURALLY (never a
///   literal), distinct from the spawn frame.
/// - a stance change (-> `Prone`) dims the tint; an aim change (-> `true`) brightens it —
///   both on the SAME presenter sprite, each a distinct, expected-direction re-tint.
///
/// This pins `resolve_ganger_appearance` + its tint composition on the real path: deleting
/// the appearance resolver (or no-opping its atlas/color writes) FAILS this test.
#[test]
fn changed_facing_reframes_and_stance_aiming_retints_the_same_sprite() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    // Spawn fixture: faction 0, facing East (RIGHT frame), Standing, not aiming.
    let at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(at, 0, Direction::East))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let roles = character_roles(&app);
    assert!(roles.is_some(), "CharacterRoles must be resident");
    let Some(roles) = roles else { return };

    let sim = sim_entity_at(&mut app, at);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    // The presenter sprite the map links to this sim entity (the SAME one we reframe).
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim));
    assert!(sprite.is_some(), "the ganger must be mapped to a sprite");
    let Some(sprite) = sprite else { return };

    // --- Facing reframe: East (RIGHT) -> North (UP). ---
    let index_east = atlas_index_of_sim(&mut app, sim);
    assert_eq!(
        index_east,
        Some(expected_index(&roles, 0, Direction::East)),
        "spawn frame = faction_0 base + East(RIGHT) offset",
    );

    set_drawn_facing(&mut app, sim, Direction::North);
    app.update();

    let index_north = atlas_index_of_sim(&mut app, sim);
    assert_eq!(
        index_north,
        Some(expected_index(&roles, 0, Direction::North)),
        "after Changed<Facing> the SAME sprite re-indexes to faction_0 base + North(UP) \
         offset, read structurally from the table + the 8->4 map",
    );
    assert_ne!(
        index_north, index_east,
        "the reframe actually changed the atlas index (East RIGHT vs North UP)",
    );

    // --- Stance re-tint: Standing -> Prone dims. ---
    let color_standing = sprite_color(&mut app, sprite);
    set_drawn_stance(&mut app, sim, StanceKind::Prone);
    app.update();
    let color_prone = sprite_color(&mut app, sprite);
    assert!(
        color_standing != color_prone,
        "a Prone ganger re-tints (standing {color_standing:?} vs prone {color_prone:?})",
    );
    assert!(
        luminance(color_prone) < luminance(color_standing),
        "Prone dims the sprite (prone luminance must be < standing luminance)",
    );

    // --- Aiming re-tint: not-aiming -> aiming brightens (from the Prone baseline). ---
    set_drawn_aiming(&mut app, sim, true);
    app.update();
    let color_prone_aiming = sprite_color(&mut app, sprite);
    assert!(
        color_prone != color_prone_aiming,
        "an aiming ganger re-tints (prone {color_prone:?} vs prone+aiming \
         {color_prone_aiming:?})",
    );
    assert!(
        luminance(color_prone_aiming) > luminance(color_prone),
        "aiming brightens the sprite (prone+aiming luminance must be > prone luminance)",
    );
}

/// A compile-time witness that the public `FacingFrame` offsets are reachable from the
/// integration boundary (the structural sum the AC1 index assert relies on).
#[test]
fn facing_frame_offsets_are_public() {
    assert_eq!(*FacingFrame::LEFT, 0);
    assert_eq!(*FacingFrame::DOWN, 1);
    assert_eq!(*FacingFrame::UP, 2);
    assert_eq!(*FacingFrame::RIGHT, 3);
}
