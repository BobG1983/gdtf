//! The **emit step** — the GTW-431 third stage of the staged assembler (424 placement ->
//! 427 fill -> 431 emit/trigger): it pours a GTW-427 [`FilledPlacement`] into the sim's
//! ONE canonical battlefield value, a [`Situation`](crate::situation::Situation), as the
//! terrain entries that value holds INLINE.
//!
//! This is render-free, deterministic MODEL logic (the *behavioral* render-free constraint
//! the rest of `procgen` holds): it reads only the [`FilledPlacement`] the seeded pipeline
//! produced and translates each placed prefab's footprint-local geometry onto the board.
//! NOTHING here wires a live battle request or touches the presenter — that is GTW-433.
//!
//! # Why it emits a `Situation`, not a procgen-local type
//!
//! The contract is that the output be the sim's CANONICAL terrain-entries representation —
//! the very fields a [`Situation`](crate::situation::Situation) holds inline
//! ([`walls`](crate::situation::Situation::walls) /
//! [`scatter`](crate::situation::Situation::scatter) /
//! [`slabs`](crate::situation::Situation::slabs) /
//! [`default_floor`](crate::situation::Situation::default_floor) /
//! [`floors`](crate::situation::Situation::floors) /
//! [`vertical_links`](crate::situation::Situation::vertical_links)), NOT a throwaway
//! procgen-local mirror. So [`emit_level`] builds and returns a `Situation` directly: a
//! `Situation` whose `gangers` are still empty (rosters are placed elsewhere — GTW-433) but
//! whose terrain is the assembled level. GTW-432/GTW-433 source THIS `Situation`'s terrain
//! to replace the inline-authored terrain.
//!
//! # The v2 model (GTW-492)
//!
//! GTW-492 (child T07b of the GTW-476 data-model refactor) switches the procgen pipeline onto
//! the UUID-keyed v2 prefab model: the assembler / fill read the
//! [`PrefabRegistry`](crate::level::PrefabRegistry) of [`Prefab`](crate::level::Prefab)
//! fragments, and each fragment carries ONE
//! [`placements`](crate::level::PrefabSpec::placements) list of
//! [`TerrainPlacementEntry`](crate::level::TerrainPlacementEntry) (a `(piece: TerrainUuid, at:
//! CellLevel)` pair) in place of the legacy schema's FOUR split lists
//! (walls / scatter / slabs / floors). The emit therefore iterates that SINGLE list and
//! CLASSIFIES each placement by its referenced
//! [`TerrainDef`](crate::terrain::def::TerrainDef)'s
//! [`TerrainSimKind`](crate::terrain::def::TerrainSimKind) (resolved against the
//! [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry)): a `Wall`/`Cover` piece
//! pours into the [`walls`](crate::situation::Situation::walls) list, a `Slab` piece into the
//! [`slabs`](crate::situation::Situation::slabs) list — the routing the legacy four-list
//! schema authored explicitly is now DERIVED from the def. The level's `theme` is the
//! UUID-keyed [`ThemeUuid`] directly, and its `default_floor` is resolved from the
//! [`UuidThemeRegistry`] (the theme nominates its ground terrain). The GTW-491 (T07a) thin
//! shim (`piece_uuid` / `theme_uuid` minting synthetic legacy UUIDs) is REMOVED — the emit
//! produces the UUID-typed `Situation` directly.
//!
//! # The translation (footprint-local -> board cells)
//!
//! A [`Prefab`](crate::level::Prefab) authors its geometry in FOOTPRINT-LOCAL cells (origin
//! at `(0, 0)`); the packer placed it at a [`RegionRect`] whose [`origin`](RegionRect::origin)
//! is its min-corner on the board. So every authored placement cell — each
//! [`TerrainPlacementEntry`](crate::level::TerrainPlacementEntry), poured into a
//! [`CoverSpawn`](crate::situation::CoverSpawn) or [`SlabSpawn`](crate::situation::SlabSpawn) by
//! its classified kind — is shifted by the region origin onto the board. The storey index
//! (`z`) rides along unchanged: the packer space-packs on the ground plane only, so a
//! fragment's own slabs keep their authored storeys.
//!
//! # The floored dead space (C3)
//!
//! The GTW-427 no-fit fallback returns the leftover free rectangles verbatim in
//! [`FilledPlacement::dead_space`] for this step to FLOOR (the playable area is never
//! shrunk). Each dead-space cell is emitted as a [`FloorSpawn`](crate::situation::FloorSpawn)
//! naming the level [`default_floor`](crate::situation::Situation::default_floor) — so a
//! dead-space cell is an explicit, walkable open floor entry, not a silently-dropped hole.
//! (The level-wide `default_floor` already floors every open cell, so this is belt-and-
//! braces; emitting the explicit entries makes the "dead space is floored" contract a
//! concrete, testable terrain entry rather than an implicit consequence.)
//!
//! # Connectivity (by construction)
//!
//! Connectivity is BY CONSTRUCTION: the 1-cell `default_floor` seam every placement reserves
//! leaves a walkable corridor lattice around every placed region, so every open board cell is
//! reachable. GTW-497 removed the old fail-closed connectivity flood / rejection (and the
//! per-prefab opening machinery) — the v2 model authors no per-prefab openings, so the seam
//! lattice alone guarantees connectivity and there is nothing to re-flood or assert here.
//!
//! # Determinism (C1/C2)
//!
//! The emit reads the [`FilledPlacement`] in a FIXED order (player, enemy, then fill prefabs
//! in placement order, then dead-space rects in free-list order) and never iterates an
//! unordered map, so the SAME `FilledPlacement` always emits an IDENTICAL `Situation` — and
//! since the [`FilledPlacement`] itself is deterministic under a fixed
//! [`ProcgenRng`](crate::rng::ProcgenRng) seed, the whole pipeline is deterministic (C1).

