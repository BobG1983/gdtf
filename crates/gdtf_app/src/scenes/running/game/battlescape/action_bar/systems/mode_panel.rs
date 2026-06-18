//! The fire-mode control — a `gdtf_ui` horizontal [`SegmentedControl`] (GTW-265 /
//! GTW-277 / GTW-284) — its constructor, its offered-mode visibility driver, its press →
//! `SelectedFireMode` write, and its active-segment sync.
//!
//! GTW-277 migrated the fire-mode control from three ad-hoc toggle buttons to ONE generic
//! `gdtf_ui` [`SegmentedControl`] (3 horizontal segments Single / Burst / Full-Auto — the
//! Fire-Mode control in the mockup). Selecting a segment sets
//! [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) DIRECTLY to that mode's
//! read-back [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) (never fabricated); the current
//! mode is the control's own [`ActiveSegment`](gdtf_ui::ActiveSegment) highlight, synced
//! FROM [`SelectedFireMode`]. Mode does NOT use [`ActIntent`](gdtf_battle_input::ActIntent)
//! (it writes the resource directly) — that is unchanged from GTW-265.
//!
//! ## Only the offered modes show, via per-segment visibility (GTW-284)
//!
//! The THREE segments are spawned ONCE — a weapon offers a SUBSET of {Single, Burst, Full}
//! — and [`rebuild_mode_segments`] reveals exactly the offered ones by toggling each
//! segment's [`Display`](bevy::ui::Display) (`Display::None` collapses a non-offered
//! segment so the row shrinks to the offered set) via `gdtf_ui`'s
//! [`set_segment_visible`](gdtf_ui::set_segment_visible) — NEVER despawning/respawning the
//! control on a selection/weapon change. So the segment [`Entity`] ids stay STABLE across a
//! weapon change ([[ui-mutate-not-respawn]] / the GTW-284 invariant) and the offered subset
//! is the only thing that visibly changes. An UNARMED selection (no
//! [`FireMode`](gdtf_battle_sim::FireMode)) hides every segment and the panel root.
//!
//! It runs `.after(UiSystems::ApplyTheme)` (`bevy-traps.md` #3) so its writes settle
//! deterministically relative to the theme pass.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
use gdtf_battle_sim::{FireMode, FireModeSpec, ModeKind};
use gdtf_ui::{
    ActiveSegment, Orientation, Segment, SegmentColors, SegmentIndex, SegmentLabel,
    SegmentSelected, SegmentedControl, set_segment_visible, spawn_segmented_control,
    theme::GdtfTheme,
};

use super::stance_panel::control_segment_colors;
use crate::scenes::running::game::battlescape::action_bar::components::{
    ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
};

/// The three fire-mode segment indices, in DISPLAY (left-to-right) order: Single / Burst /
/// Full. The index ↔ [`ModeKind`] mapping the press listener + the active-sync + the
/// visibility driver share.
pub(in crate::scenes::running::game::battlescape) const MODE_ORDER: [ModeKind; 3] =
    [ModeKind::Single, ModeKind::Burst, ModeKind::Full];

/// The segment INDEX of a [`ModeKind`] in [`MODE_ORDER`] (the active-sync direction).
fn mode_index(kind: ModeKind) -> usize {
    MODE_ORDER.iter().position(|k| *k == kind).unwrap_or(0)
}

/// The [`ModeKind`] of a segment INDEX in [`MODE_ORDER`] (the press-listener direction),
/// or [`None`] for an out-of-range index (defensive; the control has exactly three).
fn mode_for_index(index: usize) -> Option<ModeKind> {
    MODE_ORDER.get(index).copied()
}

/// The DISPLAYED firemode label for `kind` — `"single"` / `"burst"` for those modes (the
/// sim's canonical [`ModeKind`] [`Display`](std::fmt::Display) label), and the SHORTER
/// `"auto"` for [`ModeKind::Full`] (the GTW-298 presentation map: at the legible control
/// font the full sim label `"full-auto"` clipped in the narrow firemode cell, so it shows
/// as `"auto"`).
///
/// A presentation-only override local to this firemode control: it does NOT change the
/// sim's [`ModeKind`] [`Display`]. Single / Burst pass through unchanged.
fn mode_label(kind: ModeKind) -> String {
    match kind {
        ModeKind::Full => "auto".to_owned(),
        other => other.to_string(),
    }
}

