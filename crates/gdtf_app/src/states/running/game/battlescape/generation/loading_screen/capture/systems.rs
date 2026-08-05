use bevy::prelude::*;
use gdtf_screenshot::{CaptureCompletions, CapturePath, CaptureQueue};

use crate::states::BattleScapeState;

pub(super) fn request_loading_shot(
    path: Res<CapturePath>,
    completions: Res<CaptureCompletions<()>>,
    mut queue: ResMut<CaptureQueue<()>>,
) {
    if !queue.is_idle() || !completions.is_empty() {
        return;
    }
    queue.push_to(path.clone(), ());
}

pub(super) fn pin_generation_until_shot(
    queue: Res<CaptureQueue<()>>,
    completions: Res<CaptureCompletions<()>>,
    mut next: ResMut<NextState<BattleScapeState>>,
) {
    if queue.is_idle() && !completions.is_empty() {
        return;
    }
    next.set(BattleScapeState::Generation);
}