use super::{
    assembler::{PlacedPrefab, assemble_placement_with},
    error::PackingError,
    fill::{FilledPlacement, fill_placement_with},
    findings::{EmittedLevel, ProcgenFinding},
    geometry::{MinPlayerSide, RegionRect},
    packer::SplitMode,
    tuning::ProcgenTuning,
};
use crate::{
    level::{GridSize, PrefabRegistry, ThemeUuid, UuidThemeRegistry},
    metric::{Cell, CellLevel, Level},
    rng::ProcgenRng,
    situation::{CoverSpawn, FloorSpawn, Situation, SlabSpawn},
    terrain::{
        def::{TerrainDefRegistry, TerrainUuid},
        entity::TerrainPieceKind,
    },
};

/// Run the WHOLE space-packing pipeline from one injected seed and emit the assembled
/// level as a [`Situation`](crate::situation::Situation) (GTW-431 C1/C2; GTW-492 v2 model):
/// assemble the GTW-424 placement, run the GTW-427 fill, then [`emit_level`] the result.
///
/// This is the deterministic SEED HARNESS the determinism test drives twice: given the
/// same `prefabs` / `themes` / `terrain_defs` / `theme` / `grid_size` and the SAME `rng`
/// seed state, it returns an IDENTICAL `Situation` (its terrain entries are equal). The
/// RULED defaults ([`SplitMode::default`],
/// [`MinPlayerSide::DEFAULT`](super::geometry::MinPlayerSide::DEFAULT) inside the assembler)
/// are used; the unit tests drive the explicit-split form via the staged functions directly
/// when they need to.
///
/// GTW-492: `prefabs` is the UUID-keyed [`PrefabRegistry`], `theme` the stable
/// [`ThemeUuid`]; `themes` ([`UuidThemeRegistry`]) supplies the theme's `default_floor`, and
/// `terrain_defs` ([`TerrainDefRegistry`]) classifies each placed piece's
/// [`TerrainSimKind`](crate::terrain::def::TerrainSimKind) so [`emit_level`] routes it into
/// the right `Situation` terrain list.
///
/// # Errors
///
/// Propagates every [`PackingError`] the assembler / fill can raise (no prefab for a role at
/// the theme, a footprint that does not fit, or a too-small player footprint). The emit step
/// itself is infallible (connectivity is by-construction — GTW-497). It NEVER
/// `unwrap`/`expect`/`panic`s. GTW-582: degraded resolutions (a theme with no registry
/// default floor, an unresolvable placed piece) are NOT errors — they ride back as the
/// [`EmittedLevel::findings`] the app-side driver reports.
pub fn generate_level(
    prefabs: &PrefabRegistry,
    themes: &UuidThemeRegistry,
    terrain_defs: &TerrainDefRegistry,
    theme: ThemeUuid,
    grid_size: GridSize,
    rng: &mut ProcgenRng,
    tuning: &ProcgenTuning,
) -> Result<EmittedLevel, PackingError> {
    let placement = assemble_placement_with(
        prefabs,
        theme,
        grid_size,
        rng,
        SplitMode::default(),
        MinPlayerSide::DEFAULT,
    )?;
    let filled = fill_placement_with(
        placement,
        prefabs,
        theme,
        grid_size,
        tuning,
        rng,
        SplitMode::default(),
    )?;
    Ok(emit_level(&filled, theme, grid_size, themes, terrain_defs))
}

