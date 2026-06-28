//! The **first-half assembler** — anchor selection + strict-opposite enemy placement +
//! the OQ-4 connectivity assertion (GTW-424).
//!
//! This is the GTW-424 slice of the staged assembler (424 placement -> 427 fill -> 431
//! emit/trigger): it picks a player anchor from [`ProcgenRng`](crate::rng::ProcgenRng)
//! (C1), places a `>= 10x10` player-spawn prefab there (C1/OQ-5), places an enemy-spawn
//! prefab at the STRICT geometric opposite (C2/OQ-2), reserves the 1-cell seam around both
//! (OQ-3), and ASSERTS connectivity-by-construction (OQ-4 — fail-closed, never a repair).
//! It returns the two [`PlacedPrefab`]s; the GTW-427 fill pass and the GTW-431
//! emit-to-`Situation` step build on top. NOTHING here wires `BattleScapeState` (a later
//! ticket).

use super::{
    anchor::Anchor,
    error::PackingError,
    geometry::{Footprint, MinPlayerSide, RegionCount, RegionRect},
    packer::{MaxRectsPacker, SplitMode},
};
use crate::{
    level::{GridSize, LevelTheme, Prefab, PrefabRegistry, SpawnRole},
    rng::ProcgenRng,
};

/// One prefab the assembler has PLACED — the chosen [`Prefab`], the [`Anchor`] it sits at,
/// and its placed [`RegionRect`] on the board (GTW-424).
///
/// A named struct (no-bare-types: a placement is a domain value, not a bare tuple). The
/// GTW-427 fill pass reads the region to stamp interior fill; the GTW-431 emit step reads
/// the prefab + origin to pour the fragment's walls/scatter/slabs into the
/// [`Situation`](crate::situation::Situation). Holds the prefab BY VALUE (it is `Clone`),
/// so a placement survives the registry borrow ending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedPrefab {
    /// The chosen prefab (player-spawn or enemy-spawn fragment).
    prefab: Prefab,
    /// The anchor it was placed flush against.
    anchor: Anchor,
    /// Its placed footprint on the board (min-corner origin + extent).
    region: RegionRect,
}

impl PlacedPrefab {
    /// Build a placed-prefab record.
    #[must_use]
    pub const fn new(prefab: Prefab, anchor: Anchor, region: RegionRect) -> Self {
        Self {
            prefab,
            anchor,
            region,
        }
    }

    /// The chosen prefab.
    #[must_use]
    pub const fn prefab(&self) -> &Prefab {
        &self.prefab
    }

    /// The anchor it sits at.
    #[must_use]
    pub const fn anchor(&self) -> Anchor {
        self.anchor
    }

    /// Its placed region on the board.
    #[must_use]
    pub const fn region(&self) -> RegionRect {
        self.region
    }
}

/// The first-half placement RESULT — the player-spawn and enemy-spawn prefabs and where
/// they landed (GTW-424).
///
/// A named struct (no-bare-types: the placement outcome is a domain value). The C1/C2
/// deliverable: a `>= 10x10` player-spawn at a deterministically chosen anchor and an
/// enemy-spawn at its strict opposite, both seam-separated and connectivity-asserted. The
/// GTW-427 fill pass + GTW-431 emit step consume this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The placed player-spawn prefab (at the RNG-chosen anchor).
    player: PlacedPrefab,
    /// The placed enemy-spawn prefab (at the strict-opposite anchor).
    enemy:  PlacedPrefab,
}

impl Placement {
    /// The placed player-spawn prefab.
    #[must_use]
    pub const fn player(&self) -> &PlacedPrefab {
        &self.player
    }

    /// The placed enemy-spawn prefab.
    #[must_use]
    pub const fn enemy(&self) -> &PlacedPrefab {
        &self.enemy
    }
}

/// Assemble the GTW-424 first-half placement: choose a player anchor, place a `>= 10x10`
/// player-spawn prefab there, place an enemy-spawn prefab at the strict opposite, reserve
/// the 1-cell seam around both, and assert connectivity-by-construction.
///
/// The ONLY RNG draw is [`Anchor::choose`] (the player anchor, C1); the enemy anchor is
/// the zero-draw strict [`opposite`](Anchor::opposite) (OQ-2). Prefab CHOICE within a
/// `(theme, size, role)` bucket is the FIRST candidate (deterministic order — no RNG draw
/// in GTW-424; richer prefab selection is GTW-427's concern), so the same seed always
/// produces the same placement (the determinism contract).
///
/// # Errors
///
/// - [`PackingError::NoPrefabForRole`] if the registry has no player- or enemy-spawn
///   prefab for `(theme, size, role)`.
/// - [`PackingError::PlayerFootprintTooSmall`] if the chosen player prefab's footprint is
///   below [`MinPlayerSide`] (OQ-5 runtime backstop; load-time rejection is preferred).
/// - [`PackingError::FootprintDoesNotFit`] if a chosen prefab's footprint (plus seam) does
///   not fit its region (C2 — the player anchor region, or the strict-opposite enemy
///   region).
/// - [`PackingError::Disconnected`] if the OQ-4 connectivity assertion fails (fail-closed,
///   NEVER repaired).
pub fn assemble_placement(
    registry: &PrefabRegistry,
    theme: LevelTheme,
    grid_size: GridSize,
    rng: &mut ProcgenRng,
) -> Result<Placement, PackingError> {
    assemble_placement_with(
        registry,
        theme,
        grid_size,
        rng,
        SplitMode::default(),
        MinPlayerSide::DEFAULT,
    )
}

