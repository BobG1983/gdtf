use bevy::{ecs::system::SystemState, prelude::*, text::FontWeight, ui::Interaction};

use super::{
    super::{ActiveSegment, SegmentColors, SegmentLabel, SegmentSelected, spawn_segmented_control},
    support::{SEG_COLORS, segment_look, segments_of, spawn_fire_mode},
};
use crate::widgets::core::{Orientation, test_support::harness};

#[test]
fn segmented_control_active_change_repaints_all_one_update() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let segs = segments_of(&mut app, control);
    assert_eq!(segs.len(), 3, "must have 3 segments");
    let seg_a = segs[0].0; 
    let seg_b = segs[1].0; 

    let (a_bg0, a_w0, _) = segment_look(&mut app, seg_a);
    assert_eq!(a_bg0, SEG_COLORS.active_bg, "precondition: A starts active");
    assert_eq!(a_w0, FontWeight::BOLD, "precondition: A starts bold");
    assert_eq!(
        app.world().get::<Interaction>(seg_a).copied(),
        Some(Interaction::None),
        "precondition: A is not hovered",
    );

    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(seg_b) {
        *interaction = Interaction::Pressed;
    }
    app.update();

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

    let segs_after = segments_of(&mut app, control);
    assert_eq!(
        segs_after.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        segs.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        "segment entity ids must be stable across the active change",
    );
}

#[test]
fn segmented_control_repress_active_is_noop() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let segs = segments_of(&mut app, control);
    let seg_a = segs[0].0; 
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

#[test]
fn color_overrides_are_honored() {
    let mut app = harness();
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

#[test]
fn segment_active_highlight_survives_theme_interaction() {
    use crate::theme::{GdtfTheme, default_theme};

    let mut app = harness();
    app.insert_resource::<GdtfTheme>(default_theme());
    let control = spawn_fire_mode(&mut app);
    app.update();
    app.update();

    let segs = segments_of(&mut app, control);
    assert_eq!(segs.len(), 3, "must have 3 segments");
    let seg_active = segs[0].0; 

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

    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(seg_active) {
        *interaction = Interaction::Hovered;
    }
    app.update();
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(seg_active) {
        *interaction = Interaction::None;
    }
    app.update();

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
