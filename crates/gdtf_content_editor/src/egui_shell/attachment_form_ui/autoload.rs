//! The ATTACHMENT mode's ONE-SHOT open-with-an-item seed (GTW-669) — the parity twin of
//! the Gang / Armor / Injury / Sprite autoloads, run by the shell on the first
//! Attachment-mode frame.

use gdtf_battle_sim::equipment::attachments::{AttachmentName, AttachmentRegistry};

use crate::attachment_form::AttachmentDraft;

/// Seed a still-pristine [`AttachmentDraft`] from the resolved [`AttachmentRegistry`] —
/// the FIRST item by sorted [`AttachmentName`] (registry iteration order is unspecified,
/// so the keys are sorted for a deterministic pick — the Gang / Armor / Sprite modes'
/// exact open behavior), or leave the form empty when no items are loaded. Either way
/// the one-shot seed is marked done, so it never clobbers later edits / a deliberate
/// "New attachment" (idempotent under the egui multipass re-run — the first pass ends
/// the pending state).
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
        // No items loaded — start empty (the Gang / Armor modes' empty-registry branch).
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

    /// A minimal named cosmetic item for registry seeding.
    fn spec(display: &str) -> AttachmentSpec {
        AttachmentSpec {
            display_name: WeaponName::new(display.to_owned()),
            slot:         AttachmentSlot::Sight,
            effects:      Vec::new(),
        }
    }

    /// The one-shot seed loads the FIRST item by sorted key; a second call is a no-op
    /// (multipass idempotency); an empty registry just ends the pending state.
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

        // A later edit is never clobbered by a re-run.
        draft.set_name("renamed".to_owned());
        autoload_first_attachment(&mut draft, &registry);
        assert_eq!(draft.name(), "renamed");

        let mut empty_seeded = AttachmentDraft::default();
        autoload_first_attachment(&mut empty_seeded, &AttachmentRegistry::default());
        assert!(!empty_seeded.autoload_pending());
        assert_eq!(empty_seeded.name(), "");
    }
}
