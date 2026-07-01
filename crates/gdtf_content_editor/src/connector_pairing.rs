//! The prefab-editor **vertical-connector auto-pairing** rule (GTW-531) — placing an UP connector
//! at `(x, y, N)` ALSO places its paired DOWN connector at `(x, y, N+1)`, so authoring a
//! two-ended vertical connector (stair / ladder) is ONE placement, not two.
//!
//! ## Why this lives here (a prefab-editor placement rule ONLY)
//!
//! A vertical connector is two-ended: the UP endpoint on storey `N` is walkable-up to its DOWN
//! endpoint on storey `N+1`. The author should draw one endpoint and get both. This module is that
//! convenience — it does NOT change the sim terrain model or any runtime logic (the sim still sees
//! two independent placed terrains, exactly as if the author had painted both by hand).
//!
//! ## The up↔down map — how it is resolved (C1), and the FORK it forks on
//!
//! The unified terrain model ([`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef)) does NOT
//! cleanly express an up↔down pairing: a stair's [`sim_kind`](gdtf_battle_sim::terrain::def::TerrainSimKind)
//! is `Slab` (GTW-470), there is NO up/down or NS/EW *direction* field, and the up-vs-down + NS/EW
//! distinction lives ONLY in the presenter-side [`graphic_name`](gdtf_battle_sim::terrain::def::TerrainPresenterKind)
//! (e.g. `stair_ns_up` / `stair_ns_down`, `stair_ew_up` / `stair_ew_down`, and the generic
//! `stair_up` / `stair_down`). So the pairing CANNOT be read from a model field.
//!
//! **The fork (logged on GTW-531):** rather than adding a direction field to the sim model (out of
//! scope — this is a prefab-editor rule only), we ship a DEFENSIBLE **graphic-name suffix pairing**:
//! a terrain whose graphic name's final `_`-separated segment is `up` is an UP connector, and its
//! DOWN counterpart is the terrain whose graphic name is the SAME string with that final `up`
//! swapped to `down`. Resolved against the loaded
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) at placement time (find
//! the def whose graphic name equals the swapped string). This matches the shipped convention
//! exactly — `stair_ns_up`↔`stair_ns_down`, `stair_ew_up`↔`stair_ew_down`, `stair_up`↔`stair_down`,
//! and any future `ladder_up`↔`ladder_down` — and is symmetric by kind AND direction, keyed off the
//! typed [`TerrainGraphicKey`](gdtf_battle_sim::terrain::piece::TerrainGraphicKey) (no bare string
//! escapes the recognition boundary). It is data-driven (any theme adding an `_up`/`_down` graphic
//! pair is recognised, not bound to specific UUIDs) and consistent with the presenter's own
//! `stair_up`/`stair_down` role split.
//!
//! ## The placement behaviour (C2 / C4)
//!
//! [`apply_placement_with_pairing`] REUSES the shared GTW-430
//! [`apply_placement`](crate::placement::apply_placement) predicate VERBATIM — it never
//! re-implements placement or the legality rules. It:
//!
//! 1. Places the requested tile at `(x, y, N)` through `apply_placement` (respecting the existing
//!    out-of-bounds / slab-seals-ladder / ladder-auto-clear rules).
//! 2. IF that landed AND the tile is an UP connector whose DOWN counterpart resolves in the
//!    registry, ALSO places that DOWN counterpart at `(x, y, N+1)` through `apply_placement`
//!    (again, the shared legality rules apply to the pair placement — a conflict there does NOT
//!    silently corrupt: `apply_placement` rejects or auto-clears per its own contract).
//! 3. FAIL-CLOSED at the top: if `N` is the top storey (no `N+1` inside the prefab's level range),
//!    the pair is SKIPPED (log-and-continue) — only the up connector is placed, never above the top.
//!
//! Pairing is **one-way (C4)**: this places both endpoints from one UP placement, but REMOVING the
//! up connector does NOT auto-remove the paired down connector — removal stays manual. (Linked
//! removal is a possible later refinement; shipped default is one-way.)

use bevy::prelude::*;
use gdtf_battle_sim::{
    Cell,
    level::{GridSize, ThemeUuid},
    metric::{CellLevel, Level, MAX_LEVELS},
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

use crate::{
    editor_map::EditorMap,
    placement::{ProposedPlacement, apply_placement},
};

/// The `_`-separated graphic-name segment that marks the UPWARD end of a vertical connector
/// (`stair_ns_UP`, `stair_ew_UP`, `stair_UP`, a future `ladder_UP`). A documented framework
/// convention token for the graphic-name pairing (the plumbing carve-out — it is the inner text of
/// the typed [`TerrainGraphicKey`](gdtf_battle_sim::terrain::piece::TerrainGraphicKey), not a
/// free-floating domain value), not a magic literal at the call site.
const UP_SUFFIX: &str = "up";

/// The `_`-separated graphic-name segment that marks the DOWNWARD end of a vertical connector — the
/// counterpart of [`UP_SUFFIX`] the pairing swaps to.
const DOWN_SUFFIX: &str = "down";

/// The outcome of an auto-paired placement (GTW-531 C2) — whether the requested placement landed and
/// whether its paired DOWN connector was also placed.
///
/// A named domain enum (no-bare-types: the pairing outcome is a domain value, not a bare `bool`
/// tuple). The caller (the prefab viewport click-commit) reads it to decide whether to redraw and
/// what to log; a test asserts the [`PairPlaced`](PairingOutcome::PairPlaced) case fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingOutcome {
    /// The requested placement was REJECTED by the shared legality predicate (out of bounds /
    /// slab-seals-ladder). Nothing was placed; there is nothing to pair.
    Rejected,
    /// The requested tile was placed, and it is NOT an up connector (or has no resolvable DOWN
    /// counterpart) — a plain single placement, no pair.
    PlacedNoPair,
    /// The requested tile was placed and IS an up connector, but its DOWN counterpart could NOT be
    /// placed above — either `N` is the top storey (fail-closed, no `N+1` in range) or the shared
    /// predicate rejected the pair placement at `N+1` (e.g. a conflict there). The up connector
    /// still landed.
    PlacedPairSkipped,
    /// The requested UP connector was placed at `(x, y, N)` AND its paired DOWN connector was placed
    /// at `(x, y, N+1)` — the headline GTW-531 behaviour.
    PairPlaced {
        /// The DOWN counterpart terrain that was auto-placed one storey up.
        down: TerrainUuid,
        /// The slot the DOWN counterpart was placed into (`(x, y, N+1)`).
        at:   CellLevel,
    },
}

impl PairingOutcome {
    /// Whether ANY terrain was written to the map by this call — the question the viewport's
    /// redraw-on-commit asks (a rejected placement changes nothing, so no redraw is needed).
    #[must_use]
    pub const fn changed_map(&self) -> bool {
        !matches!(self, Self::Rejected)
    }
}

/// Whether `tile`'s graphic name marks it as an UPWARD vertical connector (C1) — its final
/// `_`-separated graphic-name segment is `up` (`stair_ns_up`, `stair_ew_up`, `stair_up`, a
/// future `ladder_up`).
///
/// Resolved from the terrain model at call time: looks the def up in the `registry` and reads its
/// [`presenter_kind`](gdtf_battle_sim::terrain::def::TerrainPresenterKind) graphic name. An unknown
/// tile (a stale key) is not a connector.
#[must_use]
pub fn is_up_connector(registry: &TerrainDefRegistry, tile: &TerrainUuid) -> bool {
    graphic_name(registry, tile).is_some_and(|name| ends_with_segment(name, UP_SUFFIX))
}

/// Resolve the paired DOWN counterpart of an UP connector `up_tile` (C1) — the terrain the pairing
/// auto-places one storey up.
///
/// Returns [`None`] when `up_tile` is not an up connector, or when NO def in the `registry` carries
/// the swapped-suffix graphic name (the `up`→`down` counterpart). Otherwise returns the
/// [`TerrainUuid`] of the first def whose graphic name equals the counterpart string.
///
/// This is the up↔down MAP, resolved from the loaded registry: it derives the counterpart graphic
/// name (`stair_ns_up`→`stair_ns_down`, etc.) and finds the def carrying it, so the pairing follows
/// the shipped naming convention without hardcoding any UUID.
#[must_use]
pub fn resolve_down_counterpart(
    registry: &TerrainDefRegistry,
    up_tile: &TerrainUuid,
) -> Option<TerrainUuid> {
    let up_name = graphic_name(registry, up_tile)?;
    if !ends_with_segment(up_name, UP_SUFFIX) {
        return None;
    }
    let down_name = swap_final_segment(up_name, UP_SUFFIX, DOWN_SUFFIX)?;
    registry
        .defs()
        .find(|(_, def)| graphic_key_str(def) == down_name)
        .map(|(key, _)| *key)
}

/// Commit a placement through the shared [`apply_placement`] predicate AND auto-place the paired
/// DOWN connector one storey up when the placed tile is an UP connector (GTW-531 C2 / C4) — the
/// entry point the prefab viewport click-commit drives.
///
/// REUSES the shared GTW-430 legality VERBATIM for BOTH the requested placement and the pair
/// placement (never re-implemented — C6). Behaviour:
///
/// - The requested `placement` runs through [`apply_placement`] first (out-of-bounds /
///   slab-seals-ladder reject, ladder-auto-clear side-effect). If it is rejected the whole call is
///   [`Rejected`](PairingOutcome::Rejected) and the map is unchanged.
/// - If it landed and the tile is an up connector with a resolvable DOWN counterpart, the DOWN
///   counterpart is placed at `(x, y, N+1)` through [`apply_placement`] too. If `N+1` is out of the
///   prefab's level range (fail-closed at the top) OR the shared predicate rejects the pair, the
///   pair is SKIPPED ([`PlacedPairSkipped`](PairingOutcome::PlacedPairSkipped)) — the up connector
///   still landed; the pair placement never silently corrupts existing terrain (the shared conflict
///   rules govern it).
/// - Otherwise a plain [`PlacedNoPair`](PairingOutcome::PlacedNoPair).
///
/// One-way (C4): this does NOT link removal — removing an up connector never auto-removes its pair.
pub fn apply_placement_with_pairing(
    map: &mut EditorMap,
    registry: &TerrainDefRegistry,
    theme: ThemeUuid,
    placement: &ProposedPlacement,
    size: GridSize,
) -> PairingOutcome {
    if !apply_placement(map, registry, theme, placement, size) {
        return PairingOutcome::Rejected;
    }

    let Some(down) = resolve_down_counterpart(registry, &placement.tile()) else {
        // Placed, but not an up connector (or no counterpart) — a plain single placement.
        return PairingOutcome::PlacedNoPair;
    };

    // Fail-closed at the top: no storey above `N` inside the prefab's level range → place only the
    // up connector, skip the pair (never auto-place above the top).
    let Some(above) = level_above(placement.slot(), size) else {
        info!(
            "GTW-531: up connector placed on the top storey — paired DOWN connector skipped \
             (fail-closed, no storey above)"
        );
        return PairingOutcome::PlacedPairSkipped;
    };

    // Place the DOWN counterpart at (x, y, N+1) through the SAME shared predicate — its conflict
    // rules (slab-seals-ladder, out-of-bounds) govern the pair placement; a rejection there skips
    // the pair rather than corrupting existing terrain.
    let pair = ProposedPlacement::new(above, down);
    if apply_placement(map, registry, theme, &pair, size) {
        PairingOutcome::PairPlaced { down, at: above }
    } else {
        info!(
            "GTW-531: up connector placed, but the paired DOWN connector at the storey above was \
             rejected by the shared placement predicate (conflict) — pair skipped"
        );
        PairingOutcome::PlacedPairSkipped
    }
}

/// The slot one storey ABOVE `slot`, or [`None`] if `slot` is already at the prefab's top storey
/// (fail-closed for the pair placement — C2). Mirrors the placement module's own `level_above`, but
/// clamped to the PREFAB'S `size.levels()` (not just [`MAX_LEVELS`]) so the pair is never placed
/// past the authored volume.
#[must_use]
fn level_above(slot: CellLevel, size: GridSize) -> Option<CellLevel> {
    let next = u8::try_from(slot.z).ok()?.checked_add(1)?;
    if next >= MAX_LEVELS || next >= *size.levels() {
        return None;
    }
    Some(CellLevel::new(Cell::new(slot.x, slot.y), Level::new(next)))
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
        | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name,
    }
}