/// Spawns the Mode sub-panel ([`ModePanelRoot`]) holding ONE horizontal
/// [`SegmentedControl`] of three fire-mode segments (Single / Burst / Full-Auto), and
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
/// [`rebuild_mode_segments`] reveals exactly the offered modes (GTW-273). Takes
/// `&mut Commands` + the live theme.
pub(in crate::scenes::running::game::battlescape) fn spawn_mode_panel(
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

/// The active/base color palette the Mode [`SegmentedControl`] paints with — the SAME
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
pub(in crate::scenes::running::game::battlescape) fn tag_mode_segments(
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

/// Read-write [`Query`] data for the Mode control during a rebuild: its [`Entity`] and its
/// [`Children`] (the segments to show/hide).
///
/// Named to keep [`rebuild_mode_segments`]'s signature legible (clippy `type_complexity`).
type ModeControlChildren = (Entity, &'static Children);

/// MUTATES the Mode control's per-segment visibility to show exactly the modes the SELECTED
/// weapon offers, whenever [`SelectedShooter`](gdtf_battle_input::SelectedShooter) changes
/// (GTW-265 / GTW-277 / GTW-284).
///
/// GTW-284 ([[ui-mutate-not-respawn]]) — it NEVER despawns/respawns segments. The three are
/// spawned once by [`spawn_mode_panel`]; on a selection change this system toggles each
/// segment's [`Display`](bevy::ui::Display) via `gdtf_ui`'s
/// [`set_segment_visible`](gdtf_ui::set_segment_visible) — `Display::Flex` if the selected
/// weapon's [`FireMode`](gdtf_battle_sim::FireMode) offers that
/// [`ModeKind`](gdtf_battle_sim::ModeKind) (so the row shows only the offered set),
/// `Display::None` otherwise. The segment [`Entity`] ids stay STABLE across the change. An
/// UNARMED / cleared selection hides all three. Runs `.after(UiSystems::ApplyTheme)`.
///
/// It runs its body on a real selection change OR when the [`ModeControl`] was freshly
/// spawned ([`Added<ModeControl>`](Added)) — the battle-start auto-select fills
/// [`SelectedShooter`](gdtf_battle_input::SelectedShooter) several frames BEFORE the control
/// spawns, so the change has passed by the time the control exists; the `Added` trigger
/// re-reads the CURRENT selection on the spawn frame (the GTW-255 auto-select ordering
/// trap). Otherwise it early-returns (change-detection hygiene).
///
/// GTW-273 — it also drives the [`ModePanelRoot`]'s [`Visibility`] on EVERY branch: an
/// armed selection with modes → [`Visibility::Visible`]; unarmed / cleared / no-mode →
/// [`Visibility::Hidden`], so there is never an empty Mode box. The root is never
/// despawned — only its visibility (and the segments' display) change.
///
/// Param-only (`bevy-traps.md` #7): a `Res<SelectedShooter>` read, a read-only
/// `Query<&FireMode>`, an [`Added<ModeControl>`](Added) detector, a `Query<&Children>` (to
/// walk the control's segments for [`set_segment_visible`](gdtf_ui::set_segment_visible)), a
/// `Query<(&SegmentIndex, &mut Node), With<Segment>>` write, a control-root
/// `Query<ModeControlChildren, With<ModeControl>>`, and the panel-root `&mut`[`Visibility`]
/// query — no `Commands`, no `&mut World`.
#[allow(
    clippy::type_complexity,
    reason = "param tuple aliased where possible; the \
    set_segment_visible call signature fixes the children/segments query shapes"
)]
pub(in crate::scenes::running::game::battlescape) fn rebuild_mode_segments(
    selected: Res<SelectedShooter>,
    weapons: Query<&FireMode>,
    added_controls: Query<(), Added<ModeControl>>,
    children: Query<&Children>,
    mut segments: Query<(&SegmentIndex, &mut Node), With<Segment>>,
    controls: Query<ModeControlChildren, With<ModeControl>>,
    mut panels: Query<&mut Visibility, With<ModePanelRoot>>,
) {
    // Re-read on a real selection change OR when the Mode control was JUST spawned (the
    // GTW-255 auto-select ordering trap — see the doc comment).
    let control_just_spawned = added_controls.iter().next().is_some();
    if !selected.is_changed() && !control_just_spawned {
        return;
    }

    // Which modes the SELECTED weapon offers (the closed `ModeKind` set). A cleared /
    // no-selection / unarmed (no `FireMode`) selection offers nothing → every segment hidden.
    let offered = (**selected).and_then(|shooter| weapons.get(shooter).ok());
    let offers =
        |kind: ModeKind| offered.is_some_and(|weapon| weapon.iter().any(|m| m.kind == kind));

    // MUTATE each segment's visibility per the offered set — no despawn/respawn. The control
    // root is `With<ModeControl>`; `set_segment_visible` walks its segment children by index.
    for (control, _) in &controls {
        for (index, kind) in MODE_ORDER.iter().enumerate() {
            set_segment_visible(control, index, offers(*kind), &children, &mut segments);
        }
    }

    // GTW-273 — the panel root is VISIBLE only when at least one mode is offered.
    let any_offered = MODE_ORDER.iter().any(|k| offers(*k));
    let root_want = if any_offered {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut panels {
        if *visibility != root_want {
            *visibility = root_want;
        }
    }
}

/// On a Mode segment select, set [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode)
/// DIRECTLY to that mode's read-back spec off the selected weapon (GTW-265 / GTW-277).
///
/// Reads [`SegmentSelected`](gdtf_ui::SegmentSelected) messages (emitted by
/// `gdtf_ui`'s [`select_segment_on_press`](gdtf_ui::select_segment_on_press) on a real
/// click), and for each whose control carries the [`ModeControl`] marker, maps the chosen
/// [`SegmentIndex`](gdtf_ui::SegmentIndex) → [`ModeKind`] ([`MODE_ORDER`]) and looks up the
/// matching [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) in the selected weapon's
/// [`FireMode`](gdtf_battle_sim::FireMode) selector, setting [`SelectedFireMode`] to it —
/// the read-back value, NEVER a fabricated spec. A segment is only ever offered for a mode
/// the weapon has, so the lookup is total in practice; a missing mode (defensive) is a
/// no-op. Writes only on a real change (change-detection hygiene).
///
/// Param-only (`bevy-traps.md` #7): a [`MessageReader<SegmentSelected>`](MessageReader)
/// (bevy-traps rule 4), the `ResMut<SelectedFireMode>` write, the `Res<SelectedShooter>`
/// read, a read-only `Query<&FireMode>`, and a read-only `Query<(), With<ModeControl>>` — no
/// `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn mode_segment_write(
    mut chosen: MessageReader<SegmentSelected>,
    mut fire_mode: ResMut<SelectedFireMode>,
    selected: Res<SelectedShooter>,
    weapons: Query<&FireMode>,
    mode_controls: Query<(), With<ModeControl>>,
) {
    for event in chosen.read() {
        if mode_controls.get(event.control).is_err() {
            continue;
        }
        let Some(kind) = mode_for_index(*event.index) else {
            continue;
        };
        // Read the selected weapon's spec for that kind — never a fabricated value.
        let Some(spec) = mode_spec_for(*selected, &weapons, kind) else {
            continue;
        };
        let next = SelectedFireMode::new(spec);
        if *fire_mode != next {
            *fire_mode = next;
        }
    }
}

/// Drives the Mode control's active SEGMENT from the live
/// [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) (GTW-265 / GTW-277).
///
/// Sets the control root's [`ActiveSegment`](gdtf_ui::ActiveSegment) to the index of the
/// selected mode's [`ModeKind`](gdtf_battle_sim::ModeKind) ([`MODE_ORDER`]). Writing it via
/// [`set_if_neq`](bevy::prelude::DetectChangesMut::set_if_neq) marks it changed only on a
/// real change — exactly the signal `gdtf_ui`'s
/// [`repaint_segments`](gdtf_ui::repaint_segments) keys off, so the active segment repaints
/// (filled + bold) and the de-selected one returns to base the same frame (the color-blind-
/// safe active mark). No despawn/respawn — pure index write ([[ui-mutate-not-respawn]]).
///
/// Param-only (`bevy-traps.md` #7): the `Res<SelectedFireMode>` read and a
/// `Query<&mut ActiveSegment, With<ModeControl>>` write — no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn sync_mode_active_segment(
    fire_mode: Res<SelectedFireMode>,
    mut controls: Query<&mut ActiveSegment, (With<ModeControl>, With<SegmentedControl>)>,
) {
    let want = ActiveSegment::new(mode_index(fire_mode.kind));
    for mut active in &mut controls {
        active.set_if_neq(want);
    }
}

/// The selected weapon's [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) for `kind`, read
/// back off its [`FireMode`](gdtf_battle_sim::FireMode) selector — or [`None`] when there
/// is no selection, the selection is unarmed, or the weapon does not offer `kind`.
///
/// The single read-back point so [`mode_segment_write`] never fabricates a spec (the
/// GTW-265 "read-back, never fabricated" rule). Read-only over the selection + weapon query.
fn mode_spec_for(
    selected: SelectedShooter,
    weapons: &Query<&FireMode>,
    kind: ModeKind,
) -> Option<FireModeSpec> {
    let shooter = (*selected)?;
    let weapon = weapons.get(shooter).ok()?;
    weapon.iter().find(|spec| spec.kind == kind).copied()
}
