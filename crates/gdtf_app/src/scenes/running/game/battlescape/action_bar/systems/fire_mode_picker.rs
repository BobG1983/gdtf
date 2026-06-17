//! The fire-mode popup picker modal (GTW-254) — the click-to-select replacement for
//! the removed blind fire-mode cycle.
//!
//! Pressing the action-bar [`FireModePickerButton`] OPENS a themed popup listing the
//! selected weapon's offered fire modes; clicking an entry SETS
//! [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) to that mode and closes the
//! picker. The whole thing is built from existing [`gdtf_ui`] primitives
//! ([`spawn_panel`] / [`spawn_button`] / the [`ActiveButton`] paint hook) on the GTW-120
//! UI camera — no new theme role, no picking plugin (`bevy_ui` [`Interaction`] is
//! built-in, `bevy-traps.md` #6).
//!
//! ## "Open" = the existence of [`FireModePickerRoot`]
//!
//! There is no sub-state for the modal (`bevy-traps.md` #5): the picker is OPEN exactly
//! when a [`FireModePickerRoot`] entity exists. [`toggle_fire_mode_picker`] spawns it on
//! an opener press (when an armed ganger is selected) and despawns it on a second press
//! (toggle-closed); [`select_fire_mode_entry`] and [`dismiss_fire_mode_picker_on_scrim`]
//! despawn it on a mode-click / scrim-click. Recursive despawn tears down the scrim +
//! entry children with the root.
//!
//! ## Z-order + click-through (clause 4)
//!
//! The root carries a high [`GlobalZIndex`] so it renders ABOVE the action bar + status
//! panel (which carry none), and a full-screen transparent [`FireModePickerScrim`]
//! [`Button`] behind the panel catches a click-outside → close. World click-through is
//! prevented by [`sync_world_click_suppression`], which raises the input-owned
//! [`WorldClickSuppressed`](gdtf_battle_input::WorldClickSuppressed) flag while the
//! picker is open: the `gdtf_battle_input` world-click surfaces read it and go inert, so
//! a click on a picker button / its scrim does NOT also move/fire the ganger behind the
//! modal (the flag is OWNED by `gdtf_battle_input`, written across the legal
//! `gdtf_app -> gdtf_battle_input` edge — never a reverse edge, ADR-0001 / GTW-251).
//!
//! ## Caption (Part A2)
//!
//! [`sync_fire_mode_picker_caption`] repaints the opener button's caption from the live
//! [`SelectedFireMode`] (e.g. `Mode: burst`) and toggles its [`DisabledButton`] when no
//! armed ganger is selected — the active-mode display the contract's D2 puts on the
//! opener button.

use bevy::{prelude::*, ui::Val};
use gdtf_battle_input::{SelectedFireMode, SelectedShooter, WorldClickSuppressed};
use gdtf_battle_sim::{FireMode, FireModeSpec};
use gdtf_ui::{
    ActiveButton, ButtonLabel, DisabledButton, spawn_button, spawn_panel, theme::GdtfTheme,
};

use super::actions::{PressedButton, is_press};
use crate::scenes::running::game::battlescape::action_bar::components::{
    FireModePickerButton, FireModePickerEntry, FireModePickerRoot, FireModePickerScrim,
};

/// The opener-button caption shown when NO armed ganger is selected (Part A2 — the
/// neutral / disabled caption, the deferred-button precedent).
pub(in crate::scenes::running::game::battlescape::action_bar) const NO_MODE_CAPTION: &str =
    "Mode: —";

/// The [`GlobalZIndex`] the picker root sits at, so it composites ABOVE the action bar +
/// status panel (which carry no `GlobalZIndex` and so default to 0) — clause 4a.
///
/// A framework plumbing const (an explicit stacking layer handed straight to a
/// [`GlobalZIndex`], not a domain quantity — the `CELL_PX`-class carve-out). A
/// comfortably-high value so the modal always wins the stack.
const PICKER_Z: i32 = 1000;

