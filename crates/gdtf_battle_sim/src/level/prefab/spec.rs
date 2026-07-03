//! The **prefab authoring struct** — [`PrefabSpec`], the UUID-keyed level-fragment
//! `.ron` schema (GTW-486). GTW-557 dropped the now-meaningless `V2` suffix.

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{SpawnRole, TerrainPlacementEntry};
use crate::level::{GridSize, ThemeUuid};

/// The serde DEFAULT for [`PrefabSpec::role`] — a fragment that OMITS its role
/// deserializes as a generic [`Fill`](SpawnRole::Fill) fragment.
///
/// [`SpawnRole`] has no [`Default`] impl (the legacy schema deliberately requires an
/// authored role), so the role-default-`Fill` behaviour is supplied by this
/// `#[serde(default = "...")]` helper rather than `SpawnRole::default`.
const fn default_role() -> SpawnRole {
    SpawnRole::Fill
}

/// The **authoring struct** a UUID-keyed level-fragment `.ron` deserializes into — one
/// reusable level fragment (GTW-486).
///
/// The prefab schema introduced by the GTW-476 data-model refactor and the SOLE prefab
/// schema after GTW-496. It references its theme by the stable [`ThemeUuid`] (GTW-485) and
/// every placed piece by the stable [`TerrainUuid`](crate::terrain::def::TerrainUuid)
/// (GTW-484), and authors all geometry as ONE
/// [`placements`](PrefabSpec::placements) list of [`TerrainPlacementEntry`] — the
/// per-piece behaviour lives in the referenced
/// [`TerrainDef`](crate::terrain::def::TerrainDef).
///
/// There is NO authored-opening field and NO `validate` / connectivity path: inter-fragment
/// connectivity is by-construction in the assembler (the 1-cell `default_floor` seam
/// every placement reserves), not authored per-prefab and validated fail-closed.
///
/// **Not `Copy`** — it owns a `Vec` of placements; it is `Clone` so a registry could hold
/// fragments by value (the registry itself is out of scope here — T05b). Derives
/// [`Deserialize`] so the `.ron` parses, [`Serialize`] so a fragment round-trips through its
/// authoring shape, and [`TypePath`] because a reflected `RonAsset<PrefabSpec>` payload
/// requires it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct PrefabSpec {
    /// The stable [`ThemeUuid`] this fragment draws its terrain from (GTW-485).
    pub theme:      ThemeUuid,
    /// The fragment's footprint dimensions — width × height × storey-count, each axis
    /// validated to the sim coarse-grid maximum (the [`GridSize`] reused from the level
    /// model).
    pub size:       GridSize,
    /// The [`SpawnRole`] this fragment plays in an assembled level. `#[serde(default =
    /// "default_role")]` supplies [`SpawnRole::Fill`] for a file that OMITS the field
    /// (there is no `SpawnRole::default`), so an omitted role is a generic fill fragment.
    #[serde(default = "default_role")]
    pub role:       SpawnRole,
    /// The placed terrain pieces of the fragment — ONE list of [`TerrainPlacementEntry`]
    /// replacing the legacy schema's four split lists (walls / scatter / slabs / floors).
    /// May be empty (a fragment with no authored placements still deserializes).
    pub placements: Vec<TerrainPlacementEntry>,
}

impl PrefabSpec {
    /// Build a prefab spec from its theme, footprint, role, and placement list.
    #[must_use]
    pub const fn new(
        theme: ThemeUuid,
        size: GridSize,
        role: SpawnRole,
        placements: Vec<TerrainPlacementEntry>,
    ) -> Self {
        Self {
            theme,
            size,
            role,
            placements,
        }
    }
}
