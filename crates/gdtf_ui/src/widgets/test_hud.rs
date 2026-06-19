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
    ui::{BackgroundColor, Display, Interaction, Node, Val},
};

use super::{
    ActiveSegment, FillFraction, FilledPips, Orientation, Pip, ProgressBarFill, Segment,
    SegmentColors, SegmentIndex, SegmentLabel, SegmentSelected, SegmentSubLabel, SegmentSubText,
    SegmentText, SwitchColors, SwitchKnob, SwitchState, ToggleFlipped, set_pips, set_progress_bar,
    set_segment_sub_line, set_segment_visible, spawn_pips, spawn_progress_bar,
    spawn_segmented_control, spawn_switch,
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

/// The `Vw` magnitude of a [`Val`], for proportion comparisons. Returns `None`
/// for any non-`Vw` unit so a unit regression fails the assert rather than
/// silently comparing across kinds.
fn vw(val: Val) -> Option<f32> {
    match val {
        Val::Vw(v) => Some(v),
        _ => None,
    }
}

/// AC (GTW pill fix) — the knob is a pip INSIDE a visible track, not a circle that
/// FILLS it: the track's SHORT dimension is strictly TALLER than the knob diameter
/// so track shows around the knob on the short axis, AND the track's LONG dimension
/// exceeds the knob so the knob has travel. All dims stay `Vw` (the orientation-swap
/// invariant). Pin-discriminating: shrinking the track short dim back to the knob
/// diameter (the original invisible-track bug) fails the first assert.
#[test]
fn switch_track_frames_the_knob() {
    let mut app = harness();
    let colors = SwitchColors {
        off:  LOST,
        on:   REMAINING,
        knob: Color::WHITE,
    };
    let switch = {
        let mut commands = app.world_mut().commands();
        spawn_switch(
            &mut commands,
            SwitchState::Off,
            colors,
            Orientation::Horizontal,
            (),
        )
    };
    app.world_mut().flush();

    let maybe_knob = knob_of(&mut app, switch);
    assert!(maybe_knob.is_some(), "switch must have a knob child");
    let Some(knob) = maybe_knob else { return };

    let track = app.world().get::<Node>(switch).cloned();
    let knob_node = app.world().get::<Node>(knob).cloned();
    let (Some(track), Some(knob_node)) = (track, knob_node) else {
        unreachable!("the track and knob both have a Node after flush");
    };

    // Horizontal: width is the LONG axis, height the SHORT axis.
    let (Some(long), Some(short)) = (vw(track.width), vw(track.height)) else {
        unreachable!("both track dims are Vw");
    };
    let (Some(knob_w), Some(knob_h)) = (vw(knob_node.width), vw(knob_node.height)) else {
        unreachable!("both knob dims are Vw");
    };
    assert!(
        short > knob_h,
        "the track SHORT dimension ({short}) must exceed the knob diameter ({knob_h}) \
         so track shows around the knob (the knob is a pip, not a fill)",
    );
    assert!(
        long > knob_w,
        "the track LONG dimension ({long}) must exceed the knob ({knob_w}) so the knob travels",
    );
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

/// GTW-277 (screenshot review V2): a [`SegmentedControl`] renders as ONE connected
/// container, NOT a stack of detached pills — the segments butt together with NO
/// inter-segment gap, the ROOT carries the rounded corners + clips its children (so the
/// inner segment corners are square and the whole control reads as a single rounded box),
/// and every segment AFTER the first carries a hairline divider on its leading edge (left
/// for a Row), while the first segment has no leading divider (its leading edge is the
/// control's outer edge).
///
/// Pin-discriminating: a non-zero column/row gap (the old detached-pill look), a missing
/// root radius/clip, or a leading divider on segment 0 / a missing divider on segment 1
/// each fails an assert. Asserts layout KIND (gap is zero, divider edge is non-zero), not
/// px magnitudes.
#[test]
fn segmented_control_reads_as_one_connected_control() {
    use bevy::ui::{BorderColor, Overflow};

    let mut app = harness();
    let control = spawn_fire_mode(&mut app);

    // The ROOT: no inter-segment gap, a rounded outer container, clipped children.
    let root_node = app.world().get::<Node>(control).cloned();
    assert!(root_node.is_some(), "control root must have a Node");
    let Some(root) = root_node else { return };
    assert_eq!(
        root.column_gap,
        Val::ZERO,
        "a connected control has NO inter-segment column gap (V2)",
    );
    assert_eq!(
        root.row_gap,
        Val::ZERO,
        "a connected control has NO inter-segment row gap (V2)",
    );
    assert_eq!(
        root.overflow,
        Overflow::clip(),
        "the root clips its children so the inner segment corners stay hidden (V2)",
    );
    assert_ne!(
        root.border_radius.top_left,
        Val::ZERO,
        "the root (the single container) carries the rounded corner (V2)",
    );

    // The segments: butt together (no per-segment rounding), divider on each leading edge
    // EXCEPT the first.
    let segments = segments_of(&mut app, control);
    assert_eq!(segments.len(), 3, "must have 3 segments");
    let first = segments[0].0;
    let second = segments[1].0;

    let first_node = app.world().get::<Node>(first).cloned();
    let second_node = app.world().get::<Node>(second).cloned();
    let (Some(first_n), Some(second_n)) = (first_node, second_node) else {
        return;
    };
    assert_eq!(
        first_n.border_radius.top_left,
        Val::ZERO,
        "segments carry NO per-segment rounding — the root owns the corners (V2)",
    );
    assert_eq!(
        first_n.border.left,
        Val::ZERO,
        "the FIRST segment has no leading divider (its leading edge is the outer edge)",
    );
    assert_ne!(
        second_n.border.left,
        Val::ZERO,
        "every segment after the first carries a leading-edge divider (the connected \
         control's adjacent dividers, V2)",
    );

    // The divider is the base_text color (a subtle hairline), painted once at spawn.
    let second_border = app.world().get::<BorderColor>(second).map(|b| b.left);
    assert_eq!(
        second_border,
        Some(SEG_COLORS.base_text),
        "the divider uses the base_text color",
    );
}

/// GTW-277 (V2): a VERTICAL segmented control's dividers run along the TOP edge (the
/// orientation-correct leading edge), not the left, so a stacked Stand/Kneel/Prone control
/// reads as one connected vertical control with horizontal dividers between rows.
///
/// Pin-discriminating: a vertical control whose divider is on the left (the horizontal
/// edge) instead of the top fails the assert.
#[test]
fn vertical_segmented_control_divides_on_the_top_edge() {
    let mut app = harness();
    let labels = [
        SegmentLabel::new("Stand"),
        SegmentLabel::new("Kneel"),
        SegmentLabel::new("Prone"),
    ];
    let control = {
        let mut commands = app.world_mut().commands();
        spawn_segmented_control(
            &mut commands,
            &labels,
            0,
            SEG_COLORS,
            Orientation::Vertical,
            (),
        )
    };
    app.world_mut().flush();

    let segs = segments_of(&mut app, control);
    assert_eq!(segs.len(), 3, "must have 3 segments");
    let n1 = app.world().get::<Node>(segs[1].0).cloned();
    let Some(node1) = n1 else { return };
    assert_ne!(
        node1.border.top,
        Val::ZERO,
        "a vertical control's divider runs along the TOP (leading) edge of each row (V2)",
    );
    assert_eq!(
        node1.border.left,
        Val::ZERO,
        "a vertical control's divider is NOT on the left/horizontal edge",
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

/// The `(SegmentIndex, &mut Node)` set [`SystemState`] driving [`set_segment_visible`]
/// in tests (clippy `type_complexity`).
type SegmentVisibilitySet = (
    Query<'static, 'static, &'static Children>,
    Query<'static, 'static, (&'static SegmentIndex, &'static mut Node), With<Segment>>,
);

/// Drives [`set_segment_visible`] once against the live world's queries.
fn drive_set_segment_visible(app: &mut App, control: Entity, index: usize, visible: bool) -> bool {
    let mut state: SystemState<SegmentVisibilitySet> = SystemState::new(app.world_mut());
    let (children, mut segments) = state.get_mut(app.world_mut());
    let ok = set_segment_visible(control, index, visible, &children, &mut segments);
    state.apply(app.world_mut());
    ok
}

/// The [`Display`] of a segment entity's [`Node`].
fn segment_display(app: &App, segment: Entity) -> Option<Display> {
    app.world().get::<Node>(segment).map(|n| n.display)
}

/// GTW-277 widget enhancement — per-segment visibility: [`set_segment_visible`] hides /
/// shows a single segment BY INDEX by toggling its [`Node::display`] (the row collapses a
/// hidden segment to nothing), MUTATING the existing segment in place — the segment entity
/// ids stay STABLE (the GTW-284 mutate-not-churn invariant a [`SegmentedControl`] used as
/// an offered-subset control needs).
///
/// Pin-discriminating: hiding segment 2 sets ONLY segment 2's display to `None` (0 and 1
/// stay `Flex`); re-showing it returns it to `Flex`; and the segment entity ids do NOT
/// change across the hide/show (a respawn would change them). An out-of-range index is a
/// no-op (returns `false`).
#[test]
fn set_segment_visible_hides_one_segment_keeping_stable_ids() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let before = segments_of(&mut app, control);
    assert_eq!(before.len(), 3, "must have 3 segments");
    let seg0 = before[0].0;
    let seg1 = before[1].0;
    let seg2 = before[2].0;

    // Precondition: all three segments start visible (Display::Flex).
    for seg in [seg0, seg1, seg2] {
        assert_eq!(
            segment_display(&app, seg),
            Some(Display::Flex),
            "every segment starts visible (Display::Flex)",
        );
    }

    // Hide segment 2 — ONLY it collapses; 0 and 1 stay visible.
    assert!(
        drive_set_segment_visible(&mut app, control, 2, false),
        "hiding an in-range segment must report a match",
    );
    assert_eq!(
        segment_display(&app, seg2),
        Some(Display::None),
        "the hidden segment collapses to Display::None",
    );
    assert_eq!(
        segment_display(&app, seg0),
        Some(Display::Flex),
        "segment 0 stays visible when a sibling is hidden",
    );
    assert_eq!(
        segment_display(&app, seg1),
        Some(Display::Flex),
        "segment 1 stays visible when a sibling is hidden",
    );

    // Re-show segment 2 — it returns to Display::Flex.
    assert!(drive_set_segment_visible(&mut app, control, 2, true));
    assert_eq!(
        segment_display(&app, seg2),
        Some(Display::Flex),
        "re-showing the segment returns it to Display::Flex",
    );

    // The segment entity ids are STABLE across the hide/show (mutate, not respawn).
    let after = segments_of(&mut app, control);
    assert_eq!(
        after.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        before.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        "segment entity ids must be stable across hide/show (mutate, never respawn)",
    );

    // An out-of-range index is an inert no-op.
    assert!(
        !drive_set_segment_visible(&mut app, control, 9, false),
        "an out-of-range index reports no match (no-op)",
    );
}

/// The sub-line ([`SegmentSubText`]) text child of a segment, if any, as
/// `(entity, content, font_size, color)`.
fn sub_line_of(app: &mut App, segment: Entity) -> Option<(Entity, String, f32, Color)> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let children = state.get(app.world());
    let node = children.get(segment).ok().and_then(|kids| {
        kids.iter()
            .find(|&c| app.world().get::<SegmentSubText>(c).is_some())
    })?;
    let content = app.world().get::<Text>(node).map(|t| t.0.clone())?;
    let size = app
        .world()
        .get::<TextFont>(node)
        .map_or(f32::NAN, |f| f.font_size);
    let color = app
        .world()
        .get::<UiTextColor>(node)
        .map_or(Color::NONE, |c| c.0);
    Some((node, content, size, color))
}

/// The label ([`SegmentText`]) font size of a segment, if any.
fn label_font_size(app: &mut App, segment: Entity) -> Option<f32> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let children = state.get(app.world());
    let label = children.get(segment).ok().and_then(|kids| {
        kids.iter()
            .find(|&c| app.world().get::<SegmentText>(c).is_some())
    })?;
    app.world().get::<TextFont>(label).map(|f| f.font_size)
}

/// The four-query [`SystemState`] driving [`set_segment_sub_line`] in tests (clippy
/// `type_complexity`).
type SubLineSet = (
    Commands<'static, 'static>,
    Query<'static, 'static, (&'static Children, &'static SegmentColors)>,
    Query<'static, 'static, (&'static SegmentIndex, &'static Children), With<Segment>>,
    Query<'static, 'static, &'static mut Text, With<SegmentSubText>>,
);

/// Drives [`set_segment_sub_line`] once against the live world's queries.
fn drive_set_segment_sub_line(
    app: &mut App,
    control: Entity,
    index: usize,
    sub_line: Option<SegmentSubLabel>,
) -> bool {
    let mut state: SystemState<SubLineSet> = SystemState::new(app.world_mut());
    let (mut commands, controls, segments, mut sub_texts) = state.get_mut(app.world_mut());
    let ok = set_segment_sub_line(
        &mut commands,
        control,
        index,
        sub_line.as_ref(),
        &controls,
        &segments,
        &mut sub_texts,
    );
    state.apply(app.world_mut());
    ok
}

/// GTW-303 — a segment spawned WITHOUT a sub-line renders as before: exactly ONE
/// [`SegmentText`] label child and NO [`SegmentSubText`] node (so the stance control + any
/// pre-GTW-303 caller are visually unchanged).
///
/// Pin-discriminating: if spawn were to attach a sub-line unconditionally, `sub_line_of`
/// would return `Some` and the assert fails.
#[test]
fn segment_without_sub_line_has_only_the_label() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let segs = segments_of(&mut app, control);
    assert_eq!(segs.len(), 3, "must have 3 segments");
    for (seg, _) in segs {
        // It HAS a label (the look helper finds it via SegmentText).
        let (_, weight, _) = segment_look(&mut app, seg);
        assert!(
            matches!(weight, FontWeight::NORMAL | FontWeight::BOLD),
            "every segment has a label text child",
        );
        // It has NO sub-line node.
        assert!(
            sub_line_of(&mut app, seg).is_none(),
            "a segment spawned without a sub-line has no SegmentSubText node",
        );
    }
}

