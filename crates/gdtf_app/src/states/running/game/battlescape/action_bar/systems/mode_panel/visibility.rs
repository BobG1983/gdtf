use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::weapon::{FireMode, MeleeWeapon, ModeKind, WieldedBy, Wields};
use gdtf_ui::{Segment, SegmentIndex, set_segment_visible};

use super::order::MODE_ORDER;
use crate::states::running::game::battlescape::action_bar::components::{
    ModeControl, ModePanelRoot,
};

type ModeControlChildren = (Entity, &'static Children);

#[allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "Wields and melee probe resolve offered modes off the ranged weapon entity"
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
    let control_just_spawned = added_controls.iter().next().is_some();
    if !selected.is_changed() && !control_just_spawned {
        return;
    }

    let offered = (**selected)
        .and_then(|shooter| wields.get(shooter).ok())
        .and_then(|w| w.ranged_weapon(|entity| melee.get(entity).is_ok()))
        .and_then(|weapon| weapons.get(weapon).ok());
    let offers =
        |kind: ModeKind| offered.is_some_and(|weapon| weapon.iter().any(|m| m.kind == kind));

    for (control, _) in &controls {
        for (index, kind) in MODE_ORDER.iter().enumerate() {
            set_segment_visible(control, index, offers(*kind), &children, &mut segments);
        }
    }

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
