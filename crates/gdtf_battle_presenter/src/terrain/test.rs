//! Unit tests for the static-battlefield terrain draw.

use gdtf_battle_sim::{GRID_HEIGHT, GRID_WIDTH, Level};

use super::{active_level::ActiveLevel, draw::i32_extent, roles::TileRoles};

/// The shipped `tile_roles.ron` parses into `TileRoles` and exposes every
/// documented role — a `ron::de` round-trip of the SHIPPED bytes (AC1).
///
/// It asserts the file PARSES and HAS all roles; it does NOT pin a tunable index
/// magnitude (those are data the engineer eyeballs and may adjust). A `floor`
/// failing to differ from `wall` would be a copy-paste authoring error, so the
/// distinctness check is a light structural guard, not a magnitude pin.
#[test]
fn shipped_tile_roles_ron_parses_with_all_roles() {
    const SHIPPED: &str = include_str!("../../../../assets/tiles/tile_roles.ron");
    let parsed: Result<TileRoles, _> = ron::de::from_str(SHIPPED);
    // Parsing into TileRoles proves every documented role is present (a missing
    // field would be a deserialize error). Assert the parse succeeded; if not,
    // surface the error rather than pinning any index magnitude.
    assert!(
        parsed.is_ok(),
        "shipped tile_roles.ron must parse into TileRoles, got: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(roles) = parsed else {
        return;
    };
    // Sanity-check the five draw roles are not all collapsed onto one index (an
    // authoring slip) — a structural guard, not a magnitude pin.
    let draw_roles = [
        roles.floor,
        roles.wall,
        roles.cover,
        roles.slab,
        roles.rubble,
    ];
    let all_same = draw_roles.iter().all(|r| *r == roles.floor);
    assert!(
        !all_same,
        "the five draw roles must not all share one index (authoring slip)",
    );
    // The door role is documented; the struct parsing means it is present.
    let _ = roles.door;
    let _ = roles.floor_alt_panel;
    // GTW-359 (C1): the stair / ladder indices are LOCKED SYSTEM CONSTANTS (user OQ-3
    // ruling), not tunable magnitudes the engineer eyeballs — so asserting them is the
    // locked-constant exemption, not a brittle magnitude pin. The shipped tile_roles.ron
    // must resolve stair == 77 and ladder == 235.
    assert_eq!(
        *roles.stair, 77,
        "the shipped tile_roles.ron must resolve the LOCKED stair index 77 (user OQ-3)",
    );
    assert_eq!(
        *roles.ladder, 235,
        "the shipped tile_roles.ron must resolve the LOCKED ladder index 235 (user OQ-3)",
    );
}

/// GTW-359 (C1) — the `TileRoles` field-set is DISCRIMINATING: a `tile_roles.ron` MISSING
/// the new `stair` (or `ladder`) key, or RENAMING it, fails to deserialize.
///
/// This pins the struct's field-set 1:1 with the `.ron` keys (the round-trip above proves
/// the shipped file parses; this proves the parse is not vacuous — dropping or renaming a
/// required role IS rejected, so the field-set stays in lockstep with the data). It builds
/// a complete authored body, then re-authors it MISSING the stair key (and separately with
/// the stair key RENAMED) and asserts each fails.
#[test]
fn tile_roles_field_set_is_discriminating() {
    // A complete authored body parses (the positive control).
    const COMPLETE: &str = "(\
        floor: 144, floor_alt_panel: 128, wall: 16, cover: 248, slab: 22, rubble: 295, \
        door: 339, stair: 77, ladder: 235)";
    // MISSING the `stair` key — must fail (the field is required).
    const MISSING_STAIR: &str = "(\
        floor: 144, floor_alt_panel: 128, wall: 16, cover: 248, slab: 22, rubble: 295, \
        door: 339, ladder: 235)";
    // RENAMED `stair` -> `staircase` — must fail (the field-set is fixed, no unknown key
    // substitutes for a required one).
    const RENAMED_STAIR: &str = "(\
        floor: 144, floor_alt_panel: 128, wall: 16, cover: 248, slab: 22, rubble: 295, \
        door: 339, staircase: 77, ladder: 235)";

    assert!(
        ron::de::from_str::<TileRoles>(COMPLETE).is_ok(),
        "a complete authored TileRoles body must parse",
    );
    assert!(
        ron::de::from_str::<TileRoles>(MISSING_STAIR).is_err(),
        "a TileRoles body missing the `stair` key must fail to deserialize",
    );
    assert!(
        ron::de::from_str::<TileRoles>(RENAMED_STAIR).is_err(),
        "a TileRoles body with `stair` renamed to `staircase` must fail to deserialize",
    );
}

/// `i32_extent` returns the grid extent unchanged for the real 60x60 grid.
#[test]
fn i32_extent_passes_the_real_grid_extents() {
    assert_eq!(i32_extent(GRID_WIDTH), 60, "GRID_WIDTH is 60");
    assert_eq!(i32_extent(GRID_HEIGHT), 60, "GRID_HEIGHT is 60");
}

/// `ActiveLevel` defaults to the ground floor (level 0).
#[test]
fn active_level_defaults_to_level_zero() {
    assert_eq!(
        *ActiveLevel::default(),
        Level::new(0),
        "default active level is 0"
    );
}
