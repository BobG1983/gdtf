//! [`ScreenshotPayload`] — what a queued capture carries (GTW-740, re-homed by GTW-943).

use gdtf_qa_protocol::ids::ShotName;

crate::support_item! {
    /// The pending payload for one capture: the optional file stem the caller asked for.
    ///
    /// It lives beside the pump that reads it rather than in a shared payload module,
    /// because the pump is the only thing that has ever read it — the editor's capture
    /// pump has always kept its payload the same way.
    ///
    /// The `test-support` visibility flip is what lets the pump's own suite, and the
    /// GTW-943 capture test, enqueue a capture through the REAL queue instead of a stub.
    /// Nothing on the wire pushes one today: the `TakeScreenshot` request went with the
    /// rest of the pre-command vocabulary, and GTW-945's `capture.screenshot` command is
    /// what fills this queue next.
    struct ScreenshotPayload(Option<ShotName>);
}

impl ScreenshotPayload {
    crate::support_item! {
        /// Wrap the optional capture stem.
        ///
        /// Gated to test / `test-support` builds: nothing on the wire pushes a capture today
        /// (the request that used to do so went with GTW-943's cut), so a plain build would carry
        /// an uncalled constructor.
        #[cfg(any(test, feature = "test-support"))]
        const fn new(name: Option<ShotName>) -> Self {
            Self(name)
        }
    }

    /// The wrapped stem — the pump's read (borrows without consuming; the pump confines it
    /// into a path).
    pub(in crate::dev::net_qa) const fn name(&self) -> Option<&ShotName> {
        self.0.as_ref()
    }
}

impl core::fmt::Debug for ScreenshotPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ScreenshotPayload").field(&self.0).finish()
    }
}
