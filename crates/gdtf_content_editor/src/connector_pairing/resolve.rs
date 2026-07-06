//! The typed up↔down connector recognition / resolution over the [`TileRole`] vocabulary
//! (GTW-566 C6) — changes with the role vocabulary, not with placement behaviour.

use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainUuid};

/// Whether `tile`'s graphic name marks it as an UPWARD vertical connector (C1) — it classifies
/// to a [`TileRole`] whose [`is_up_connector`](TileRole::is_up_connector) holds (`stair_up`,
/// `stair_ns_up`, `stair_ew_up` — the typed GTW-566 C6 recognition).
///
/// Resolved from the terrain model at call time: looks the def up in the `registry`, reads its
/// [`presenter_kind`](gdtf_battle_sim::terrain::def::TerrainPresenterKind) graphic name, and
/// classifies it through [`TileRole::from_key`]. An unknown tile (a stale key) or an
/// out-of-vocabulary graphic name is NOT a connector (fail-closed — see the module doc).
#[must_use]
pub fn is_up_connector(registry: &TerrainDefRegistry, tile: &TerrainUuid) -> bool {
    graphic_name(registry, tile)
        .and_then(TileRole::from_key)
        .is_some_and(TileRole::is_up_connector)
}

/// Resolve the paired DOWN counterpart of an UP connector `up_tile` (C1) — the terrain the pairing
/// auto-places one storey up.
///
/// Returns [`None`] when `up_tile` is not an up connector (including an out-of-vocabulary
/// graphic name — fail-closed, GTW-566 C6), or when NO def in the `registry` carries the
/// counterpart role's graphic name. Otherwise returns the [`TerrainUuid`] of the first def whose
/// graphic name equals the counterpart key.
///
/// This is the up↔down MAP, typed through the vocabulary and resolved from the loaded registry:
/// the up role's [`TileRole::counterpart`] names the DOWN role
/// (`stair_ns_up`→`stair_ns_down`, etc.), and the def carrying that role's
/// [`as_key`](TileRole::as_key) is found — so the pairing follows the shipped vocabulary without
/// hardcoding any UUID and without string-suffix surgery.
#[must_use]
pub fn resolve_down_counterpart(
    registry: &TerrainDefRegistry,
    up_tile: &TerrainUuid,
) -> Option<TerrainUuid> {
    let up_role = TileRole::from_key(graphic_name(registry, up_tile)?)?;
    if !up_role.is_up_connector() {
        return None;
    }
    let down_name = up_role.counterpart()?.as_key();
    registry
        .defs()
        .find(|(_, def)| graphic_key_str(def) == down_name)
        .map(|(key, _)| *key)
}

/// The graphic name (as `&str`) of the def keyed `tile` in the `registry`, or [`None`] if the tile
/// is not registered — the read the connector recognition + counterpart resolution share.
#[must_use]
fn graphic_name<'a>(registry: &'a TerrainDefRegistry, tile: &TerrainUuid) -> Option<&'a str> {
    registry.def(tile).map(graphic_key_str)
}

/// The graphic-name `&str` of a terrain def, across every presenter kind (each variant carries a
/// `graphic_name`).
#[must_use]
fn graphic_key_str(def: &gdtf_battle_sim::terrain::def::TerrainDef) -> &str {
    use gdtf_battle_sim::terrain::def::TerrainPresenterKind;
    match &def.presenter_kind {
        TerrainPresenterKind::Wall { graphic_name }
        | TerrainPresenterKind::Cover { graphic_name }
        | TerrainPresenterKind::Emplacement { graphic_name }
        | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name,
    }
}
