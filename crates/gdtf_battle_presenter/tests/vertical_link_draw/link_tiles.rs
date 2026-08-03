use bevy::{app::App, math::Rect, prelude::Visibility, sprite::Sprite};
use gdtf_battle_presenter::{ActiveLevel, VerticalLinkSprite};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level},
    test_support::SituationBuilder,
    vertical::{LinkKind, VerticalLink},
};

use super::harness::*;

const STAIR_UP_RECT: Rect = Rect {
    min: bevy::math::Vec2::new(208.0, 16.0),
    max: bevy::math::Vec2::new(224.0, 32.0),
};

const STAIR_DOWN_RECT: Rect = Rect {
    min: bevy::math::Vec2::new(192.0, 16.0),
    max: bevy::math::Vec2::new(208.0, 32.0),
};

const LADDER_RECT: Rect = Rect {
    min: bevy::math::Vec2::new(176.0, 224.0),
    max: bevy::math::Vec2::new(192.0, 240.0),
};

fn visible_link_rects(app: &mut App) -> Vec<Rect> {
    let mut q = app
        .world_mut()
        .query::<(&VerticalLinkSprite, &Sprite, &Visibility)>();
    q.iter(app.world())
        .filter(|(_, _, vis)| !matches!(vis, Visibility::Hidden))
        .filter_map(|(_, sprite, _)| sprite.rect)
        .collect()
}

fn settle_link_sprites(app: &mut App) {
    for _ in 0..MAX_UPDATES {
        if !visible_link_rects(app).is_empty() {
            return;
        }
        app.update();
    }
}

#[test]
fn loaded_defs_resolve_locked_regions_and_link_cells_carry_them() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

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

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();
    settle_link_sprites(&mut app);
    assert_eq!(
        visible_link_rects(&mut app),
        vec![STAIR_DOWN_RECT],
        "at active level 1 exactly the UPPER stair endpoint draws (still one, mutated in \
         place), carrying the stair_down region (192, 16 — you descend from here)",
    );
}
