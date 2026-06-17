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
    let _ = roles.floor_alt_stone;
    let _ = roles.floor_alt_dirt;
    let _ = roles.floor_alt_grass;
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
