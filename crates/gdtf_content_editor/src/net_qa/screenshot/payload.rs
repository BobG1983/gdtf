//! The editor's pending-request payload for a queued capture (GTW-880, re-homed onto the
//! command layer by GTW-943).

use gdtf_qa_protocol::ids::ShotName;

/// What one queued capture carries — the optional file stem the caller asked for, read by
/// the capture pump that claims it.
///
/// `Debug` because [`sweep_pending`](gdtf_net_qa_transport::sweep_pending) names the payload
/// in the log line it writes when an entry expires unclaimed; the derive is enough here
/// because [`ShotName`] is itself `Debug`.
///
/// Nothing on the wire pushes one today: the `TakeScreenshot` request that used to do so went
/// with the rest of the pre-command vocabulary in GTW-943, and the editor's capture command is
/// the next editor ticket. It is `pub` (behind the crate's `net_qa` gate) so the capture suite
/// enqueues through the REAL queue rather than a stub — which is what keeps the settle window,
/// the confinement, the poll loop and the PNG verification under test in the meantime.
#[derive(Debug)]
pub struct EditorScreenshotPayload(Option<ShotName>);

impl EditorScreenshotPayload {
    /// Queue the caller's optional file stem.
    #[must_use]
    pub const fn new(name: Option<ShotName>) -> Self {
        Self(name)
    }

    /// The file stem the caller asked for, or `None` for a host-chosen name.
    pub(in crate::net_qa) const fn name(&self) -> Option<&ShotName> {
        self.0.as_ref()
    }
}
