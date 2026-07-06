//! The fire-mode control's construction: the Mode sub-panel + segmented-control spawn,
//! its shared palette, and the post-flush segment tagging / flex sizing. Split out of
//! the monolithic `mode_panel.rs` (GTW-583); the control rationale lives on the parent
//! `mode_panel` module.

use bevy::prelude::*;
use gdtf_battle_sim::weapon::ModeKind;
use gdtf_ui::{
    Orientation, Segment, SegmentColors, SegmentIndex, SegmentLabel, spawn_segmented_control,
    theme::GdtfTheme,
};

use super::{
    super::stance_panel::control_segment_colors,
    order::{MODE_ORDER, mode_for_index, mode_index, mode_label},
};
use crate::states::running::game::battlescape::action_bar::components::{
    ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
};

/// Spawns the Mode sub-panel ([`ModePanelRoot`]) holding ONE horizontal
/// [`SegmentedControl`](gdtf_ui::SegmentedControl) of three fire-mode segments (Single / Burst / Full-Auto), and
/// returns the panel [`Entity`] so the caller can parent it under the bottom-left grid cell
/// (GTW-265 / GTW-277 / GTW-284).
///
/// A themed [`spawn_panel`](gdtf_ui::spawn_panel) (`Themed(Panel)`, re-painted by
/// `apply_theme`) laid out as a full-size row that CLIPS its content (so a wide caption
/// never overflows into a sibling cell — GTW-298 item 8), holding the segmented control.
/// The control's root carries the [`ModeControl`] identity marker (so the
/// [`SegmentSelected`](gdtf_ui::SegmentSelected) listener maps a select to a fire mode);
/// each segment carries its per-mode marker ([`ModeSingleButton`] / [`ModeBurstButton`] /
/// [`ModeFullButton`]) — tagged once the segments exist ([`tag_mode_segments`]). The panel
/// (and every segment) starts [`Visibility::Hidden`] / collapsed until
/// [`rebuild_mode_segments`](super::visibility::rebuild_mode_segments) reveals exactly the offered modes (GTW-273). Takes
/// `&mut Commands` + the live theme.
pub(in crate::states::running::game::battlescape) fn spawn_mode_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    let panel = gdtf_ui::spawn_panel(commands, theme);
    commands.entity(panel).insert((
        ModePanelRoot,
        Node {
            // GTW-298: the Firemode panel FILLS its bottom-left grid cell; its 1–3 visible
            // segments sit side by side in a ROW (the control itself is the row). Clip any
            // segment wider than its share so the row never overflows the cell.
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            overflow: bevy::ui::Overflow {
                x: bevy::ui::OverflowAxis::Hidden,
                y: bevy::ui::OverflowAxis::Hidden,
            },
            ..default()
        },
        // GTW-273: HIDDEN until an armed selection with modes reveals the offered segments.
        Visibility::Hidden,
    ));

    let labels: Vec<SegmentLabel> = MODE_ORDER
        .iter()
        .map(|k| SegmentLabel::new(mode_label(*k)))
        .collect();
    let control = spawn_segmented_control(
        commands,
        &labels,
        mode_index(ModeKind::Single),
        mode_segment_colors(theme),
        Orientation::Horizontal,
        ModeControl,
    );
    // FILL the panel cell (GTW-298: width varies by visible count, height fills the panel —
    // each segment's flex share). MUTATE only width/height — a wholesale `insert(Node {
    // ..default() })` would DROP the widget's connected-look fields (the root's rounded
    // `border_radius` + `Overflow::clip`, the zero inter-segment gap, the Row direction),
    // re-breaking the offered segments into loose boxes (the V2 defect). The widget already
    // lays out as a zero-gap, clipped, rounded Row; we only re-size it to fill the cell.
    commands
        .entity(control)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = Val::Percent(100.0);
            node.height = Val::Percent(100.0);
        });
    commands.entity(panel).add_children(&[control]);
    panel
}

/// The active/base color palette the Mode [`SegmentedControl`](gdtf_ui::SegmentedControl) paints with — the SAME
/// theme-derived palette the Stance control uses (shared [`control_segment_colors`]).
fn mode_segment_colors(theme: &GdtfTheme) -> SegmentColors {
    control_segment_colors(theme)
}

