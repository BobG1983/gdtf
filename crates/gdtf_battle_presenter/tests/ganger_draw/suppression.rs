//! Suppression desaturate + restore (GTW-526 C8).

use bevy::{app::App, prelude::Entity};
use gdtf_battle_presenter::GangerSprites;
use gdtf_battle_sim::{
    ganger::{Suppressed, SuppressorCell},
    prelude::{Cell, CellLevel, Direction, Level},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

/// Whether two sprite colors are the SAME colour within tolerance, compared on their linear
/// RGB channels — so two `Color`s carrying the same colour compare equal despite a possibly
/// different enum variant (the appearance classifier composes through the linear pipeline).
/// `None` colors never match.
fn same_color(a: Option<bevy::color::Color>, b: Option<bevy::color::Color>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            let (la, lb) = (a.to_linear(), b.to_linear());
            (la.red - lb.red).abs() < 1.0e-4
                && (la.green - lb.green).abs() < 1.0e-4
                && (la.blue - lb.blue).abs() < 1.0e-4
                && (la.alpha - lb.alpha).abs() < 1.0e-4
        }
        _ => false,
    }
}

/// The saturation of a sprite color — the spread between its brightest and dimmest linear
/// channel, normalized by the brightest, so a fully-grey swatch is `0.0` and a saturated one
/// approaches `1.0`. Enough to assert the DESATURATION direction of the suppressed re-tint
/// without pinning exact channel values. `None` colors (no sprite) sort to `0.0`.
fn saturation(color: Option<bevy::color::Color>) -> f32 {
    match color {
        Some(c) => {
            let lin = c.to_linear();
            let max = lin.red.max(lin.green).max(lin.blue);
            let min = lin.red.min(lin.green).min(lin.blue);
            if max <= f32::EPSILON {
                0.0
            } else {
                (max - min) / max
            }
        }
        None => 0.0,
    }
}

/// Insert `Suppressed` on sim ganger `sim` (the real APPLY transition — `Changed<Suppressed>`
/// the appearance resolver keys on), anchored to `from` the way the sim producer does.
fn suppress(app: &mut App, sim: Entity, from: CellLevel) {
    app.world_mut()
        .entity_mut(sim)
        .insert(Suppressed::new(SuppressorCell::new(from)));
}

/// Remove `Suppressed` from sim ganger `sim` (the real CLEAR transition — a component REMOVAL,
/// which `Changed` does NOT observe, so `resolve_ganger_appearance` drains it via
/// `RemovedComponents<Suppressed>`).
fn unsuppress(app: &mut App, sim: Entity) {
    app.world_mut().entity_mut(sim).remove::<Suppressed>();
}

/// GTW-526 C8 — a `Changed<Suppressed>` desaturates + darkens the ganger's sprite (the
/// distinct "pinned" look), and REMOVING `Suppressed` (a component removal, which `Changed`
/// does NOT observe) drains through `RemovedComponents<Suppressed>` to restore the ordinary
/// tint.
///
/// This pins the appearance resolver's suppression arm on the REAL path:
/// - APPLY: inserting `Suppressed` trips `Changed<Suppressed>` — the sprite re-tints to a
///   colour-drained (lower-saturation) AND dimmer swatch than the un-suppressed tint.
/// - CLEAR: removing `Suppressed` is a removal, not a `Changed`; the system's
///   `RemovedComponents<Suppressed>` arm re-tints the sprite BACK to exactly the original
///   un-suppressed tint.
///
/// Deleting the `Changed<Suppressed>` arm (or the removal arm) FAILS this test.
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

    // --- APPLY: insert Suppressed -> Changed<Suppressed> desaturates + darkens. ---
    let suppressor = CellLevel::new(Cell::new(9, 6), Level::new(0));
    suppress(&mut app, sim, suppressor);
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

    // --- CLEAR: remove Suppressed -> RemovedComponents<Suppressed> restores the tint. ---
    unsuppress(&mut app, sim);
    app.update();
    let color_restored = sprite_color(&mut app, sprite);
    // Compare on the linear channels (not variant-equality): the appearance re-stamp emits a
    // `LinearRgba` — same colour, possibly a different enum variant than the captured
    // baseline — so assert the linear RGB match within tolerance rather than `==`.
    assert!(
        same_color(color_restored, color_clear),
        "clearing suppression (a component REMOVAL) must restore the ORIGINAL un-suppressed \
         tint via the RemovedComponents<Suppressed> arm (restored {color_restored:?} vs \
         clear {color_clear:?})",
    );
    // And it is genuinely BACK to the un-suppressed look, not still washed out.
    assert!(
        saturation(color_restored) > saturation(color_suppressed),
        "the restored tint must be MORE saturated than the suppressed one (the desaturation \
         was undone)",
    );
}
