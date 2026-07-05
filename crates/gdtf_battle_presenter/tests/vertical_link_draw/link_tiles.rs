//! Vertical-link tile draw: the locked role indices, the hard-cut, and the
//! directional stair pick (C6b, GTW-373).

use bevy::{app::App, prelude::Visibility, sprite::Sprite};
use gdtf_battle_presenter::{ActiveLevel, TileRoles, VerticalLinkSprite};
use gdtf_battle_sim::{
    Cell, CellLevel, Level, LinkKind, VerticalLink, test_support::SituationBuilder,
};

use super::harness::*;

/// The stair-UP tile index (GTW-373, supersedes the OQ-3 single-stair 77) — drawn when the
/// active storey is the link's LOWER cell (you ascend). Asserted as a system constant.
const STAIR_UP_INDEX: usize = 29;

/// The stair-DOWN tile index (GTW-373) — drawn when the active storey is the link's UPPER
/// cell (you descend). Asserted as a system constant.
const STAIR_DOWN_INDEX: usize = 28;

/// The ladder tile index (user OQ-3 ruling, UNCHANGED) — asserted as a system constant.
const LADDER_INDEX: usize = 235;

/// The atlas indices of every VISIBLE pooled vertical-link sprite (hidden pooled sprites,
/// which carry stale state, are excluded — the draw shows exactly the on-storey set).
fn visible_link_indices(app: &mut App) -> Vec<usize> {
    let mut q = app
        .world_mut()
        .query::<(&VerticalLinkSprite, &Sprite, &Visibility)>();
    q.iter(app.world())
        .filter(|(_, _, vis)| !matches!(vis, Visibility::Hidden))
        .filter_map(|(_, sprite, _)| sprite.texture_atlas.as_ref().map(|a| a.index))
        .collect()
}

/// Drive bounded `update()`s until at least one VISIBLE vertical-link sprite exists
/// (the link draw fires once `BattleInProgress` + the graph + `TileRoles` + atlases are
/// all live; its spawn lands a frame after the setup flush under parallel contention).
fn settle_link_sprites(app: &mut App) {
    for _ in 0..MAX_UPDATES {
        if !visible_link_indices(app).is_empty() {
            return;
        }
        app.update();
    }
}

