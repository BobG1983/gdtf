//! Tests for the GTW-276 generic HUD widgets: [`ProgressBar`](super::spawn_progress_bar),
//! [`Pips`](super::spawn_pips), [`Switch`](super::Switch), and
//! [`SegmentedControl`](super::SegmentedControl).
//!
//! All run on the established in-crate harness (`MinimalPlugins` + `InputPlugin` +
//! `UiPlugin`) — `gdtf_ui` cannot depend on `gdtf_test_utils` (cycle). Each test is
//! pin-discriminating: it asserts MUTATE-in-place (stable entity ids across an
//! update), the active-driven repaint-all, the flip/select messages, both
//! orientations, and color overrides.

use bevy::{
    MinimalPlugins,
    ecs::system::SystemState,
    input::InputPlugin,
    prelude::*,
    text::{FontWeight, TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, Interaction, Node, Val},
};

use super::{
    ActiveSegment, FillFraction, FilledPips, Orientation, Pip, ProgressBarFill, Segment,
    SegmentColors, SegmentIndex, SegmentLabel, SegmentSelected, SegmentText, SwitchColors,
    SwitchKnob, SwitchState, ToggleFlipped, set_pips, set_progress_bar, spawn_pips,
    spawn_progress_bar, spawn_segmented_control, spawn_switch,
};
use crate::UiPlugin;

/// A test color tuple, kept distinct per role so asserts discriminate.
const REMAINING: Color = Color::srgb(0.2, 0.8, 0.2);
/// A second distinct test color.
const LOST: Color = Color::srgb(0.8, 0.2, 0.2);

/// The two-query [`SystemState`] driving [`set_progress_bar`] in tests (clippy
/// `type_complexity`).
type ProgressBarSet = (
    Query<'static, 'static, &'static Children>,
    Query<'static, 'static, &'static mut Node, With<ProgressBarFill>>,
);

/// The two-query [`SystemState`] driving [`set_pips`] in tests (clippy
/// `type_complexity`).
type PipsSet = (
    Query<'static, 'static, &'static Children>,
    Query<'static, 'static, &'static mut BackgroundColor, With<Pip>>,
);

/// Builds the minimal harness used by every HUD-widget test: `MinimalPlugins`
/// (schedules + time), `InputPlugin` (so `Interaction` plumbing is sane), and
/// `UiPlugin` (which registers the GTW-276 driver systems + message buffers).
fn harness() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(InputPlugin)
        .add_plugins(UiPlugin);
    app
}

// ---- ProgressBar ----------------------------------------------------------

/// The fill child of the bar rooted at `track`, if any.
fn fill_of(app: &mut App, track: Entity) -> Option<Entity> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let children = state.get(app.world());
    children
        .get(track)
        .ok()?
        .iter()
        .find(|&child| app.world().get::<ProgressBarFill>(child).is_some())
}

/// Drives [`set_progress_bar`] once against the live world's queries.
fn drive_set_progress_bar(app: &mut App, track: Entity, fraction: FillFraction) -> bool {
    let mut state: SystemState<ProgressBarSet> = SystemState::new(app.world_mut());
    let (children, mut fills) = state.get_mut(app.world_mut());
    let ok = set_progress_bar(track, fraction, &children, &mut fills);
    state.apply(app.world_mut());
    ok
}

/// AC — a `ProgressBar` renders its fill at the requested fraction width, and the
/// `lost` color is on the track while `remaining` is on the fill.
///
/// Pin-discriminating: a wrong fraction → percent mapping, or a swapped track/fill
/// color, fails an assert (the two colors are distinct).
#[test]
fn progress_bar_renders_at_fraction() {
    let mut app = harness();
    let track = {
        let mut commands = app.world_mut().commands();
        spawn_progress_bar(
            &mut commands,
            FillFraction::from_ratio(20.0, 60.0),
            REMAINING,
            LOST,
            (),
        )
    };
    app.world_mut().flush();

    assert_eq!(
        app.world().get::<BackgroundColor>(track).map(|c| c.0),
        Some(LOST),
        "the track must carry the lost color",
    );
    let maybe_fill = fill_of(&mut app, track);
    assert!(maybe_fill.is_some(), "bar must have a fill child");
    let Some(fill) = maybe_fill else { return };
    assert_eq!(
        app.world().get::<BackgroundColor>(fill).map(|c| c.0),
        Some(REMAINING),
        "the fill must carry the remaining color",
    );
    assert_eq!(
        app.world().get::<Node>(fill).map(|n| n.width),
        Some(Val::Percent(20.0 / 60.0 * 100.0)),
        "the fill width must be the value fraction as a percent",
    );
}

