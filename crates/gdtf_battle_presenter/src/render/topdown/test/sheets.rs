use bevy::image::ImageSampler;

use super::super::atlases::SheetRole;

#[test]
fn portraits_use_a_32px_tile_while_render_sheets_stay_16px() {
    assert_eq!(SheetRole::Terrain.tile_px(), 16, "terrain tiles are 16 px");
    assert_eq!(
        SheetRole::Characters.tile_px(),
        16,
        "character tiles are 16 px"
    );
    assert_eq!(SheetRole::Effects.tile_px(), 16, "effect tiles are 16 px");

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

#[test]
fn portraits_override_to_nearest_sampler_render_sheets_keep_default() {
    assert_eq!(
        SheetRole::Portraits.sampler_override(),
        Some(ImageSampler::nearest()),
        "the portraits sheet must override to the NEAREST sampler (the white-line fix)",
    );

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
