//! The fire-mode 3-toggle sub-panel (GTW-265) — the always-visible replacement for the
//! removed GTW-254 popup picker.
//!
//! The Mode sub-panel ([`ModePanelRoot`]) is a vertical [`spawn_panel`] column inside the
//! action bar whose CHILDREN are per-mode toggle buttons — [`ModeSingleButton`] /
//! [`ModeBurstButton`] / [`ModeFullButton`] — built to exactly the modes the SELECTED
//! weapon offers (a Single+Burst weapon shows two toggles, no Full). Clicking a toggle
//! sets [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) DIRECTLY to that mode's
//! read-back [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) (never fabricated), and the
//! current mode is shown via the `gdtf_ui` [`ActiveButton`] paint marker (the GTW-253
//! hook, made sticky by GTW-266). No modal, no scrim, no z-stacking, no world
//! click-through — by construction the whole popup bug class is gone.
//!
//! ## (Re)building the toggles on selection change
//!
//! The offered modes change with the selected weapon, so [`rebuild_mode_buttons`]
//! despawns the panel's existing toggle children and respawns one per offered mode
//! whenever [`SelectedShooter`](gdtf_battle_input::SelectedShooter) changes. It runs
//! `.after(UiSystems::ApplyTheme)` (`bevy-traps.md` #3): the spawned toggles are `Themed`,
//! and `gdtf_ui::apply_theme` (change-driven) inserts their look the frame after they
//! spawn — running the rebuild after `apply_theme` keeps its inserts ahead of the next
//! rebuild's despawn in command-apply order, so a re-selection never races a theme-insert
//! onto an about-to-die toggle (the GTW-254 despawned-entity lesson). An UNARMED selection
//! (no [`FireMode`](gdtf_battle_sim::FireMode)) clears the toggles.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
use gdtf_battle_sim::{FireMode, FireModeSpec, ModeKind};
use gdtf_ui::{ActiveButton, ButtonLabel, spawn_button, theme::GdtfTheme};

use super::actions::{PressedButton, is_press};
use crate::scenes::running::game::battlescape::action_bar::components::{
    ModeBurstButton, ModeFullButton, ModePanelRoot, ModeSingleButton,
};

/// Query FILTER matching ANY mode toggle (Single / Burst / Full).
///
/// Factored into a named alias so [`rebuild_mode_buttons`]'s existing-toggle query stays
/// legible (clippy `type_complexity`); it is the set of all mode-toggle markers, the
/// children `rebuild_mode_buttons` tears down before respawning to the new selection.
type AnyModeToggle = Or<(
    With<ModeSingleButton>,
    With<ModeBurstButton>,
    With<ModeFullButton>,
)>;

/// Vertical gap between the Mode sub-panel's toggle buttons, in logical pixels.
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule): it is
/// layout spacing, not a theme color/size (the action-bar `BarGapPx` precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct ModeGapPx(f32);

impl ModeGapPx {
    /// The Mode sub-panel's inter-toggle gap: 4 px (a tight stacked toggle column).
    const PANEL: Self = Self(4.0);
}

/// Spawns the empty Mode sub-panel column ([`ModePanelRoot`]) and returns its [`Entity`]
/// so `spawn_action_bar` can parent it under the bar root (GTW-265).
///
/// A themed [`spawn_panel`](gdtf_ui::spawn_panel) laid out as a vertical column; its
/// toggle children are filled in by [`rebuild_mode_buttons`] on the first / each
/// selection change. Returns the panel so the caller parents it in the bar's left-to-right
/// row. Takes `&mut Commands` + the live theme (the `spawn_action_bar` precedent).
pub(in crate::scenes::running::game::battlescape) fn spawn_mode_panel(
    commands: &mut Commands,
    theme: &GdtfTheme,
) -> Entity {
    let panel = gdtf_ui::spawn_panel(commands, theme);
    commands.entity(panel).insert((
        ModePanelRoot,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(*ModeGapPx::PANEL),
            ..default()
        },
    ));
    panel
}

