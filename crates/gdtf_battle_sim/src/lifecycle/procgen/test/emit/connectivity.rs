//! C2 — the by-construction connectivity invariant on the real packer output, its
//! shared flood machinery, and the seam-removal control that proves the pin
//! discriminating.

use super::support::*;
use crate::{
    metric::{Cell, CellLevel},
    procgen::{FilledPlacement, Footprint, Margin, PlacedPrefab, RegionRect, generate_level},
    rng::{BattleSeed, ProcgenRng},
};

/// C2 (the by-construction connectivity INVARIANT): a generated level under a FIXED
/// [`ProcgenRng`] seed is connected BY CONSTRUCTION — every board cell that is NOT inside a
/// placed region is reachable, in 4-connectivity, from any single open cell. The open
/// (non-region) cells form ONE connected component because the 1-cell `default_floor` seam
/// every placement reserves leaves a continuous walkable corridor lattice between every pair
/// of placed regions. This is asserted WITHOUT the removed connectivity flood: it floods the
/// open cells of the REAL packer output (the [`FilledPlacement`]'s placed + filled region
/// rectangles — the very rectangles the seam is reserved around), then additionally runs the
/// full [`generate_level`] entry point and verifies the emitted `Situation`.
///
/// Pin-discriminating: the discrimination is proved by [`seam_separated_regions_stay_connected`],
/// which feeds the SAME flood helper a control layout where two regions ABUT (the layout the
/// packer would produce if [`Margin::DEFAULT`] were dropped to a zero seam) and asserts that
/// control's open cells split into TWO components. So a packer with the seam removed would
/// make this invariant FAIL. (The in-bounds + non-empty-walls checks below additionally pin
/// that the emit translated the footprint-local cells onto the board.)
#[test]
fn emitted_level_is_in_bounds_and_fully_connected() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let Some(prefabs) = registry_with_fill(
        theme,
        player_fp,
        enemy_fp,
        &[("hall", 8, 8), ("nook", 4, 4)],
    ) else {
        return;
    };
    let themes = theme_registry(theme);
    let terrain_defs = terrain_defs();
    let knobs = tuning(0.8, 49, 2);

    // Drive the REAL staged pipeline (the exact functions `generate_level` calls) under a
    // fixed seed to recover the FilledPlacement — its placed + filled region rectangles ARE
    // the packer's seam-reserved output (the seam is the 1-cell gap BETWEEN these regions).
    let seed = BattleSeed::new(0xB0_1234);
    let Some(filled) = run_pipeline(&prefabs, theme, board, seed, &knobs) else {
        return;
    };

    let board_rect = RegionRect::board(board);
    let board_w = board_rect.footprint().width();
    let board_h = board_rect.footprint().height();

    // The by-construction connectivity INVARIANT (C2): every cell NOT inside a placed region
    // is reachable. The placed/filled regions are the only ground-plane blockers; the 1-cell
    // seam reserved around each leaves a walkable lattice, so the open cells are ONE
    // connected component. (Proved discriminating by `seam_separated_regions_stay_connected`:
    // remove the seam and the control layout below fails this same helper.)
    let occupied = occupied_regions(&filled);
    assert!(
        open_cells_form_one_component(&occupied, board_w, board_h),
        "the generated level must be connected BY CONSTRUCTION: every cell outside a placed \
         region reachable from any open cell (the 1-cell default_floor seam lattice). If a \
         placement could wall off part of the board, this would fail.",
    );

    // Run the FULL entry point and verify the emitted Situation (the real output of the real
    // pipeline) — the emit translated the footprint-local cells onto the board.
    let mut rng = ProcgenRng::from_root(seed);
    let result = generate_level(
        &prefabs,
        &themes,
        &terrain_defs,
        theme,
        board,
        &mut rng,
        &knobs,
    );
    assert!(
        result.is_ok(),
        "the generate must succeed (a valid placement): {:?}",
        result.as_ref().err(),
    );
    let Ok(emitted) = result else {
        return;
    };
    let situation = emitted.situation;

    // GTW-492: the theme is the UUID-keyed key directly (no shim), and the default_floor
    // resolved from the theme registry (the test FLOOR piece).
    assert_eq!(
        situation.theme, theme,
        "the emitted level's theme must be the requested ThemeUuid (no shim)",
    );

    // The emitted level carries the translated prefab walls (the single v2 placements list
    // poured into `walls` by the Wall def classification).
    assert!(
        !situation.walls.is_empty(),
        "the emitted level must carry the translated prefab walls (C1 — the v2 placements \
         list was poured into the situation, classified into the walls list)",
    );

    // Every authored terrain cell must be in-bounds: 0 <= x < board_w, 0 <= y < board_h.
    let in_bounds = |c: CellLevel| c.x >= 0 && c.x < board_w && c.y >= 0 && c.y < board_h;
    for w in &situation.walls {
        assert!(
            in_bounds(w.at),
            "every emitted wall cell must be in-bounds: {:?} on a {board_w}x{board_h} board",
            w.at,
        );
    }
    for s in &situation.scatter {
        assert!(
            in_bounds(s.at),
            "every emitted scatter cell must be in-bounds: {:?}",
            s.at
        );
    }
    for f in &situation.floors {
        assert!(
            in_bounds(f.at),
            "every emitted floored dead-space cell must be in-bounds: {:?}",
            f.at
        );
    }

    // The dead space was floored (C3): with a partial density floor there is leftover free
    // space, so the emit must have produced explicit `default_floor` overrides for it.
    assert!(
        !situation.floors.is_empty(),
        "the leftover dead space must be FLOORED with explicit default_floor entries (C3)",
    );
    assert!(
        !situation.default_floor.is_nil(),
        "the emitted level must carry a default_floor (the theme's nominated ground terrain)",
    );
    assert_eq!(
        situation.default_floor,
        floor_piece(),
        "the default_floor must resolve from the theme registry's nominated terrain (GTW-492)",
    );
}

