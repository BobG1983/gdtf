use gdtf_qa_protocol::ids::ShotName;

#[derive(Debug)]
pub struct EditorScreenshotPayload(Option<ShotName>);

impl EditorScreenshotPayload {
        #[must_use]
    pub const fn new(name: Option<ShotName>) -> Self {
        Self(name)
    }

        pub(in crate::net_qa) const fn name(&self) -> Option<&ShotName> {
        self.0.as_ref()
    }
}