/// The full assembler with explicit [`SplitMode`] (the OQ-7 A/B flag) and minimum
/// player-side floor — [`assemble_placement`] is the RULED-defaults wrapper.
///
/// Exposed so the A/B comparison (`MaxRects` vs Guillotine) and the unit tests can drive
/// the packer's split strategy without changing the shipped default.
///
/// # Errors
///
/// Same as [`assemble_placement`].
pub fn assemble_placement_with(
    registry: &PrefabRegistry,
    theme: LevelTheme,
    grid_size: GridSize,
    rng: &mut ProcgenRng,
    split: SplitMode,
    min_player_side: MinPlayerSide,
) -> Result<Placement, PackingError> {
    let board = RegionRect::board(grid_size);
    let mut packer = MaxRectsPacker::new(board, split, super::geometry::Margin::DEFAULT);

    // C1: choose the player anchor — the ONLY RNG draw.
    let player_anchor = Anchor::choose(rng);
    // OQ-2: the enemy anchor is the zero-draw strict opposite.
    let enemy_anchor = player_anchor.opposite();

    // C1: pick the player-spawn prefab — the largest registered fragment that clears the
    // OQ-5 minimum AND fits (with its seam) flush at the player anchor. OQ-5's cap on the
    // player footprint is realised here: a fragment too large for the board (so the
    // opposite enemy region could not also fit) is skipped, never chosen.
    let player_prefab = pick_player_prefab(
        registry,
        theme,
        &packer,
        board,
        player_anchor,
        min_player_side,
    )?;
    let player_footprint = Footprint::of(player_prefab.spec().size);
    let player_region = board.place_at_anchor(player_anchor, player_footprint);
    // The fit was already checked in selection, but commit it (and re-guard fail-closed).
    if !packer.place(player_region) {
        return Err(PackingError::FootprintDoesNotFit {
            anchor:    player_anchor,
            footprint: player_footprint,
            region:    board,
        });
    }

    // C2: pick + place the enemy-spawn prefab at the strict-opposite anchor — the largest
    // registered enemy fragment that FITS the remaining space at the opposite anchor.
    let enemy_prefab = pick_fitting_prefab(
        registry,
        theme,
        SpawnRole::Enemy,
        &packer,
        board,
        enemy_anchor,
    )?;
    let enemy_footprint = Footprint::of(enemy_prefab.spec().size);
    let enemy_region = board.place_at_anchor(enemy_anchor, enemy_footprint);
    if !packer.place(enemy_region) {
        return Err(PackingError::FootprintDoesNotFit {
            anchor:    enemy_anchor,
            footprint: enemy_footprint,
            region:    board,
        });
    }

    let placement = Placement {
        player: PlacedPrefab::new(player_prefab, player_anchor, player_region),
        enemy:  PlacedPrefab::new(enemy_prefab, enemy_anchor, enemy_region),
    };

    // OQ-4: connectivity-by-construction ASSERTION (fail-closed, never a repair).
    assert_connected(&placement, board)?;

    Ok(placement)
}

/// Every `(theme, role)` prefab the registry holds, in a DETERMINISTIC order (sorted by
/// footprint area DESC then by name, so the iteration order does not depend on the
/// registry's unordered `HashMap`) — the candidate list both pickers scan.
///
/// Prefabs are level FRAGMENTS smaller than the board: their registry `size` key is the
/// fragment footprint, NOT the board. So the assembler enumerates ACROSS sizes for a
/// `(theme, role)` and picks a FITTING one — it never assumes a fragment fills the board.
fn candidates(registry: &PrefabRegistry, theme: LevelTheme, role: SpawnRole) -> Vec<Prefab> {
    let mut out: Vec<Prefab> = registry
        .keys()
        .filter(|k| k.theme == theme && k.spawn_role == role)
        .flat_map(|k| registry.prefabs_for(k).iter().cloned())
        .collect();
    // Deterministic order: largest fragment first (prefer the densest deployment zone that
    // fits), ties broken by name so the order is total and seed-independent.
    out.sort_by(|a, b| {
        let area = |p: &Prefab| {
            let f = Footprint::of(p.spec().size);
            i64::from(f.width()) * i64::from(f.height())
        };
        area(b)
            .cmp(&area(a))
            .then_with(|| (**a.name()).cmp(&**b.name()))
    });
    out
}

