use super::support::*;
use crate::{
    metric::{Cell, CellLevel},
    procgen::{FilledPlacement, Footprint, Margin, PlacedPrefab, RegionRect, generate_level},
    rng::{BattleSeed, ProcgenRng},
};

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

    let seed = BattleSeed::new(0xB0_1234);
    let Some(filled) = run_pipeline(&prefabs, theme, board, seed, &knobs) else {
        return;
    };

    let board_rect = RegionRect::board(board);
    let board_w = board_rect.footprint().width();
    let board_h = board_rect.footprint().height();

    let occupied = occupied_regions(&filled);
    assert!(
        open_cells_form_one_component(&occupied, board_w, board_h),
        "the generated level must be connected BY CONSTRUCTION: every cell outside a placed \
         region reachable from any open cell (the 1-cell default_floor margin lattice). If a \
         placement could wall off part of the board, this would fail.",
    );

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

    assert_eq!(
        situation.theme, theme,
        "the emitted level's theme must be the requested ThemeUuid (no shim)",
    );

    assert!(
        !situation.walls.is_empty(),
        "the emitted level must carry the translated prefab walls (C1 — the v2 placements \
         list was poured into the situation, classified into the walls list)",
    );

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

    assert!(
        !situation.floors.is_empty(),
        "the leftover dead space must be FLOORED with explicit default_floor entries (C3)",
    );
    assert!(
        !*situation.default_floor.is_nil(),
        "the emitted level must carry a default_floor (the theme's nominated ground terrain)",
    );
    assert_eq!(
        situation.default_floor,
        floor_piece(),
        "the default_floor must resolve from the theme registry's nominated terrain ",
    );
}

#[test]
fn margin_separated_regions_stay_connected() {
    let board_w = 20;
    let board_h = 20;
    let margin = *Margin::DEFAULT.cells();

    let left = RegionRect::new(Cell::new(0, 5), Footprint::new(10, 10));
    let right_with_margin = RegionRect::new(
        Cell::new(10 + margin, 5),
        Footprint::new(board_w - 10 - margin, 10),
    );
    let right_abutting = RegionRect::new(Cell::new(10, 5), Footprint::new(board_w - 10, 10));

    assert!(
        open_cells_form_one_component(&[left, right_with_margin], board_w, board_h),
        "with the 1-cell margin reserved between two abutting-axis blocks, the open cells stay \
         ONE connected component (the margin lattice keeps a walkable corridor through the \
         barrier)",
    );
    assert!(
        !open_cells_form_one_component(&[left, right_abutting], board_w, board_h),
        "with the margin REMOVED the two blocks abut into a board-spanning wall and the open \
         cells split into TWO components (above vs below) — so the by-construction \
         connectivity invariant is sensitive to the packer's Margin::DEFAULT margin \
         (pin-discriminating)",
    );
}

fn occupied_regions(filled: &FilledPlacement) -> Vec<RegionRect> {
    let mut out = vec![
        filled.placement().player().region(),
        filled.placement().enemy().region(),
    ];
    out.extend(filled.fill().iter().map(PlacedPrefab::region));
    out
}

fn open_cells_form_one_component(occupied: &[RegionRect], board_w: i32, board_h: i32) -> bool {
    let width = usize::try_from(board_w.max(0)).unwrap_or(0);
    let height = usize::try_from(board_h.max(0)).unwrap_or(0);
    let total = width.saturating_mul(height);
    if total == 0 {
        return true;
    }
    let index = |col: usize, row: usize| -> usize { row * width + col };

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
        return true; 
    };

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