/// Emit a GTW-427 [`FilledPlacement`] into the sim's canonical
/// [`Situation`](crate::situation::Situation) — the GTW-431 emit step (C3); GTW-492 v2 model.
///
/// Translates every placed prefab's footprint-local geometry onto the board (shifted by its
/// placed region origin), resolves the level-wide `default_floor` from the `themes` registry
/// (the theme nominates its ground terrain), and floors the
/// [`dead_space`](FilledPlacement::dead_space) cells with that `default_floor`. The returned
/// [`EmittedLevel::situation`] has empty `gangers` (rosters are placed by GTW-433, not by
/// this terrain emit); its `theme` is the UUID-keyed [`ThemeUuid`] directly, its `grid_size`
/// the assembled level's.
///
/// Connectivity is by-construction via the 1-cell `default_floor` seam every placement
/// reserves (GTW-497 removed the old fail-closed connectivity flood), so the emit is
/// infallible — it returns an [`EmittedLevel`] directly, never a `Result`. GTW-582: every
/// DEGRADED resolution it takes rides back as an [`EmittedLevel::findings`] entry (never
/// silent):
///
/// - a `theme` absent from `themes` (or one nominating no floor) pours the NIL-sentinel
///   `default_floor` (setup then skips registry floor resolution — playable but degraded)
///   and records [`ProcgenFinding::MissingThemeDefaultFloor`];
/// - a placed piece whose UUID is NOT in `terrain_defs` pours FAIL-OPEN into `walls` (so it
///   surfaces as a
///   [`BattleSetupError::TerrainNotFound`](crate::situation::BattleSetupError) at setup
///   rather than being silently dropped) and records
///   [`ProcgenFinding::UnresolvedTerrainPiece`] once per unique UUID.
///
/// GTW-492: iterates each fragment's SINGLE
/// [`placements`](crate::level::PrefabSpec::placements) list (not four split lists) and
/// CLASSIFIES each placement by its referenced
/// [`TerrainDef`](crate::terrain::def::TerrainDef)'s
/// [`TerrainSimKind`](crate::terrain::def::TerrainSimKind) (resolved against `terrain_defs`):
/// a `Wall`/`Cover` piece pours into [`walls`](crate::situation::Situation::walls), a `Slab`
/// piece into [`slabs`](crate::situation::Situation::slabs).
#[must_use]
pub fn emit_level(
    filled: &FilledPlacement,
    theme: ThemeUuid,
    grid_size: GridSize,
    themes: &UuidThemeRegistry,
    terrain_defs: &TerrainDefRegistry,
) -> EmittedLevel {
    let placement = filled.placement();
    let mut findings: Vec<ProcgenFinding> = Vec::new();

    // The level-wide default floor: the THEME's nominated ground terrain (GTW-492 — the seam
    // lattice is this floor). Every open cell — incl. the floored dead space — is this. A
    // theme absent from `themes` (the headless empty-registry harness, or a dangling authored
    // theme) yields the nil sentinel, which skips registry floor resolution at setup — the
    // LAST-RESORT degraded pour, recorded as a finding so it is never silent (GTW-582 C3(d)).
    let default_floor = themes.default_floor(&theme).unwrap_or_else(|| {
        findings.push(ProcgenFinding::MissingThemeDefaultFloor { theme });
        crate::terrain::def::TerrainUuid::default()
    });

    let mut situation = Situation::new();
    situation.theme = theme;
    situation.grid_size = grid_size;
    situation.default_floor = default_floor;

    // Pour every placed prefab (player, enemy, then fill in placement order) — a FIXED
    // order, so the emit is deterministic (C1/C2). Each prefab's footprint-local placements
    // are translated by its placed region origin onto the board and classified by kind; a
    // fail-open unresolved pour records its finding (deduplicated, first-encounter order).
    pour_prefab(
        placement.player(),
        &mut situation,
        terrain_defs,
        &mut findings,
    );
    pour_prefab(
        placement.enemy(),
        &mut situation,
        terrain_defs,
        &mut findings,
    );
    for placed in filled.fill() {
        pour_prefab(placed, &mut situation, terrain_defs, &mut findings);
    }

    // Floor the dead space (C3): each leftover free-rect cell is emitted as an explicit
    // `default_floor` override, so the dead space is concretely floored (never dropped).
    for rect in filled.dead_space() {
        floor_region(*rect, default_floor, &mut situation);
    }

    EmittedLevel {
        situation,
        findings,
    }
}

