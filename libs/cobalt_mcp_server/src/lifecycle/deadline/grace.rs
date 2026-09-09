use std::time::Instant;

use super::at::DeadlineAt;
use crate::lifecycle::values::KillGrace;

// The instant a grace period runs out, or None when it never does.
#[must_use]
pub(in crate::lifecycle) fn grace_deadline(now: Instant, grace: KillGrace) -> Option<DeadlineAt> {
    now.checked_add(*grace).map(DeadlineAt::new)
}
