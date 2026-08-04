use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    ganger::{Aiming, TuMax},
    magazine::mode_tu_cost,
    tuning::CombatTuning,
    weapon::{FireMode, MeleeWeapon, WieldedBy, Wields},
};
use gdtf_ui::{
    Segment, SegmentColors, SegmentIndex, SegmentSubLabel, SegmentSubText, set_segment_sub_line,
};

use super::order::MODE_ORDER;
use crate::states::running::game::battlescape::action_bar::components::ModeControl;

type CostShooter = (&'static TuMax, &'static Aiming);

type CostWeapon = &'static FireMode;

type ModeSegment = (&'static SegmentIndex, &'static Children);

#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct ModeCostInputs<'w, 's> {
    selected:       Res<'w, SelectedShooter>,
    tuning:         Res<'w, CombatTuning>,
    shooters:       Query<'w, 's, CostShooter>,
    wields:         Query<'w, 's, &'static Wields>,
    weapons:        Query<'w, 's, CostWeapon, With<WieldedBy>>,
    melee:          Query<'w, 's, (), With<MeleeWeapon>>,
    aim_changed:    Query<'w, 's, (), Changed<Aiming>>,
    added_controls: Query<'w, 's, (), Added<ModeControl>>,
}

#[expect(
    clippy::type_complexity,
    reason = "param tuple aliased where possible; the set_segment_sub_line call signature \
    fixes the controls / segments / sub-texts query shapes"
)]
pub(in crate::states::running::game::battlescape) fn sync_mode_tu_cost_lines(
    mut commands: Commands,
    inputs: ModeCostInputs,
    mode_controls: Query<Entity, With<ModeControl>>,
    controls: Query<(&Children, &SegmentColors)>,
    segments: Query<ModeSegment, With<Segment>>,
    mut sub_texts: Query<&mut Text, With<SegmentSubText>>,
) {
    let control_just_spawned = inputs.added_controls.iter().next().is_some();
    let aim_flipped = inputs.aim_changed.iter().next().is_some();
    if !inputs.selected.is_changed() && !aim_flipped && !control_just_spawned {
        return;
    }

    let ganger = **inputs.selected;
    let shooter_inputs = ganger.and_then(|shooter| inputs.shooters.get(shooter).ok());
    let weapon_mode = ganger
        .and_then(|shooter| inputs.wields.get(shooter).ok())
        .and_then(|w| w.ranged_weapon(|entity| inputs.melee.get(entity).is_ok()))
        .and_then(|weapon| inputs.weapons.get(weapon).ok());

    for control in &mode_controls {
        for (index, mode) in MODE_ORDER.iter().enumerate() {
            let label = shooter_inputs
                .zip(weapon_mode)
                .and_then(|((tu_max, aiming), weapon)| {
                    let spec = weapon.iter().find(|s| s.kind == *mode)?;
                    let cost = mode_tu_cost(spec, tu_max, aiming, &inputs.tuning);
                    Some(SegmentSubLabel::new(format!("{} TU", *cost)))
                });
            set_segment_sub_line(
                &mut commands,
                control,
                index,
                label.as_ref(),
                &controls,
                &segments,
                &mut sub_texts,
            );
        }
    }
}