/// GTW-303 — setting a segment's sub-line SPAWNS a [`SegmentSubText`] node carrying the
/// caption, at a SMALLER font than the label and a DIMMER color than the segment's base text.
///
/// Pin-discriminating: a sub-line at the same size as the label, or at the un-dimmed base
/// text color, fails the size / alpha asserts.
#[test]
fn segment_with_sub_line_has_both_texts_smaller_and_dimmer() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let segments = segments_of(&mut app, control);
    let burst = segments[1].0; // Burst (base, not active).

    assert!(
        drive_set_segment_sub_line(&mut app, control, 1, Some(SegmentSubLabel::new("3 TU")),),
        "setting an in-range segment's sub-line reports a match",
    );

    let label_size = label_font_size(&mut app, burst);
    assert_eq!(
        label_size,
        Some(16.0),
        "the LABEL keeps its own (larger) font size",
    );

    let sub = sub_line_of(&mut app, burst);
    assert!(sub.is_some(), "the segment now has a SegmentSubText node");
    let Some((_, content, size, color)) = sub else {
        return;
    };
    assert_eq!(content, "3 TU", "the sub-line carries the caption");
    assert!(
        size < 16.0,
        "the sub-line font ({size}) is SMALLER than the 16pt label",
    );
    // The sub-line is dimmer than the segment's base_text: same hue, reduced alpha.
    assert!(
        color.alpha() < SEG_COLORS.base_text.alpha(),
        "the sub-line color ({:?}) is DIMMER (lower alpha) than the base text ({:?})",
        color,
        SEG_COLORS.base_text,
    );
}

