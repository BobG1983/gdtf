//! The three registries the member row's own combo boxes read their options from.

use cobalt_mcp_protocol::command::RefusalNote;
use gdtf_battle_sim::{
    armor::{ArmorName, ArmorRegistry},
    weapon::{MeleeWeaponRegistry, WeaponName, WeaponRegistry},
};

use crate::mcp::{commands::write::form_fault::FormWriteFault, wire::EditorKeyNet};

const NO_WEAPONS: RefusalNote = RefusalNote::from_static(
    "the member's weapon combo reads the weapon registry, and it holds no key, so the form \
     itself offers no name to choose",
);

const NO_MELEE: RefusalNote = RefusalNote::from_static(
    "the member's melee combo reads the melee weapon registry, and it holds no key, so the form \
     itself offers no name to choose",
);

const NO_ARMOR: RefusalNote = RefusalNote::from_static(
    "the member's armor combo reads the armor registry, and it holds no key, so the form itself \
     offers no name to choose",
);

// The line a key none of the combo's rows carry is refused with.
fn unknown(key: &EditorKeyNet, combo: &str) -> FormWriteFault {
    FormWriteFault::bad(format!(
        "`{}` is not a key the {combo} registry holds, so the member's {combo} combo offers no \
         such row",
        key.as_str(),
    ))
}

/// The weapon name the combo offers for this key, or why it offers none.
pub(super) fn weapon(
    registry: Option<&WeaponRegistry>,
    key: &EditorKeyNet,
) -> Result<WeaponName, FormWriteFault> {
    let Some(registry) = registry.filter(|registry| !registry.is_empty()) else {
        return Err(FormWriteFault::MissingModel(NO_WEAPONS));
    };
    let wanted = WeaponName::new(key.as_str().to_owned());
    if registry.spec(&wanted).is_some() {
        Ok(wanted)
    } else {
        Err(unknown(key, "weapon"))
    }
}

/// The melee weapon name the combo offers for this key, or why it offers none.
pub(super) fn melee_weapon(
    registry: Option<&MeleeWeaponRegistry>,
    key: &EditorKeyNet,
) -> Result<WeaponName, FormWriteFault> {
    let Some(registry) = registry.filter(|registry| !registry.is_empty()) else {
        return Err(FormWriteFault::MissingModel(NO_MELEE));
    };
    let wanted = WeaponName::new(key.as_str().to_owned());
    if registry.spec(&wanted).is_some() {
        Ok(wanted)
    } else {
        Err(unknown(key, "melee"))
    }
}

/// The armor name the combo offers for this key, or why it offers none.
pub(super) fn armor(
    registry: Option<&ArmorRegistry>,
    key: &EditorKeyNet,
) -> Result<ArmorName, FormWriteFault> {
    let Some(registry) = registry.filter(|registry| !registry.is_empty()) else {
        return Err(FormWriteFault::MissingModel(NO_ARMOR));
    };
    let wanted = ArmorName::new(key.as_str().to_owned());
    if registry.spec(&wanted).is_some() {
        Ok(wanted)
    } else {
        Err(unknown(key, "armor"))
    }
}
