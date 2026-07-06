//! The member stat panel builder — [`spawn_member_stat_panel`] plus the per-row editable
//! attribute and readonly derived-stat builders.
//!
//! Separated from the row builder ([`super::row`]) so the expanded-panel layout and accordion
//! wiring have their own focused file.

use bevy::{
    prelude::*,
    ui::{Overflow, Val},
};
use gdtf_battle_sim::{DerivedStats, GangerStatTuning, derive_stats};
use gdtf_ui::{
    AccordionAnim, AccordionContent, AccordionContentFit, AccordionExpandedVh, AccordionProgress,
    FieldColors, NumericRange, spawn_numeric_field, theme::GdtfTheme,
};

use crate::states::{
    RunningState,
    running::gang_editor::{
        components::{
            AttributeField, BaseAttribute, DerivedStat, DerivedStatText, MemberRowIndex,
            MemberStatPanel,
        },
        model::EditableMember,
        systems::derived_display::format_derived,
    },
};

/// The inter-field vertical gap inside the expanded stat panel, in `Vh` — reuses the editor's
/// calibrated row gap so the panel spacing matches the rest of the screen (matches
/// `EDITOR_GAP_VH` in the shell). Relative units.
const STAT_PANEL_GAP_VH: f32 = 1.388_89;

/// The inclusive lower bound an editable base-attribute numeric field clamps into (GTW-428 C2).
/// Attributes are non-negative dimensionless magnitudes (`docs/combat/stats.md`), so the floor is
/// zero — a negative attribute is meaningless.
const ATTRIBUTE_MIN: f32 = 0.0;

/// The inclusive upper bound an editable base-attribute numeric field clamps into (GTW-428 C2). A
/// generous ceiling well above any authored attribute (the shipped gangers sit in low single
/// digits), so the editor never refuses a plausible value yet still clamps absurd input.
const ATTRIBUTE_MAX: f32 = 100.0;

/// The `Vh` height the GTW-428 member stat panel's open ANIMATION lerps up to before it settles
/// (the GTW-428 layout fix). This is only the animation waypoint, NOT the final rest height: the
/// panel also carries [`AccordionContentFit`], so once the lerp settles fully open the shared
/// `drive_accordions` switches the height to [`Val::Auto`] and the rest-open panel sizes to its
/// EXACT content — every one of the sixteen stat lines renders regardless of the rendered font /
/// padding (a fixed `Vh` ceiling could not reliably clear the taller eight-attribute-field column
/// at the live window size — the round-2 QA defect). A generous-but-sub-viewport waypoint so the
/// open reads as a clear animation; the now-taller rest-open row scrolls within the member-list
/// scroll area, so one open member never crowds the others off. Relative units (the
/// responsive-UI rule); fed to the generalized accordion via [`AccordionExpandedVh`].
const STAT_PANEL_EXPANDED_VH: f32 = 38.0;

/// The clamp range every editable base-attribute numeric field uses (`[ATTRIBUTE_MIN,
/// ATTRIBUTE_MAX]`) — built once and shared by every attribute field a row spawns (C2).
const fn attribute_range() -> NumericRange<f32> {
    NumericRange::new(ATTRIBUTE_MIN, ATTRIBUTE_MAX)
}

/// The [`FieldColors`] a text / numeric field paints with, read from the theme. Pure UI plumbing.
pub(super) fn field_colors(theme: &GdtfTheme) -> FieldColors {
    FieldColors {
        background: *theme.panel.color,
        text:       *theme.text.text_color,
        caret:      *theme.text.text_color,
    }
}

