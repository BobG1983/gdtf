use gdtf_qa_protocol::ids::ShotName;

crate::support_item! {
                                                struct ScreenshotPayload(Option<ShotName>);
}

impl ScreenshotPayload {
    crate::support_item! {
        /// Carry the requested shot name, if one was given.
        #[cfg(any(test, feature = "headless_test"))]
        const fn new(name: Option<ShotName>) -> Self {
            Self(name)
        }
    }

    pub(in crate::dev::net_qa) const fn name(&self) -> Option<&ShotName> {
        self.0.as_ref()
    }
}

impl core::fmt::Debug for ScreenshotPayload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ScreenshotPayload").field(&self.0).finish()
    }
}
