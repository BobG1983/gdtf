use bevy::prelude::*;
use gdtf_battle_input::{SelectedFireMode, SelectedShooter, mode_spec_for};
use gdtf_battle_sim::weapon::{FireMode, MeleeWeapon, WieldedBy, Wields};
use gdtf_ui::{ActiveSegment, SegmentSelected, SegmentedControl};

use super::order::{mode_for_index, mode_index};
use crate::states::running::game::battlescape::action_bar::components::ModeControl;

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
        let Some(spec) = mode_spec_for(*selected, &wields, &weapons, &melee, kind) else {
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
