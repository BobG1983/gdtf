//! The content types this build can delete a record from.
mod prefab;
mod weighting;

#[cfg(test)]
pub(crate) use prefab::PREFAB_FAMILY;
pub(crate) use prefab::prefab_delete_entry;
#[cfg(test)]
pub(crate) use weighting::WEIGHTING_FAMILY;
pub(crate) use weighting::weighting_delete_entry;