/// Pick the PLAYER-spawn prefab: the first candidate (largest-first, deterministic) that
/// clears the OQ-5 minimum side AND fits — with its seam — flush at the player anchor.
///
/// OQ-5's cap is realised here: a fragment too large to leave room for the opposite enemy
/// region is simply not chosen (it fails the packer fit). Fails closed with
/// [`PackingError::NoPrefabForRole`] if no player prefab exists at all, or
/// [`PackingError::PlayerFootprintTooSmall`] if EVERY candidate is below the minimum side,
/// or [`PackingError::FootprintDoesNotFit`] if every (large-enough) candidate is too large
/// to fit the board with its seam.
fn pick_player_prefab(
    registry: &PrefabRegistry,
    theme: LevelTheme,
    packer: &MaxRectsPacker,
    board: RegionRect,
    anchor: Anchor,
    min_player_side: MinPlayerSide,
) -> Result<Prefab, PackingError> {
    let candidates = candidates(registry, theme, SpawnRole::Player);
    if candidates.is_empty() {
        return Err(PackingError::NoPrefabForRole {
            theme,
            role: SpawnRole::Player,
        });
    }

    // Track the best diagnostic error: a too-small one only matters if NO candidate
    // clears the minimum; a does-not-fit one if none of the big-enough ones fit.
    let mut last_too_small: Option<PackingError> = None;
    let mut last_no_fit: Option<PackingError> = None;
    for prefab in candidates {
        let footprint = Footprint::of(prefab.spec().size);
        if footprint.min_side() < min_player_side.cells() {
            last_too_small = Some(PackingError::PlayerFootprintTooSmall {
                footprint,
                min_side: min_player_side,
            });
            continue;
        }
        let region = board.place_at_anchor(anchor, footprint);
        if packer.fits(region) {
            return Ok(prefab);
        }
        last_no_fit = Some(PackingError::FootprintDoesNotFit {
            anchor,
            footprint,
            region: board,
        });
    }
    // Prefer the "does not fit" diagnostic (a big-enough candidate existed but did not
    // fit) over "too small" (no candidate even reached the minimum).
    Err(last_no_fit
        .or(last_too_small)
        .unwrap_or(PackingError::NoPrefabForRole {
            theme,
            role: SpawnRole::Player,
        }))
}

/// Pick a prefab of `role` that FITS — with its seam — flush at `anchor` against the
/// current free space (the enemy-spawn picker, C2). Largest fitting fragment first.
///
/// Fails closed with [`PackingError::NoPrefabForRole`] if none exists, or
/// [`PackingError::FootprintDoesNotFit`] if every candidate is too large for the remaining
/// space at the opposite anchor.
fn pick_fitting_prefab(
    registry: &PrefabRegistry,
    theme: LevelTheme,
    role: SpawnRole,
    packer: &MaxRectsPacker,
    board: RegionRect,
    anchor: Anchor,
) -> Result<Prefab, PackingError> {
    let candidates = candidates(registry, theme, role);
    if candidates.is_empty() {
        return Err(PackingError::NoPrefabForRole { theme, role });
    }
    let mut last_no_fit: Option<PackingError> = None;
    for prefab in candidates {
        let footprint = Footprint::of(prefab.spec().size);
        let region = board.place_at_anchor(anchor, footprint);
        if packer.fits(region) {
            return Ok(prefab);
        }
        last_no_fit = Some(PackingError::FootprintDoesNotFit {
            anchor,
            footprint,
            region: board,
        });
    }
    Err(last_no_fit.unwrap_or(PackingError::NoPrefabForRole { theme, role }))
}

/// Assert the placed prefabs are connected BY CONSTRUCTION (OQ-4) — a fail-closed
/// assertion, NEVER a repair step.
///
/// Connectivity is structural: the 1-cell `default_floor` seam lattice plus each prefab's
/// `>= 1` edge opening onto its boundary means every placed region is reachable from any
/// other through the open seam corridors that fill all cells NOT covered by a prefab. Two
/// regions are seam-reachable iff their seam-padded extents touch (share a boundary) OR
/// both touch the surrounding board seam — which, on a board with a 1-cell margin between
/// the only two prefabs, always holds. We flood-fill region adjacency through the padded
/// (seam-inclusive) extents starting from the player region; if any placed region is
/// unreachable, generation is REJECTED with [`PackingError::Disconnected`] (no carving).
fn assert_connected(placement: &Placement, board: RegionRect) -> Result<(), PackingError> {
    let regions = [placement.player.region(), placement.enemy.region()];
    let (reached, placed) = count_seam_reachable(&regions, board);
    if reached == placed {
        Ok(())
    } else {
        Err(PackingError::Disconnected { reached, placed })
    }
}