/// (Re)builds the Mode sub-panel's toggle children to exactly the modes the SELECTED
/// weapon offers, whenever [`SelectedShooter`](gdtf_battle_input::SelectedShooter) changes
/// (GTW-265 AC: a Single+Burst weapon shows exactly two toggles, no Full).
///
/// On a change of the selection it despawns the panel's current toggle children, then —
/// for an armed selection — spawns one [`spawn_button`] per mode in the weapon's
/// [`FireMode`](gdtf_battle_sim::FireMode) selector, each tagged with the marker for its
/// [`ModeKind`](gdtf_battle_sim::ModeKind) ([`ModeSingleButton`] / [`ModeBurstButton`] /
/// [`ModeFullButton`]) and labelled by the kind's [`Display`](std::fmt::Display). An
/// UNARMED selection (no `FireMode`) or a cleared selection leaves the panel empty. Runs
/// `.after(UiSystems::ApplyTheme)` (see the module docs) and only on a real selection
/// change (change-detection hygiene).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawn/despawn, the
/// `Res<SelectedShooter>` read, a read-only `Query<&FireMode>`, the panel-root query, and a
/// read-only existing-toggle query — no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn rebuild_mode_buttons(
    mut commands: Commands,
    selected: Res<SelectedShooter>,
    theme: Option<Res<GdtfTheme>>,
    weapons: Query<&FireMode>,
    panels: Query<Entity, With<ModePanelRoot>>,
    existing: Query<Entity, AnyModeToggle>,
) {
    // Only rebuild when the selection actually changed this update.
    if !selected.is_changed() {
        return;
    }
    let Some(theme) = theme else {
        // No theme yet — leave the panel for the next change once the theme is present
        // (the bar-spawn precedent: never spawn un-themed buttons).
        return;
    };
    // Tear down the current toggles (they belong to the previous selection's weapon).
    for toggle in &existing {
        commands.entity(toggle).despawn();
    }
    // An armed selection offers a `FireMode` selector; an unarmed / cleared one leaves the
    // panel empty.
    let Some(shooter) = **selected else {
        return;
    };
    let Ok(weapon) = weapons.get(shooter) else {
        return;
    };

    // One toggle per offered mode, in the weapon's authored order, parented under each
    // Mode panel root. The mode→marker mapping is the closed `ModeKind` set.
    let mut toggles: Vec<Entity> = Vec::new();
    for mode in weapon.iter() {
        let label = ButtonLabel::new(mode.kind.to_string());
        let toggle = match mode.kind {
            ModeKind::Single => spawn_button(&mut commands, &theme, label, ModeSingleButton),
            ModeKind::Burst => spawn_button(&mut commands, &theme, label, ModeBurstButton),
            ModeKind::Full => spawn_button(&mut commands, &theme, label, ModeFullButton),
        };
        toggles.push(toggle);
    }
    for panel in &panels {
        commands.entity(panel).add_children(&toggles);
    }
}