/// AC — updating the value MUTATES the SAME fill entity to the new width (a respawn
/// would change the fill entity id).
///
/// Pin-discriminating: capture the fill id, update, re-find the fill, assert the id
/// is unchanged AND the width changed.
#[test]
fn progress_bar_update_mutates_same_fill_entity() {
    let mut app = harness();
    let track = {
        let mut commands = app.world_mut().commands();
        spawn_progress_bar(&mut commands, FillFraction::new(1.0), REMAINING, LOST, ())
    };
    app.world_mut().flush();

    let maybe_before = fill_of(&mut app, track);
    assert!(maybe_before.is_some(), "bar must have a fill child");
    let Some(fill_before) = maybe_before else {
        return;
    };
    assert_eq!(
        app.world().get::<Node>(fill_before).map(|n| n.width),
        Some(Val::Percent(100.0)),
        "precondition: starts full",
    );

    let ok = drive_set_progress_bar(&mut app, track, FillFraction::from_ratio(1.0, 4.0));
    assert!(ok, "set_progress_bar must find the fill child");

    let maybe_after = fill_of(&mut app, track);
    assert!(maybe_after.is_some(), "bar must still have a fill child");
    let Some(fill_after) = maybe_after else {
        return;
    };
    assert_eq!(
        fill_after, fill_before,
        "the fill entity id must be STABLE across the update (mutate, not respawn)",
    );
    assert_eq!(
        app.world().get::<Node>(fill_after).map(|n| n.width),
        Some(Val::Percent(25.0)),
        "the fill width must reflect the updated fraction",
    );
}

// ---- Pips -----------------------------------------------------------------

/// All pip entities of the row rooted at `row`, in child order.
fn pips_of(app: &mut App, row: Entity) -> Vec<Entity> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let children = state.get(app.world());
    let Ok(kids) = children.get(row) else {
        return Vec::new();
    };
    kids.iter()
        .filter(|&c| app.world().get::<Pip>(c).is_some())
        .collect()
}

/// Drives [`set_pips`] once against the live world's queries.
fn drive_set_pips(
    app: &mut App,
    row: Entity,
    filled: FilledPips,
    remaining: Color,
    lost: Color,
) -> usize {
    let mut state: SystemState<PipsSet> = SystemState::new(app.world_mut());
    let (children, mut pips) = state.get_mut(app.world_mut());
    let n = set_pips(row, filled, remaining, lost, &children, &mut pips);
    state.apply(app.world_mut());
    n
}

/// AC — M-of-N pips carry the remaining vs lost color split.
#[test]
fn pips_render_m_of_n_split() {
    let mut app = harness();
    let row = {
        let mut commands = app.world_mut().commands();
        spawn_pips(&mut commands, 3, FilledPips::new(2), REMAINING, LOST, ())
    };
    app.world_mut().flush();

    let pips = pips_of(&mut app, row);
    assert_eq!(pips.len(), 3, "row must spawn exactly N pips");
    let colors: Vec<Color> = pips
        .iter()
        .map(|&p| app.world().get::<BackgroundColor>(p).map_or(LOST, |c| c.0))
        .collect();
    assert_eq!(
        colors,
        vec![REMAINING, REMAINING, LOST],
        "the first M pips are remaining, the rest lost",
    );
}

/// AC — updating M mutates the pip colors IN PLACE (stable pip entity ids).
#[test]
fn pips_update_mutates_same_entities() {
    let mut app = harness();
    let row = {
        let mut commands = app.world_mut().commands();
        spawn_pips(&mut commands, 3, FilledPips::new(0), REMAINING, LOST, ())
    };
    app.world_mut().flush();

    let before = pips_of(&mut app, row);
    assert_eq!(before.len(), 3);

    let n = drive_set_pips(&mut app, row, FilledPips::new(2), REMAINING, LOST);
    assert_eq!(n, 3, "set_pips must re-color every pip");

    let after = pips_of(&mut app, row);
    assert_eq!(
        after, before,
        "pip entity ids must be STABLE across the update (mutate, not respawn)",
    );
    let colors: Vec<Color> = after
        .iter()
        .map(|&p| app.world().get::<BackgroundColor>(p).map_or(LOST, |c| c.0))
        .collect();
    assert_eq!(
        colors,
        vec![REMAINING, REMAINING, LOST],
        "the new M-of-N split must be reflected after the update",
    );
}

