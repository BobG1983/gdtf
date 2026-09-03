//! The content types this build can delete a record from.
mod armor;
mod attachment;
mod field;
mod gangs;
mod injury;
mod melee_weapon;
mod prefab;
mod weighting;

#[cfg(test)]
pub(crate) use armor::ARMOR_FAMILY;
pub(crate) use armor::armor_delete_entry;
#[cfg(test)]
pub(crate) use attachment::ATTACHMENT_FAMILY;
pub(crate) use attachment::attachment_delete_entry;
#[cfg(test)]
pub(crate) use field::FIELD_FAMILY;
pub(crate) use field::field_delete_entry;
#[cfg(test)]
pub(crate) use injury::INJURY_FAMILY;
pub(crate) use injury::injury_delete_entry;
#[cfg(test)]
pub(crate) use melee_weapon::MELEE_WEAPON_FAMILY;
pub(crate) use melee_weapon::melee_weapon_delete_entry;
#[cfg(test)]
pub(crate) use prefab::PREFAB_FAMILY;
pub(crate) use prefab::prefab_delete_entry;
#[cfg(test)]
pub(crate) use weighting::WEIGHTING_FAMILY;
pub(crate) use weighting::weighting_delete_entry;
