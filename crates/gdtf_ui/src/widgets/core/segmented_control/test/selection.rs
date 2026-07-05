//! Active-segment selection tests: the repaint-all-in-one-update contract, the
//! selection message identity, the no-op repress, the palette override, and the
//! GTW-277 painter exclusion.

use bevy::{ecs::system::SystemState, prelude::*, text::FontWeight, ui::Interaction};

use super::{
    super::{ActiveSegment, SegmentColors, SegmentLabel, SegmentSelected, spawn_segmented_control},
    support::{SEG_COLORS, segment_look, segments_of, spawn_fire_mode},
};
use crate::widgets::core::{Orientation, test_support::harness};

/// AC — setting the active segment to B (from A) repaints BOTH in ONE update: B
/// filled/active (bold), A returned to base (normal) — active-driven, no hover (the
/// GTW-280 stale-highlight pin). The selection message carries B's identity; segment
/// entity ids are stable.
#[test]
fn segmented_control_active_change_repaints_all_one_update() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    // Settle the initial paint.
    app.update();

    let segs = segments_of(&mut app, control);
    assert_eq!(segs.len(), 3, "must have 3 segments");
    let seg_a = segs[0].0; // Single (active at spawn)
    let seg_b = segs[1].0; // Burst

    // Precondition: A active (bold + active bg), B base. None hovered (Interaction None).
    let (a_bg0, a_w0, _) = segment_look(&mut app, seg_a);
    assert_eq!(a_bg0, SEG_COLORS.active_bg, "precondition: A starts active");
    assert_eq!(a_w0, FontWeight::BOLD, "precondition: A starts bold");
    assert_eq!(
        app.world().get::<Interaction>(seg_a).copied(),
        Some(Interaction::None),
        "precondition: A is not hovered",
    );

    // Press B (without hovering A) and run ONE update.
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(seg_b) {
        *interaction = Interaction::Pressed;
    }
    app.update();

    // In ONE update: B is active+bold, A returned to base+normal.
    let (b_bg, b_w, b_text) = segment_look(&mut app, seg_b);
    assert_eq!(b_bg, SEG_COLORS.active_bg, "B must be the active fill");
    assert_eq!(
        b_w,
        FontWeight::BOLD,
        "B must be bold (color-blind-safe channel)"
    );
    assert_eq!(
        b_text, SEG_COLORS.active_text,
        "B must be the active text color"
    );

    let (a_bg, a_w, a_text) = segment_look(&mut app, seg_a);
    assert_eq!(
        a_bg, SEG_COLORS.base_bg,
        "A must return to the base fill (no stale active)"
    );
    assert_eq!(a_w, FontWeight::NORMAL, "A must return to normal weight");
    assert_eq!(
        a_text, SEG_COLORS.base_text,
        "A must return to the base text color"
    );
    assert_eq!(
        app.world().get::<ActiveSegment>(control).map(|s| **s),
        Some(1),
        "the root's active index is now B",
    );

    // The selection message carries B's identity.
    let mut state: SystemState<MessageReader<SegmentSelected>> = SystemState::new(app.world_mut());
    let reader_result = state.get_mut(app.world_mut());
    assert!(reader_result.is_ok(), "MessageReader params must validate");
    let Ok(mut reader) = reader_result else {
        return;
    };
    let msgs: Vec<SegmentSelected> = reader.read().copied().collect();
    assert_eq!(
        msgs.len(),
        1,
        "exactly one SegmentSelected per active change"
    );
    assert_eq!(
        msgs[0].control, control,
        "the message carries the control id"
    );
    assert_eq!(*msgs[0].index, 1, "the message carries B's index");

    // Segment entity ids are stable (mutate, not respawn).
    let segs_after = segments_of(&mut app, control);
    assert_eq!(
        segs_after.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        segs.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        "segment entity ids must be stable across the active change",
    );
}

/// AC — re-pressing the ALREADY-active segment neither repaints nor re-emits (the
/// `set_if_neq` no-op).
#[test]
fn segmented_control_repress_active_is_noop() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let segs = segments_of(&mut app, control);
    let seg_a = segs[0].0; // already active
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(seg_a) {
        *interaction = Interaction::Pressed;
    }
    app.update();

    let mut state: SystemState<MessageReader<SegmentSelected>> = SystemState::new(app.world_mut());
    let reader_result = state.get_mut(app.world_mut());
    assert!(reader_result.is_ok(), "MessageReader params must validate");
    let Ok(mut reader) = reader_result else {
        return;
    };
    let count = reader.read().count();
    assert_eq!(
        count, 0,
        "re-pressing the active segment emits no SegmentSelected"
    );
}

