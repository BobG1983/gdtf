//! The ATTACHMENTS content family (GTW-549 items, generic seam since GTW-619).

use gdtf_assets::{ContentFamily, ContentFileStem};
use gdtf_battle_sim::weapon::{AttachmentName, AttachmentRegistry, AttachmentSpec};

/// The attachments family: `assets/content/attachments/*.attachment.ron` → the
/// name-keyed [`AttachmentRegistry`] the battle setup resolves each weapon's
/// authored `attachments:` keys against (a missing key fails closed — nothing
/// applied).
///
/// STEM-KEYED: `scoped_sight.attachment.ron` keys `scoped_sight` (the
/// [`AttachmentName`] key the weapon RONs reference). The last GTW-570 folder
/// hold-out: its bespoke resolve/redrive chain predated the seam and was
/// byte-shape-identical to
/// [`MeleeWeaponsFamily`](crate::MeleeWeaponsFamily)'s, so GTW-619 collapsed
/// it onto this one impl.
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
        // Stem-keyed: a handle with no resolvable path/stem is skipped
        // defensively (it would carry no usable key).
        let Some(stem) = stem else { return };
        registry.insert(AttachmentName::new(stem.into_inner()), spec.clone());
    }
}
