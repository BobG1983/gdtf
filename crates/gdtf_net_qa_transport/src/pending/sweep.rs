//! The frame-deadline pump a host registers per payload type (GTW-736).

use bevy::prelude::*;

use super::queue::PendingQueue;

/// The deadline pump for ONE pending-queue kind — ticks every entry and times out any
/// that expired unclaimed. Registered once per payload type by the host's plugin.
pub fn sweep_pending<P: Send + Sync + core::fmt::Debug + 'static>(
    mut queue: ResMut<PendingQueue<P>>,
) {
    // Read emptiness through the immutable accessor so an idle frame never dirties the
    // resource's change-detection flag.
    if queue.is_empty() {
        return;
    }
    queue.sweep_expired();
}
