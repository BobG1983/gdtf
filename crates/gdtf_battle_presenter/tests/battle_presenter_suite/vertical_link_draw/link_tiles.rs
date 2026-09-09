use bevy::{app::App, math::Rect, prelude::Visibility, sprite::Sprite};
use gdtf_battle_presenter::{ActiveLevel, VerticalLinkSprite};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Level},
    test_support::{SituationBuilder, test_pieces},
    vertical::{LinkKind, VerticalLink},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use super::harness::*;

// The sprite keys these cases read: two `test_stair()` view rows and the ladder marker.
const STAIR_FROM_BELOW: &str = "stair_ns_up";
const STAIR_FROM_ABOVE: &str = "stair_ns_down";
const LADDER: &str = "ladder";

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

// Both keys resolve to a sheet rect and the two differ, or two `None` reads compare equal.
fn distinct_rects(defs: &SpriteDefRegistry, one: &str, other: &str) -> (Rect, Rect) {
    let first = def_rect(defs, one);
    let second = def_rect(defs, other);
    assert!(
        first.is_some() && second.is_some(),
        "`{one}` and `{other}` must both resolve to a sheet rect — got {first:?} and {second:?}",
    );
    assert_ne!(
        first, second,
        "`{one}` and `{other}` must name different rects, or resolving both endpoints to one \
         view would pass unnoticed",
    );
    let (Some(first), Some(second)) = (first, second) else {
        return (Rect::default(), Rect::default());
    };
    (first.as_rect(), second.as_rect())
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
        .slab_piece_at(stair_lo, test_pieces::STAIR)
        .slab_piece_at(stair_hi, test_pieces::STAIR)
        .slab_piece_at(ladder_lo, test_pieces::SLAB)
        .slab_piece_at(ladder_hi, test_pieces::SLAB)
        .vertical_link(VerticalLink::new(stair_lo, stair_hi, LinkKind::stair()))
        .vertical_link(VerticalLink::new(ladder_lo, ladder_hi, LinkKind::ladder()))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };
    let (from_below, from_above) = distinct_rects(&defs, STAIR_FROM_BELOW, STAIR_FROM_ABOVE);
    let (ladder, _slab) = distinct_rects(&defs, LADDER, "slab");

    settle_link_sprites(&mut app);
    let rects = visible_link_rects(&mut app);
    assert_eq!(
        rects.len(),
        2,
        "exactly two link-cell sprites draw at active level 0 (one stair, one ladder), got {rects:?}",
    );
    assert!(
        rects.contains(&from_below),
        "the stair link's LOWER endpoint must draw the stair def's own FromBelow view, got \
         {rects:?}",
    );
    assert!(
        !rects.contains(&from_above),
        "the stair def's FromAbove view belongs to the upper endpoint, which is culled at \
         active level 0, got {rects:?}",
    );
    assert!(
        rects.contains(&ladder),
        "a ladder endpoint draws the `ladder` key from the link kind — resolving it from the \
         slab piece under it would return that def's `Single` view instead, got {rects:?}",
    );
}

#[test]
fn link_draw_hard_cuts_and_picks_directional_stair_tile() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let lo = CellLevel::new(Cell::new(4, 4), Level::new(0));
    let hi = CellLevel::new(Cell::new(4, 4), Level::new(1));
    let situation = SituationBuilder::new()
        .slab_piece_at(lo, test_pieces::STAIR)
        .slab_piece_at(hi, test_pieces::STAIR)
        .vertical_link(VerticalLink::new(lo, hi, LinkKind::stair()))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };
    let (from_below, from_above) = distinct_rects(&defs, STAIR_FROM_BELOW, STAIR_FROM_ABOVE);

    settle_link_sprites(&mut app);
    assert_eq!(
        visible_link_rects(&mut app),
        vec![from_below],
        "at active level 0 exactly the LOWER stair endpoint draws (the hard cut), carrying its \
         def's FromBelow view",
    );

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();
    settle_link_sprites(&mut app);
    assert_eq!(
        visible_link_rects(&mut app),
        vec![from_above],
        "at active level 1 exactly the UPPER stair endpoint draws (still one, mutated in \
         place), carrying its def's FromAbove view",
    );
}