/// The opener caption for `mode` (e.g. `Mode: burst`) — the active-mode display (D2).
///
/// Derives the human label from the mode's closed [`ModeKind`](gdtf_battle_sim::ModeKind)
/// via its [`Display`](std::fmt::Display) (GTW-260; no stored name string). A free helper
/// (not a method) so the caption format lives in one place, reused by the opener-caption
/// sync and each picker entry's label.
fn mode_caption(mode: &FireModeSpec) -> String {
    format!("Mode: {}", mode.kind)
}

/// The picker-entry label for `mode` — the same human label as the caption, sans the
/// `Mode:` prefix (the entry list reads as the bare mode names).
fn entry_label(mode: &FireModeSpec) -> String {
    mode.kind.to_string()
}

/// Whether a picker is currently open (a [`FireModePickerRoot`] exists).
///
/// The single "is the modal open" test, so the open/toggle/suppression systems read it
/// identically. Read-only over the root query.
fn picker_is_open(roots: &Query<Entity, With<FireModePickerRoot>>) -> bool {
    !roots.is_empty()
}

/// On a [`FireModePickerButton`] press, OPEN the picker (spawn the modal) when an armed
/// ganger is selected — or TOGGLE it closed if already open (GTW-254 clause B1).
///
/// Reads the press the SAME way the action-bar acts do (a `Changed<Interaction>` ==
/// `Pressed` on the opener, `Without<DisabledButton>` so a disabled opener is inert) and:
///
/// - if a picker is already open → DESPAWN it (a second opener press toggles closed);
/// - else, if the [`SelectedShooter`] is an armed ganger (has a [`FireMode`]) → SPAWN the
///   modal: a full-screen transparent [`FireModePickerScrim`] [`Button`] (click-outside
///   catcher) and a themed [`spawn_panel`] holding one [`spawn_button`] per offered mode,
///   each tagged [`FireModePickerEntry`] with the read-back [`FireModeSpec`] and marked
///   [`ActiveButton`] when it is the CURRENTLY-selected mode (the GTW-253 paint hook);
/// - else (no armed selection) → nothing (the opener is disabled then anyway).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawn/despawn, read-only
/// `Query`s + the theme read — no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn toggle_fire_mode_picker(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    openers: Query<&Interaction, PressedButton<FireModePickerButton>>,
    roots: Query<Entity, With<FireModePickerRoot>>,
    selected: Res<SelectedShooter>,
    fire_mode: Res<SelectedFireMode>,
    weapons: Query<&FireMode>,
) {
    // Only act on a fresh opener press (the action-bar `is_press` mechanism, reused).
    if !openers.iter().copied().any(is_press) {
        return;
    }
    // Already open → a second press toggles it closed.
    if let Some(root) = roots.iter().next() {
        commands.entity(root).despawn();
        return;
    }
    let Some(theme) = theme else {
        // No theme yet — do not spawn an un-themed modal (the bar-spawn precedent).
        return;
    };
    // Open only for an armed selection — read the weapon's offered modes off the
    // SelectedShooter entity (exactly as `sync_fire_mode_on_select` does). An empty or
    // unarmed selection opens nothing.
    let Some(shooter) = **selected else {
        return;
    };
    let Ok(weapon) = weapons.get(shooter) else {
        return;
    };
    spawn_picker(&mut commands, &theme, weapon, &fire_mode);
}

