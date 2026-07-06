//! The **type vocabulary** of the save path (GTW-432; swept onto the v2 UUID schema in
//! GTW-495): the [`SavePrefabError`] failure enum, the authored spawn-role default, and the
//! size directory helper. (GTW-512: the `bevy_ui` save-widget markers were dropped — the egui
//! save controls are the C4 child. GTW-634: the path constants — assets root, prefab folder,
//! compound extension — and the theme-dir helper moved to their single owners:
//! [`gdtf_assets::WORKSPACE_ASSETS_ROOT`], [`gdtf_content_families::prefabs`], and
//! [`crate::theme_dir`].)

use gdtf_assets::RonSaveError;
use gdtf_battle_sim::{
    level::{GridSize, SpawnRole},
    metric::CellLevel,
};

/// The spawn role a saved prefab is authored with (GTW-432) — the connective
/// [`Fill`](SpawnRole::Fill) default.
///
/// The editor has no spawn-role control, so every saved fragment is a generic FILL fragment (the
/// schema's serde-default role) — the assembler buckets it under the connective interior. A
/// future spawn-role selector would override this; until then Fill is the safe, documented
/// default.
pub(super) const SAVED_SPAWN_ROLE: SpawnRole = SpawnRole::Fill;

// GTW-512: the prefab-NAME field + "Save prefab" button markers were DROPPED here — they only ever
// marked the deleted `bevy_ui` save controls. The C4 child (GTW-515) re-adds the egui save controls
// + their markers when it re-points the save trigger.

/// Why a prefab save was REJECTED — the handled, no-panic failure of the save path (GTW-432;
/// the old zero-opening rejection variant was DROPPED in GTW-495, the schema authors no
/// openings).
///
/// A named domain enum (no-bare-types: the rejection reason is a domain value). The
/// domain-validation variants stay bespoke here (GTW-577 P9); the duplicated serialize/write
/// tail collapsed onto the shared [`RonSaveError`] seam error, wrapped by
/// [`Save`](Self::Save) (GTW-577 C3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavePrefabError {
    /// The author entered no prefab name (an empty / whitespace-only field) — there is no file
    /// name to write to.
    EmptyName,
    /// A painted cell is an ILLEGAL placement per the GTW-430 [`evaluate_placement`](crate::evaluate_placement) predicate
    /// (C3) — the save is rejected so a written prefab never contains an illegal cell. Names the
    /// offending slot.
    IllegalCell(CellLevel),
    /// The shared serialize/write tail failed (GTW-577 C3) — wraps the seam's
    /// [`RonSaveError`], whose `Display` names the failed stage exactly once.
    Save(RonSaveError),
}

impl From<RonSaveError> for SavePrefabError {
    /// The per-type conversion off the shared seam error (GTW-577 C3) — lets the save path
    /// `?` a [`write_ron_pretty`](gdtf_assets::write_ron_pretty) /
    /// [`serialize_ron_pretty`](gdtf_assets::serialize_ron_pretty) failure straight into the
    /// form's error.
    fn from(err: RonSaveError) -> Self {
        Self::Save(err)
    }
}

impl std::fmt::Display for SavePrefabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "no prefab name entered — nothing to save"),
            Self::IllegalCell(slot) => {
                write!(
                    f,
                    "painted cell {slot:?} is an illegal placement; refusing to save"
                )
            }
            Self::Save(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SavePrefabError {}

/// The `<width>x<height>` directory name for a [`GridSize`] — e.g. a `3 × 3 × 1` footprint →
/// `3x3`, matching the shipped `assets/content/maps/<theme>/<size>/` layout (GTW-432).
#[must_use]
pub(super) fn size_dir(size: GridSize) -> String {
    format!("{}x{}", *size.width(), *size.height())
}
