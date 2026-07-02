//! The editor's **placement-legality rules** — the SINGLE SHARED predicate both the hover-ghost
//! preview and the click-commit run, plus the vertical auto-handling for multi-level tiles
//! (GTW-430; swept onto the UUID model in GTW-495).
//!
//! ## One source of truth (C3)
//!
//! [`evaluate_placement`] is the ONE legality function. The hover-ghost preview calls it to decide
//! whether to tint the preview RED (an illegal placement) and the click-commit calls it to decide
//! whether to REJECT the paint — there is no second copy of the rule. (The egui viewport that
//! consumes both is the GTW-512 C4 child; the predicate is the shared FOUNDATION it builds on.) Both
//! consume the same `(map, registry, theme, placement)` inputs and the same [`PlacementVerdict`].
//!
//! ## The vertical rules
//!
//! The editor classifies each terrain definition into an [`EditorTileClass`] (see [`classify`]):
//!
//! - A **slab** is identified by the sim's own [`TerrainSimKind::Slab`] — semantics-driven off the
//!   terrain registry, not a magic key.
//! - A **ladder** is a vertical link between storeys ([`LinkKind::Ladder`](gdtf_battle_sim::terrain::vertical::LinkKind) in the sim).
//!   The unified terrain model has no ladder *kind* (its [`TerrainSimKind`] is WALL/COVER/SLAB), so
//!   the editor recognises a ladder by a DATA-driven convention: a terrain def whose
//!   [`TerrainDisplayName`](gdtf_battle_sim::terrain::def::TerrainDisplayName) reads as a ladder
//!   ([`names_a_ladder`]). This is not bound to one specific UUID — any theme that adds a
//!   ladder-named terrain is recognised — and is documented as the chosen recognition because the
//!   sim kind does not model ladders.
//!
//! Two symmetric rules over those classes (`docs/combat/combat.md`: gangers change storeys only over
//! authored stair/ladder links; a slab seals a z-boundary):
//!
//! - **C1 auto-handling — placing a LADDER auto-clears a SLAB directly above it.** A ladder placed
//!   at `(cell, level)` connects up to `(cell, level + 1)`; a slab there would seal the ladder's
//!   destination. The latest authoring intent wins (the [`EditorMap`] repaint-overwrites precedent),
//!   so the editor AUTO-CLEARS the slab above rather than rejecting the ladder. The placement is
//!   therefore LEGAL, and [`apply_placement`] performs the clear.
//! - **C2 illegal — placing a SLAB onto a cell whose ladder it would seal is REJECTED** (red tint).
//!   The inverse of C1: a slab dropped where it would seal a ladder is rejected. A slab is illegal
//!   when the ladder it would seal sits at the SAME slot (a ladder rises THROUGH the cell) OR
//!   directly BELOW it (the ladder's destination).
//!
//! Out-of-bounds is also illegal (the [`EditorMap`] clamp, surfaced through the verdict so the one
//! predicate covers every rejection).

use gdtf_battle_sim::{
    level::ThemeUuid,
    metric::{CellLevel, Level},
    terrain::def::{TerrainDefRegistry, TerrainSimKind, TerrainUuid},
};

use crate::editor_map::EditorMap;

/// The editor's classification of a terrain definition for the vertical placement rules
/// (GTW-430; swept to the UUID model in GTW-495).
///
/// A named domain enum (no-bare-types: a tile's placement class is a domain value, not a bare
/// discriminant). Derived from the terrain registry by [`classify`]: [`Slab`](EditorTileClass::Slab)
/// from the sim's [`TerrainSimKind::Slab`], [`Ladder`](EditorTileClass::Ladder) from the
/// ladder-naming convention ([`names_a_ladder`] over the def's display name), and
/// [`Other`](EditorTileClass::Other) for every tile the vertical rules do not constrain (walls,
/// cover).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTileClass {
    /// A floor/roof slab — seals a z-boundary (the sim's [`TerrainSimKind::Slab`]).
    Slab,
    /// A ladder — a vertical link between storeys (recognised by name; the terrain model has no
    /// ladder kind — see the module docs).
    Ladder,
    /// Any tile the vertical placement rules do not constrain (wall / cover).
    Other,
}

