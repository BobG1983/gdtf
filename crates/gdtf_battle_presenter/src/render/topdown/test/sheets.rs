//! Unit tests for the sheet-spec surface (per-sheet grid / tile size / asset path /
//! sampler pins).

use bevy::image::ImageSampler;

use super::super::atlases::SheetRole;

/// GTW-278 — the per-sheet tile size + grid: the render sheets stay 16-px tiles while the
/// portrait sheet is 32-px faces in a 10×10 grid. Pins the per-sheet `tile_px`/`grid`
/// behavior so a regression (e.g. Portraits reverting to 16, carving each 32-px face into
/// four wrong sub-tiles) is caught directly, without an app harness.
#[test]
fn portraits_use_a_32px_tile_while_render_sheets_stay_16px() {
    // The render sheets are 16-px tiles.
    assert_eq!(SheetRole::Terrain.tile_px(), 16, "terrain tiles are 16 px");
    assert_eq!(
        SheetRole::Characters.tile_px(),
        16,
        "character tiles are 16 px"
    );
    assert_eq!(SheetRole::Effects.tile_px(), 16, "effect tiles are 16 px");

    // The GTW-278 portrait sheet is a 10×10 grid of 32-px faces (100 indices).
    assert_eq!(
        SheetRole::Portraits.tile_px(),
        32,
        "portrait faces are 32 px (NOT 16 — a 16 would mis-carve each face into four tiles)",
    );
    assert_eq!(
        SheetRole::Portraits.grid(),
        (10, 10),
        "the portrait sheet is a 10×10 grid (100 faces, indices 0..=99)",
    );
}

/// GTW-295 — the PORTRAITS sheet overrides its sampler to NEAREST (so its HUD upscale point-
/// samples and does not bilinearly blend the tiles' transparent-white top rows into a fringe),
/// while the render sheets keep the DEFAULT sampler (`None`).
///
/// Pins the per-sheet sampler DECISION on the real code path `load_topdown_atlases` takes
/// (`SheetRole::sampler_override`), without an app — the loaded image's sampler is unreachable
/// headlessly (the image asset never finishes decoding without a render device). A revert to a
/// plain default load for Portraits makes its override `None` and fails this assert.
#[test]
fn portraits_override_to_nearest_sampler_render_sheets_keep_default() {
    // The portrait sheet overrides specifically to the NEAREST sampler (ImageSampler derives
    // PartialEq, so this pins the exact descriptor — not merely "some override").
    assert_eq!(
        SheetRole::Portraits.sampler_override(),
        Some(ImageSampler::nearest()),
        "the portraits sheet must override to the NEAREST sampler (the white-line fix)",
    );

    // The render sheets keep the DEFAULT sampler (no override) — they draw ~1:1 and do not
    // bleed, so the nearest override is Portraits-only.
    assert_eq!(
        SheetRole::Terrain.sampler_override(),
        None,
        "the terrain sheet keeps the default sampler (no override)",
    );
    assert_eq!(
        SheetRole::Characters.sampler_override(),
        None,
        "the characters sheet keeps the default sampler (no override)",
    );
    assert_eq!(
        SheetRole::Effects.sampler_override(),
        None,
        "the effects sheet keeps the default sampler (no override)",
    );
}
