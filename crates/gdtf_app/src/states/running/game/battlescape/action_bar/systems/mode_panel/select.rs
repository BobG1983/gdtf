use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{
    ChosenFireMode, FiredWeaponModes, SelectedShooter, firing_weapon_of, mode_spec_for,
    set_chosen_mode,
};
use gdtf_battle_sim::weapon::{FireMode, MeleeWeapon, MountedWeapon, WieldedBy, Wields};
use gdtf_ui::{ActiveSegment, SegmentSelected, SegmentedControl};

use super::order::{mode_for_index, mode_index};
use crate::states::running::game::battlescape::action_bar::components::ModeControl;

/// The lookups a clicked mode segment resolves its spec against.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct ClickedModeWeapon<'w, 's> {
    wields:  Query<'w, 's, &'static Wields>,
    weapons: Query<'w, 's, &'static FireMode, With<WieldedBy>>,
    mounted: Query<'w, 's, (), With<MountedWeapon>>,
    melee:   Query<'w, 's, (), With<MeleeWeapon>>,
}

pub(in crate::states::running::game::battlescape) fn mode_segment_write(
    mut picked: MessageReader<SegmentSelected>,
    selected: Res<SelectedShooter>,
    arms: ClickedModeWeapon,
    mode_controls: Query<(), With<ModeControl>>,
    mut chosen: Query<&mut ChosenFireMode>,
    mut commands: Commands,
) {
    for event in picked.read() {
        if mode_controls.get(event.control).is_err() {
            continue;
        }
        let (Some(shooter), Some(kind)) = (**selected, mode_for_index(*event.index)) else {
            continue;
        };
        let Some(weapon) = firing_weapon_of(shooter, &arms.wields, &arms.mounted, &arms.melee)
        else {
            continue;
        };
        if mode_spec_for(
            *selected,
            &arms.wields,
            &arms.weapons,
            &arms.mounted,
            &arms.melee,
            kind,
        )
        .is_none()
        {
            continue;
        }
        set_chosen_mode(weapon, kind, &arms.weapons, &mut chosen, &mut commands);
    }
}

pub(in crate::states::running::game::battlescape) fn sync_mode_active_segment(
    selected: Res<SelectedShooter>,
    arms: FiredWeaponModes,
    mut controls: Query<&mut ActiveSegment, (With<ModeControl>, With<SegmentedControl>)>,
) {
    let Some(spec) = arms.spec_of_selected(*selected) else {
        return;
    };
    let want = ActiveSegment::new(mode_index(spec.kind));
    for mut active in &mut controls {
        active.set_if_neq(want);
    }
}
