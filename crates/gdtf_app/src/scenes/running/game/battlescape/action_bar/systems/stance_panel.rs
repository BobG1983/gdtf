//! The Stance 3-toggle sub-panel constructor (GTW-267 / GTW-298).
//!
//! [`spawn_stance_panel`] builds a PLAIN vertical [`Node`] column (no chrome of its own)
//! holding the THREE mutually-exclusive stance toggles — [`StanceStandingButton`] /
//! [`StanceKneelingButton`] / [`StanceProneButton`] — in a sensible height order (Stand,
//! Kneel, Prone). It mirrors [`spawn_mode_panel`](super::mode_panel::spawn_mode_panel): the
//! column is a reusable spawn helper returning the column [`Entity`] so a caller can parent it
//! wherever the layout needs it.
//!
//! GTW-298 relocated the controls cluster: the Stance panel now lives in the weapon-cluster's
//! Stance Panel (to the right of the Overall Weapon Panel), spawned by the weapon-panel module
//! through this shared constructor — NOT inside the action bar. The press → intent routing
//! (`action_bar_button_intents`) and the active-mark sync
//! ([`sync_stance_buttons_active`](super::stance_active::sync_stance_buttons_active)) are
//! UNCHANGED: they query the stance markers parent-agnostically, so the toggles work wherever
//! they are parented.
//!
//! D-B (2026-06-18 screenshot review, SUPERSEDES the earlier D4 plain-box ruling): the Stance
//! column is its OWN bordered sub-panel — a themed [`spawn_panel`](gdtf_ui::spawn_panel)
//! (`Themed(Panel)` — border + corner radius + fill, re-painted by `apply_theme`), exactly like
//! the weapon cluster's panels. The user now wants the three stance toggles framed in a distinct
//! bordered box positioned to the RIGHT of the Overall Weapon Panel INSIDE the bottom bar, NOT
//! sitting loose on the bar fill. (The earlier D4 fix made it a plain transparent [`Node`]; this
//! reverses that, per the new ruling.) The caller (the weapon-panel module) sizes it — same HEIGHT
//! as the Overall Weapon Panel, a sensible fixed relative width — and parents it inside the bar.

use bevy::{
    prelude::*,
    ui::{Node, UiRect, Val},
};
use gdtf_ui::{ButtonLabel, spawn_button, spawn_panel, theme::GdtfTheme};

use crate::scenes::running::game::battlescape::action_bar::components::{
    StanceKneelingButton, StancePanelRoot, StanceProneButton, StanceStandingButton,
};

/// Vertical gap between the Stance sub-panel's toggle buttons, in logical pixels.
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule): the
/// vertical stance column's inter-toggle spacing (the `mode_panel` `ModeGapPx` precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct StanceGapPx(f32);

impl StanceGapPx {
    /// The Stance sub-panel's inter-toggle gap: 4 px (a tight stacked toggle column).
    const PANEL: Self = Self(4.0);
}

/// Spawns the Stance sub-panel ([`StancePanelRoot`]) — its OWN bordered box — with its THREE
/// stance toggles as children (Stand / Kneel / Prone), and returns the panel [`Entity`] so a
/// caller can parent + size it inside the host layout (GTW-267 / GTW-298 / D-B).
///
/// A themed [`spawn_panel`](gdtf_ui::spawn_panel) box (`Themed(Panel)` — border / radius / fill,
/// re-painted by `apply_theme`, like the weapon cluster's panels — D-B, SUPERSEDING the earlier
/// D4 plain-box ruling) laid out as a vertical column whose children are the three
/// mutually-exclusive stance toggle buttons, each carrying its own marker so the
/// `action_bar_button_intents` press router + the
/// [`sync_stance_buttons_active`](super::stance_active::sync_stance_buttons_active) active-mark
/// sync find them by meaning (the per-marker disjointness, GTW-122). The toggles are sized to
/// FILL the column — `1/3` height each, full width — so they read as the contract's three stacked
/// stance buttons (GTW-298) framed in a distinct bordered panel. The caller sizes the panel (same
/// HEIGHT as the Overall Weapon Panel, a fixed relative width) and parents it inside the bottom
/// bar. Takes `&mut Commands` + the live theme (the `spawn_mode_panel` precedent).
pub(in crate::scenes::running::game::battlescape) fn spawn_stance_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    // The Stance column is its OWN bordered sub-panel (D-B): `spawn_panel` paints the border /
    // radius / fill (re-applied by `apply_theme` every run), and we overwrite its `Node` with the
    // vertical-column layout for the three toggles. The layout fields survive the theme pass (it
    // overrides only the theme-owned border / radius / padding for the Panel role — the
    // weapon-panel-root precedent). The caller sizes (height/width) + parents it.
    let panel = spawn_panel(commands, theme);
    commands.entity(panel).insert((
        StancePanelRoot,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(*StanceGapPx::PANEL),
            border: UiRect::all(Val::Px(*theme.panel.border_width_px)),
            ..default()
        },
    ));
    let stand = spawn_stance_toggle(commands, theme, "Stand", StanceStandingButton);
    let kneel = spawn_stance_toggle(commands, theme, "Kneel", StanceKneelingButton);
    let prone = spawn_stance_toggle(commands, theme, "Prone", StanceProneButton);
    commands.entity(panel).add_children(&[stand, kneel, prone]);
    panel
}

/// Spawns one stance toggle [`spawn_button`] labelled `label`, tagged with its `marker`, sized
/// to FILL its third of the Stance panel — full width, `1/3` height (GTW-298: three stacked
/// stance buttons each `1/3` height).
///
/// The shared toggle constructor for [`spawn_stance_panel`]'s Stand / Kneel / Prone set: each
/// toggle carries its fixed label + marker; the `flex_grow` lets the three share the column
/// height evenly. Returns the toggle so the caller parents it under the panel root.
fn spawn_stance_toggle<M: Component>(
    commands: &mut Commands,
    theme: &GdtfTheme,
    label: &str,
    marker: M,
) -> Entity {
    let toggle = spawn_button(commands, theme, ButtonLabel::new(label.to_owned()), marker);
    // Overwrite the auto-sized `box_node` with a FILL layout — `apply_theme` re-applies the
    // theme-owned border / radius / padding every run, preserving these layout fields (the
    // weapon-panel-root `insert((marker, Node))` precedent). `flex_grow` + zero `flex_basis`
    // makes the three toggles share the column height in equal thirds.
    commands.entity(toggle).insert(Node {
        width: Val::Percent(100.0),
        flex_grow: 1.0,
        flex_basis: Val::Percent(0.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    });
    toggle
}