/// Read-write [`Query`] data for one freshly-spawned Mode segment to be tagged + flex-sized:
/// its [`Entity`], its [`SegmentIndex`](gdtf_ui::SegmentIndex), and its [`Node`] (to set its
/// even flex share so the three firemode segments fit the narrow cell — V1/V4 fix).
///
/// Named to keep [`tag_mode_segments`]'s signature legible (clippy `type_complexity`).
type NewModeSegment = (Entity, &'static SegmentIndex, &'static mut Node);

/// Tags each Mode [`SegmentedControl`](gdtf_ui::SegmentedControl) segment with its per-mode
/// marker ([`ModeSingleButton`] / [`ModeBurstButton`] / [`ModeFullButton`]) AND sizes it to a
/// flex-EVEN share of the firemode row, once the control's segment children exist (GTW-277).
///
/// `spawn_segmented_control` spawns the segments via the command buffer, so they do not
/// exist until that flush — the per-mode markers + the segment flex sizing cannot be attached
/// synchronously in [`spawn_mode_panel`]. This system runs on the spawn frame (gated
/// `Added<`[`ModeControl`]`>`), finds the Mode control, walks its [`Children`], gives each
/// segment an even flex share (`flex_grow: 1` + `flex_basis: 0` + `min_width: 0` +
/// `overflow: Hidden` — so three segments fit the SHORT/NARROW firemode cell and a too-wide
/// caption clips WITHIN its segment, never overflowing the cell — the V1/V4 clip fix), and
/// inserts the marker for each segment's [`SegmentIndex`](gdtf_ui::SegmentIndex) ([`MODE_ORDER`]
/// maps index → mode). The segments are spawned once (GTW-284), so it runs exactly once and
/// never churns anything (it inserts a unit marker + sizes the existing segment).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`], an `Added<ModeControl>` detector with the
/// control's [`Children`], and a `Query<(Entity, &SegmentIndex, &mut Node), With<Segment>>`
/// over the freshly-spawned segments — no `&mut World`.
pub(in crate::states::running::game::battlescape) fn tag_mode_segments(
    mut commands: Commands,
    controls: Query<&Children, (With<ModeControl>, Added<ModeControl>)>,
    mut segments: Query<NewModeSegment, With<Segment>>,
) {
    for children in &controls {
        for &child in children {
            let Ok((segment, index, mut node)) = segments.get_mut(child) else {
                continue;
            };
            // V1/V4 fix (GTW-277 screenshot review): the firemode control sits in a SHORT,
            // NARROW (bottom 1/4) cell where three content-sized segments + the panel inset
            // overflowed, CLIPPING the third ("auto") at the cell's right edge. Make each
            // segment flex-SHARE the row evenly (`flex_grow: 1` + `flex_basis: 0` +
            // `min_width: 0`) so all three always fit the cell width — and clip a too-wide
            // caption WITHIN its own segment (`overflow: Hidden`) rather than overflowing the
            // row. The three even segments read as a connected segmented control (the mockup),
            // not three loosely-sized boxes.
            node.flex_grow = 1.0;
            node.flex_basis = Val::ZERO;
            node.min_width = Val::ZERO;
            // GTW-303 clip fix (2026-06-19): the firemode segments are now TWO lines (the mode
            // name over its `"{n} TU"` cost sub-line). Without `min_height: 0`, a segment's flex
            // MIN height is its intrinsic two-line content height, which can exceed the short
            // firemode cell's share — forcing the segment taller than the cell and clipping the
            // cost line at the cell's bottom edge. Letting the segment shrink below its content
            // min (`min_height: 0`) keeps it within the cell; the cell is sized (the larger
            // `BOTTOM_CELL_PCT` band over the taller bottom bar) to fit both lines.
            node.min_height = Val::ZERO;
            node.overflow = bevy::ui::Overflow {
                x: bevy::ui::OverflowAxis::Hidden,
                y: bevy::ui::OverflowAxis::Hidden,
            };
            match mode_for_index(**index) {
                Some(ModeKind::Single) => {
                    commands.entity(segment).insert(ModeSingleButton);
                }
                Some(ModeKind::Burst) => {
                    commands.entity(segment).insert(ModeBurstButton);
                }
                Some(ModeKind::Full) => {
                    commands.entity(segment).insert(ModeFullButton);
                }
                None => {}
            }
        }
    }
}