/// Why a placement is illegal — the reason a [`PlacementVerdict::Illegal`] carries (GTW-430 C2).
///
/// A named domain enum (no-bare-types: a rejection reason is a domain value). Each variant is one
/// of the editor's placement rules; the ghost shows the cell red and the commit rejects for ANY of
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllegalReason {
    /// The target slot is outside the drawable volume (the [`EditorMap`] clamp — C3).
    OutOfBounds,
    /// A slab would seal an existing ladder — either at the SAME slot (a ladder rises through the
    /// cell) or directly BELOW (the ladder's destination). The single-plane canvas reaches the
    /// same-slot case (C2).
    SlabSealsLadder,
}

/// The verdict the SINGLE SHARED legality predicate returns (GTW-430 C3).
///
/// A named domain enum (no-bare-types: a legality verdict is a domain value, not a bare `bool` /
/// `Result`). [`Legal`](PlacementVerdict::Legal) carries any vertical SIDE-EFFECT the commit must
/// perform (the C1 auto-clear); [`Illegal`](PlacementVerdict::Illegal) carries the
/// [`IllegalReason`] the ghost tints red for and the commit rejects on. Both the hover-ghost
/// preview and the click-commit consume this same verdict — there is no duplicated rule logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementVerdict {
    /// The placement is legal — the commit may write it. Carries the optional vertical side-effect
    /// the commit applies first (the C1 auto-clear of a slab above a ladder).
    Legal {
        /// The slot whose paint the placement AUTO-CLEARS (C1: the slab one storey above a placed
        /// ladder), or [`None`] when the placement has no vertical side-effect.
        auto_clear: Option<CellLevel>,
    },
    /// The placement is illegal — the ghost tints the target red and the commit rejects it.
    Illegal(IllegalReason),
}

impl PlacementVerdict {
    /// A legal placement with no vertical side-effect.
    #[must_use]
    pub const fn legal() -> Self {
        Self::Legal { auto_clear: None }
    }

    /// A legal placement that AUTO-CLEARS the paint at `slot` first (C1).
    #[must_use]
    pub const fn legal_clearing(slot: CellLevel) -> Self {
        Self::Legal {
            auto_clear: Some(slot),
        }
    }

    /// Whether this verdict is illegal — the question the ghost (tint red?) and the commit
    /// (reject?) both ask.
    #[must_use]
    pub const fn is_illegal(&self) -> bool {
        matches!(self, Self::Illegal(_))
    }
}

/// A proposed placement — the `(slot, tile)` the author is hovering / clicking (GTW-430).
///
/// A named struct (not a bare tuple) so the placement the legality predicate consumes is
/// self-describing: the [`CellLevel`] target slot (cell + storey) and the [`TerrainUuid`] of the
/// tile being placed. The hover-ghost builds one for the hovered cell + selected tile; the commit
/// builds the same for the clicked cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProposedPlacement {
    /// The target slot (cell + storey) the tile would be painted into.
    slot: CellLevel,
    /// The tile being placed.
    tile: TerrainUuid,
}

impl ProposedPlacement {
    /// Build a proposed placement from its target slot and the terrain being placed.
    #[must_use]
    pub const fn new(slot: CellLevel, tile: TerrainUuid) -> Self {
        Self { slot, tile }
    }

    /// The target slot (cell + storey).
    #[must_use]
    pub const fn slot(&self) -> CellLevel {
        self.slot
    }

    /// The terrain being placed.
    #[must_use]
    pub const fn tile(&self) -> TerrainUuid {
        self.tile
    }
}

