use crate::{
    armor_form::ArmorDraft,
    attachment_form::AttachmentDraft,
    gang_form::GangDraft,
    injury_form::InjuryDraft,
    melee_weapon_form::MeleeWeaponDraft,
    net_qa::commands::write::blank::families::{
        blank_armor, blank_attachment, blank_gang, blank_injury, blank_melee_weapon, blank_sprite,
        blank_weapon,
    },
    sprite_form::SpriteDraft,
    weapon_form::WeaponDraft,
};

// A default draft is still waiting to be seeded; only a `new_*` draft is settled.
const STILL_PENDING: &str = "the blank helper left the draft's autoload flag Pending, so the form's own sync would seed \
     the first sorted registry entry over it on the next egui frame — which is what a draft \
     built with `default()` instead of that form's `new_*` constructor does";

#[test]
fn a_blanked_gang_is_not_reseeded_by_the_form() {
    let mut draft = GangDraft::default();
    assert!(draft.autoload_pending(), "a default draft starts Pending");
    blank_gang(&mut draft);
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_blanked_armor_is_not_reseeded_by_the_form() {
    let mut draft = ArmorDraft::default();
    assert!(draft.autoload_pending(), "a default draft starts Pending");
    blank_armor(&mut draft);
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_blanked_injury_is_not_reseeded_by_the_form() {
    let mut draft = InjuryDraft::default();
    assert!(draft.autoload_pending(), "a default draft starts Pending");
    blank_injury(&mut draft);
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_blanked_sprite_is_not_reseeded_by_the_form() {
    let mut draft = SpriteDraft::default();
    assert!(draft.autoload_pending(), "a default draft starts Pending");
    blank_sprite(&mut draft);
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_blanked_attachment_is_not_reseeded_by_the_form() {
    let mut draft = AttachmentDraft::default();
    assert!(draft.autoload_pending(), "a default draft starts Pending");
    blank_attachment(&mut draft);
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_blanked_weapon_is_not_reseeded_by_the_form() {
    let mut draft = WeaponDraft::default();
    assert!(draft.autoload_pending(), "a default draft starts Pending");
    blank_weapon(&mut draft);
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}

#[test]
fn a_blanked_melee_weapon_is_not_reseeded_by_the_form() {
    let mut draft = MeleeWeaponDraft::default();
    assert!(draft.autoload_pending(), "a default draft starts Pending");
    blank_melee_weapon(&mut draft);
    assert!(!draft.autoload_pending(), "{STILL_PENDING}");
}
