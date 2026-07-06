//! Unit tests for the static-battlefield terrain draw.

use gdtf_battle_sim::{
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    prelude::Level,
};

use super::{active_level::ActiveLevel, roles::TileRoles, static_map::i32_extent};

/// The shipped `tile_roles.ron` parses into `TileRoles` and exposes every
/// documented role — a `ron::de` round-trip of the SHIPPED bytes (AC1).
///
/// It asserts the file PARSES and HAS all roles; it does NOT pin a tunable index
/// magnitude (those are data the engineer eyeballs and may adjust). A `floor`
/// failing to differ from `wall` would be a copy-paste authoring error, so the
/// distinctness check is a light structural guard, not a magnitude pin.
#[test]
fn shipped_tile_roles_ron_parses_with_all_roles() {
    const SHIPPED: &str = include_str!("../../../../../assets/sprites/tile_roles.spritedef.ron");
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
    // GTW-373 (the role -> index CONTRACT, the user's chosen mapping): floor/wall and the
    // stair up/down split + ladder are SYSTEM CONSTANTS the user picked for this ticket
    // (config/coordinate constants), not balance-tuning magnitudes the engineer eyeballs —
    // so pinning them asserts the CONTRACT (the locked-constant carve-out), not a brittle
    // tunable. The shipped tile_roles.ron must resolve these exact roles.
    assert_eq!(
        *roles.floor, 6,
        "the shipped tile_roles.ron must resolve floor == 6 (GTW-373)",
    );
    assert_eq!(
        *roles.wall, 0,
        "the shipped tile_roles.ron must resolve wall == 0 (GTW-373)",
    );
    assert_eq!(
        *roles.stair_up, 29,
        "the shipped tile_roles.ron must resolve stair_up == 29 (GTW-373, supersedes OQ-3 77)",
    );
    assert_eq!(
        *roles.stair_down, 28,
        "the shipped tile_roles.ron must resolve stair_down == 28 (GTW-373, supersedes OQ-3 77)",
    );
    assert_eq!(
        *roles.ladder, 235,
        "the shipped tile_roles.ron must resolve the ladder index 235 (UNCHANGED, user OQ-3)",
    );
}

/// GTW-373 (C1) — the `TileRoles` field-set is DISCRIMINATING: a `tile_roles.ron` MISSING
/// the new `stair_up` (or `stair_down` / `ladder`) key, or RENAMING it, fails to
/// deserialize.
///
/// This pins the struct's field-set 1:1 with the `.ron` keys (the round-trip above proves
/// the shipped file parses; this proves the parse is not vacuous — dropping or renaming a
/// required role IS rejected, so the field-set stays in lockstep with the data). It builds
/// a complete authored body, then re-authors it MISSING the `stair_up` key (and separately
/// with `stair_up` RENAMED) and asserts each fails — proving the GTW-373 split keys are
/// each required, and that the OLD single `stair` key is no longer accepted.
#[test]
fn tile_roles_field_set_is_discriminating() {
    // A complete authored body parses (the positive control) — the GTW-373 split keys plus
    // the GTW-367 `slab_destroyed` key plus the GTW-469 `wall_ew` key plus the GTW-470
    // orientation/direction door + stair keys (door_ns/door_ew/stair_ns_up/stair_ns_down/
    // stair_ew_up/stair_ew_down).
    const COMPLETE: &str = "(\
        floor: 6, floor_alt_panel: 128, wall: 0, wall_ew: 16, cover: 248, \
        emplacement: 346, emplacement_occupied: 347, slab: 22, rubble: 295, \
        slab_destroyed: 295, door: 339, stair_up: 29, stair_down: 28, ladder: 235, \
        door_ns: 340, door_ew: 341, stair_ns_up: 342, stair_ns_down: 343, stair_ew_up: 344, \
        stair_ew_down: 345)";
    // MISSING the `stair_up` key — must fail (the field is required).
    const MISSING_STAIR_UP: &str = "(\
        floor: 6, floor_alt_panel: 128, wall: 0, wall_ew: 16, cover: 248, \
        emplacement: 346, emplacement_occupied: 347, slab: 22, rubble: 295, \
        slab_destroyed: 295, door: 339, stair_down: 28, ladder: 235, \
        door_ns: 340, door_ew: 341, stair_ns_up: 342, stair_ns_down: 343, stair_ew_up: 344, \
        stair_ew_down: 345)";
    // RENAMED `stair_up` -> `stair` (the OLD single-stair key) — must fail (the field-set
    // is fixed; the superseded `stair` key no longer substitutes for the split role).
    const RENAMED_STAIR_UP: &str = "(\
        floor: 6, floor_alt_panel: 128, wall: 0, wall_ew: 16, cover: 248, \
        emplacement: 346, emplacement_occupied: 347, slab: 22, rubble: 295, \
        slab_destroyed: 295, door: 339, stair: 29, stair_down: 28, ladder: 235, \
        door_ns: 340, door_ew: 341, stair_ns_up: 342, stair_ns_down: 343, stair_ew_up: 344, \
        stair_ew_down: 345)";

    assert!(
        ron::de::from_str::<TileRoles>(COMPLETE).is_ok(),
        "a complete authored TileRoles body must parse",
    );
    assert!(
        ron::de::from_str::<TileRoles>(MISSING_STAIR_UP).is_err(),
        "a TileRoles body missing the `stair_up` key must fail to deserialize",
    );
    assert!(
        ron::de::from_str::<TileRoles>(RENAMED_STAIR_UP).is_err(),
        "a TileRoles body with `stair_up` renamed to the old `stair` key must fail to deserialize",
    );
}

/// GTW-373 (C4 (a)) — the shipped `tile_roles.ron` ROUND-TRIPS by IDENTITY: load ->
/// serialize -> load yields a structurally identical `TileRoles`.
///
/// Proves the `Serialize`/`Deserialize` pair is a faithful inverse on the SHIPPED bytes
/// (no field dropped, reordered into a different role, or re-typed by the re-emit), so the
/// data table is a stable contract. Distinct from the magnitude-mapping assertions above:
/// this checks STRUCTURE survives a serialize round-trip, those check the chosen indices.
#[test]
fn shipped_tile_roles_round_trips_by_identity() {
    const SHIPPED: &str = include_str!("../../../../../assets/sprites/tile_roles.spritedef.ron");
    let first: Result<TileRoles, _> = ron::de::from_str(SHIPPED);
    assert!(
        first.is_ok(),
        "shipped tile_roles.ron must parse: {:?}",
        first.as_ref().err(),
    );
    let Ok(first) = first else { return };
    let reserialized = ron::ser::to_string(&first);
    assert!(
        reserialized.is_ok(),
        "TileRoles must serialize back to RON: {:?}",
        reserialized.as_ref().err(),
    );
    let Ok(reserialized) = reserialized else {
        return;
    };
    let second: Result<TileRoles, _> = ron::de::from_str(&reserialized);
    assert!(
        second.is_ok(),
        "the re-serialized TileRoles must parse back: {:?}",
        second.as_ref().err(),
    );
    let Ok(second) = second else { return };
    assert_eq!(
        first, second,
        "load -> serialize -> load must yield a structurally identical TileRoles",
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