/// Whether `name`'s final `_`-separated segment equals `segment` (case-insensitive) — the
/// up/down-suffix test. `stair_ns_up` ends with the `up` segment; `stir_up_ns` does NOT (its final
/// segment is `ns`), so a substring match would over-fire where this does not.
#[must_use]
fn ends_with_segment(name: &str, segment: &str) -> bool {
    name.rsplit('_')
        .next()
        .is_some_and(|last| last.eq_ignore_ascii_case(segment))
}

/// Swap `name`'s final `_`-separated segment from `from` to `to`, or [`None`] if the final segment
/// is not `from` — `stair_ns_up` + (`up`→`down`) = `stair_ns_down`. Preserves the case of the rest
/// of the name; emits the `to` segment verbatim (the shipped convention is lowercase).
#[must_use]
fn swap_final_segment(name: &str, from: &str, to: &str) -> Option<String> {
    let (head, last) = name
        .rsplit_once('_')
        .map_or((None, name), |(h, l)| (Some(h), l));
    if !last.eq_ignore_ascii_case(from) {
        return None;
    }
    Some(match head {
        Some(head) => format!("{head}_{to}"),
        None => to.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        Cell,
        armor::{ArmorHardness, ArmorProtection},
        level::{GridHeight, GridLevels, GridSize, GridWidth, ThemeUuid},
        metric::{CellLevel, Level},
        slab::SlabHp,
        terrain::{
            def::{
                TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
                TerrainSimKind, TerrainUuid,
            },
            piece::TerrainGraphicKey,
        },
    };

    use super::{
        PairingOutcome, apply_placement_with_pairing, is_up_connector, resolve_down_counterpart,
    };
    use crate::{editor_map::EditorMap, placement::ProposedPlacement};

    fn theme() -> ThemeUuid {
        ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_1490_0002))
    }

    const fn tu(n: u128) -> TerrainUuid {
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
    }

    /// A `4 × 4 × 3` volume — enough storeys for the N / N+1 pairing, with a fallback.
    fn size() -> GridSize {
        GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
            .unwrap_or_else(|_| GridSize::default())
    }

    /// A stair-like def (`sim_kind` Slab, per GTW-470) whose graphic name is `graphic`. Magnitudes
    /// are throwaway data (not pinned).
    fn stair_def(key: TerrainUuid, label: &str, graphic: &str) -> TerrainDef {
        TerrainDef {
            key,
            display_name: TerrainDisplayName::new(label.to_owned()),
            sim_kind: TerrainSimKind::Slab {
                hp:               SlabHp::new(120),
                armor_protection: ArmorProtection::new(6),
                armor_hardness:   ArmorHardness::new(3),
            },
            presenter_kind: TerrainPresenterKind::Slab {
                graphic_name: TerrainGraphicKey::new(graphic.to_owned()),
                footfall:     None,
            },
            tags: Vec::new(),
        }
    }

    const STAIR_NS_UP: TerrainUuid = tu(0x0d);
    const STAIR_NS_DOWN: TerrainUuid = tu(0x0e);
    const STAIR_EW_UP: TerrainUuid = tu(0x0f);
    const STAIR_EW_DOWN: TerrainUuid = tu(0x10);

    /// A registry mirroring the shipped `industrial_hive` stair set (up/down × NS/EW).
    fn registry() -> TerrainDefRegistry {
        TerrainDefRegistry::new([
            (
                STAIR_NS_UP,
                stair_def(STAIR_NS_UP, "Deck Stair Up (NS)", "stair_ns_up"),
            ),
            (
                STAIR_NS_DOWN,
                stair_def(STAIR_NS_DOWN, "Deck Stair Down (NS)", "stair_ns_down"),
            ),
            (
                STAIR_EW_UP,
                stair_def(STAIR_EW_UP, "Deck Stair Up (EW)", "stair_ew_up"),
            ),
            (
                STAIR_EW_DOWN,
                stair_def(STAIR_EW_DOWN, "Deck Stair Down (EW)", "stair_ew_down"),
            ),
        ])
    }

    fn at(cell: Cell, level: u8) -> CellLevel {
        CellLevel::new(cell, Level::new(level))
    }

    /// C1: an `_up` graphic classifies as an up connector; its `_down` counterpart does NOT, and
    /// the counterpart resolves symmetrically by direction.
    #[test]
    fn up_connector_recognition_and_counterpart_resolution() {
        let reg = registry();
        assert!(
            is_up_connector(&reg, &STAIR_NS_UP),
            "stair_ns_up is an up connector"
        );
        assert!(
            is_up_connector(&reg, &STAIR_EW_UP),
            "stair_ew_up is an up connector"
        );
        assert!(
            !is_up_connector(&reg, &STAIR_NS_DOWN),
            "stair_ns_down is NOT an up connector",
        );

        assert_eq!(
            resolve_down_counterpart(&reg, &STAIR_NS_UP),
            Some(STAIR_NS_DOWN),
            "NS up pairs to NS down (symmetric by direction)",
        );
        assert_eq!(
            resolve_down_counterpart(&reg, &STAIR_EW_UP),
            Some(STAIR_EW_DOWN),
            "EW up pairs to EW down (symmetric by direction)",
        );
        assert_eq!(
            resolve_down_counterpart(&reg, &STAIR_NS_DOWN),
            None,
            "a down connector has no up→down counterpart (one-way pairing)",
        );
    }

    /// C2 (the headline): placing an UP connector at `(x, y, N)` auto-places its paired DOWN
    /// connector at `(x, y, N+1)` through the shared placement predicate.
    ///
    /// Pin-discriminating: without the pairing the map would hold ONLY the up connector at N and
    /// nothing at N+1 — every N+1 assertion here would flip.
    #[test]
    fn placing_up_connector_auto_places_down_pair_above() {
        let reg = registry();
        let th = theme();
        let mut map = EditorMap::new();
        let cell = Cell::new(1, 1);

        let placement = ProposedPlacement::new(at(cell, 0), STAIR_NS_UP);
        let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());

        assert_eq!(
            outcome,
            PairingOutcome::PairPlaced {
                down: STAIR_NS_DOWN,
                at:   at(cell, 1),
            },
            "placing an up connector at N auto-places the down pair at N+1 (C2)",
        );
        assert_eq!(
            map.tile_at_level(at(cell, 0)),
            Some(STAIR_NS_UP),
            "the up connector is placed at N (C2)",
        );
        assert_eq!(
            map.tile_at_level(at(cell, 1)),
            Some(STAIR_NS_DOWN),
            "the paired down connector is auto-placed at N+1 (C2)",
        );
        assert_eq!(map.painted_count(), 2, "both endpoints are present (C2)");
    }

    /// C2 fail-closed: placing an up connector on the TOP storey places only it — the pair is
    /// skipped, never placed above the prefab's level range.
    #[test]
    fn up_connector_on_top_storey_skips_pair_fail_closed() {
        let reg = registry();
        let th = theme();
        let mut map = EditorMap::new();
        let cell = Cell::new(2, 2);
        // size() has 3 levels (0..=2); N = 2 is the top storey, so N+1 = 3 is out of range.
        let placement = ProposedPlacement::new(at(cell, 2), STAIR_EW_UP);
        let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());

        assert_eq!(
            outcome,
            PairingOutcome::PlacedPairSkipped,
            "an up connector on the top storey skips the pair (fail-closed C2)",
        );
        assert_eq!(
            map.tile_at_level(at(cell, 2)),
            Some(STAIR_EW_UP),
            "the up connector still landed on the top storey (C2)",
        );
        assert_eq!(
            map.painted_count(),
            1,
            "only the up connector — no pair above the top (C2)"
        );
    }

    /// A non-connector placement is a plain single placement (no spurious pair).
    #[test]
    fn non_connector_placement_places_no_pair() {
        let reg = registry();
        let th = theme();
        let mut map = EditorMap::new();
        let cell = Cell::new(0, 0);
        // stair_ns_down is not an UP connector — placing it pairs nothing.
        let placement = ProposedPlacement::new(at(cell, 0), STAIR_NS_DOWN);
        let outcome = apply_placement_with_pairing(&mut map, &reg, th, &placement, size());
        assert_eq!(
            outcome,
            PairingOutcome::PlacedNoPair,
            "a down connector places no pair"
        );
        assert_eq!(map.painted_count(), 1, "only the placed tile — no pair");
    }
}