/// Spawns one member row's EXPANDED stat panel (GTW-428 C1/C2/C3) and returns its root
/// [`Entity`] (the GTW-416 [`AccordionContent`] the pip drives).
///
/// The panel IS an accordion content node — it carries the [`AccordionContent`] marker that
/// `drive_accordions` phase 2 iterates (WITHOUT it the height lerp never runs), plus
/// [`MemberStatPanel`] + the row's [`MemberRowIndex`] + the shared `drive_accordions` animation
/// components ([`AccordionAnim::Collapsed`], [`AccordionProgress`]`(0.0)`). The open lerp animates
/// up to the PER-INSTANCE [`AccordionExpandedVh`]`(`[`STAT_PANEL_EXPANDED_VH`]`)` waypoint, then —
/// because the panel also carries [`AccordionContentFit`] — settles to a [`Val::Auto`]
/// CONTENT-FIT height so its sixteen stat lines all show regardless of the rendered font / padding
/// (the GTW-428 round-2 layout fix — a fixed `Vh` ceiling clipped the taller column's bottom lines
/// at the live window size). It starts at the collapsed height, CLIPPING its overflow so it shows
/// nothing until the pip toggles it open (the height lerp reveals it; at rest `Auto` exactly fits
/// the content so the clip cuts nothing).
///
/// The sixteen lines lay in TWO COLUMNS (the GTW-428 layout fix): a LEFT column of the eight
/// editable [`AttributeField`] numeric fields (each clamped to [`attribute_range`], seeded from the
/// member's current attribute — C2) and a RIGHT column of the eight readonly [`DerivedStatText`]
/// displays seeded from the GTW-384 [`derive_stats`] pipeline over the member's current attributes
/// (C3). Two columns of eight halve the panel's vertical extent versus one column of sixteen, so
/// the open panel clears the now-fitting height with every line visible and legible.
pub(super) fn spawn_member_stat_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    member: &EditableMember,
    tuning: &GangerStatTuning,
) -> Entity {
    // The panel content node — collapsed + clipping, carrying the accordion animation state the
    // shared `drive_accordions` lerps and the pip toggle flips (C1), plus the per-instance expanded
    // height so it opens tall enough to show its sixteen lines (the GTW-428 layout fix).
    let panel = commands
        .spawn((
            MemberStatPanel,
            AccordionContent,
            // Settle the rest-open panel to a CONTENT-FIT (`Val::Auto`) height so all sixteen
            // lines show regardless of the rendered font / padding — a fixed `Vh` ceiling could
            // not reliably clear the taller (eight-attribute-field) column at the live window
            // size (the GTW-428 round-2 QA defect). The lerp still opens through `Vh` toward the
            // per-instance target below for a visible animation; `Auto` takes over only at rest.
            AccordionContentFit,
            row_index,
            AccordionAnim::Collapsed,
            AccordionProgress::new(0.0),
            AccordionExpandedVh::new(STAT_PANEL_EXPANDED_VH),
            BackgroundColor(*theme.panel.color),
            stat_panel_node(),
            DespawnOnExit(RunningState::DebugGangEditor),
        ))
        .id();

    // The two side-by-side stat columns: editable attributes (left) | readonly derived (right).
    let attributes_column = commands
        .spawn((
            stat_column_node(),
            DespawnOnExit(RunningState::DebugGangEditor),
        ))
        .id();
    let derived_column = commands
        .spawn((
            stat_column_node(),
            DespawnOnExit(RunningState::DebugGangEditor),
        ))
        .id();

    // LEFT column — the eight editable attribute fields (C2).
    for attribute in BaseAttribute::ALL {
        let field = spawn_attribute_field(commands, theme, row_index, member, attribute);
        commands.entity(attributes_column).add_child(field);
    }

    // RIGHT column — the eight readonly derived-stat displays, seeded from the real GTW-384
    // pipeline (C3).
    let stats: DerivedStats = derive_stats(&member.attributes(), tuning);
    for stat in DerivedStat::ALL {
        let display = spawn_derived_display(commands, theme, row_index, stat, &stats);
        commands.entity(derived_column).add_child(display);
    }

    commands
        .entity(panel)
        .add_children(&[attributes_column, derived_column]);
    panel
}

