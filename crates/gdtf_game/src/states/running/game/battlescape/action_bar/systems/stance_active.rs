use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};
use gdtf_battle_sim::prelude::{Stance, StanceKind};
use gdtf_ui::{ActiveSegment, SegmentSelected, SegmentedControl};

use super::stance_panel::{stance_for_index, stance_index};
use crate::states::running::game::battlescape::action_bar::components::StanceControl;

pub(in crate::states::running::game::battlescape) fn stance_segment_intent(
    mut chosen: MessageReader<SegmentSelected>,
    mut pending: ResMut<PendingActIntent>,
    stance_controls: Query<(), With<StanceControl>>,
) {
    for event in chosen.read() {
        if stance_controls.get(event.control).is_err() {
            continue;
        }
        if let Some(kind) = stance_for_index(*event.index) {
            pending.push(ActIntent::SetStance(kind));
        }
    }
}

pub(in crate::states::running::game::battlescape) fn sync_stance_active_segment(
    selected: Res<SelectedShooter>,
    stances: Query<&Stance>,
    mut controls: Query<&mut ActiveSegment, (With<StanceControl>, With<SegmentedControl>)>,
) {
    let current: Option<StanceKind> = (**selected)
        .and_then(|entity| stances.get(entity).ok())
        .map(|stance| **stance);
    let Some(kind) = current else {
        return;
    };
    let want = ActiveSegment::new(stance_index(kind));
    for mut active in &mut controls {
        active.set_if_neq(want);
    }
}
