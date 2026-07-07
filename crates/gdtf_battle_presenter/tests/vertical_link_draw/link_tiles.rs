//! Vertical-link tile draw: the locked stair/ladder sprite regions, the hard-cut, and
//! the directional stair pick (C6b, GTW-373 / GTW-665).

use bevy::{app::App, math::Rect, prelude::Visibility, sprite::Sprite};
use gdtf_battle_presenter::{ActiveLevel, VerticalLinkSprite};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level},
    test_support::SituationBuilder,
    vertical::{LinkKind, VerticalLink},
};

use super::harness::*;

/// The stair-UP sheet region (GTW-373, supersedes the OQ-3 single-stair 77) — the
/// seeded `stair_up.spritedef.ron` rect, i.e. the locked atlas index 29 unpacked on the
/// 16-column/16-px terrain sheet (`(29 % 16) * 16 = 208`, `(29 / 16) * 16 = 16`).
/// Drawn when the active storey is the link's LOWER cell (you ascend). Asserted as a
/// system constant (the GTW-373 user-locked contract, carried in rect form since the
/// GTW-665 table retirement).
const STAIR_UP_RECT: Rect = Rect {
    min: bevy::math::Vec2::new(208.0, 16.0),
    max: bevy::math::Vec2::new(224.0, 32.0),
};

/// The stair-DOWN sheet region (GTW-373) — the seeded `stair_down.spritedef.ron` rect
/// (index 28 → `(192, 16)`). Drawn when the active storey is the link's UPPER cell (you
/// descend). Asserted as a system constant.
const STAIR_DOWN_RECT: Rect = Rect {
    min: bevy::math::Vec2::new(192.0, 16.0),
    max: bevy::math::Vec2::new(208.0, 32.0),
};

/// The ladder sheet region (user OQ-3 ruling, UNCHANGED) — the seeded
/// `ladder.spritedef.ron` rect (index 235 → `(176, 224)`). Asserted as a system constant.
const LADDER_RECT: Rect = Rect {
    min: bevy::math::Vec2::new(176.0, 224.0),
    max: bevy::math::Vec2::new(192.0, 240.0),
};

/// The sheet regions of every VISIBLE pooled vertical-link sprite (hidden pooled sprites,
/// which carry stale state, are excluded — the draw shows exactly the on-storey set).
/// Post-GTW-665 a link tile is an atlas-free `Sprite` whose `rect` IS the def's authored
/// region.
fn visible_link_rects(app: &mut App) -> Vec<Rect> {
    let mut q = app
        .world_mut()
        .query::<(&VerticalLinkSprite, &Sprite, &Visibility)>();
    q.iter(app.world())
        .filter(|(_, _, vis)| !matches!(vis, Visibility::Hidden))
        .filter_map(|(_, sprite, _)| sprite.rect)
        .collect()
}

/// Drive bounded `update()`s until at least one VISIBLE vertical-link sprite exists
/// (the link draw fires once `BattleInProgress` + the graph + the sprite-def registry +
/// the missing marker are all live; its spawn lands a frame after the setup flush under
/// parallel contention).
fn settle_link_sprites(app: &mut App) {
    for _ in 0..MAX_UPDATES {
        if !visible_link_rects(app).is_empty() {
            return;
        }
        app.update();
    }
}

/// C6 (b) / GTW-373 / GTW-665 — the loaded SHIPPED sprite-def catalog resolves
/// `stair_up` == the (208, 16) region + `stair_down` == (192, 16) + `ladder` ==
/// (176, 224) (the system constants — OQ-3 ladder UNCHANGED, the stair split is the
/// GTW-373 ruling; carried as seeded-def rects since the table retirement), AND the
/// rendered link-cell tile carries the direction-keyed region: a Stair endpoint on the
/// link's LOWER cell (you ascend) draws `stair_up`, a Ladder endpoint draws the ladder
/// region — proving the direction-keyed role mapping (C2 / GTW-373) on the real draw
/// path, resolved through the sprite-def registry.
#[test]
fn loaded_defs_resolve_locked_regions_and_link_cells_carry_them() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

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
    let rects = visible_link_rects(&mut app);
    // At active level 0 exactly the two LOWER endpoints draw (the hard cut excludes the
    // level-1 endpoints), one stair-UP (ascend from the lower cell) and one ladder.
    assert_eq!(
        rects.len(),
        2,
        "exactly two link-cell sprites draw at active level 0 (one stair, one ladder), got {rects:?}",
    );
    assert!(
        rects.contains(&STAIR_UP_RECT),
        "a Stair link cell on the link's LOWER endpoint (you ascend) must draw the stair_up \
         region (208, 16), got {rects:?}",
    );
    assert!(
        rects.contains(&LADDER_RECT),
        "a Ladder link cell must draw the ladder region (176, 224), got {rects:?}",
    );
}

/// C6 (b) cont. / GTW-373 — the link draw HARD-CUTS to the active storey (AC4) AND picks
/// the direction-keyed stair tile: only the on-storey endpoint of a link is drawn, and a
/// stair endpoint draws the `stair_up` region on the LOWER cell (you ascend) vs the
/// `stair_down` region on the UPPER cell (you descend). With ONE stair link L0 ⇄ L1,
/// exactly one stair sprite is visible at active level 0 carrying `stair_up`; switching
/// to active level 1 still shows exactly one stair sprite (the upper endpoint, mutated in
/// place — not respawned), now carrying `stair_down`, never both endpoints. This is the
/// direction selection on the real draw path: it FAILS if the up/down arms are swapped.
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
        visible_link_rects(&mut app),
        vec![STAIR_UP_RECT],
        "at active level 0 exactly the LOWER stair endpoint draws (the hard cut), carrying \
         the stair_up region (208, 16 — you ascend from here)",
    );

    // Switch to active level 1 — the upper endpoint draws, still exactly one (mutated in
    // place, never both endpoints at once), and now the DESCEND tile (stair_down).
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();
    // Settle the Draw-schedule re-draw for the new active level before reading (the link draw
    // runs in `PresenterSystems::Draw` and can trail the `ActiveLevel` switch a frame under
    // parallel contention; the assert stands).
    settle_link_sprites(&mut app);
    assert_eq!(
        visible_link_rects(&mut app),
        vec![STAIR_DOWN_RECT],
        "at active level 1 exactly the UPPER stair endpoint draws (still one, mutated in \
         place), carrying the stair_down region (192, 16 — you descend from here)",
    );
}
