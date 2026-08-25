use std::path::Path;

use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    equipment::attachments::{AttachmentName, AttachmentSlot, AttachmentSpec},
    weapon::{
        FireModeSpec, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, WeaponName, WeaponPunch,
    },
};

use super::{
    draft::AttachmentDraft,
    save::{attachment_file_name, attachment_save_path_in, draft_to_attachment_spec},
};

fn authored_spec() -> AttachmentSpec {
    AttachmentSpec {
        display_name: WeaponName::new("Whisper Choke".to_owned()),
        slot:         AttachmentSlot::Muzzle,
        effects:      vec![
            AttachmentEffect::Silence,
            AttachmentEffect::Penetration(WeaponPunch::new(3)),
        ],
    }
}

/// schema's `#[serde(default)]` empty list), an empty display name, and the palette's
#[test]
fn empty_effects_list_is_legal_and_reachable() {
    let mut draft = AttachmentDraft::new_attachment();
    assert!(
        draft.spec().effects.is_empty(),
        "a fresh attachment seeds the cosmetic identity (no effects)",
    );
    assert_eq!(draft.spec().slot, AttachmentSlot::Muzzle);

    draft
        .spec_mut()
        .effects
        .push(AttachmentEffect::Aim(AimDelta::new(0.4)));
    draft.spec_mut().effects.push(AttachmentEffect::Brace);
    assert_eq!(draft.spec().effects.len(), 2);
    draft.spec_mut().effects.remove(1);
    draft.spec_mut().effects.remove(0);
    assert!(
        draft.spec().effects.is_empty(),
        "removing every effect is legal — an empty list is the cosmetic identity",
    );

    let (_, spec) = draft_to_attachment_spec(&draft);
    assert_eq!(spec.effects, []);
}

#[test]
fn attachment_effects_remove_down_to_empty() {
    let mut draft = AttachmentDraft::new_attachment();
    draft.add_effect();
    draft.add_effect();
    assert_eq!(draft.effects().len(), 2, "both added effects landed");

    assert!(
        draft.set_effect(0, AttachmentEffect::Silence),
        "index 0 is in bounds",
    );
    assert!(
        matches!(draft.effects()[0], AttachmentEffect::Silence),
        "the in-bounds write landed on the first row",
    );
    assert!(
        !draft.set_effect(9, AttachmentEffect::Silence),
        "index 9 is past the end of a two-effect list",
    );

    assert!(draft.remove_effect(0), "the first removal takes a row out");
    assert!(
        draft.remove_effect(0),
        "the second removal takes the last row out. This list has no minimum",
    );
    assert!(
        draft.effects().is_empty(),
        "an empty list is the cosmetic identity",
    );
    assert!(
        !draft.remove_effect(0),
        "removing from an empty list reports nothing removed",
    );
}

#[test]
fn load_is_verbatim_and_projection_trims_the_name() {
    let authored = authored_spec();
    let key = AttachmentName::new("whisper_choke".to_owned());

    let mut draft = AttachmentDraft::default();
    assert!(draft.autoload_pending(), "a default draft awaits the seed");
    draft.load_attachment(&key, &authored);
    assert!(!draft.autoload_pending(), "a load ends the one-shot seed");
    assert_eq!(draft.name(), "whisper_choke");
    assert_eq!(draft.spec(), &authored, "load copies the spec verbatim");

    draft.set_name("  re keyed  ".to_owned());
    let (name, spec) = draft_to_attachment_spec(&draft);
    assert_eq!(name.as_str(), "re keyed", "the projection trims the buffer");
    assert_eq!(
        &spec,
        draft.spec(),
        "the projection copies the working spec"
    );
}

#[test]
fn gain_fire_mode_payload_edits_in_place() {
    let mut draft = AttachmentDraft::new_attachment();
    draft
        .spec_mut()
        .effects
        .push(AttachmentEffect::GainFireMode(FireModeSpec::new(
            ModeKind::Single,
            ModeConeMult::new(1.0),
            ModeTuPercent::new(0.0),
            ModeShots::new(1),
        )));
    if let Some(AttachmentEffect::GainFireMode(mode)) = draft.spec_mut().effects.get_mut(0) {
        mode.kind = ModeKind::Burst;
        mode.shots = ModeShots::new(3);
    }
    let Some(AttachmentEffect::GainFireMode(mode)) = draft.spec().effects.first() else {
        unreachable!("the pushed GainFireMode effect is still row 0");
    };
    assert_eq!(mode.kind, ModeKind::Burst, "the kind edit landed");
    assert_eq!(*mode.shots, 3, "the shots edit landed");
}

#[test]
fn save_path_derives_from_the_family_consts() {
    let name = AttachmentName::new("Gore-Sump Drum (mk II)".to_owned());
    assert_eq!(
        attachment_file_name(&name),
        "gore_sump_drum_mk_ii.attachment.ron"
    );
    assert_eq!(
        attachment_save_path_in(Path::new("/tmp/assets"), &name),
        Path::new("/tmp/assets/content/attachments/gore_sump_drum_mk_ii.attachment.ron"),
    );
    assert_eq!(
        attachment_file_name(&AttachmentName::new("!!!".to_owned())),
        "unnamed_attachment.attachment.ron",
        "a name that sanitizes to nothing falls back to the documented stem",
    );
}