/// Spawns the picker modal tree (the scrim + the panel of mode entries) and returns
/// nothing — the spawned [`FireModePickerRoot`]'s existence is the "open" signal.
///
/// The root is a full-screen transparent container at a high [`GlobalZIndex`] (above the
/// bars, clause 4a) holding: (1) the full-screen [`FireModePickerScrim`] [`Button`]
/// (click-outside → close), and (2) a centred themed [`spawn_panel`] with one
/// [`spawn_button`] per mode in the weapon's [`FireMode`] selector, each tagged
/// [`FireModePickerEntry`] carrying that mode's read-back [`FireModeSpec`] and
/// [`ActiveButton`] when it is the currently-selected mode.
fn spawn_picker(
    commands: &mut Commands,
    theme: &GdtfTheme,
    weapon: &FireMode,
    fire_mode: &SelectedFireMode,
) {
    // The full-screen root: a transparent absolute container above the bars, centring the
    // panel. The scrim sits inside it as a full-screen sibling BEHIND the panel.
    let root = commands
        .spawn((
            FireModePickerRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(PICKER_Z),
        ))
        .id();

    // The click-outside scrim: a full-screen transparent Button behind the panel. It is a
    // Button so `bevy_ui`'s ui_focus_system drives its Interaction for the click-outside
    // press; its fill is fully transparent (it must not dim/obscure the battlefield, just
    // catch the click).
    let scrim = commands
        .spawn((
            FireModePickerScrim,
            Button,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .id();

    // The themed panel holding the mode entries, laid out as a vertical column.
    let panel = spawn_panel(commands, theme);
    commands.entity(panel).insert(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(*EntryGapPx::PICKER),
        ..default()
    });

    // One entry button per offered mode, read back off the weapon's selector (never a
    // fabricated spec — clause B3). The currently-selected mode additionally gets the
    // GTW-253 ActiveButton paint marker so the picker shows which mode is live.
    let mut entries: Vec<Entity> = Vec::new();
    for mode in weapon.iter() {
        let entry = spawn_button(
            commands,
            theme,
            ButtonLabel::new(entry_label(mode)),
            FireModePickerEntry(*mode),
        );
        if mode.kind == fire_mode.kind {
            commands.entity(entry).insert(ActiveButton);
        }
        entries.push(entry);
    }
    commands.entity(panel).add_children(&entries);
    commands.entity(root).add_children(&[scrim, panel]);
}

/// Vertical gap between the picker's mode-entry buttons, in logical pixels.
///
/// A named newtype over the gap rather than a bare `f32` (no-bare-types rule): it is
/// layout spacing, not a theme color/size (the action-bar `BarGapPx` precedent).
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
struct EntryGapPx(f32);

impl EntryGapPx {
    /// The picker's inter-entry gap: 4 px (a tight stacked mode list).
    const PICKER: Self = Self(4.0);
}

/// On a [`FireModePickerEntry`] press, SET [`SelectedFireMode`] to that entry's carried
/// [`FireModeSpec`] and CLOSE the picker (GTW-254 clause B3 / AC4).
///
/// The chosen spec is the read-back from the weapon's selector the entry was spawned with
/// — never a fabricated value. After setting the mode, the picker root is recursively
/// despawned (the modal closes). Param-only (`bevy-traps.md` #7): [`Commands`] for the
/// despawn, read-only entry query, the [`ResMut<SelectedFireMode>`] write.
pub(in crate::scenes::running::game::battlescape) fn select_fire_mode_entry(
    mut commands: Commands,
    entries: Query<(&Interaction, &FireModePickerEntry), Changed<Interaction>>,
    roots: Query<Entity, With<FireModePickerRoot>>,
    mut fire_mode: ResMut<SelectedFireMode>,
) {
    // The first pressed entry wins this frame (a modal shows one weapon's modes; only one
    // can be clicked per frame in practice).
    let Some((_, entry)) = entries
        .iter()
        .find(|(interaction, _)| matches!(interaction, Interaction::Pressed))
    else {
        return;
    };
    let next = SelectedFireMode::new(entry.0);
    if *fire_mode != next {
        *fire_mode = next;
    }
    // Close the picker now that a mode was chosen.
    for root in &roots {
        commands.entity(root).despawn();
    }
}

/// On a [`FireModePickerScrim`] press (a click OUTSIDE the panel), DISMISS the picker
/// with NO change to [`SelectedFireMode`] (GTW-254 clause B3 / AC5).
///
/// Recursively despawns the picker root; it touches no fire-mode state, so the selected
/// mode is unchanged. Param-only (`bevy-traps.md` #7): [`Commands`] + read-only queries.
pub(in crate::scenes::running::game::battlescape) fn dismiss_fire_mode_picker_on_scrim(
    mut commands: Commands,
    scrims: Query<&Interaction, (Changed<Interaction>, With<FireModePickerScrim>)>,
    roots: Query<Entity, With<FireModePickerRoot>>,
) {
    if !scrims.iter().any(|i| matches!(i, Interaction::Pressed)) {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn();
    }
}

/// Repaints the [`FireModePickerButton`] opener caption from the live
/// [`SelectedFireMode`] and toggles its [`DisabledButton`] by whether an armed ganger is
/// selected (GTW-254 Part A2 / AC2).
///
/// When the [`SelectedShooter`] is an armed ganger, the opener is ENABLED and its caption
/// reads the active mode (`Mode: burst`), tracking [`SelectedFireMode`] as it changes
/// (whether the picker or the default-on-select set it). With no armed selection it is
/// [`DisabledButton`] with the [`NO_MODE_CAPTION`] neutral caption (the deferred-button
/// precedent). The caption write goes through the opener button's `Text` child (the
/// `spawn_button` tree), only when the content differs (change-detection hygiene).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the disabled-marker toggle, the
/// `SelectedShooter` / `SelectedFireMode` reads, the armed-ganger `Query<&FireMode>`, the
/// opener `Query`, and a `Query<&mut Text>` over the opener's caption child via
/// [`Children`] — no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn sync_fire_mode_picker_caption(
    mut commands: Commands,
    selected: Res<SelectedShooter>,
    fire_mode: Res<SelectedFireMode>,
    weapons: Query<&FireMode>,
    openers: Query<(Entity, &Children), With<FireModePickerButton>>,
    mut captions: Query<&mut Text>,
) {
    // Armed iff there is a selection whose entity carries a FireMode (a weapon).
    let armed = (**selected)
        .and_then(|entity| weapons.get(entity).ok())
        .is_some();
    let caption = if armed {
        mode_caption(&fire_mode)
    } else {
        NO_MODE_CAPTION.to_owned()
    };

    for (opener, children) in &openers {
        // Enable/disable the opener with the armed state (the deferred-button precedent:
        // a DisabledButton emits no press and shows the disabled fill).
        let mut entity = commands.entity(opener);
        if armed {
            entity.remove::<DisabledButton>();
        } else {
            entity.insert(DisabledButton);
        }
        // Repaint the caption child's Text (only on a real change).
        for &child in children {
            if let Ok(mut text) = captions.get_mut(child)
                && text.as_str() != caption
            {
                caption.clone_into(&mut text.0);
            }
        }
    }
}

