use gdtf_battle_sim::equipment::attachments::{AttachmentName, AttachmentRegistry};

use crate::attachment_form::AttachmentDraft;

pub(crate) fn autoload_first_attachment(
    draft: &mut AttachmentDraft,
    registry: &AttachmentRegistry,
) {
    if !draft.autoload_pending() {
        return;
    }
    let mut names: Vec<&AttachmentName> = registry.keys().collect();
    names.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    match names.first().and_then(|name| {
        registry
            .spec(name)
            .map(|spec| ((*name).clone(), spec.clone()))
    }) {
        Some((name, spec)) => draft.load_attachment(&name, &spec),
        None => draft.mark_autoloaded(),
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        equipment::attachments::{
            AttachmentName, AttachmentRegistry, AttachmentSlot, AttachmentSpec,
        },
        weapon::WeaponName,
    };

    use super::autoload_first_attachment;
    use crate::attachment_form::AttachmentDraft;

        fn spec(display: &str) -> AttachmentSpec {
        AttachmentSpec {
            display_name: WeaponName::new(display.to_owned()),
            slot:         AttachmentSlot::Sight,
            effects:      Vec::new(),
        }
    }

            #[test]
    fn seeds_first_sorted_attachment_exactly_once() {
        let registry = AttachmentRegistry::new([
            (
                AttachmentName::new("whisper_bore".to_owned()),
                spec("Whisper Bore"),
            ),
            (
                AttachmentName::new("ash_optic".to_owned()),
                spec("Ash Optic"),
            ),
        ]);
        let mut draft = AttachmentDraft::default();
        autoload_first_attachment(&mut draft, &registry);
        assert_eq!(draft.name(), "ash_optic", "sorted-first pick");

        draft.set_name("renamed".to_owned());
        autoload_first_attachment(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = AttachmentDraft::default();
        autoload_first_attachment(&mut empty_seeded, &AttachmentRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
