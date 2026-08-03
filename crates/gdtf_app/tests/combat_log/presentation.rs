use bevy::{
    color::Alpha,
    prelude::*,
    text::{FontSize, LineHeight, TextColor, TextFont},
    ui::Node,
};
use gdtf_app::test_support::CombatLogLine;
use gdtf_battle_sim::{acts::MovementOccurred, prelude::Cell};

use super::harness::*;

fn max_line_alpha(app: &mut App) -> f32 {
    let entities = all_with::<CombatLogLine>(app);
    entities
        .into_iter()
        .filter_map(|e| app.world().get::<TextColor>(e).map(|c| c.0.alpha()))
        .fold(0.0_f32, f32::max)
}

#[test]
fn a_fresh_line_fades_in_rather_than_snapping_to_full_opacity() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    play(
        &mut app,
        MovementOccurred::new(ganger, Cell::new(3, 4), Cell::new(3, 6)),
    );
    app.update();
    assert_eq!(
        all_with::<CombatLogLine>(&mut app).len(),
        1,
        "the movement event appended exactly one line",
    );
    let just_appeared = max_line_alpha(&mut app);
    assert!(
        just_appeared < 0.95,
        "a freshly appeared line must be FADING IN (alpha below full), got {just_appeared}",
    );

    for _ in 0..64 {
        app.update();
    }
    let after_hold = max_line_alpha(&mut app);
    assert!(
        after_hold > just_appeared,
        "after the fade-in window the line's alpha must have climbed (fade-in ramp), \
         {after_hold} must exceed {just_appeared}",
    );
}

const DEFAULT_LINE_FONT_PT: f32 = 20.0;

const THEME_BODY_FONT_PT: f32 = 18.0;

fn first_line_font_px(app: &mut App) -> Option<f32> {
    let entity = *all_with::<CombatLogLine>(app).first()?;
    match app.world().get::<TextFont>(entity)?.font_size {
        FontSize::Px(px) => Some(px),
        _ => None,
    }
}

#[test]
fn a_log_line_draws_at_the_larger_tuned_size_not_the_body_text() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Alex Mercer");
    app.update();

    play(
        &mut app,
        MovementOccurred::new(ganger, Cell::new(15, 11), Cell::new(14, 12)),
    );
    app.update();

    let size = first_line_font_px(&mut app);
    assert_eq!(
        size,
        Some(DEFAULT_LINE_FONT_PT),
        "a normal-emphasis log line draws at the tuned line_font_pt ({DEFAULT_LINE_FONT_PT}), got \
         {size:?}",
    );
    let size = size.unwrap_or_default();
    assert!(
        size > THEME_BODY_FONT_PT,
        "the log line size ({size}) must be clearly LARGER than the {THEME_BODY_FONT_PT}pt theme \
         body text (the readability fix), so the log is legible",
    );
}

#[test]
fn a_log_line_reserves_its_full_glyph_box_so_it_is_not_clipped() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Alex Mercer");
    app.update();

    play(
        &mut app,
        MovementOccurred::new(ganger, Cell::new(15, 11), Cell::new(14, 12)),
    );
    app.update();

    let lines = all_with::<CombatLogLine>(&mut app);
    assert_eq!(
        lines.len(),
        1,
        "the movement event appends exactly one line"
    );
    let Some(&entity) = lines.first() else {
        return;
    };

    let line_height = app.world().get::<LineHeight>(entity).copied();
    assert!(
        line_height.is_some_and(|h| matches!(h, LineHeight::Px(px) if px >= DEFAULT_LINE_FONT_PT)),
        "a combat-log line must carry an explicit Px LineHeight box reserving at least the full \
         font height ({DEFAULT_LINE_FONT_PT}px) plus leading so descenders are not clipped; was \
         {line_height:?}",
    );

    let min_height = app.world().get::<Node>(entity).map(|n| n.min_height);
    assert!(
        min_height.is_some_and(|m| matches!(m, Val::Px(px) if px >= DEFAULT_LINE_FONT_PT)),
        "the line Node must reserve a Px min_height for the full glyph box ({DEFAULT_LINE_FONT_PT}px) \
         so a settled line is fully visible under the clip; was {min_height:?}",
    );
}
