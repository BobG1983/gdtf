//! Line rendering: fade-in alpha, tuned font size, the full glyph box.

use bevy::{
    color::Alpha,
    prelude::*,
    text::{FontSize, LineHeight, TextColor, TextFont},
    ui::Node,
};
use gdtf_app::test_support::CombatLogLine;
use gdtf_battle_sim::{acts::MovementOccurred, prelude::Cell};

use super::harness::*;

/// The greatest rendered alpha across every combat-log line — the brightest line on screen.
/// `0.0` when there are no lines. Used to prove the GTW-328 slice-B fade-IN (a fresh line starts
/// transparent and ramps up) without depending on the exact per-update delta.
fn max_line_alpha(app: &mut App) -> f32 {
    let entities = all_with::<CombatLogLine>(app);
    entities
        .into_iter()
        .filter_map(|e| app.world().get::<TextColor>(e).map(|c| c.0.alpha()))
        .fold(0.0_f32, f32::max)
}

/// GTW-328 slice B: a freshly appended line does NOT snap to full opacity — it FADES IN. The
/// frame it spawns its alpha is well below full (it spawned transparent and the fade system has
/// only ramped it a sliver), and over subsequent updates it climbs toward full as the hold phase
/// begins. (The unit test in `components.rs` proves the exact three-phase curve; this proves the
/// real spawned line is driven by it through the app stack.)
#[test]
fn a_fresh_line_fades_in_rather_than_snapping_to_full_opacity() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    play(
        &mut app,
        MovementOccurred::new(ganger, Cell::new(3, 4), Cell::new(3, 6)),
    );
    // The drain frame: the line spawns (transparent) and the fade system ramps it a sliver.
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

    // Drive enough updates to clear the (short, sub-second) fade-in window — the line reaches the
    // hold phase at (near) full opacity, proving the ramp climbs rather than staying dim or off.
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

/// The shipped default combat-log line size (`combat_log.tuning.ron` / `LineFontPt::DEFAULT`) — the size
/// the headless harness (no RON, defaulted tuning) draws each line at. Mirrors the tuning default
/// so the readability test is independent of the (unloaded) RON.
const DEFAULT_LINE_FONT_PT: f32 = 20.0;

/// The theme's body text size (`assets/core_tuning/ui_theme.tuning.ron` / fallback) — the size the log USED to
/// (too-small-ly) render at before the fix. The log line size must be clearly LARGER than this.
const THEME_BODY_FONT_PT: f32 = 18.0;

/// The font size of the first combat-log line (its [`TextFont`] is set on the line entity itself).
/// `None` if there are no lines or the line carries no `TextFont` / non-`Px` size.
fn first_line_font_px(app: &mut App) -> Option<f32> {
    let entity = *all_with::<CombatLogLine>(app).first()?;
    match app.world().get::<TextFont>(entity)?.font_size {
        FontSize::Px(px) => Some(px),
        _ => None,
    }
}

/// GTW combat-log readability fix: a log line draws at the tuned `line_font_pt` (the log's OWN
/// readable size, `20.0` pt by default), NOT the theme's smaller `18.0` pt body text — so a line
/// like "Alex Mercer moved (15, 11) -> (14, 12)" is legible at a glance rather than rendering too
/// small.
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

/// GTW combat-log cut-off fix: each spawned line reserves its FULL glyph box — an explicit
/// [`LineHeight::Px`] AND a matching `min_height` at least as tall as the font size — so the panel
/// clip never shaves a settled line's ascenders/descenders. (The slide-in reveal from below still
/// happens; this only guarantees the line box is fully reserved so a resting line is uncut.)
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
    // `first()` is `Some` (len == 1 asserted above); bind without `unwrap` (denied even in tests).
    let Some(&entity) = lines.first() else {
        return;
    };

    // An explicit Px line height (not bevy's default RelativeToFont) at least the font size, so the
    // glyph box is reserved against the clip rather than collapsing. `is_some_and` asserts the
    // Px box AND its size in one go (no bind-then-panic — restriction lints deny `panic!` in tests).
    let line_height = app.world().get::<LineHeight>(entity).copied();
    assert!(
        line_height.is_some_and(|h| matches!(h, LineHeight::Px(px) if px >= DEFAULT_LINE_FONT_PT)),
        "a combat-log line must carry an explicit Px LineHeight box reserving at least the full \
         font height ({DEFAULT_LINE_FONT_PT}px) plus leading so descenders are not clipped; was \
         {line_height:?}",
    );

    // And the line Node's min_height matches that box, so the laid-out line never collapses shorter
    // than its glyphs (and the clipped panel's summed height accounts for it).
    let min_height = app.world().get::<Node>(entity).map(|n| n.min_height);
    assert!(
        min_height.is_some_and(|m| matches!(m, Val::Px(px) if px >= DEFAULT_LINE_FONT_PT)),
        "the line Node must reserve a Px min_height for the full glyph box ({DEFAULT_LINE_FONT_PT}px) \
         so a settled line is fully visible under the clip; was {min_height:?}",
    );
}
