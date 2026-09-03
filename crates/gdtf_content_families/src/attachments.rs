use gdtf_assets::{ContentFamily, ContentFileStem, ContentMemberKey};
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
    ) -> Option<ContentMemberKey> {
        let key = stem?.into_inner();
        registry.insert(AttachmentName::new(key.clone()), spec.clone());
        Some(ContentMemberKey::new(key))
    }
}
