//! The **type vocabulary** of the save path (GTW-432; swept onto the v2 UUID schema in
//! GTW-495): compile-time path constants, the component markers on the save UI widgets, the
//! [`SavePrefabError`] failure enum, and the theme/size directory helpers.

use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{GridSize, SpawnRole},
    metric::CellLevel,
};

/// The workspace `assets/` root — byte-identical to the editor's `AssetPlugin.file_path`
/// (`crates/gdtf_editor` → up two levels → `assets`), computed at compile time relative to THIS
/// crate's manifest. So a prefab the editor SAVES lands exactly where the running game (and the
/// GTW-489 v2 folder loader) READS v2 prefabs from — `assets/maps/<theme>/<size>/`.
pub(super) const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

/// The folder under the assets root the GTW-489 v2 loader scans for `*.prefab_v2.ron` fragments
/// — the `<theme>/<size>/` subfolders are nested under this. The NEW `maps/` root (SEPARATE from
/// the legacy `content/maps/` tree), the SOLE prefab root after GTW-494.
pub(super) const MAPS_SUBDIR: &str = "maps";

/// The compound file extension the GTW-489 v2 prefab loader keys on
/// (`init_ron_asset_with_extensions::<PrefabSpecV2>(vec!["prefab_v2.ron"])`) — a saved prefab MUST
/// use it or the loader never picks the file up. The loader strips a trailing `.prefab_v2` from
/// the file stem to recover the prefab NAME, so `entry_room.prefab_v2.ron` keys `entry_room`.
pub(super) const PREFAB_EXTENSION: &str = "prefab_v2.ron";

/// The spawn role a saved prefab is authored with (GTW-432) — the connective
/// [`Fill`](SpawnRole::Fill) default.
///
/// The editor has no spawn-role control, so every saved fragment is a generic FILL fragment (the
/// v2 schema's serde-default role) — the assembler buckets it under the connective interior. A
/// future spawn-role selector would override this; until then Fill is the safe, documented
/// default.
pub(super) const SAVED_SPAWN_ROLE: SpawnRole = SpawnRole::Fill;

/// Marker on the prefab-NAME `TextField` (the GTW-411 widget) the author types the prefab name
/// into. A unit marker (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PrefabNameField;

/// Marker on the "Save prefab" [`Button`] (the GTW-432 save TRIGGER). A unit marker (no-bare-types
/// rule). A PLAIN marker (NOT re-exported / not a `support_item!`) — no external test names it, so
/// it stays `pub(crate)` to dodge the `unreachable_pub` the test-support feature would trip.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct SavePrefabButton;

/// Why a prefab save was REJECTED — the handled, no-panic failure of the save path (GTW-432;
/// the `NoEdgeOpening` variant DROPPED in GTW-495, the v2 schema has no edge openings).
///
/// A named domain enum (no-bare-types: the rejection reason is a domain value). Each variant names
/// what was wrong so the `error!` line is precise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SavePrefabError {
    /// The author entered no prefab name (an empty / whitespace-only field) — there is no file
    /// name to write to.
    EmptyName,
    /// A painted cell is an ILLEGAL placement per the GTW-430 [`evaluate_placement`] predicate
    /// (C3) — the save is rejected so a written prefab never contains an illegal cell. Names the
    /// offending slot.
    IllegalCell(CellLevel),
    /// Serializing the built [`PrefabSpecV2`](gdtf_battle_sim::level::PrefabSpecV2) to RON failed.
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
/// The v2 maps tree is `assets/maps/<theme>/<size>/`; the `<theme>` segment is the slugified
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
/// `3x3`, matching the shipped `assets/maps/<theme>/<size>/` layout (GTW-432).
#[must_use]
pub(super) fn size_dir(size: GridSize) -> String {
    format!("{}x{}", *size.width(), *size.height())
}
