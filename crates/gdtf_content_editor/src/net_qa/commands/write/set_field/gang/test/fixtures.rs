use gdtf_battle_sim::{
    armor::{ArmorName, ArmorPiece, ArmorRegistry, ArmorSpec},
    weapon::{MeleeWeaponRegistry, WeaponName, WeaponRegistry},
};

use crate::{
    gang_form::GangDraft,
    melee_weapon_form::MeleeWeaponDraft,
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{EditorFieldNet, EditorKeyNet, EditorListIndexNet},
    },
    weapon_form::WeaponDraft,
};

/// The one key every fixture registry below holds.
pub(super) const SEEDED_KEY: &str = "alpha_seed";

/// The member every case writes.
pub(super) const FIRST: EditorListIndexNet = EditorListIndexNet::new(0);

/// A key as a client sends it over the wire.
pub(super) fn asked_key(key: &str) -> EditorKeyNet {
    EditorKeyNet::new(key.to_owned())
}

/// A draft holding one default member, as an Add on the roster leaves it.
pub(super) fn one_member() -> GangDraft {
    let mut draft = GangDraft::new_gang();
    draft.add_member();
    draft
}

/// A weapon registry holding the seeded key.
pub(super) fn weapons() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(SEEDED_KEY.to_owned()),
        WeaponDraft::new_weapon().spec().clone(),
    )])
}

/// A melee weapon registry holding the seeded key.
pub(super) fn melee_weapons() -> MeleeWeaponRegistry {
    MeleeWeaponRegistry::new([(
        WeaponName::new(SEEDED_KEY.to_owned()),
        MeleeWeaponDraft::new_melee_weapon().spec().clone(),
    )])
}

/// An armor registry holding the seeded key.
pub(super) fn armor() -> ArmorRegistry {
    ArmorRegistry::new([(
        ArmorName::new(SEEDED_KEY.to_owned()),
        ArmorSpec::uniform(ArmorPiece::default()),
    )])
}

/// The field a write answered, with any refusal flattened to the line it carries.
pub(super) fn answered(
    result: Result<EditorFieldNet, FormWriteFault>,
) -> Result<EditorFieldNet, String> {
    result.map_err(|fault| match fault {
        FormWriteFault::ForeignArm => "the arm was read as another form's".to_owned(),
        FormWriteFault::Gated(note) | FormWriteFault::MissingModel(note) => {
            note.as_str().to_owned()
        }
        FormWriteFault::BadArguments(detail) => detail.as_str().to_owned(),
    })
}

/// Which refusal a write answered and the line it carries, or the field it wrote instead.
pub(super) fn refusal(
    result: Result<EditorFieldNet, FormWriteFault>,
) -> Result<(&'static str, String), String> {
    match result {
        Ok(field) => Err(format!("expected a refusal, got {field:?}")),
        Err(FormWriteFault::ForeignArm) => Ok(("ForeignArm", String::new())),
        Err(FormWriteFault::Gated(note)) => Ok(("Gated", note.as_str().to_owned())),
        Err(FormWriteFault::MissingModel(note)) => Ok(("MissingModel", note.as_str().to_owned())),
        Err(FormWriteFault::BadArguments(detail)) => {
            Ok(("BadArguments", detail.as_str().to_owned()))
        }
    }
}
