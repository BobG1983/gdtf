//! The fire-mode control's offered-mode visibility driver:
//! [`rebuild_mode_segments`] shows exactly the selected weapon's modes (GTW-284,
//! mutate-not-respawn). Split out of the monolithic `mode_panel.rs` (GTW-583); the
//! control rationale lives on the parent `mode_panel` module.

use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{FireMode, MeleeWeapon, ModeKind, WieldedBy, Wields};
use gdtf_ui::{Segment, SegmentIndex, set_segment_visible};

use super::order::MODE_ORDER;
use crate::states::running::game::battlescape::action_bar::components::{
    ModeControl, ModePanelRoot,
};

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
/// spawned once by [`spawn_mode_panel`](super::spawn::spawn_mode_panel); on a selection change this system toggles each
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
/// `Query<&Wields>` (the relationship) + a `Query<&FireMode, With<WieldedBy>>` weapon-entity
/// query (the offered modes live on the related weapon entity since GTW-323 slice 3), an
/// [`Added<ModeControl>`](Added) detector, a `Query<&Children>` (to walk the control's
/// segments for [`set_segment_visible`](gdtf_ui::set_segment_visible)), a
/// `Query<(&SegmentIndex, &mut Node), With<Segment>>` write, a control-root
/// `Query<ModeControlChildren, With<ModeControl>>`, and the panel-root `&mut`[`Visibility`]
/// query — no `Commands`, no `&mut World`.
#[allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "param tuple aliased where possible; the set_segment_visible call signature fixes the \
    children/segments query shapes, GTW-323 slice 3 adds the Wields relationship query so the \
    offered modes resolve off the related weapon entity, and GTW-505 C5 adds the MeleeWeapon marker \
    probe so the RANGED weapon resolves excluding the melee one"
)]
pub(in crate::states::running::game::battlescape) fn rebuild_mode_segments(
    selected: Res<SelectedShooter>,
    wields: Query<&Wields>,
    weapons: Query<&FireMode, With<WieldedBy>>,
    melee: Query<(), With<MeleeWeapon>>,
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

    // Which modes the SELECTED weapon offers (the closed `ModeKind` set). The FireMode
    // selector lives on the wielded RANGED WEAPON entity (GTW-323 slice 3): resolve
    // `ganger → Wields → the ranged weapon entity → FireMode`. GTW-505 C5: a ganger wields
    // BOTH a ranged AND a melee weapon, so resolve through `Wields::ranged_weapon` (excluding
    // the `MeleeWeapon`-marked entity) — NOT `Wields::weapon` (the spawn-order fragile first
    // entity). A cleared / no-selection / unarmed (no Wields / no ranged weapon / no FireMode)
    // selection offers nothing → every segment hidden.
    let offered = (**selected)
        .and_then(|shooter| wields.get(shooter).ok())
        .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))
        .and_then(|weapon| weapons.get(weapon).ok());
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