/// Whether a terrain's display name reads as a LADDER — the data-driven ladder recognition
/// (GTW-430; swept to read the def's display name in GTW-495).
///
/// The unified terrain model has no ladder *kind* ([`TerrainSimKind`] is WALL/COVER/SLAB), and
/// ladders are a sim vertical-link concept, not a terrain piece. So the editor recognises a ladder
/// by NAME: a display name containing `"ladder"` (case-insensitive). This is data-driven (any theme
/// that adds a ladder-named terrain is recognised, not bound to one specific UUID) and consistent
/// with the sim's `LinkKind::Ladder` vocabulary.
#[must_use]
pub fn names_a_ladder(text: &str) -> bool {
    text.to_ascii_lowercase().contains("ladder")
}

/// Classify a terrain definition (looked up by `key` in the [`TerrainDefRegistry`]) into its
/// [`EditorTileClass`] for the vertical placement rules (GTW-430; UUID-keyed in GTW-495).
///
/// Semantics-driven off the terrain registry: the slab class comes from the sim's
/// [`TerrainSimKind::Slab`]; the ladder class from the [`names_a_ladder`] convention over the def's
/// display name. A tile the registry does not know (a stale key after a theme switch) is
/// [`Other`](EditorTileClass::Other) — the conservative class the vertical rules never constrain.
/// The `theme` parameter is retained for the shared predicate signature even though the UUID-keyed
/// registry resolves a terrain without it (a UUID is globally unique).
#[must_use]
pub fn classify(
    registry: &TerrainDefRegistry,
    _theme: ThemeUuid,
    key: &TerrainUuid,
) -> EditorTileClass {
    let Some(def) = registry.def(key) else {
        return EditorTileClass::Other;
    };
    if names_a_ladder(&def.display_name) {
        return EditorTileClass::Ladder;
    }
    match def.sim_kind {
        TerrainSimKind::Slab { .. } => EditorTileClass::Slab,
        // GTW-543: an emplacement is a same-level structure (like Wall/Cover) placed on the
        // canvas, not a slab z-boundary — it classifies as Other.
        TerrainSimKind::Wall { .. }
        | TerrainSimKind::Cover { .. }
        | TerrainSimKind::Emplacement { .. } => EditorTileClass::Other,
    }
}

/// Whether the slot currently holds a tile that classifies as a LADDER (GTW-430).
#[must_use]
fn is_ladder(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    theme: ThemeUuid,
    slot: CellLevel,
) -> bool {
    map.tile_at_level(slot)
        .is_some_and(|key| classify(registry, theme, &key) == EditorTileClass::Ladder)
}

/// Whether the slot currently holds a tile that classifies as a SLAB (GTW-430).
#[must_use]
fn is_slab(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    theme: ThemeUuid,
    slot: CellLevel,
) -> bool {
    map.tile_at_level(slot)
        .is_some_and(|key| classify(registry, theme, &key) == EditorTileClass::Slab)
}

/// The slot one storey ABOVE `slot`, or [`None`] if `slot` is already at the storey ceiling
/// ([`MAX_LEVELS`](gdtf_battle_sim::metric::MAX_LEVELS) − 1).
///
/// The vertical rules reason about a cell's neighbour one level up (a ladder's destination / the
/// slab that would seal it). Saturating at the ceiling so a ladder on the top storey simply has no
/// slab-above to consider (and a placement there is never illegal for that reason).
#[must_use]
fn level_above(slot: CellLevel) -> Option<CellLevel> {
    // `slot.z` is a storey index (0-based); the cell x/y are the same. Rebuild the slot one storey
    // up via the typed constructor (never the raw IVec3).
    let next = u8::try_from(slot.z).ok()?.checked_add(1)?;
    if next >= gdtf_battle_sim::metric::MAX_LEVELS {
        return None;
    }
    Some(CellLevel::new(
        gdtf_battle_sim::Cell::new(slot.x, slot.y),
        Level::new(next),
    ))
}

