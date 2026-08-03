use bevy::prelude::*;

use super::queue::PendingQueue;

pub fn sweep_pending<P: Send + Sync + core::fmt::Debug + 'static>(
    mut queue: ResMut<PendingQueue<P>>,
) {
    if queue.is_empty() {
        return;
    }
    queue.sweep_expired();
}
