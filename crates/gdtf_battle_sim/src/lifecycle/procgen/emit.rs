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
//! # The translation (footprint-local -> board cells)
//!
//! A [`Prefab`](crate::level::Prefab) authors its geometry in FOOTPRINT-LOCAL cells (origin
//! at `(0, 0)`); the packer placed it at a [`RegionRect`] whose [`origin`](RegionRect::origin)
//! is its min-corner on the board. So every authored cell — each
//! [`CoverSpawn`](crate::situation::CoverSpawn) wall/scatter, each
//! [`SlabSpawn`](crate::situation::SlabSpawn), each
//! [`FloorSpawn`](crate::situation::FloorSpawn) override, each
//! [`VerticalLink`](crate::vertical::VerticalLink) endpoint — is shifted by the region
//! origin onto the board. The storey index (`z`) rides along unchanged: the packer
//! space-packs on the ground plane only, so a fragment's own slabs / links keep their
//! authored storeys.
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
//! # Connectivity (fail-closed, C3)
//!
//! Before returning, [`emit_level`] re-runs the OQ-4 [`count_seam_reachable`] flood over the
//! player + enemy + every fill region. A disconnected level returns
//! [`PackingError::Disconnected`] — it NEVER silently emits a broken level (the fail-closed
//! assertion, NEVER a repair). By construction the 1-cell seam every placement reserves plus
//! each prefab's `>= 1` edge opening keeps it connected, so this never fires in practice; it
//! is the contract backstop.
//!
//! # Determinism (C1/C2)
//!
//! The emit reads the [`FilledPlacement`] in a FIXED order (player, enemy, then fill prefabs
//! in placement order, then dead-space rects in free-list order) and never iterates an
//! unordered map, so the SAME `FilledPlacement` always emits an IDENTICAL `Situation` — and
//! since the [`FilledPlacement`] itself is deterministic under a fixed
//! [`ProcgenRng`](crate::rng::ProcgenRng) seed, the whole pipeline is deterministic (C1).

use super::{
    assembler::{PlacedPrefab, assemble_placement_with, count_seam_reachable},
    error::PackingError,
    fill::{FilledPlacement, fill_placement_with},
    geometry::{MinPlayerSide, RegionRect},
    packer::SplitMode,
    tuning::ProcgenTuning,
};
use crate::{
    level::{GridSize, LevelTheme, PrefabRegistry},
    metric::{Cell, CellLevel, Level},
    rng::ProcgenRng,
    situation::{CoverSpawn, FloorSpawn, Situation, SlabSpawn},
    terrain::piece::TerrainName,
    vertical::VerticalLink,
};

/// Run the WHOLE space-packing pipeline from one injected seed and emit the assembled
/// level as a [`Situation`](crate::situation::Situation) (GTW-431 C1/C2): assemble the
/// GTW-424 placement, run the GTW-427 fill, then [`emit_level`] the result.
///
/// This is the deterministic SEED HARNESS the determinism test drives twice: given the
/// same `registry` / `theme` / `grid_size` and the SAME `rng` seed state, it returns an
/// IDENTICAL `Situation` (its terrain entries are equal). The RULED defaults
/// ([`SplitMode::default`], [`MinPlayerSide::DEFAULT`](super::geometry::MinPlayerSide::DEFAULT)
/// inside the assembler) are used; the unit tests drive the explicit-split form via the
/// staged functions directly when they need to.
///
/// # Errors
///
/// Propagates every [`PackingError`] the assembler / fill / emit can raise (no prefab for a
/// role, a footprint that does not fit, a too-small player footprint, or — the fail-closed
/// backstop — a disconnected emitted level). It NEVER `unwrap`/`expect`/`panic`s.
pub fn generate_level(
    registry: &PrefabRegistry,
    theme: LevelTheme,
    grid_size: GridSize,
    rng: &mut ProcgenRng,
    tuning: &ProcgenTuning,
) -> Result<Situation, PackingError> {
    let placement = assemble_placement_with(
        registry,
        theme,
        grid_size,
        rng,
        SplitMode::default(),
        MinPlayerSide::DEFAULT,
    )?;
    let filled = fill_placement_with(
        placement,
        registry,
        theme,
        grid_size,
        tuning,
        rng,
        SplitMode::default(),
    )?;
    emit_level(&filled, theme, grid_size)
}

