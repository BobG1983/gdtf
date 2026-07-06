//! The Stance control — a `gdtf_ui` vertical [`SegmentedControl`] — its constructor and
//! its sim-driven active-segment sync (GTW-267 / GTW-277 / GTW-298).
//!
//! GTW-277 migrated the Stance control from three ad-hoc toggle buttons to ONE generic
//! `gdtf_ui` [`SegmentedControl`] (3 vertical segments Stand / Kneel / Prone — the stance
//! control in the mockup: adjacent labeled segments, exactly one active). The press →
//! intent mapping ([`stance_segment_intent`](super::stance_active::stance_segment_intent))
//! reads the widget's [`SegmentSelected`](gdtf_ui::SegmentSelected) message; the active
//! mark is the widget's own [`ActiveSegment`](gdtf_ui::ActiveSegment) highlight, synced
//! FROM the selected ganger's [`Stance`](gdtf_battle_sim::ganger::Stance).
//!
//! [`spawn_stance_panel`] builds the Stance sub-panel: its OWN bordered
//! [`spawn_panel`](gdtf_ui::spawn_panel) box (`Themed(Panel)` — D-B, the same framed look
//! the weapon cluster's panels use, re-painted by `apply_theme`) tagged [`StancePanelRoot`],
//! holding ONE vertical [`SegmentedControl`] whose three segments carry the per-stance
//! markers ([`StanceStandingButton`] / [`StanceKneelingButton`] / [`StanceProneButton`]) so
//! the integration tests and the message-listener can name each stance.
//!
//! GTW-298 relocated the controls cluster: the Stance panel lives in the weapon-cluster's
//! Stance Panel (to the right of the Overall Weapon Panel), spawned by the weapon-panel
//! module through this shared constructor — NOT inside the action bar. The caller sizes it
//! (same HEIGHT as the Overall Weapon Panel, a fixed relative width) and parents it inside
//! the bar.
//!
//! GTW-284 (stable ids): the three segments are spawned ONCE (a Stance always offers all
//! three postures), and only the active highlight + the per-segment look are MUTATED — no
//! despawn/respawn ([[ui-mutate-not-respawn]]).

use bevy::{
    prelude::*,
    ui::{Node, UiRect, Val},
};
use gdtf_battle_sim::prelude::StanceKind;
use gdtf_ui::{
    Orientation, SegmentColors, SegmentLabel, spawn_panel, spawn_segmented_control,
    theme::GdtfTheme,
};

use crate::states::running::game::battlescape::action_bar::components::{
    StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton, StanceStandingButton,
};

/// The DISPLAYED stance label for a [`StanceKind`] in the Stance segmented control.
///
/// `Standing` → "Stand", `Crouching` → "Kneel" (the doc's "kneel" = `Crouching`),
/// `Prone` → "Prone". A presentation-only label, local to this control.
const fn stance_label(kind: StanceKind) -> &'static str {
    match kind {
        StanceKind::Standing => "Stand",
        StanceKind::Crouching => "Kneel",
        StanceKind::Prone => "Prone",
    }
}

/// The three stance segment indices, in DISPLAY (top-to-bottom) order: Stand / Kneel /
/// Prone. The index ↔ [`StanceKind`] mapping the press listener + the active-sync share.
pub(in crate::states::running::game::battlescape) const STANCE_ORDER: [StanceKind; 3] = [
    StanceKind::Standing,
    StanceKind::Crouching,
    StanceKind::Prone,
];

/// The segment INDEX of a [`StanceKind`] in [`STANCE_ORDER`] (the active-sync direction).
pub(in crate::states::running::game::battlescape) fn stance_index(kind: StanceKind) -> usize {
    STANCE_ORDER.iter().position(|k| *k == kind).unwrap_or(0)
}

/// The [`StanceKind`] of a segment INDEX in [`STANCE_ORDER`] (the press-listener
/// direction) — or [`None`] for an out-of-range index (defensive; the control has exactly
/// three segments).
pub(in crate::states::running::game::battlescape) fn stance_for_index(
    index: usize,
) -> Option<StanceKind> {
    STANCE_ORDER.get(index).copied()
}

/// The active/base color palette the Stance [`SegmentedControl`] paints with, derived from
/// the live theme so the control reads as part of the HUD (GTW-277).
///
/// Active = the theme's active / toggled-on button fill + the button text color (the same
/// "engaged" look a `paint_active_buttons` toggle had); base = the resting button fill +
/// text. The theme color newtypes [`Deref`] to [`Color`]. Shared with the Mode control.
pub(in crate::states::running::game::battlescape) fn control_segment_colors(
    theme: &GdtfTheme,
) -> SegmentColors {
    SegmentColors {
        active_bg:   *theme.button.active,
        active_text: *theme.button.text_color,
        base_bg:     *theme.button.color,
        base_text:   *theme.button.text_color,
    }
}