/// The SINGLE SHARED placement-legality predicate (GTW-430 C3) — the one function the hover-ghost
/// preview and the click-commit both call.
///
/// Given the current [`EditorMap`], the [`TerrainDefRegistry`], the active theme, and a
/// [`ProposedPlacement`], returns the [`PlacementVerdict`]:
///
/// - **Out of bounds** → [`Illegal`](PlacementVerdict::Illegal) ([`IllegalReason::OutOfBounds`]).
/// - **Slab sealing an existing ladder** → [`Illegal`](PlacementVerdict::Illegal)
///   ([`IllegalReason::SlabSealsLadder`]).
/// - **Ladder with a slab directly above** → [`Legal`](PlacementVerdict::Legal) carrying the
///   `auto_clear` of that slab's slot (the C1 auto-handling).
/// - otherwise → a plain [`Legal`](PlacementVerdict::legal).
#[must_use]
pub fn evaluate_placement(
    map: &EditorMap,
    registry: &TerrainDefRegistry,
    theme: ThemeUuid,
    placement: &ProposedPlacement,
    size: gdtf_battle_sim::level::GridSize,
) -> PlacementVerdict {
    let slot = placement.slot();
    if !slot_in_bounds(slot, size) {
        return PlacementVerdict::Illegal(IllegalReason::OutOfBounds);
    }
    let class = classify(registry, theme, &placement.tile());
    match class {
        EditorTileClass::Ladder => {
            // C1: a ladder's destination is the storey above. A slab there would seal it — auto-
            // clear it so the just-placed ladder is coherent (legal, with a side-effect).
            if let Some(above) = level_above(slot)
                && is_slab(map, registry, theme, above)
            {
                return PlacementVerdict::legal_clearing(above);
            }
            PlacementVerdict::legal()
        }
        EditorTileClass::Slab => {
            // C2: a slab is illegal where it would seal a ladder — at the SAME slot (a ladder rises
            // through the cell, reachable on the L0 canvas) or directly BELOW (the ladder's
            // destination). Reject it (the ghost tints the target red, the commit refuses).
            let seals_ladder = is_ladder(map, registry, theme, slot)
                || level_below(slot).is_some_and(|below| is_ladder(map, registry, theme, below));
            if seals_ladder {
                return PlacementVerdict::Illegal(IllegalReason::SlabSealsLadder);
            }
            PlacementVerdict::legal()
        }
        EditorTileClass::Other => PlacementVerdict::legal(),
    }
}

/// The slot one storey BELOW `slot`, or [`None`] if `slot` is already the ground storey (`L0`).
///
/// The C2 slab rule checks the cell directly below for an existing ladder.
#[must_use]
fn level_below(slot: CellLevel) -> Option<CellLevel> {
    let below = u8::try_from(slot.z).ok()?.checked_sub(1)?;
    Some(CellLevel::new(
        gdtf_battle_sim::Cell::new(slot.x, slot.y),
        Level::new(below),
    ))
}

/// Whether `slot` falls inside the drawable volume — the legality predicate's bounds check, kept in
/// sync with the [`EditorMap`] clamp so the verdict and the model agree (C3).
fn slot_in_bounds(slot: CellLevel, size: gdtf_battle_sim::level::GridSize) -> bool {
    let width = i32::from(*size.width());
    let height = i32::from(*size.height());
    let levels = i32::from(*size.levels());
    slot.x >= 0
        && slot.x < width
        && slot.y >= 0
        && slot.y < height
        && slot.z >= 0
        && slot.z < levels
}

