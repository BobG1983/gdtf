//! Suppression desaturate + restore (GTW-526 C8).

use gdtf_battle_presenter::GangerSprites;
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Level},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

/// GTW-526 C8, re-sourced onto the GTW-727 C17 posture mirror — a suppressed
/// [`DrawnPose`](gdtf_battle_presenter::DrawnPose) desaturates + darkens the ganger's sprite
/// (the distinct "pinned" look), and clearing it restores the ordinary tint.
///
/// This pins the appearance resolver's suppression arm on the REAL path:
/// - APPLY: showing a suppressed pose trips `Changed<DrawnPose>` — the sprite re-tints to a
///   colour-drained (lower-saturation) AND dimmer swatch than the un-suppressed tint.
/// - CLEAR: showing the un-suppressed pose again is another ordinary `Changed<DrawnPose>`
///   (GTW-727 C17 made suppression a pose FIELD and deleted the `RemovedComponents<Suppressed>`
///   drain), re-tinting the sprite BACK to exactly the original un-suppressed tint.
///
/// Deleting the suppression arm FAILS this test.
#[test]
fn suppression_desaturates_the_sprite_and_clearing_restores_it() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    // Spawn fixture: faction 0, facing East, Standing, not aiming (an un-suppressed live tint).
    let at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(at, 0, Direction::East))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let sim = sim_entity_at(&mut app, at);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim));
    assert!(sprite.is_some(), "the ganger must be mapped to a sprite");
    let Some(sprite) = sprite else { return };

    // The un-suppressed baseline tint.
    let color_clear = sprite_color(&mut app, sprite);

    // --- APPLY: show a suppressed pose -> Changed<DrawnPose> desaturates + darkens. ---
    set_drawn_suppressed(&mut app, sim, true);
    app.update();
    let color_suppressed = sprite_color(&mut app, sprite);
    assert!(
        color_clear != color_suppressed,
        "a suppressed ganger must re-tint (clear {color_clear:?} vs suppressed \
         {color_suppressed:?})",
    );
    assert!(
        luminance(color_suppressed) < luminance(color_clear),
        "suppression darkens the sprite (suppressed luminance must be < the un-suppressed one)",
    );
    assert!(
        saturation(color_suppressed) < saturation(color_clear),
        "suppression desaturates the sprite (suppressed saturation {:?} must be < the \
         un-suppressed one {:?}) — the distinct washed-out pinned look",
        saturation(color_suppressed),
        saturation(color_clear),
    );

    // --- CLEAR: show the un-suppressed pose again -> Changed<DrawnPose> restores the tint. ---
    set_drawn_suppressed(&mut app, sim, false);
    app.update();
    let color_restored = sprite_color(&mut app, sprite);
    // Compare on the linear channels (not variant-equality): the appearance re-stamp emits a
    // `LinearRgba` — same colour, possibly a different enum variant than the captured
    // baseline — so assert the linear RGB match within tolerance rather than `==`.
    assert!(
        same_color(color_restored, color_clear),
        "clearing suppression (a DrawnPose field change) must restore the ORIGINAL \
         un-suppressed tint (restored {color_restored:?} vs clear {color_clear:?})",
    );
    // And it is genuinely BACK to the un-suppressed look, not still washed out.
    assert!(
        saturation(color_restored) > saturation(color_suppressed),
        "the restored tint must be MORE saturated than the suppressed one (the desaturation \
         was undone)",
    );
}