/// AC — color overrides are honored: a non-default remaining/lost pair shows up on
/// the bar and pips (already exercised in their own test modules), and a custom
/// segment palette shows up on the segments. This test pins the segment palette
/// override end-to-end through a fresh active selection.
#[test]
fn color_overrides_are_honored() {
    let mut app = harness();
    // A deliberately unusual palette, all distinct.
    let palette = SegmentColors {
        active_bg:   Color::srgb(0.9, 0.1, 0.5),
        active_text: Color::srgb(0.0, 0.0, 0.0),
        base_bg:     Color::srgb(0.05, 0.05, 0.4),
        base_text:   Color::srgb(0.7, 0.7, 0.1),
    };
    let labels = [SegmentLabel::new("X"), SegmentLabel::new("Y")];
    let control = {
        let mut commands = app.world_mut().commands();
        spawn_segmented_control(
            &mut commands,
            &labels,
            0,
            palette,
            Orientation::Horizontal,
            (),
        )
    };
    app.world_mut().flush();
    app.update();

    let segs = segments_of(&mut app, control);
    let (active_bg, _, active_text) = segment_look(&mut app, segs[0].0);
    let (base_bg, _, base_text) = segment_look(&mut app, segs[1].0);
    assert_eq!(active_bg, palette.active_bg, "active bg override honored");
    assert_eq!(
        active_text, palette.active_text,
        "active text override honored"
    );
    assert_eq!(base_bg, palette.base_bg, "base bg override honored");
    assert_eq!(base_text, palette.base_text, "base text override honored");
}

/// GTW-277 (screenshot review V1/V2): a `SegmentedControl` segment IS a
/// [`Button`](bevy::ui::widget::Button), so the generic button-interaction painters
/// ([`theme_interaction`](crate::interaction::theme_interaction) and
/// [`repaint_deactivated_buttons`](crate::interaction::repaint_deactivated_buttons)) used to
/// CLOBBER the active segment's [`SegmentColors::active_bg`] highlight with the theme's resting
/// button fill the frame the segment's [`Interaction`](bevy::ui::Interaction) changed — so the
/// active segment never read as selected on screen. The fix EXCLUDES `Segment` from those
/// painters (`Without<Segment>`), so the segment background comes ONLY from
/// [`repaint_segments`](super::super::repaint_segments).
///
/// This test pins it on the REAL code path: it inserts a `GdtfTheme` (so the interaction
/// painters actually run — they early-return when the resource is absent, as in the other HUD
/// tests), then changes the ACTIVE segment's `Interaction` (triggering `theme_interaction`'s
/// `Changed<Interaction>`), and asserts the active segment KEEPS its `active_bg` rather than
/// being repainted to `theme.button.color`. Without the exclusion this is RED (the active
/// segment goes resting-gray); with it, GREEN.
#[test]
fn segment_active_highlight_survives_theme_interaction() {
    use crate::theme::{GdtfTheme, default_theme};

    let mut app = harness();
    // Insert a theme so the generic interaction painters run (they guard
    // `Option<Res<GdtfTheme>>` and are inert without it — the same reason the other HUD
    // tests don't see them). The default theme's resting button fill is what WOULD clobber
    // the segment if it were not excluded.
    app.insert_resource::<GdtfTheme>(default_theme());
    let control = spawn_fire_mode(&mut app);
    // Settle: spawn paint + apply_theme + the first interaction pass.
    app.update();
    app.update();

    let segs = segments_of(&mut app, control);
    assert_eq!(segs.len(), 3, "must have 3 segments");
    let seg_active = segs[0].0; // Single — active at spawn.

    // Precondition: the active segment carries the active fill, NOT the theme's resting fill.
    let (bg0, ..) = segment_look(&mut app, seg_active);
    assert_eq!(
        bg0, SEG_COLORS.active_bg,
        "precondition: the active segment starts with the active fill",
    );
    let resting = {
        let theme = app.world().resource::<GdtfTheme>();
        *theme.button.color
    };
    assert_ne!(
        SEG_COLORS.active_bg, resting,
        "test is only meaningful if the active fill differs from the theme resting fill",
    );

    // Force a `Changed<Interaction>` on the ACTIVE segment (what a hover/click would do) —
    // this is exactly what would invite `theme_interaction` to repaint it to the resting fill.
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(seg_active) {
        *interaction = Interaction::Hovered;
    }
    app.update();
    // And back to None (a second `Changed<Interaction>` that maps to the resting fill).
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(seg_active) {
        *interaction = Interaction::None;
    }
    app.update();

    // The active segment STILL shows its active fill — the interaction painters left it alone.
    let (bg_after, ..) = segment_look(&mut app, seg_active);
    assert_eq!(
        bg_after, SEG_COLORS.active_bg,
        "the active segment must keep its active fill (not the theme resting fill) across an \
         Interaction change — the segment fill is owned by repaint_segments, not the generic \
         button-interaction painters",
    );
    assert_ne!(
        bg_after, resting,
        "the active segment must NOT be clobbered to the theme's resting button fill",
    );
}
