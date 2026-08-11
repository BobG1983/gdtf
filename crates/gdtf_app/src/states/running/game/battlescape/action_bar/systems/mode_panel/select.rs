use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{SelectedFireMode, SelectedShooter, mode_spec_for};
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
    mut chosen: MessageReader<SegmentSelected>,
    mut fire_mode: ResMut<SelectedFireMode>,
    selected: Res<SelectedShooter>,
    arms: ClickedModeWeapon,
    mode_controls: Query<(), With<ModeControl>>,
) {
    for event in chosen.read() {
        if mode_controls.get(event.control).is_err() {
            continue;
        }
        let Some(kind) = mode_for_index(*event.index) else {
            continue;
        };
        let Some(spec) = mode_spec_for(
            *selected,
            &arms.wields,
            &arms.weapons,
            &arms.mounted,
            &arms.melee,
            kind,
        ) else {
            continue;
        };
        let next = SelectedFireMode::new(spec);
        if *fire_mode != next {
            *fire_mode = next;
        }
    }
}

pub(in crate::states::running::game::battlescape) fn sync_mode_active_segment(
    fire_mode: Res<SelectedFireMode>,
    mut controls: Query<&mut ActiveSegment, (With<ModeControl>, With<SegmentedControl>)>,
) {
    let want = ActiveSegment::new(mode_index(fire_mode.kind));
    for mut active in &mut controls {
        active.set_if_neq(want);
    }
}
