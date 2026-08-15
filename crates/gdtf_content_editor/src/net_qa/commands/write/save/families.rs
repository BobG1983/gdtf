//! Write one content-family draft through the same `write_*_in` its save button calls.

use std::path::Path;

use crate::{
    armor_form::{self, ArmorDraft},
    attachment_form::{self, AttachmentDraft},
    gang_form::{self, GangDraft},
    injury_form::{self, InjuryDraft},
    melee_weapon_form::{self, MeleeWeaponDraft},
    save_record::SaveOutcome,
    sprite_form::{self, SpriteDraft},
    weapon_form::{self, WeaponDraft},
};

pub(in crate::net_qa::commands::write::save) fn save_gang(
    draft: &GangDraft,
    root: &Path,
) -> SaveOutcome {
    let (name, roster) = gang_form::draft_to_roster(draft);
    SaveOutcome::from_result(gang_form::write_gang_in(root, &name, &roster))
}

pub(in crate::net_qa::commands::write::save) fn save_armor(
    draft: &ArmorDraft,
    root: &Path,
) -> SaveOutcome {
    let (name, spec) = armor_form::draft_to_spec(draft);
    SaveOutcome::from_result(armor_form::write_armor_in(root, &name, &spec))
}

pub(in crate::net_qa::commands::write::save) fn save_injury(
    draft: &InjuryDraft,
    root: &Path,
) -> SaveOutcome {
    let (key, def) = injury_form::draft_to_def(draft);
    SaveOutcome::from_result(injury_form::write_injury_in(root, &key, &def))
}

pub(in crate::net_qa::commands::write::save) fn save_sprite(
    draft: &SpriteDraft,
    root: &Path,
) -> SaveOutcome {
    let (name, def) = sprite_form::draft_to_sprite_def(draft);
    SaveOutcome::from_result(sprite_form::write_sprite_in(root, &name, &def))
}

pub(in crate::net_qa::commands::write::save) fn save_attachment(
    draft: &AttachmentDraft,
    root: &Path,
) -> SaveOutcome {
    let (name, spec) = attachment_form::draft_to_attachment_spec(draft);
    SaveOutcome::from_result(attachment_form::write_attachment_in(root, &name, &spec))
}

pub(in crate::net_qa::commands::write::save) fn save_weapon(
    draft: &WeaponDraft,
    root: &Path,
) -> SaveOutcome {
    let (name, spec) = weapon_form::draft_to_weapon_spec(draft);
    SaveOutcome::from_result(weapon_form::write_weapon_in(root, &name, &spec))
}

pub(in crate::net_qa::commands::write::save) fn save_melee_weapon(
    draft: &MeleeWeaponDraft,
    root: &Path,
) -> SaveOutcome {
    let (name, spec) = melee_weapon_form::draft_to_melee_weapon_spec(draft);
    SaveOutcome::from_result(melee_weapon_form::write_melee_weapon_in(root, &name, &spec))
}
