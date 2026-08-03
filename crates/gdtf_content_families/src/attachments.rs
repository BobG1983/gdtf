use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::equipment::attachments::{AttachmentName, AttachmentRegistry, AttachmentSpec};

/// authored `attachments:` keys against (a missing key fails closed — nothing
pub struct AttachmentsFamily;

impl ContentFamily for AttachmentsFamily {
    type Spec = AttachmentSpec;
    type Registry = AttachmentRegistry;

    const EXTENSION: &'static str = "attachment.ron";
    const FOLDER: &'static str = "content/attachments";

    fn insert_member(
        registry: &mut AttachmentRegistry,
        stem: Option<ContentFileStem>,
        spec: &AttachmentSpec,
    ) {
        let Some(stem) = stem else { return };
        registry.insert(AttachmentName::new(stem.into_inner()), spec.clone());
    }
}
