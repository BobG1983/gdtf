use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::weapon::{ModeKind, WieldsChanged};

use super::{
    order::MODE_ORDER,
    probes::{ControlSegments, OfferedFireModes},
};
use crate::states::running::game::battlescape::action_bar::components::{
    ModeControl, ModePanelRoot,
};

type ModeControlChildren = (Entity, &'static Children);

pub(in crate::states::running::game::battlescape) fn rebuild_mode_segments(
    selected: Res<SelectedShooter>,
    offered: OfferedFireModes,
    added_controls: Query<(), Added<ModeControl>>,
    mut wields_changed: WieldsChanged,
    mut segments: ControlSegments,
    controls: Query<ModeControlChildren, With<ModeControl>>,
    mut panels: Query<&mut Visibility, With<ModePanelRoot>>,
) {
    let control_just_spawned = added_controls.iter().next().is_some();
    let armament_moved = wields_changed.any();
    if !selected.is_changed() && !control_just_spawned && !armament_moved {
        return;
    }

    let shooter = **selected;
    let offers = |kind: ModeKind| offered.offers(shooter, kind);

    for (control, _) in &controls {
        for (index, kind) in MODE_ORDER.iter().enumerate() {
            segments.set_visible(control, index, *offers(*kind));
        }
    }

    let any_offered = MODE_ORDER.iter().any(|k| *offers(*k));
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