/// C6 (b) / GTW-373 — the loaded SHIPPED `TileRoles` resolves `stair_up` == 29 +
/// `stair_down` == 28 + ladder == 235 (the system constants — OQ-3 ladder UNCHANGED, the
/// stair split is the GTW-373 ruling), AND the rendered link-cell tile carries the
/// direction-keyed atlas index: a Stair endpoint on the link's LOWER cell (you ascend)
/// draws `stair_up` (29), a Ladder endpoint draws 235.
///
/// Authors two links on level 0 ⇄ level 1: a STAIR (cells (3,3)) and a LADDER (cells
/// (6,6)), each `from` = the L0 endpoint. At active level 0 the LOWER endpoint of each is
/// drawn; the stair sprite must carry `stair_up` (29) — the active storey is the link's
/// lower cell so the ascend tile is chosen — and the ladder sprite index 235 — proving the
/// direction-keyed role mapping (C2 / GTW-373) on the real draw path.
#[test]
fn loaded_roles_resolve_locked_indices_and_link_cells_carry_them() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    // The loaded SHIPPED tile_roles.ron resolves the system constants (the locked-constant
    // exemption — these are the user-chosen role->index contract, not tunable magnitudes).
    let roles = app.world().get_resource::<TileRoles>().cloned();
    assert!(
        roles.is_some(),
        "TileRoles must be resident after the load settles",
    );
    let Some(roles) = roles else { return };
    assert_eq!(
        *roles.stair_up, STAIR_UP_INDEX,
        "the shipped tile_roles.ron must resolve stair_up == 29 (GTW-373)",
    );
    assert_eq!(
        *roles.stair_down, STAIR_DOWN_INDEX,
        "the shipped tile_roles.ron must resolve stair_down == 28 (GTW-373)",
    );
    assert_eq!(
        *roles.ladder, LADDER_INDEX,
        "the shipped tile_roles.ron must resolve ladder == 235 (UNCHANGED, user OQ-3)",
    );

    // A stair link (3,3) L0 <-> L1 and a ladder link (6,6) L0 <-> L1, each `from` = the L0
    // endpoint. Author the endpoints as slabs so the link validation passes.
    let stair_lo = CellLevel::new(Cell::new(3, 3), Level::new(0));
    let stair_hi = CellLevel::new(Cell::new(3, 3), Level::new(1));
    let ladder_lo = CellLevel::new(Cell::new(6, 6), Level::new(0));
    let ladder_hi = CellLevel::new(Cell::new(6, 6), Level::new(1));
    let situation = SituationBuilder::new()
        .slab_at(stair_lo)
        .slab_at(stair_hi)
        .slab_at(ladder_lo)
        .slab_at(ladder_hi)
        .vertical_link(VerticalLink::new(stair_lo, stair_hi, LinkKind::stair()))
        .vertical_link(VerticalLink::new(ladder_lo, ladder_hi, LinkKind::ladder()))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    settle_link_sprites(&mut app);
    let indices = visible_link_indices(&mut app);
    // At active level 0 exactly the two LOWER endpoints draw (the hard cut excludes the
    // level-1 endpoints), one stair-UP (29 — ascend from the lower cell) and one ladder (235).
    assert_eq!(
        indices.len(),
        2,
        "exactly two link-cell sprites draw at active level 0 (one stair, one ladder), got {indices:?}",
    );
    assert!(
        indices.contains(&STAIR_UP_INDEX),
        "a Stair link cell on the link's LOWER endpoint (you ascend) must draw stair_up (29), got {indices:?}",
    );
    assert!(
        indices.contains(&LADDER_INDEX),
        "a Ladder link cell must draw the ladder tile (index 235), got {indices:?}",
    );
}

/// C6 (b) cont. / GTW-373 — the link draw HARD-CUTS to the active storey (AC4) AND picks
/// the direction-keyed stair tile: only the on-storey endpoint of a link is drawn, and a
/// stair endpoint draws `stair_up` (29) on the LOWER cell (you ascend) vs `stair_down` (28)
/// on the UPPER cell (you descend). With ONE stair link L0 ⇄ L1, exactly one stair sprite
/// is visible at active level 0 carrying `stair_up` (29); switching to active level 1 still
/// shows exactly one stair sprite (the upper endpoint, mutated in place — not respawned),
/// now carrying `stair_down` (28), never both endpoints. This is the direction selection on
/// the real draw path: it FAILS if the up/down arms are swapped.
#[test]
fn link_draw_hard_cuts_and_picks_directional_stair_tile() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let lo = CellLevel::new(Cell::new(4, 4), Level::new(0));
    let hi = CellLevel::new(Cell::new(4, 4), Level::new(1));
    let situation = SituationBuilder::new()
        .slab_at(lo)
        .slab_at(hi)
        .vertical_link(VerticalLink::new(lo, hi, LinkKind::stair()))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    settle_link_sprites(&mut app);
    assert_eq!(
        visible_link_indices(&mut app),
        vec![STAIR_UP_INDEX],
        "at active level 0 exactly the LOWER stair endpoint draws (the hard cut), carrying \
         stair_up (29 — you ascend from here)",
    );

    // Switch to active level 1 — the upper endpoint draws, still exactly one (mutated in
    // place, never both endpoints at once), and now the DESCEND tile (stair_down 28).
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();
    // Settle the Draw-schedule re-draw for the new active level before reading (the link draw
    // runs in `PresenterSystems::Draw` and can trail the `ActiveLevel` switch a frame under
    // parallel contention; the assert stands).
    settle_link_sprites(&mut app);
    assert_eq!(
        visible_link_indices(&mut app),
        vec![STAIR_DOWN_INDEX],
        "at active level 1 exactly the UPPER stair endpoint draws (still one, mutated in \
         place), carrying stair_down (28 — you descend from here)",
    );
}