/// Raises / clears the input-owned
/// [`WorldClickSuppressed`](gdtf_battle_input::WorldClickSuppressed) flag by whether the
/// picker is open (GTW-254 clause 4b) — the cross-crate click-through guard.
///
/// While a [`FireModePickerRoot`] exists, the flag is `true`, so the `gdtf_battle_input`
/// world-click surfaces are inert and a click on a picker button / scrim does NOT also act
/// on the world cell behind the modal; with the picker closed the flag is `false`, so the
/// world click is live again. Writes only on a real change (change-detection hygiene).
/// This is the `gdtf_app` WRITER of the flag `gdtf_battle_input` OWNS — the legal
/// `gdtf_app -> gdtf_battle_input` edge (never a reverse edge, ADR-0001 / GTW-251).
///
/// Param-only (`bevy-traps.md` #7): read-only `Query<Entity, With<FireModePickerRoot>>` +
/// the [`ResMut<WorldClickSuppressed>`] write — no `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn sync_world_click_suppression(
    roots: Query<Entity, With<FireModePickerRoot>>,
    mut suppressed: ResMut<WorldClickSuppressed>,
) {
    let next = WorldClickSuppressed(picker_is_open(&roots));
    if *suppressed != next {
        *suppressed = next;
    }
}

/// Despawns any open picker on `OnExit(BattleScapeState::BattleRunning)` so the modal is
/// battle-scoped (the action-bar / status-panel cleanup precedent).
///
/// The picker root is spawned at runtime (not in `spawn_action_bar`), so it is not torn
/// down by `despawn_action_bar` (a different root). This dedicated cleanup ensures a
/// picker left open when the battle ends does not leak. Param-only (`bevy-traps.md` #7):
/// [`Commands`] + a `Query<Entity, With<FireModePickerRoot>>`.
pub(in crate::scenes::running::game::battlescape) fn despawn_fire_mode_picker(
    mut commands: Commands,
    roots: Query<Entity, With<FireModePickerRoot>>,
) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}