/// Translate one placed prefab's footprint-local geometry onto the board and append it to
/// `situation` — the SINGLE GTW-486 [`placements`](crate::level::PrefabSpec::placements)
/// list, each cell shifted by the prefab's placed region origin and CLASSIFIED by kind.
///
/// GTW-492: a v2 fragment carries ONE placements list (not four split lists). Each placement
/// is routed into the right `Situation` terrain list by its referenced
/// [`TerrainDef`](crate::terrain::def::TerrainDef)'s
/// [`TerrainSimKind`](crate::terrain::def::TerrainSimKind) (resolved against `terrain_defs`):
/// `Wall`/`Cover` → [`walls`](crate::situation::Situation::walls), `Slab` →
/// [`slabs`](crate::situation::Situation::slabs). An UNRESOLVABLE piece falls open into
/// `walls` (so it surfaces at setup, never silently dropped — see [`emit_level`]) and is
/// recorded in `findings` once per unique UUID (GTW-582 C3(d)).
fn pour_prefab(
    placed: &PlacedPrefab,
    situation: &mut Situation,
    terrain_defs: &TerrainDefRegistry,
    findings: &mut Vec<ProcgenFinding>,
) {
    let origin = placed.region().origin();
    let spec = placed.prefab().spec();

    for placement in &spec.placements {
        let at = translate(placement.at, origin);
        match classify(placement.piece, terrain_defs) {
            // Slab → the slabs list (the per-slab structural HP resolves from the def).
            PlacedKind::Slab => situation.slabs.push(SlabSpawn::new(at, placement.piece)),
            // Wall / Cover → the walls list.
            PlacedKind::Cover => situation.walls.push(CoverSpawn::new(at, placement.piece)),
            // The fail-open unresolved fallback → the walls list, PLUS a deduplicated
            // finding so the degraded pour is on the record (never silent).
            PlacedKind::Unresolved => {
                situation.walls.push(CoverSpawn::new(at, placement.piece));
                let finding = ProcgenFinding::UnresolvedTerrainPiece {
                    piece: placement.piece,
                };
                if !findings.contains(&finding) {
                    findings.push(finding);
                }
            }
        }
    }
}

