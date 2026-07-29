//! The editor's pending-request payload for a queued screenshot (GTW-880).

use gdtf_qa_protocol::ids::ShotName;

/// The pending payload for a
/// [`TakeScreenshot`](gdtf_qa_protocol::envelope::QaRequest::TakeScreenshot) the editor's
/// router queued — the optional file stem the client asked for, carried to the
/// [`pump`](super::pump) that claims it.
///
/// `Debug` because [`sweep_pending`](gdtf_net_qa_transport::sweep_pending) names the payload
/// in the log line it writes when an entry expires unclaimed; the derive is enough here
/// because [`ShotName`] is itself `Debug`.
#[derive(Debug)]
pub(in crate::net_qa) struct EditorScreenshotPayload(Option<ShotName>);

impl EditorScreenshotPayload {
    /// Queue the client's optional file stem.
    pub(in crate::net_qa) const fn new(name: Option<ShotName>) -> Self {
        Self(name)
    }

    /// The file stem the client asked for, or `None` for a server-chosen name.
    pub(in crate::net_qa) const fn name(&self) -> Option<&ShotName> {
        self.0.as_ref()
    }
}
