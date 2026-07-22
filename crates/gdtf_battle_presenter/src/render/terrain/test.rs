//! Unit tests for the static-battlefield terrain draw's pure helpers.
//!
//! The retired role-table serde pins (parse / field-set / round-trip over
//! `tile_roles.spritedef.ron`) left with the table (GTW-665); the successor data
//! pins live in `roles/test.rs` (vocabulary↔seeded-catalog lockstep) and the
//! `terrain_draw` / `vertical_link_draw` integration suites (the drawn rects
//! against the seeded defs).

use bevy::math::{URect, UVec2, Vec2};
use gdtf_battle_sim::{
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    prelude::Level,
};
use gdtf_content_families::sprites::{
    SpriteAnchor, SpriteDef, SpriteImagePath, SpritePx, SpriteRect, SpriteSource,
};

use super::{
    active_level::ActiveLevel,
    resolve::{anchor_world_offset, single_rect_layout, source_parts, source_px_size},
    static_map::i32_extent,
};

/// A Sheet-source def fixture at the given rect + anchor.
fn sheet_def(x: u32, y: u32, w: u32, h: u32, ax: u32, ay: u32) -> SpriteDef {
    SpriteDef {
        source:    SpriteSource::Sheet {
            sheet: SpriteImagePath::new("sprites/alt_tileset_terrain.png".to_owned()),
            rect:  SpriteRect {
                x: SpritePx::new(x),
                y: SpritePx::new(y),
                w: SpritePx::new(w),
                h: SpritePx::new(h),
            },
        },
        anchor:    SpriteAnchor {
            x: SpritePx::new(ax),
            y: SpritePx::new(ay),
        },
        facings:   None,
        animation: None,
    }
}

/// GTW-665 C1 — `source_parts` / `source_px_size` split a Sheet source into its sheet
/// path + rect (with a knowable pixel extent) and a File source into its path alone
/// (extent unknowable headlessly — the GTW-664 layered-knowledge rule: no invented dims).
#[test]
fn source_projections_split_sheet_and_file() {
    let sheet = sheet_def(96, 0, 16, 16, 8, 8);
    let (path, rect) = source_parts(&sheet.source);
    assert_eq!(path.as_str(), "sprites/alt_tileset_terrain.png");
    assert!(rect.is_some(), "a Sheet source carries its rect");
    assert_eq!(
        source_px_size(&sheet.source),
        Some(UVec2::new(16, 16)),
        "a Sheet source's extent is its rect's w × h",
    );

    let file = SpriteSource::File(SpriteImagePath::new("sprites/lone_crate.png".to_owned()));
    let (path, rect) = source_parts(&file);
    assert_eq!(path.as_str(), "sprites/lone_crate.png");
    assert!(
        rect.is_none(),
        "a File source has no rect (the whole image)"
    );
    assert_eq!(
        source_px_size(&file),
        None,
        "a File source's extent is unknowable without the decoded image",
    );
}

/// GTW-665 C1 — `single_rect_layout` carries EXACTLY the authored region at index 0:
/// the material's UV bake reads `layout.textures[0]`, so this is the rect the shader
/// samples — identical to a grid layout's entry for a grid-aligned rect.
#[test]
fn single_rect_layout_carries_exactly_the_authored_region() {
    let def = sheet_def(208, 16, 16, 16, 8, 8);
    let (_, rect) = source_parts(&def.source);
    let region = rect.map(super::resolve::source_urect);
    assert_eq!(
        region,
        Some(URect {
            min: UVec2::new(208, 16),
            max: UVec2::new(224, 32),
        }),
        "the authored rect projects to its pixel URect",
    );
    let Some(region) = region else { return };
    let layout = single_rect_layout(region);
    assert_eq!(
        layout.textures,
        vec![region],
        "the single-rect layout holds exactly the authored region at index 0",
    );
}

/// GTW-665 C2 — the anchor offset: a CENTER anchor is exactly `Vec2::ZERO` (the
/// identical-pixels guarantee for every seeded def), an off-center anchor displaces the
/// sprite CENTER so the anchor point sits ON the cell position (bottom-center anchor →
/// the sprite rises by half its drawn height), and a degenerate zero extent is a
/// documented no-op.
#[test]
fn anchor_offset_center_is_zero_and_bottom_anchor_lifts() {
    let drawn = Vec2::splat(16.0);
    let centered = sheet_def(0, 0, 16, 16, 8, 8);
    assert_eq!(
        anchor_world_offset(&centered, UVec2::new(16, 16), drawn),
        Vec2::ZERO,
        "the seeded CENTER anchor must be a zero offset (identical pixels)",
    );

    // Bottom-center ground contact (8, 16): the anchor sits at the sprite's bottom edge,
    // so the CENTER rises by half the drawn height (+y is up in world space).
    let bottom = sheet_def(0, 0, 16, 16, 8, 16);
    assert_eq!(
        anchor_world_offset(&bottom, UVec2::new(16, 16), drawn),
        Vec2::new(0.0, 8.0),
        "a bottom-center anchor lifts the sprite center by half its drawn height",
    );

    let degenerate = sheet_def(0, 0, 0, 0, 0, 0);
    assert_eq!(
        anchor_world_offset(&degenerate, UVec2::ZERO, drawn),
        Vec2::ZERO,
        "a zero-extent sprite yields the documented centered no-op (no NaN)",
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