/// On a Mode toggle press, set [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode)
/// DIRECTLY to that mode's read-back spec off the selected weapon (GTW-265).
///
/// For each pressed toggle ([`ModeSingleButton`] / [`ModeBurstButton`] /
/// [`ModeFullButton`]), it looks up the matching [`FireModeSpec`](gdtf_battle_sim::FireModeSpec)
/// in the selected weapon's [`FireMode`](gdtf_battle_sim::FireMode) selector and sets
/// [`SelectedFireMode`] to it — the read-back value, NEVER a fabricated spec. A toggle was
/// spawned only for a mode the weapon offers, so the lookup is total in practice; a
/// missing mode (defensive) is a no-op. Writes only on a real change (change-detection
/// hygiene). The three press queries are disjoint per marker, so they never conflict.
///
/// Param-only (`bevy-traps.md` #7): the `ResMut<SelectedFireMode>` write, the
/// `Res<SelectedShooter>` read, a read-only `Query<&FireMode>`, and three read-only
/// per-marker press queries — no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn mode_button_pressed(
    mut fire_mode: ResMut<SelectedFireMode>,
    selected: Res<SelectedShooter>,
    weapons: Query<&FireMode>,
    single: Query<&Interaction, PressedButton<ModeSingleButton>>,
    burst: Query<&Interaction, PressedButton<ModeBurstButton>>,
    full: Query<&Interaction, PressedButton<ModeFullButton>>,
) {
    // Which mode (if any) was pressed this frame. The toggles are disjoint, so at most one
    // matches per frame in practice; the first match wins.
    let pressed: Option<ModeKind> = if single.iter().copied().any(is_press) {
        Some(ModeKind::Single)
    } else if burst.iter().copied().any(is_press) {
        Some(ModeKind::Burst)
    } else if full.iter().copied().any(is_press) {
        Some(ModeKind::Full)
    } else {
        None
    };
    let Some(kind) = pressed else {
        return;
    };
    // Read the selected weapon's spec for that kind — never a fabricated value.
    let Some(spec) = mode_spec_for(*selected, &weapons, kind) else {
        return;
    };
    let next = SelectedFireMode::new(spec);
    if *fire_mode != next {
        *fire_mode = next;
    }
}

/// Drives the three Mode toggles' active (toggled-on) look from the live
/// [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) (GTW-265).
///
/// Marks exactly the toggle whose [`ModeKind`](gdtf_battle_sim::ModeKind) equals the
/// selected mode's kind [`ActiveButton`](gdtf_ui::ActiveButton) and removes it from the
/// other two, so the current mode reads as toggled-on (the GTW-266 sticky paint). Insert /
/// remove are idempotent, so it runs every frame. Mirrors
/// [`sync_stance_buttons_active`](super::stance_active::sync_stance_buttons_active).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert/remove, the
/// `Res<SelectedFireMode>` read, and one `Query<Entity, With<…>>` per mode toggle — no
/// `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn sync_mode_buttons_active(
    mut commands: Commands,
    fire_mode: Res<SelectedFireMode>,
    single_buttons: Query<Entity, With<ModeSingleButton>>,
    burst_buttons: Query<Entity, With<ModeBurstButton>>,
    full_buttons: Query<Entity, With<ModeFullButton>>,
) {
    let active = fire_mode.kind;
    set_active(&mut commands, &single_buttons, active == ModeKind::Single);
    set_active(&mut commands, &burst_buttons, active == ModeKind::Burst);
    set_active(&mut commands, &full_buttons, active == ModeKind::Full);
}

/// The selected weapon's [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) for `kind`, read
/// back off its [`FireMode`](gdtf_battle_sim::FireMode) selector — or [`None`] when there
/// is no selection, the selection is unarmed, or the weapon does not offer `kind`.
///
/// The single read-back point so [`mode_button_pressed`] never fabricates a spec (the
/// GTW-265 "read-back, never fabricated" rule). Read-only over the selection + weapon
/// query.
fn mode_spec_for(
    selected: SelectedShooter,
    weapons: &Query<&FireMode>,
    kind: ModeKind,
) -> Option<FireModeSpec> {
    let shooter = (*selected)?;
    let weapon = weapons.get(shooter).ok()?;
    weapon.iter().find(|spec| spec.kind == kind).copied()
}

/// Inserts or removes [`ActiveButton`](gdtf_ui::ActiveButton) on every button matched by
/// `buttons`, by whether that toggle is the `active` one this frame.
///
/// Shared by [`sync_mode_buttons_active`] across the three mode toggles (the stance-panel
/// `set_active` shape).
fn set_active<M: Component>(
    commands: &mut Commands,
    buttons: &Query<Entity, With<M>>,
    active: bool,
) {
    for button in buttons {
        let mut entity = commands.entity(button);
        if active {
            entity.insert(ActiveButton);
        } else {
            entity.remove::<ActiveButton>();
        }
    }
}
