//! The **type vocabulary** of the GTW-432 save path: compile-time path constants, the
//! component markers on the save UI widgets, and the [`SavePrefabError`] failure enum.

use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{GridSize, LevelTheme, SpawnRole},
    metric::CellLevel,
};

/// The workspace `assets/` root — byte-identical to the editor's `AssetPlugin.file_path`
/// (`crates/gdtf_editor` → up two levels → `assets`), computed at compile time relative to THIS
/// crate's manifest. So a prefab the editor SAVES lands exactly where the running game (and the
/// GTW-418 folder loader) READS prefabs from — `assets/content/maps/<theme>/<size>/`.
pub(super) const WORKSPACE_ASSETS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets");

/// The folder under the assets root the GTW-418 loader scans for `*.prefab.ron` fragments — the
/// `<theme>/<size>/` subfolders are nested under this.
pub(super) const MAPS_SUBDIR: &str = "content/maps";

/// The compound file extension the GTW-418 prefab loader keys on
/// (`init_ron_asset_with_extensions::<PrefabSpec>(vec!["prefab.ron"])`) — a saved prefab MUST use
/// it or the loader never picks the file up. The loader then strips a trailing `.prefab` from the
/// file stem to recover the prefab NAME, so `entry_room.prefab.ron` keys `entry_room`. (The ticket
/// title's `<prefab_name>.ron` is the human shorthand; the loader the contract round-trips through
/// requires the `.prefab` infix, so the saver writes it — see the module docs.)
pub(super) const PREFAB_EXTENSION: &str = "prefab.ron";

/// The spawn role a saved prefab is authored with (GTW-432) — the connective
/// [`Fill`](SpawnRole::Fill) default.
///
/// The editor has no spawn-role control, so every saved fragment is a generic FILL fragment (the
/// [`PrefabSpec::default`](gdtf_battle_sim::level::PrefabSpec::default) role) — the assembler
/// buckets it under the connective interior. A future spawn-role selector would override this;
/// until then Fill is the safe, documented default (the contract names no spawn role).
pub(super) const SAVED_SPAWN_ROLE: SpawnRole = SpawnRole::Fill;

/// Marker on the prefab-NAME `TextField` (the GTW-411 widget) the author types the prefab name
/// into. A unit marker (no-bare-types rule): the save press reads the committed value of the
/// `TextField` carrying it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PrefabNameField;

/// Marker on the "Save prefab" [`Button`] (the GTW-432 save TRIGGER). A unit marker (no-bare-types
/// rule); the press system reads its [`Interaction`] edge. A PLAIN marker (NOT re-exported / not a
/// `support_item!`) — no external test names it, so it stays `pub(crate)` to dodge the
/// `unreachable_pub` the test-support feature would otherwise trip (the GTW-429
/// `SaveGangButton` precedent).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct SavePrefabButton;

/// Why a prefab save was REJECTED — the handled, no-panic failure of the save path (GTW-432).
///
/// A named domain enum (no-bare-types: the rejection reason is a domain value, not a bare
/// `()`/`bool`; the no-panic contract — the press handler logs it rather than `unwrap`/`panic`).
/// Each variant names what was wrong so the `error!` line is precise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SavePrefabError {
    /// The author entered no prefab name (an empty / whitespace-only field) — there is no file
    /// name to write to.
    EmptyName,
    /// A painted cell is an ILLEGAL placement per the GTW-430 [`evaluate_placement`] predicate
    /// (C3) — the save is rejected so a written prefab never contains an illegal cell. Names the
    /// offending slot.
    IllegalCell(CellLevel),
    /// The map authors NO walkable boundary cell, so no [`EdgeOpening`](gdtf_battle_sim::level::EdgeOpening) could be derived — the
    /// prefab could not connect (the loader would reject it anyway — C6).
    NoEdgeOpening,
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
            Self::NoEdgeOpening => write!(
                f,
                "the map has no walkable boundary cell, so no edge opening could be derived; a \
                 prefab with zero edge openings cannot connect (refusing to save)",
            ),
            Self::Serialize(err) => write!(f, "failed to serialize the prefab: {err}"),
            Self::Write(err) => write!(f, "failed to write the prefab file: {err}"),
        }
    }
}

impl std::error::Error for SavePrefabError {}

/// The `snake_case` directory name for a [`LevelTheme`] — `IndustrialHive` → `industrial_hive`,
/// matching the shipped `assets/content/maps/<theme>/` layout (GTW-432).
///
/// An explicit per-variant map (the closed [`LevelTheme`] set) so the on-disk folder name is the
/// authored one, not a derived guess — and so adding a theme variant is a compile error here until
/// its directory name is named.
#[must_use]
pub(super) const fn theme_dir(theme: LevelTheme) -> &'static str {
    match theme {
        LevelTheme::IndustrialHive => "industrial_hive",
        LevelTheme::Underhive => "underhive",
        LevelTheme::SumpWaste => "sump_waste",
    }
}

/// The `<width>x<height>` directory name for a [`GridSize`] — e.g. a `3 × 3 × 1` footprint →
/// `3x3`, matching the shipped `assets/content/maps/<theme>/<size>/` layout (GTW-432).
///
/// The size folder names the GROUND footprint (width × height); the storey count is part of the
/// prefab's own `size` field, not the folder name (the shipped `3x3` precedent).
#[must_use]
pub(super) fn size_dir(size: GridSize) -> String {
    format!("{}x{}", *size.width(), *size.height())
}