/// C2 (pin-discrimination): PROVE the by-construction connectivity invariant is sensitive to
/// the 1-cell `default_floor` seam — that it would FAIL if the packer's [`Margin::DEFAULT`]
/// seam reservation were removed.
///
/// Two regions that each span a full board axis with the OTHER axis abutting form a
/// board-spanning barrier UNLESS a walkable gap separates them. This is exactly what the
/// packer's seam guarantees: with the 1-cell seam reserved between them, a walkable corridor
/// remains and the open cells stay ONE component; with NO seam (the zero-margin layout a
/// seam-less packer would emit) the two regions touch into a solid wall that splits the
/// board, and the open cells become TWO components.
///
/// The same [`open_cells_form_one_component`] helper that backs the real-pipeline assertion
/// is exercised here on both layouts: it returns `true` for the seam-separated layout and
/// `false` for the abutting one. So the real-pipeline assertion is NOT vacuous — remove the
/// seam from the packer and the abutting layout (which a seam-less packer would produce)
/// fails this helper.
#[test]
fn seam_separated_regions_stay_connected() {
    let board_w = 20;
    let board_h = 20;
    let seam = Margin::DEFAULT.cells();

    // Two region blocks side by side across a mid band (rows 5..15), leaving open rows above
    // (0..5) and below (15..20). A LEFT block on columns `0..10`, and a RIGHT block that, with
    // the 1-cell seam, starts at column `10 + seam` (leaving column 10 as a walkable corridor
    // through the barrier — the open rows above and below stay joined). With the seam REMOVED
    // the right block starts at column 10, abutting the left block into a FULL-WIDTH wall on
    // rows 5..15 that splits the board's open cells into two components (above vs below).
    let left = RegionRect::new(Cell::new(0, 5), Footprint::new(10, 10));
    let right_with_seam = RegionRect::new(
        Cell::new(10 + seam, 5),
        Footprint::new(board_w - 10 - seam, 10),
    );
    let right_abutting = RegionRect::new(Cell::new(10, 5), Footprint::new(board_w - 10, 10));

    assert!(
        open_cells_form_one_component(&[left, right_with_seam], board_w, board_h),
        "with the 1-cell seam reserved between two abutting-axis blocks, the open cells stay \
         ONE connected component (the seam lattice keeps a walkable corridor through the \
         barrier)",
    );
    assert!(
        !open_cells_form_one_component(&[left, right_abutting], board_w, board_h),
        "with the seam REMOVED the two blocks abut into a board-spanning wall and the open \
         cells split into TWO components (above vs below) — so the by-construction \
         connectivity invariant is sensitive to the packer's Margin::DEFAULT seam \
         (pin-discriminating)",
    );
}