// ---- Switch ---------------------------------------------------------------

/// A caller-attached identity marker on a switch, used to confirm the flip message
/// maps back to the right widget.
#[derive(Component, Clone, Copy)]
struct AimToggle;

/// The knob child of the switch rooted at `switch`, if any.
fn knob_of(app: &mut App, switch: Entity) -> Option<Entity> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let children = state.get(app.world());
    children
        .get(switch)
        .ok()?
        .iter()
        .find(|&c| app.world().get::<SwitchKnob>(c).is_some())
}

/// Presses the switch (sets `Interaction::Pressed`) and runs one update so
/// `drive_switches` fires.
fn click_switch(app: &mut App, switch: Entity) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(switch) {
        *interaction = Interaction::Pressed;
    }
    app.update();
}

/// AC — a click flips the state, emits `ToggleFlipped` with the right identity, and
/// the TRACK color reflects the new state while the knob color is unchanged. The
/// switch entity is stable.
#[test]
fn switch_click_flips_emits_and_recolors_track() {
    let mut app = harness();
    let track_off = Color::srgb(0.1, 0.1, 0.1);
    let track_on = Color::srgb(0.3, 0.7, 0.3);
    let knob_color = Color::WHITE;
    let switch = {
        let mut commands = app.world_mut().commands();
        spawn_switch(
            &mut commands,
            SwitchState::Off,
            SwitchColors {
                off:  track_off,
                on:   track_on,
                knob: knob_color,
            },
            Orientation::Horizontal,
            AimToggle,
        )
    };
    app.world_mut().flush();

    let maybe_knob = knob_of(&mut app, switch);
    assert!(maybe_knob.is_some(), "switch must have a knob child");
    let Some(knob) = maybe_knob else { return };
    assert_eq!(
        app.world().get::<BackgroundColor>(switch).map(|c| c.0),
        Some(track_off),
        "precondition: starts in the off track color",
    );

    click_switch(&mut app, switch);

    // State flipped to On.
    assert_eq!(
        app.world().get::<SwitchState>(switch).copied(),
        Some(SwitchState::On),
        "the click must flip the stored state to On",
    );
    // The track recolored to the on color; the knob color is unchanged.
    assert_eq!(
        app.world().get::<BackgroundColor>(switch).map(|c| c.0),
        Some(track_on),
        "the TRACK must take the on color after the flip",
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(knob).map(|c| c.0),
        Some(knob_color),
        "the KNOB color must be unchanged by the flip",
    );
    // The flip message carries this switch's identity, and the entity is stable.
    let mut state: SystemState<MessageReader<ToggleFlipped>> = SystemState::new(app.world_mut());
    let mut reader = state.get_mut(app.world_mut());
    let msgs: Vec<ToggleFlipped> = reader.read().copied().collect();
    assert_eq!(msgs.len(), 1, "exactly one ToggleFlipped per click");
    assert_eq!(msgs[0].switch, switch, "the message carries the switch id");
    assert_eq!(
        msgs[0].state,
        SwitchState::On,
        "the message carries the new state"
    );
    assert!(
        app.world().get::<AimToggle>(switch).is_some(),
        "the switch entity (with its caller marker) is stable across the flip",
    );
}

/// AC — both orientations lay out: a horizontal switch is wider than tall, a vertical
/// switch is taller than wide.
#[test]
fn switch_supports_both_orientations() {
    let mut app = harness();
    let colors = SwitchColors {
        off:  LOST,
        on:   REMAINING,
        knob: Color::WHITE,
    };
    let (h, v) = {
        let mut commands = app.world_mut().commands();
        let h = spawn_switch(
            &mut commands,
            SwitchState::Off,
            colors,
            Orientation::Horizontal,
            (),
        );
        let v = spawn_switch(
            &mut commands,
            SwitchState::Off,
            colors,
            Orientation::Vertical,
            (),
        );
        (h, v)
    };
    app.world_mut().flush();

    let maybe_h = app.world().get::<Node>(h).cloned();
    let maybe_v = app.world().get::<Node>(v).cloned();
    assert!(
        maybe_h.is_some() && maybe_v.is_some(),
        "both switches have a Node"
    );
    let (Some(hn), Some(vn)) = (maybe_h, maybe_v) else {
        return;
    };
    // The two orientations swap width/height, so the horizontal track is the
    // mirror of the vertical track.
    assert_eq!(hn.width, vn.height, "horizontal width == vertical height");
    assert_eq!(hn.height, vn.width, "horizontal height == vertical width");
    assert_ne!(hn.width, hn.height, "a switch track is not square");
}