/// Emit a GTW-427 [`FilledPlacement`] into the sim's canonical
/// [`Situation`](crate::situation::Situation) — the GTW-431 emit step (C3).
///
/// Translates every placed prefab's footprint-local geometry onto the board (shifted by its
/// placed region origin), pours each prefab's `default_floor` choice (the player prefab's is
/// the level-wide one), floors the [`dead_space`](FilledPlacement::dead_space) cells with
/// the level `default_floor`, and re-runs the OQ-4 connectivity flood as a fail-closed
/// assertion. The returned `Situation` has empty `gangers` (rosters are placed by GTW-433,
/// not by this terrain emit); its `theme` / `grid_size` are the assembled level's.
///
/// # Errors
///
/// [`PackingError::Disconnected`] if the connectivity flood finds a placed region
/// unreachable from the player region (the fail-closed backstop — by construction the seam
/// lattice keeps the level connected, so this never fires in practice). It NEVER panics.
pub fn emit_level(
    filled: &FilledPlacement,
    theme: LevelTheme,
    grid_size: GridSize,
) -> Result<Situation, PackingError> {
    let placement = filled.placement();

    // Fail-closed connectivity assertion (C3): the player + enemy + every fill region must
    // be seam-reachable from the player region. NEVER a repair — a disconnected level is
    // rejected, it does not silently ship.
    let mut regions: Vec<RegionRect> =
        vec![placement.player().region(), placement.enemy().region()];
    regions.extend(filled.fill().iter().map(PlacedPrefab::region));
    let board = RegionRect::board(grid_size);
    let (reached, placed) = count_seam_reachable(&regions, board);
    if reached != placed {
        return Err(PackingError::Disconnected { reached, placed });
    }

    // The level-wide default floor: the player-spawn prefab's `default_floor` (the seam
    // lattice is this floor). Every open cell — incl. the floored dead space — is this.
    let default_floor = placement.player().prefab().spec().default_floor.clone();

    let mut situation = Situation::new();
    situation.theme = theme;
    situation.grid_size = grid_size;
    situation.default_floor = default_floor.clone();

    // Pour every placed prefab (player, enemy, then fill in placement order) — a FIXED
    // order, so the emit is deterministic (C1/C2). Each prefab's footprint-local cells are
    // translated by its placed region origin onto the board.
    pour_prefab(placement.player(), &mut situation);
    pour_prefab(placement.enemy(), &mut situation);
    for placed in filled.fill() {
        pour_prefab(placed, &mut situation);
    }

    // Floor the dead space (C3): each leftover free-rect cell is emitted as an explicit
    // `default_floor` override, so the dead space is concretely floored (never dropped).
    for rect in filled.dead_space() {
        floor_region(*rect, &default_floor, &mut situation);
    }

    Ok(situation)
}

/// Translate one placed prefab's footprint-local geometry onto the board and append it to
/// `situation` — walls / scatter / slabs / floor overrides / vertical links, each cell
/// shifted by the prefab's placed region origin.
fn pour_prefab(placed: &PlacedPrefab, situation: &mut Situation) {
    let origin = placed.region().origin();
    let spec = placed.prefab().spec();

    for wall in &spec.walls {
        situation.walls.push(CoverSpawn::new(
            translate(wall.at, origin),
            wall.piece.clone(),
        ));
    }
    for prop in &spec.scatter {
        situation.scatter.push(CoverSpawn::new(
            translate(prop.at, origin),
            prop.piece.clone(),
        ));
    }
    for slab in &spec.slabs {
        situation.slabs.push(SlabSpawn::new(
            translate(slab.at, origin),
            slab.piece.clone(),
        ));
    }
    for floor in &spec.floors {
        situation.floors.push(FloorSpawn::new(
            translate(floor.at, origin),
            floor.piece.clone(),
        ));
    }
    for link in &spec.vertical_links {
        situation.vertical_links.push(VerticalLink::new(
            translate(link.from, origin),
            translate(link.to, origin),
            link.kind,
        ));
    }
}

/// Emit every cell of a dead-space `rect` as an explicit `default_floor`
/// [`FloorSpawn`](crate::situation::FloorSpawn) override on level `0` (the ground plane the
/// packer reasons on), so the floored dead space is a concrete terrain entry (C3).
fn floor_region(rect: RegionRect, default_floor: &TerrainName, situation: &mut Situation) {
    let origin = rect.origin();
    let footprint = rect.footprint();
    for dy in 0..footprint.height() {
        for dx in 0..footprint.width() {
            let cell = Cell::new(origin.x + dx, origin.y + dy);
            situation.floors.push(FloorSpawn::new(
                CellLevel::new(cell, Level::new(0)),
                default_floor.clone(),
            ));
        }
    }
}

/// Translate a footprint-local `(cell, level)` by a placed region `origin` onto the board —
/// the ground-plane `x`/`y` are shifted by the origin; the storey index `z` rides along
/// unchanged (the packer space-packs on the ground plane only).
fn translate(local: CellLevel, origin: Cell) -> CellLevel {
    let cell = Cell::new(local.x + origin.x, local.y + origin.y);
    // `local.z` is the prefab's authored storey index (`0..MAX_LEVELS`), built through the
    // `CellLevel` constructor, so it is always a small non-negative storey — clamp the i32
    // -> u8 cast fail-closed (a real authored storey can never exceed `u8`).
    let storey = u8::try_from(local.z).unwrap_or(0);
    CellLevel::new(cell, Level::new(storey))
}
