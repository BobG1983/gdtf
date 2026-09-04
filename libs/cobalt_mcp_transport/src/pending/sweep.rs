//! System that times out unclaimed pending requests.

use bevy::prelude::*;

use super::queue::PendingQueue;

/// Tick deadlines on the pending queue and reply Timeout for expired entries.
pub fn sweep_pending<P: Send + Sync + core::fmt::Debug + 'static>(
    mut queue: ResMut<PendingQueue<P>>,
) {
    if queue.is_empty() {
        return;
    }
    queue.sweep_expired();
}