// ---- SegmentedControl -----------------------------------------------------

/// A caller-attached identity marker on a segmented control.
#[derive(Component, Clone, Copy)]
struct FireMode;

/// The segments of the control rooted at `control`, ordered by child order, as
/// `(entity, index)`.
fn segments_of(app: &mut App, control: Entity) -> Vec<(Entity, usize)> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let children = state.get(app.world());
    let Ok(kids) = children.get(control) else {
        return Vec::new();
    };
    kids.iter()
        .filter_map(|c| {
            app.world()
                .get::<Segment>(c)
                .and(app.world().get::<SegmentIndex>(c))
                .map(|idx| (c, **idx))
        })
        .collect()
}

/// The (background color, label weight, label color) of a segment entity.
fn segment_look(app: &mut App, segment: Entity) -> (Color, FontWeight, Color) {
    let bg = app
        .world()
        .get::<BackgroundColor>(segment)
        .map_or(Color::NONE, |c| c.0);
    // Find the label text child.
    let label = {
        let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
        let children = state.get(app.world());
        children.get(segment).ok().and_then(|kids| {
            kids.iter()
                .find(|&c| app.world().get::<SegmentText>(c).is_some())
        })
    };
    let (weight, color) = label.map_or((FontWeight::NORMAL, Color::NONE), |l| {
        let w = app
            .world()
            .get::<TextFont>(l)
            .map_or(FontWeight::NORMAL, |f| f.weight);
        let c = app
            .world()
            .get::<UiTextColor>(l)
            .map_or(Color::NONE, |c| c.0);
        (w, c)
    });
    (bg, weight, color)
}

/// A four-color segment palette with all four colors distinct so asserts discriminate.
const SEG_COLORS: SegmentColors = SegmentColors {
    active_bg:   Color::srgb(0.2, 0.7, 0.2),
    active_text: Color::srgb(1.0, 1.0, 1.0),
    base_bg:     Color::srgb(0.1, 0.1, 0.1),
    base_text:   Color::srgb(0.5, 0.5, 0.5),
};

/// Builds a 3-segment fire-mode control with segment 0 (`Single`) active.
fn spawn_fire_mode(app: &mut App) -> Entity {
    let labels = [
        SegmentLabel::new("Single"),
        SegmentLabel::new("Burst"),
        SegmentLabel::new("Full-Auto"),
    ];
    let control = {
        let mut commands = app.world_mut().commands();
        spawn_segmented_control(
            &mut commands,
            &labels,
            0,
            SEG_COLORS,
            Orientation::Horizontal,
            FireMode,
        )
    };
    app.world_mut().flush();
    control
}

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
    let mut reader = state.get_mut(app.world_mut());
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

/// AC — both Row and Column orientations lay out (the flex-direction differs).
#[test]
fn segmented_control_supports_row_and_column() {
    let mut app = harness();
    let labels = [SegmentLabel::new("A"), SegmentLabel::new("B")];
    let (row, col) = {
        let mut commands = app.world_mut().commands();
        let row = spawn_segmented_control(
            &mut commands,
            &labels,
            0,
            SEG_COLORS,
            Orientation::Horizontal,
            (),
        );
        let col = spawn_segmented_control(
            &mut commands,
            &labels,
            0,
            SEG_COLORS,
            Orientation::Vertical,
            (),
        );
        (row, col)
    };
    app.world_mut().flush();

    assert_eq!(
        app.world().get::<Node>(row).map(|n| n.flex_direction),
        Some(FlexDirection::Row),
        "horizontal orientation lays out as a Row",
    );
    assert_eq!(
        app.world().get::<Node>(col).map(|n| n.flex_direction),
        Some(FlexDirection::Column),
        "vertical orientation lays out as a Column",
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
    let mut reader = state.get_mut(app.world_mut());
    let count = reader.read().count();
    assert_eq!(
        count, 0,
        "re-pressing the active segment emits no SegmentSelected"
    );
}

/// AC — color overrides are honored: a non-default remaining/lost pair shows up on
/// the bar and pips (already exercised above with REMAINING/LOST), and a custom
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