/// Count how many placed regions are reachable from region 0 (the player region) through
/// the open `default_floor` seam — the OQ-4 connectivity measure (public so the
/// fail-closed path is unit-testable with a constructed disconnected placement).
///
/// This is a REAL cell-grid flood, not a pairwise heuristic: every board cell NOT inside a
/// placed region's interior is walkable `default_floor` (the seam lattice). We flood that
/// floor in 4-connectivity from the cells of region 0, then a region is "reached" if ANY of
/// its boundary cells abuts the flood. A connected placement has `reached == total`; a
/// region walled off by another region that spans a full board axis (no floor corridor
/// around it) has `reached < total`, which the assembler's internal connectivity check
/// rejects fail-closed (OQ-4: NEVER a repair). Boards are tiny (`<= 60x60`), so the flood
/// is cheap.
#[must_use]
pub fn count_seam_reachable(
    regions: &[RegionRect],
    board: RegionRect,
) -> (RegionCount, RegionCount) {
    let total = regions.len();
    if total == 0 {
        return (RegionCount::new(0), RegionCount::new(0));
    }
    let bw = board.footprint().width().max(0);
    let bh = board.footprint().height().max(0);
    if bw == 0 || bh == 0 {
        return (RegionCount::new(0), RegionCount::new(total));
    }
    let bx = board.origin().x;
    let by = board.origin().y;
    let width = usize::try_from(bw).unwrap_or(0);
    let cells = width.saturating_mul(usize::try_from(bh).unwrap_or(0));
    // Index a board-local cell (col, row) — caller passes in-bounds coords only.
    let idx = |x: i32, y: i32| -> usize {
        let col = usize::try_from(x - bx).unwrap_or(0);
        let row = usize::try_from(y - by).unwrap_or(0);
        row * width + col
    };

    // floor[c] == true => cell c is walkable seam floor (not inside ANY region).
    let mut floor = vec![true; cells];
    for &r in regions {
        let x0 = r.origin().x.max(bx);
        let y0 = r.origin().y.max(by);
        let x1 = r.max_x().min(bx + bw);
        let y1 = r.max_y().min(by + bh);
        for y in y0..y1 {
            for x in x0..x1 {
                floor[idx(x, y)] = false;
            }
        }
    }

    // Flood the floor from every floor cell adjacent to region 0 (the player region).
    let mut flooded = vec![false; cells];
    let mut stack: Vec<(i32, i32)> = Vec::new();
    for (x, y) in boundary_floor_cells(regions[0], board) {
        let c = idx(x, y);
        if floor[c] && !flooded[c] {
            flooded[c] = true;
            stack.push((x, y));
        }
    }
    while let Some((x, y)) = stack.pop() {
        for (nx, ny) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
            if nx < bx || nx >= bx + bw || ny < by || ny >= by + bh {
                continue;
            }
            let c = idx(nx, ny);
            if floor[c] && !flooded[c] {
                flooded[c] = true;
                stack.push((nx, ny));
            }
        }
    }

    // Region 0 is reached by definition; any other region is reached if a boundary cell of
    // it abuts a flooded floor cell.
    let mut reached = 1usize; // region 0
    for &r in regions.iter().skip(1) {
        let touches =
            boundary_floor_cells(r, board).any(|(x, y)| floor[idx(x, y)] && flooded[idx(x, y)]);
        if touches {
            reached += 1;
        }
    }
    (RegionCount::new(reached), RegionCount::new(total))
}

/// The board cells immediately OUTSIDE a region's footprint on its four edges (clamped to
/// the board) — the seam cells a region's edge opening connects to. Used to seed / test the
/// floor flood in [`count_seam_reachable`].
fn boundary_floor_cells(region: RegionRect, board: RegionRect) -> impl Iterator<Item = (i32, i32)> {
    let bx = board.origin().x;
    let by = board.origin().y;
    let bx1 = board.max_x();
    let by1 = board.max_y();
    let x0 = region.origin().x;
    let y0 = region.origin().y;
    let x1 = region.max_x();
    let y1 = region.max_y();
    // Left & right columns, top & bottom rows, just outside the region, clamped in-board.
    let left = (y0..y1).filter_map(move |y| (x0 > bx).then_some((x0 - 1, y)));
    let right = (y0..y1).filter_map(move |y| (x1 < bx1).then_some((x1, y)));
    let bottom = (x0..x1).filter_map(move |x| (y0 > by).then_some((x, y0 - 1)));
    let top = (x0..x1).filter_map(move |x| (y1 < by1).then_some((x, y1)));
    left.chain(right).chain(bottom).chain(top)
}
