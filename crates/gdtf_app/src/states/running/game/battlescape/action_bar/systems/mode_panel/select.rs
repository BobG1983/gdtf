//! The fire-mode control's selection sync: a segment press writes
//! [`SelectedFireMode`] directly, the active segment mirrors it back, and the
//! read-back spec lookup keeps the write never-fabricated. Split out of the
//! monolithic `mode_panel.rs` (GTW-583); the control rationale lives on the parent
//! `mode_panel` module.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
use gdtf_battle_sim::weapon::{FireMode, FireModeSpec, MeleeWeapon, ModeKind, WieldedBy, Wields};
use gdtf_ui::{ActiveSegment, SegmentSelected, SegmentedControl};

use super::order::{mode_for_index, mode_index};
use crate::states::running::game::battlescape::action_bar::components::ModeControl;

/// On a Mode segment select, set [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode)
/// DIRECTLY to that mode's read-back spec off the selected weapon (GTW-265 / GTW-277).
///
/// Reads [`SegmentSelected`](gdtf_ui::SegmentSelected) messages (emitted by
/// `gdtf_ui`'s [`select_segment_on_press`](gdtf_ui::select_segment_on_press) on a real
/// click), and for each whose control carries the [`ModeControl`] marker, maps the chosen
/// [`SegmentIndex`](gdtf_ui::SegmentIndex) → [`ModeKind`] ([`MODE_ORDER`](super::order::MODE_ORDER)) and looks up the
/// matching [`FireModeSpec`](gdtf_battle_sim::weapon::FireModeSpec) in the selected weapon's
/// [`FireMode`](gdtf_battle_sim::weapon::FireMode) selector, setting [`SelectedFireMode`] to it —
/// the read-back value, NEVER a fabricated spec. A segment is only ever offered for a mode
/// the weapon has, so the lookup is total in practice; a missing mode (defensive) is a
/// no-op. Writes only on a real change (change-detection hygiene).
///
/// Param-only (`bevy-traps.md` #7): a [`MessageReader<SegmentSelected>`](MessageReader)
/// (bevy-traps rule 4), the `ResMut<SelectedFireMode>` write, the `Res<SelectedShooter>`
/// read, a read-only `Query<&Wields>` (the relationship) + a `Query<&FireMode,
/// With<WieldedBy>>` weapon-entity query (the modes live on the related weapon entity since
/// GTW-323 slice 3) + a `Query<(), With<MeleeWeapon>>` marker probe (GTW-505 C5 — so the
/// RANGED weapon resolves excluding the melee one), and a read-only
/// `Query<(), With<ModeControl>>` — no `&mut World`.
#[allow(
    clippy::too_many_arguments,
    reason = "GTW-505 C5 adds the MeleeWeapon marker probe on top of the GTW-323 slice-3 Wields + \
    weapon-entity queries so the RANGED weapon's mode resolves excluding the melee weapon"
)]
pub(in crate::states::running::game::battlescape) fn mode_segment_write(
    mut chosen: MessageReader<SegmentSelected>,
    mut fire_mode: ResMut<SelectedFireMode>,
    selected: Res<SelectedShooter>,
    wields: Query<&Wields>,
    weapons: Query<&FireMode, With<WieldedBy>>,
    melee: Query<(), With<MeleeWeapon>>,
    mode_controls: Query<(), With<ModeControl>>,
) {
    for event in chosen.read() {
        if mode_controls.get(event.control).is_err() {
            continue;
        }
        let Some(kind) = mode_for_index(*event.index) else {
            continue;
        };
        // Read the selected RANGED weapon's spec for that kind — never a fabricated value.
        let Some(spec) = mode_spec_for(*selected, &wields, &weapons, &melee, kind) else {
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
/// selected mode's [`ModeKind`](gdtf_battle_sim::weapon::ModeKind) ([`MODE_ORDER`](super::order::MODE_ORDER)). Writing it via
/// [`set_if_neq`](bevy::prelude::DetectChangesMut::set_if_neq) marks it changed only on a
/// real change — exactly the signal `gdtf_ui`'s
/// [`repaint_segments`](gdtf_ui::repaint_segments) keys off, so the active segment repaints
/// (filled + bold) and the de-selected one returns to base the same frame (the color-blind-
/// safe active mark). No despawn/respawn — pure index write ([[ui-mutate-not-respawn]]).
///
/// Param-only (`bevy-traps.md` #7): the `Res<SelectedFireMode>` read and a
/// `Query<&mut ActiveSegment, With<ModeControl>>` write — no `&mut World`.
pub(in crate::states::running::game::battlescape) fn sync_mode_active_segment(
    fire_mode: Res<SelectedFireMode>,
    mut controls: Query<&mut ActiveSegment, (With<ModeControl>, With<SegmentedControl>)>,
) {
    let want = ActiveSegment::new(mode_index(fire_mode.kind));
    for mut active in &mut controls {
        active.set_if_neq(want);
    }
}

/// The selected weapon's [`FireModeSpec`](gdtf_battle_sim::weapon::FireModeSpec) for `kind`, read
/// back off its [`FireMode`](gdtf_battle_sim::weapon::FireMode) selector — or [`None`] when there
/// is no selection, the selection wields no weapon, or the weapon does not offer `kind`.
///
/// The single read-back point so [`mode_segment_write`] never fabricates a spec (the
/// GTW-265 "read-back, never fabricated" rule). Resolves the [`FireMode`] off the related
/// RANGED WEAPON entity (`ganger → Wields → the ranged weapon entity`, GTW-323 slice 3).
/// GTW-505 C5: a ganger wields BOTH a ranged AND a melee weapon, so it resolves through
/// [`Wields::ranged_weapon`](gdtf_battle_sim::weapon::Wields::ranged_weapon) (excluding the
/// [`MeleeWeapon`]-marked entity via the `melee` probe) — NOT `Wields::weapon`. Read-only over
/// the selection + the [`Wields`] relationship + the weapon-entity query + the melee probe.
fn mode_spec_for(
    selected: SelectedShooter,
    wields: &Query<&Wields>,
    weapons: &Query<&FireMode, With<WieldedBy>>,
    melee: &Query<(), With<MeleeWeapon>>,
    kind: ModeKind,
) -> Option<FireModeSpec> {
    let shooter = (*selected)?;
    let weapon = wields
        .get(shooter)
        .ok()
        .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))?;
    let fire_mode = weapons.get(weapon).ok()?;
    fire_mode.iter().find(|spec| spec.kind == kind).copied()
}