/// GTW-303 — updating a sub-line MUTATES the SAME node in place (stable id), and clearing it
/// (`None`) DESPAWNS the node — never respawning the segment ([[ui-mutate-not-respawn]]).
///
/// Pin-discriminating: capture the sub-line node id, update, assert id unchanged + new text;
/// then clear and assert the node is gone while the SEGMENT entity id is stable.
#[test]
fn segment_sub_line_set_update_clear_mutates_in_place() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let segs_before = segments_of(&mut app, control);
    let seg0 = segs_before[0].0;

    // Set.
    assert!(drive_set_segment_sub_line(
        &mut app,
        control,
        0,
        Some(SegmentSubLabel::new("3 TU")),
    ));
    let first = sub_line_of(&mut app, seg0);
    assert!(first.is_some(), "the sub-line node was spawned");
    let Some((node_before, text_before, ..)) = first else {
        return;
    };
    assert_eq!(text_before, "3 TU");

    // Update — same node id, new text.
    assert!(drive_set_segment_sub_line(
        &mut app,
        control,
        0,
        Some(SegmentSubLabel::new("5 TU")),
    ));
    let updated = sub_line_of(&mut app, seg0);
    let Some((node_after, text_after, ..)) = updated else {
        unreachable!("the sub-line node still exists after an update");
    };
    assert_eq!(
        node_after, node_before,
        "the sub-line node id is STABLE across an update (mutate, not respawn)",
    );
    assert_eq!(text_after, "5 TU", "the sub-line text is updated in place");

    // Clear — the node is despawned.
    assert!(drive_set_segment_sub_line(&mut app, control, 0, None));
    assert!(
        sub_line_of(&mut app, seg0).is_none(),
        "clearing the sub-line despawns its node",
    );

    // The SEGMENT entity ids are stable across the whole set/update/clear cycle.
    let segs_after = segments_of(&mut app, control);
    assert_eq!(
        segs_after.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        segs_before.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        "segment entity ids are stable across the sub-line lifecycle",
    );

    // An out-of-range index is an inert no-op.
    assert!(
        !drive_set_segment_sub_line(&mut app, control, 9, Some(SegmentSubLabel::new("x"))),
        "an out-of-range index reports no match (no-op)",
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

/// GTW-277 (screenshot review V1/V2): a [`SegmentedControl`] segment IS a
/// [`Button`](bevy::ui::widget::Button), so the generic button-interaction painters
/// ([`theme_interaction`](crate::interaction::theme_interaction) and
/// [`repaint_deactivated_buttons`](crate::interaction::repaint_deactivated_buttons)) used to
/// CLOBBER the active segment's [`SegmentColors::active_bg`] highlight with the theme's resting
/// button fill the frame the segment's [`Interaction`](bevy::ui::Interaction) changed — so the
/// active segment never read as selected on screen. The fix EXCLUDES `Segment` from those
/// painters (`Without<Segment>`), so the segment background comes ONLY from
/// [`repaint_segments`](super::repaint_segments).
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