/// Spawn one EDITABLE base-attribute row inside a stat panel: a label beside a clamped numeric
/// field seeded with the member's current value (GTW-428 C2). Returns the row [`Entity`]. The
/// numeric field carries the [`AttributeField`] marker + the row's [`MemberRowIndex`] + the
/// [`BaseAttribute`] so a [`NumericFieldCommitted`](gdtf_ui::NumericFieldCommitted)`<f32>` maps to
/// the right member's right attribute.
fn spawn_attribute_field(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    member: &EditableMember,
    attribute: BaseAttribute,
) -> Entity {
    let row = commands
        .spawn((
            stat_line_node(),
            DespawnOnExit(RunningState::DebugGangEditor),
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(attribute.label().to_owned()),
            TextColor(*theme.text.text_color),
            DespawnOnExit(RunningState::DebugGangEditor),
        ))
        .id();
    let field = spawn_numeric_field(
        commands,
        member.attribute(attribute),
        attribute_range(),
        field_colors(theme),
        (
            AttributeField,
            row_index,
            attribute,
            DespawnOnExit(RunningState::DebugGangEditor),
        ),
    );
    commands.entity(row).add_children(&[label, field]);
    row
}

/// Spawn one READONLY derived-stat row inside a stat panel: a label beside a [`DerivedStatText`]
/// value node seeded from the GTW-384 pipeline output for the member's current attributes (GTW-428
/// C3). Returns the row [`Entity`]. The value node carries the [`DerivedStatText`] marker + the
/// row's [`MemberRowIndex`] + the [`DerivedStat`] so the recompute mutates the right node in place
/// (NOT a numeric field — derived stats are readonly, C2).
fn spawn_derived_display(
    commands: &mut Commands,
    theme: &GdtfTheme,
    row_index: MemberRowIndex,
    stat: DerivedStat,
    stats: &DerivedStats,
) -> Entity {
    let row = commands
        .spawn((
            stat_line_node(),
            DespawnOnExit(RunningState::DebugGangEditor),
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(stat.label().to_owned()),
            TextColor(*theme.text.text_color),
            DespawnOnExit(RunningState::DebugGangEditor),
        ))
        .id();
    let value = commands
        .spawn((
            DerivedStatText,
            row_index,
            stat,
            Text::new(format_derived(stats, stat)),
            TextColor(*theme.text.text_color),
            DespawnOnExit(RunningState::DebugGangEditor),
        ))
        .id();
    commands.entity(row).add_children(&[label, value]);
    row
}

/// The expanded stat panel's [`Node`] (the GTW-416 accordion content): a full-width flex ROW
/// holding the two stat COLUMNS side by side (the GTW-428 2-column layout fix), starting at zero
/// height and CLIPPING its overflow, so a collapsed panel shows nothing and the `drive_accordions`
/// lerp reveals it by animating the height up to the panel's per-instance
/// [`AccordionExpandedVh`] target (C1). The shared driver writes the height while animating; the
/// panel is seeded collapsed (`Val::Vh(0.0)`). A column gap separates the two columns. Relative units.
fn stat_panel_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Vh(0.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Start,
        column_gap: Val::Vw(1.2),
        padding: bevy::ui::UiRect::all(Val::Vh(0.6)),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// One stat COLUMN's [`Node`] inside the expanded panel: a flex COLUMN that takes an equal share of
/// the panel width (`flex_basis: 0` + `flex_grow: 1`) and stacks its eight stat lines top-to-bottom
/// with the calibrated inter-line gap (the GTW-428 2-column layout fix). A `min_height: 0` lets the
/// column shrink below its content while the panel is mid-lerp (the flex-item bottom-cramp guard).
/// Relative units.
fn stat_column_node() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        flex_basis: Val::Px(0.0),
        flex_grow: 1.0,
        min_height: Val::Px(0.0),
        row_gap: Val::Vh(STAT_PANEL_GAP_VH),
        ..default()
    }
}

/// One stat LINE's [`Node`] inside the panel (a label beside its field / value): a full-width flex
/// ROW with a gap so the label and the value read as one line. Relative units.
fn stat_line_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Vw(0.6),
        ..default()
    }
}