/// Every ground-plane region a [`FilledPlacement`] occupies — the player + enemy spawn
/// regions and every fill prefab's region (the rectangles the packer reserved the seam
/// around). The dead-space regions are walkable `default_floor`, so they are NOT occupied.
fn occupied_regions(filled: &FilledPlacement) -> Vec<RegionRect> {
    let mut out = vec![
        filled.placement().player().region(),
        filled.placement().enemy().region(),
    ];
    out.extend(filled.fill().iter().map(PlacedPrefab::region));
    out
}

/// Whether the OPEN (non-`occupied`-region) ground-plane cells of a `board_w` x `board_h`
/// board form ONE 4-connected component — the by-construction connectivity invariant (C2).
///
/// A cell inside any `occupied` region blocks the flood; every other cell is open (the
/// `default_floor` seam lattice + the floored dead space). Floods the open cells from the
/// first open cell found and returns `true` iff the flood reaches every open cell. A board
/// with no open cell trivially returns `true` (the caller's other assertions pin a non-empty
/// level). This is the SHARED helper both the real-pipeline assertion and the
/// pin-discrimination control ([`seam_separated_regions_stay_connected`]) exercise.
fn open_cells_form_one_component(occupied: &[RegionRect], board_w: i32, board_h: i32) -> bool {
    let width = usize::try_from(board_w.max(0)).unwrap_or(0);
    let height = usize::try_from(board_h.max(0)).unwrap_or(0);
    let total = width.saturating_mul(height);
    if total == 0 {
        return true;
    }
    let index = |col: usize, row: usize| -> usize { row * width + col };

    // Ground-plane occupancy mask: a cell inside any placed region blocks the flood.
    let mut blocked = vec![false; total];
    for region in occupied {
        let origin = region.origin();
        let footprint = region.footprint();
        for dy in 0..footprint.height() {
            for dx in 0..footprint.width() {
                let x = origin.x + dx;
                let y = origin.y + dy;
                if x >= 0
                    && x < board_w
                    && y >= 0
                    && y < board_h
                    && let (Ok(col), Ok(row)) = (usize::try_from(x), usize::try_from(y))
                {
                    blocked[index(col, row)] = true;
                }
            }
        }
    }

    let open_total = blocked.iter().filter(|b| !**b).count();
    let Some(start) = blocked.iter().position(|b| !*b) else {
        return true; // no open cell — trivially one (empty) component
    };

    // Flood the open cells in 4-connectivity from the first open cell, collecting the
    // in-bounds 4-neighbours of each popped cell.
    let mut seen = vec![false; total];
    seen[start] = true;
    let mut stack = vec![start];
    let mut reached = 0usize;
    while let Some(cell) = stack.pop() {
        reached += 1;
        let col = cell % width;
        let row = cell / width;
        let mut neighbours: Vec<usize> = Vec::with_capacity(4);
        if col > 0 {
            neighbours.push(index(col - 1, row));
        }
        if col + 1 < width {
            neighbours.push(index(col + 1, row));
        }
        if row > 0 {
            neighbours.push(index(col, row - 1));
        }
        if row + 1 < height {
            neighbours.push(index(col, row + 1));
        }
        for neighbour in neighbours {
            if !blocked[neighbour] && !seen[neighbour] {
                seen[neighbour] = true;
                stack.push(neighbour);
            }
        }
    }
    reached == open_total
}
