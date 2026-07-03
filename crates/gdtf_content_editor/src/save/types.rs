//! The **type vocabulary** of the save path (GTW-432; swept onto the v2 UUID schema in
//! GTW-495): compile-time path constants, the [`SavePrefabError`] failure enum, and the theme/size
//! directory helpers. (GTW-512: the `bevy_ui` save-widget markers were dropped — the egui save
//! controls are the C4 child.)

use gdtf_battle_sim::{
    level::{GridSize, SpawnRole},
    metric::CellLevel,
};

/// The workspace `assets/` root — byte-identical to the editor's `AssetPlugin.file_path`
/// (`crates/gdtf_content_editor` → up two levels → `assets`), computed at compile time relative to THIS
/// crate's manifest. So a prefab the editor SAVES lands exactly where the running game (and the
/// GTW-489 folder loader) READS prefabs from — `assets/content/maps/<theme>/<size>/`.
pub(super) const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

/// The folder under the assets root the GTW-489 loader scans for `*.prefab.ron` fragments
/// — the `<theme>/<size>/` subfolders are nested under this. Lives under `content/maps/`,
/// the SOLE prefab root after GTW-494 (GTW-556 moved it from the top-level `maps/`).
pub(super) const MAPS_SUBDIR: &str = "content/maps";

/// The compound file extension the GTW-489 prefab loader keys on
/// (`init_ron_asset_with_extensions::<PrefabSpec>(vec!["prefab.ron"])`) — a saved prefab MUST
/// use it or the loader never picks the file up. The loader strips a trailing `.prefab` from
/// the file stem to recover the prefab NAME, so `entry_room.prefab.ron` keys `entry_room`.
pub(super) const PREFAB_EXTENSION: &str = "prefab.ron";

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
/// A named domain enum (no-bare-types: the rejection reason is a domain value). Each variant names
/// what was wrong so the `error!` line is precise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavePrefabError {
    /// The author entered no prefab name (an empty / whitespace-only field) — there is no file
    /// name to write to.
    EmptyName,
    /// A painted cell is an ILLEGAL placement per the GTW-430 [`evaluate_placement`](crate::evaluate_placement) predicate
    /// (C3) — the save is rejected so a written prefab never contains an illegal cell. Names the
    /// offending slot.
    IllegalCell(CellLevel),
    /// Serializing the built [`PrefabSpec`](gdtf_battle_sim::level::PrefabSpec) to RON failed.
    Serialize(String),
    /// Writing the serialized prefab to disk failed (a missing dir / permissions error / io).
    Write(String),
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
            Self::Serialize(err) => write!(f, "failed to serialize the prefab: {err}"),
            Self::Write(err) => write!(f, "failed to write the prefab file: {err}"),
        }
    }
}

impl std::error::Error for SavePrefabError {}

/// The `snake_case` directory name for a theme, derived from its human label (GTW-495).
///
/// The maps tree is `assets/content/maps/<theme>/<size>/`; the `<theme>` segment is the slugified
/// theme DISPLAY NAME (e.g. `"Industrial Hive"` → `industrial_hive`), matching the shipped
/// per-theme layout. The save path resolves the theme's display name from the
/// [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry); this folds it to the
/// directory convention. NOT a closed-enum match (the UUID model has no closed theme enum):
/// lowercase, spaces / dashes → underscores, anything outside `[a-z0-9_]` dropped. An empty
/// result falls back to the (still-unique) nil/unknown bucket name so a save never targets the
/// assets root.
#[must_use]
pub(super) fn theme_dir(display_name: &str) -> String {
    let slug: String = display_name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if slug.is_empty() {
        "unknown_theme".to_owned()
    } else {
        slug
    }
}

/// The `<width>x<height>` directory name for a [`GridSize`] — e.g. a `3 × 3 × 1` footprint →
/// `3x3`, matching the shipped `assets/content/maps/<theme>/<size>/` layout (GTW-432).
#[must_use]
pub(super) fn size_dir(size: GridSize) -> String {
    format!("{}x{}", *size.width(), *size.height())
}
