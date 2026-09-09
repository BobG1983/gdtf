use gdtf_battle_presenter::GangerSprites;
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Level},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

#[test]
fn suppression_desaturates_the_sprite_and_clearing_restores_it() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

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

    let color_clear = sprite_color(&mut app, sprite);

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

    set_drawn_suppressed(&mut app, sim, false);
    app.update();
    let color_restored = sprite_color(&mut app, sprite);
    assert!(
        same_color(color_restored, color_clear),
        "clearing suppression (a DrawnPose field change) must restore the ORIGINAL \
         un-suppressed tint (restored {color_restored:?} vs clear {color_clear:?})",
    );
    assert!(
        saturation(color_restored) > saturation(color_suppressed),
        "the restored tint must be MORE saturated than the suppressed one (the desaturation \
         was undone)",
    );
}