/// Commit a proposed placement through the SHARED legality predicate (GTW-430 C2 + C1) — the path
/// the click-commit drives.
///
/// Calls [`evaluate_placement`]; on an [`Illegal`](PlacementVerdict::Illegal) verdict it REJECTS
/// (returns `false`, leaving the [`EditorMap`] UNCHANGED — C2). On a [`Legal`](PlacementVerdict::Legal)
/// verdict it first performs any vertical side-effect (the C1 auto-clear of a slab above a placed
/// ladder), then paints the tile and returns `true`. Returns whether the paint landed so the caller
/// redraws only on a committed paint.
pub fn apply_placement(
    map: &mut EditorMap,
    registry: &TerrainDefRegistry,
    theme: ThemeUuid,
    placement: &ProposedPlacement,
    size: gdtf_battle_sim::level::GridSize,
) -> bool {
    match evaluate_placement(map, registry, theme, placement, size) {
        PlacementVerdict::Illegal(_) => false,
        PlacementVerdict::Legal { auto_clear } => {
            // C1: clear the slab the ladder would otherwise be sealed by, before painting.
            if let Some(slot) = auto_clear {
                map.clear(slot);
            }
            map.paint_at(placement.slot(), placement.tile(), size)
        }
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::{
        Cell,
        armor::{ArmorHardness, ArmorProtection},
        cover::{CoverHp, HeightBand},
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
        EditorTileClass, IllegalReason, PlacementVerdict, ProposedPlacement, apply_placement,
        classify, evaluate_placement,
    };
    use crate::editor_map::EditorMap;

    /// The theme key the test registry is associated with (a UUID; the registry resolves a
    /// terrain by UUID regardless of theme).
    fn theme() -> ThemeUuid {
        ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0149_1490_0001))
    }

    /// A terrain UUID built from a small constant (the test registry's keys).
    const fn tu(n: u128) -> TerrainUuid {
        TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(n))
    }

    /// A `4 × 4 × 3` drawable volume — enough storeys for the L0/L1 vertical rules, with a
    /// `1 × 1 × 1` fallback (the constructor is fallible; the fallback keeps the test panic-free).
    fn size() -> GridSize {
        GridSize::new(GridWidth::new(4), GridHeight::new(4), GridLevels::new(3))
            .unwrap_or_else(|_| GridSize::default())
    }

    /// A SLAB terrain def (the magnitudes are throwaway DATA — not pinned).
    fn slab_def(key: TerrainUuid, label: &str) -> TerrainDef {
        TerrainDef {
            key,
            display_name: TerrainDisplayName::new(label.to_owned()),
            sim_kind: TerrainSimKind::Slab {
                hp:               SlabHp::new(50),
                armor_protection: ArmorProtection::new(6),
                armor_hardness:   ArmorHardness::new(3),
            },
            presenter_kind: TerrainPresenterKind::Slab {
                graphic_name: TerrainGraphicKey::new("slab".to_owned()),
                footfall:     None,
            },
            tags: Vec::new(),
            on_death: None,
        }
    }

    /// A COVER terrain def (a tile the vertical rules never constrain).
    fn cover_def(key: TerrainUuid, label: &str) -> TerrainDef {
        TerrainDef {
            key,
            display_name: TerrainDisplayName::new(label.to_owned()),
            sim_kind: TerrainSimKind::Cover {
                hp:               CoverHp::new(20),
                armor_protection: ArmorProtection::new(2),
                armor_hardness:   ArmorHardness::new(1),
                height_band:      HeightBand::Low,
            },
            presenter_kind: TerrainPresenterKind::Cover {
                graphic_name: TerrainGraphicKey::new("cover".to_owned()),
            },
            tags: Vec::new(),
            on_death: None,
        }
    }

    /// A registry with a cover tile (`floor`), a slab tile (`slab`), and a ladder-named tile
    /// (`ladder`, recognised by display name). The ladder tile's `sim_kind` is plain Cover so the
    /// classifier's name path is what flags it.
    const FLOOR: TerrainUuid = tu(0x01);
    const SLAB: TerrainUuid = tu(0x02);
    const LADDER: TerrainUuid = tu(0x03);

    fn registry() -> TerrainDefRegistry {
        TerrainDefRegistry::new([
            (FLOOR, cover_def(FLOOR, "Deck Floor")),
            (SLAB, slab_def(SLAB, "Deck Slab")),
            (LADDER, cover_def(LADDER, "Steel Ladder")),
        ])
    }

    fn ground(cell: Cell) -> CellLevel {
        CellLevel::new(cell, Level::new(0))
    }

    fn above(cell: Cell) -> CellLevel {
        CellLevel::new(cell, Level::new(1))
    }

    /// Classification is semantics-driven: slab from the sim KIND, ladder from the NAME,
    /// everything else Other.
    #[test]
    fn classify_reads_kind_and_name() {
        let reg = registry();
        let th = theme();
        assert_eq!(
            classify(&reg, th, &SLAB),
            EditorTileClass::Slab,
            "a TerrainSimKind::Slab def classifies as a slab",
        );
        assert_eq!(
            classify(&reg, th, &LADDER),
            EditorTileClass::Ladder,
            "a ladder-named def classifies as a ladder (the model has no ladder kind)",
        );
        assert_eq!(
            classify(&reg, th, &FLOOR),
            EditorTileClass::Other,
            "a plain cover/floor def is unconstrained by the vertical rules",
        );
    }

    /// A plain legal placement: painting a non-slab on the ground plane commits and mutates the
    /// map.
    #[test]
    fn legal_floor_placement_commits_and_mutates_map() {
        let reg = registry();
        let th = theme();
        let mut map = EditorMap::new();
        let cell = Cell::new(1, 1);
        let placement = ProposedPlacement::new(ground(cell), FLOOR);

        assert_eq!(
            evaluate_placement(&map, &reg, th, &placement, size()),
            PlacementVerdict::legal(),
            "a non-slab on empty ground is plainly legal",
        );
        assert!(
            apply_placement(&mut map, &reg, th, &placement, size()),
            "a legal placement commits",
        );
        assert_eq!(
            map.tile_at(cell),
            Some(FLOOR),
            "a committed placement mutates the map (legal case)",
        );
        assert_eq!(map.painted_count(), 1);
    }

    /// C2: a slab onto a cell that already holds a ladder (SAME slot) is ILLEGAL — rejected, map
    /// UNCHANGED. The case the single-plane (`L0`) canvas reaches.
    ///
    /// Pin-discriminating: with the rule reverted (a slab onto a ladder treated as legal) the
    /// verdict would be `Legal`, `apply_placement` would return `true`, and the ladder would be
    /// OVERWRITTEN by the slab — every assertion here would flip.
    #[test]
    fn slab_on_ladder_same_cell_is_illegal_and_rejected() {
        let reg = registry();
        let th = theme();
        let mut map = EditorMap::new();
        let cell = Cell::new(2, 2);

        // Place a ladder on the ground (L0). Recognised by display name; a plain legal placement.
        let ladder = ProposedPlacement::new(ground(cell), LADDER);
        assert!(apply_placement(&mut map, &reg, th, &ladder, size()));
        assert_eq!(map.painted_count(), 1, "the ladder commits at L0");

        // Now try to slab the SAME cell (L0) — illegal (a slab can't seal a ladder's shaft).
        let slab = ProposedPlacement::new(ground(cell), SLAB);
        assert_eq!(
            evaluate_placement(&map, &reg, th, &slab, size()),
            PlacementVerdict::Illegal(IllegalReason::SlabSealsLadder),
            "a slab onto a ladder seals it — illegal (C2)",
        );
        assert!(
            !apply_placement(&mut map, &reg, th, &slab, size()),
            "an illegal placement is rejected (C2)",
        );
        assert_eq!(
            map.painted_count(),
            1,
            "a rejected placement leaves the map UNCHANGED (C2)",
        );
        assert_eq!(
            map.tile_at(cell),
            Some(LADDER),
            "the ladder is untouched — the illegal slab never overwrote it (C2)",
        );
    }

    /// C2 (multi-level): a slab directly ABOVE an existing ladder is ILLEGAL — rejected, map
    /// UNCHANGED (the ladder's destination would be sealed).
    #[test]
    fn slab_above_ladder_is_illegal_and_rejected() {
        let reg = registry();
        let th = theme();
        let mut map = EditorMap::new();
        let cell = Cell::new(1, 3);

        let ladder = ProposedPlacement::new(ground(cell), LADDER);
        assert!(apply_placement(&mut map, &reg, th, &ladder, size()));

        let slab = ProposedPlacement::new(above(cell), SLAB);
        assert_eq!(
            evaluate_placement(&map, &reg, th, &slab, size()),
            PlacementVerdict::Illegal(IllegalReason::SlabSealsLadder),
            "a slab over a ladder seals its destination — illegal (C2)",
        );
        assert!(
            !apply_placement(&mut map, &reg, th, &slab, size()),
            "an illegal placement is rejected (C2)",
        );
        assert!(
            map.tile_at_level(above(cell)).is_none(),
            "the illegal slab never entered the map (C2)",
        );
        assert_eq!(map.painted_count(), 1, "only the ladder remains (C2)");
    }

    /// C1: placing a LADDER auto-clears a SLAB directly above it — legal, with the slab removed.
    ///
    /// Pin-discriminating: with the C1 rule reverted (no auto-clear) the verdict would be a plain
    /// `Legal { auto_clear: None }`, the slab above would SURVIVE, and the painted count would be 2.
    #[test]
    fn ladder_auto_clears_slab_above() {
        let reg = registry();
        let th = theme();
        let mut map = EditorMap::new();
        let cell = Cell::new(0, 0);

        // Seed a slab one storey UP (L1) directly, as if previously painted.
        assert!(map.paint_at(above(cell), SLAB, size()));
        assert_eq!(map.painted_count(), 1, "the slab is seeded at L1");

        // Now place a ladder below it (L0): legal, and it auto-clears the slab above (C1).
        let ladder = ProposedPlacement::new(ground(cell), LADDER);
        assert_eq!(
            evaluate_placement(&map, &reg, th, &ladder, size()),
            PlacementVerdict::legal_clearing(above(cell)),
            "a ladder under a slab is legal and auto-clears the slab above (C1)",
        );
        assert!(
            apply_placement(&mut map, &reg, th, &ladder, size()),
            "the ladder placement commits (C1)",
        );
        assert!(
            map.tile_at_level(above(cell)).is_none(),
            "the slab directly above the ladder was AUTO-CLEARED (C1)",
        );
        assert_eq!(
            map.tile_at(cell),
            Some(LADDER),
            "the ladder was painted at L0 (C1)",
        );
        assert_eq!(
            map.painted_count(),
            1,
            "the slab is gone and the ladder is present — one entry (C1)",
        );
    }

    /// A ladder with NO slab above is a plain legal placement (no spurious auto-clear).
    #[test]
    fn ladder_without_slab_above_is_plain_legal() {
        let reg = registry();
        let th = theme();
        let map = EditorMap::new();
        let cell = Cell::new(3, 3);
        let ladder = ProposedPlacement::new(ground(cell), LADDER);
        assert_eq!(
            evaluate_placement(&map, &reg, th, &ladder, size()),
            PlacementVerdict::legal(),
            "a ladder with nothing above is plainly legal, no auto-clear",
        );
    }

    /// An out-of-bounds slot is illegal through the shared predicate (C3 surfaced as a verdict).
    #[test]
    fn out_of_bounds_is_illegal() {
        let reg = registry();
        let th = theme();
        let map = EditorMap::new();
        // x = 4 is past the 4-wide extent (valid 0..4).
        let placement = ProposedPlacement::new(ground(Cell::new(4, 0)), FLOOR);
        assert_eq!(
            evaluate_placement(&map, &reg, th, &placement, size()),
            PlacementVerdict::Illegal(IllegalReason::OutOfBounds),
            "a slot off the grid is illegal (C3)",
        );
    }
}