/// Which `Situation` terrain list a v2 placement pours into — derived from its referenced
/// [`TerrainDef`](crate::terrain::def::TerrainDef)'s
/// [`TerrainSimKind`](crate::terrain::def::TerrainSimKind) (GTW-492).
enum PlacedKind {
    /// A `Wall`/`Cover` piece — pours into [`Situation::walls`](crate::situation::Situation).
    Cover,
    /// A `Slab` piece — pours into [`Situation::slabs`](crate::situation::Situation).
    Slab,
    /// A piece whose UUID resolves NO definition — poured fail-OPEN into the walls list
    /// AND recorded as a [`ProcgenFinding::UnresolvedTerrainPiece`] (GTW-582 C3(d)).
    Unresolved,
}

/// Classify a placed piece by its canonical [`TerrainPieceKind`] (projected from its
/// [`TerrainSimKind`](crate::terrain::def::TerrainSimKind) in `terrain_defs` — GTW-574) —
/// `Slab` defs route to the slabs list, everything else routes to the walls list.
///
/// The fail-OPEN fallback for an unresolved UUID keeps the piece in the emitted level (poured
/// into `walls`) so it surfaces as a
/// [`BattleSetupError::TerrainNotFound`](crate::situation::BattleSetupError) at setup, rather
/// than being silently dropped from the procgen-generated terrain — distinguished as
/// [`PlacedKind::Unresolved`] so the pour records the GTW-582 finding.
fn classify(piece: TerrainUuid, terrain_defs: &TerrainDefRegistry) -> PlacedKind {
    // A kind-IDENTITY decision (no per-variant payload), so it classifies over the
    // canonical `TerrainPieceKind` projection (GTW-574 C2) — exhaustive, no wildcard.
    match terrain_defs.def(&piece).map(|def| def.sim_kind.kind()) {
        Some(TerrainPieceKind::Slab) => PlacedKind::Slab,
        // Wall / Cover / Emplacement (a cover-like smashable structure resolved via the
        // cover path) route to the walls list.
        Some(TerrainPieceKind::Wall | TerrainPieceKind::Cover | TerrainPieceKind::Emplacement) => {
            PlacedKind::Cover
        }
        // The fail-open unresolved fallback — walls list + finding.
        None => PlacedKind::Unresolved,
    }
}

/// Emit every cell of a dead-space `rect` as an explicit `default_floor`
/// [`FloorSpawn`](crate::situation::FloorSpawn) override on level `0` (the ground plane the
/// packer reasons on), so the floored dead space is a concrete terrain entry (C3).
fn floor_region(rect: RegionRect, default_floor: TerrainUuid, situation: &mut Situation) {
    let origin = rect.origin();
    let footprint = rect.footprint();
    for dy in 0..footprint.height() {
        for dx in 0..footprint.width() {
            let cell = Cell::new(origin.x + dx, origin.y + dy);
            situation.floors.push(FloorSpawn::new(
                CellLevel::new(cell, Level::new(0)),
                default_floor,
            ));
        }
    }
}

/// Translate a footprint-local `(cell, level)` by a placed region `origin` onto the board —
/// the ground-plane `x`/`y` are shifted by the origin; the storey index `z` rides along
/// unchanged (the packer space-packs on the ground plane only).
fn translate(local: CellLevel, origin: Cell) -> CellLevel {
    let cell = Cell::new(local.x + origin.x, local.y + origin.y);
    // The authored storey rides along unchanged, via the canonical
    // CellLevel::level accessor (GTW-565).
    CellLevel::new(cell, local.level())
}