/// Spawns the Stance sub-panel ([`StancePanelRoot`]) — its OWN bordered `Themed(Panel)`
/// box (D-B) holding one vertical [`SegmentedControl`] of three stance segments
/// (Stand / Kneel / Prone) — and returns the panel [`Entity`] so a caller can parent +
/// size it inside the host layout (GTW-267 / GTW-277 / GTW-298 / D-B).
///
/// The panel keeps the framed-box look (`spawn_panel`, re-painted by `apply_theme`); the
/// inner control is a [`spawn_segmented_control`] tagged with the [`StanceControl`]
/// identity marker on its root (so the [`SegmentSelected`](gdtf_ui::SegmentSelected)
/// listener maps a select to a stance) and each segment carries its per-stance marker
/// ([`StanceStandingButton`] / [`StanceKneelingButton`] / [`StanceProneButton`]) so the
/// integration tests find each by meaning. The control starts with `Standing` active; the
/// active-sync moves the highlight to the selected ganger's stance. The caller sizes the
/// panel (same HEIGHT as the Overall Weapon Panel, a fixed relative width) and parents it
/// inside the bottom bar. Takes `&mut Commands` + the live theme.
pub(in crate::states::running::game::battlescape) fn spawn_stance_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    // The Stance column is its OWN bordered sub-panel (D-B): `spawn_panel` paints the border /
    // radius / fill (re-applied by `apply_theme` every run), and we overwrite its `Node` with
    // a vertical column layout that FILLS the panel. The layout fields survive the theme pass
    // (it overrides only the theme-owned border / radius / padding for the Panel role). The
    // caller sizes (height/width) + parents it.
    let panel = spawn_panel(commands, theme);
    commands.entity(panel).insert((
        StancePanelRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Vw(*theme.panel.border_width)),
            ..default()
        },
    ));

    let labels: Vec<SegmentLabel> = STANCE_ORDER
        .iter()
        .map(|k| SegmentLabel::new(stance_label(*k)))
        .collect();
    let control = spawn_segmented_control(
        commands,
        &labels,
        stance_index(StanceKind::Standing),
        control_segment_colors(theme),
        Orientation::Vertical,
        StanceControl,
    );
    // FILL the panel column (GTW-298: the three stacked stance segments fill the panel).
    // MUTATE only width/height — a wholesale `insert(Node { ..default() })` would DROP the
    // widget's connected-look fields (the root's rounded `border_radius` + `Overflow::clip`,
    // the zero inter-segment gaps, the Column direction), re-breaking the three segments into
    // detached boxes (the V2 defect). The widget already lays out as a zero-gap, clipped,
    // rounded Column; we only re-size it to fill the panel.
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

/// Read-write [`Query`] data for one freshly-spawned Stance segment to be tagged + sized: its
/// [`Entity`], its [`SegmentIndex`](gdtf_ui::SegmentIndex), and its [`Node`] (to make it FILL
/// the column width + take an even height share, so the three stack reads as a connected
/// vertical segmented control — V2 fix).
///
/// Named to keep [`tag_stance_segments`]'s signature legible (clippy `type_complexity`).
type NewStanceSegment = (Entity, &'static gdtf_ui::SegmentIndex, &'static mut Node);

/// Tags each Stance [`SegmentedControl`](gdtf_ui::SegmentedControl) segment with its
/// per-stance marker ([`StanceStandingButton`] / [`StanceKneelingButton`] /
/// [`StanceProneButton`]) AND sizes it to FILL the column width + an even height share, once
/// the control's segment children exist (GTW-277).
///
/// `spawn_segmented_control` spawns the segments via the command buffer, so the segment
/// child entities do not exist until that flush — the per-stance markers + the segment sizing
/// cannot be attached synchronously in [`spawn_stance_panel`]. This system runs on the spawn
/// frame, finds the Stance control by its [`StanceControl`] identity marker, walks its
/// [`Children`], gives each segment `width: 100%` + an even flex height share
/// (`flex_grow: 1` + `flex_basis: 0` + `min_height: 0`) — so the three stacked segments are
/// the SAME width and read as a connected vertical control rather than three detached,
/// differently-sized boxes (V2 fix) — and inserts the marker for each segment's
/// [`SegmentIndex`](gdtf_ui::SegmentIndex) ([`STANCE_ORDER`] maps index → stance). It is gated
/// on `Added<`[`StanceControl`]`>` so it runs only the frame the control appears (then never
/// again — the segments are spawned once, GTW-284), and it does NOT despawn/respawn anything
/// (mutate-only — it inserts a unit marker + sizes each existing segment).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the inserts, an
/// `Added<StanceControl>` detector with the control's [`Children`], and a
/// `Query<(Entity, &SegmentIndex, &mut Node), With<Segment>>` over the freshly-spawned
/// segments — no `&mut World`.
pub(in crate::states::running::game::battlescape) fn tag_stance_segments(
    mut commands: Commands,
    controls: Query<&Children, (With<StanceControl>, Added<StanceControl>)>,
    mut segments: Query<NewStanceSegment, With<gdtf_ui::Segment>>,
) {
    for children in &controls {
        for &child in children {
            let Ok((segment, index, mut node)) = segments.get_mut(child) else {
                continue;
            };
            // V2 fix (GTW-277 screenshot review): make every stance segment the SAME width
            // (`width: 100%` of the column) and an even height share (`flex_grow: 1` +
            // `flex_basis: 0` + `min_height: 0`) so the three stack as a connected vertical
            // segmented control instead of three detached, differently-sized boxes.
            node.width = Val::Percent(100.0);
            node.flex_grow = 1.0;
            node.flex_basis = Val::ZERO;
            node.min_height = Val::ZERO;
            match stance_for_index(**index) {
                Some(StanceKind::Standing) => {
                    commands.entity(segment).insert(StanceStandingButton);
                }
                Some(StanceKind::Crouching) => {
                    commands.entity(segment).insert(StanceKneelingButton);
                }
                Some(StanceKind::Prone) => {
                    commands.entity(segment).insert(StanceProneButton);
                }
                None => {}
            }
        }
    }
}
