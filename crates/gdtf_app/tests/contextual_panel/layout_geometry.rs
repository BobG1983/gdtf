use bevy::{
    ecs::{entity::Entity, query::With},
    math::Vec2,
    prelude::*,
    ui::{ComputedNode, UiGlobalTransform},
};
use gdtf_app::test_support::{BottomBarRoot, ContextualPanelRoot, ThrowGrenadeButton};
use gdtf_battle_input::{InspectTarget, SelectedShooter};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Faction, Level},
    weapon::{TrajectoryStyle, WieldedBy},
};

use super::{
    actors::at,
    harness::{battle_running_app, parent_of, single_with},
    real_layout_harness::real_layout_battle_running_app,
};

const EPSILON_PX: f32 = 1.0;

fn spawn_throw_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app.world_mut().spawn((at(x, y), Faction::new(gang))).id();
    app.world_mut()
        .spawn((WieldedBy::new(actor), TrajectoryStyle::Arc));
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

fn hover_cell(app: &mut App, x: i32, y: i32) {
    app.world_mut()
        .insert_resource(InspectTarget::new(Some(CellLevel::new(
            Cell::new(x, y),
            Level::new(0),
        ))));
}

fn clear_hover(app: &mut App) {
    app.world_mut().insert_resource(InspectTarget::new(None));
}

struct PixelRect {
    min: Vec2,
    max: Vec2,
}

impl PixelRect {
    fn from_node(node: &ComputedNode, transform: &UiGlobalTransform) -> Self {
        let size = node.size();
        let center = transform.translation;
        Self {
            min: center - size / 2.0,
            max: center + size / 2.0,
        }
    }

        fn contained_in(&self, other: &Self) -> bool {
        self.min.x >= other.min.x - EPSILON_PX
            && self.min.y >= other.min.y - EPSILON_PX
            && self.max.x <= other.max.x + EPSILON_PX
            && self.max.y <= other.max.y + EPSILON_PX
    }
}

fn pixel_rect_of<M: Component>(app: &mut App) -> Option<PixelRect> {
    let mut query = app
        .world_mut()
        .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<M>>();
    let mut results: Vec<PixelRect> = query
        .iter(app.world())
        .map(|(node, transform)| PixelRect::from_node(node, transform))
        .collect();
    if results.len() == 1 {
        results.pop()
    } else {
        None
    }
}

#[test]
fn contextual_panel_is_contained_inside_the_bottom_bar() {
    let app_opt = real_layout_battle_running_app();
    assert!(
        app_opt.is_some(),
        "the real-layout harness must reach BattleScapeState::BattleRunning with its font loaded",
    );
    let Some(mut app) = app_opt else { return };

    spawn_throw_actor(&mut app, 5, 5, 0);
    for _ in 0..8 {
        hover_cell(&mut app, 15, 15);
        app.update();
    }

    let root = single_with::<ContextualPanelRoot>(&mut app);
    let bar = single_with::<BottomBarRoot>(&mut app);
    assert!(
        root.is_some() && bar.is_some(),
        "both the contextual panel root and the bottom bar must exist in the live battle",
    );
    let (Some(root), Some(bar_entity)) = (root, bar) else {
        return;
    };
    assert_eq!(
        parent_of(&app, root),
        Some(bar_entity),
        "the contextual panel root must be a CHILD of the bottom bar (GTW-726)",
    );

    assert_eq!(
        app.world().get::<Visibility>(root).copied(),
        Some(Visibility::Visible),
        "an offered Throw must reveal the panel root so its geometry is meaningful",
    );

    let panel_rect = pixel_rect_of::<ContextualPanelRoot>(&mut app);
    let bar_rect = pixel_rect_of::<BottomBarRoot>(&mut app);
    assert!(
        panel_rect.is_some() && bar_rect.is_some(),
        "both the panel root and the bottom bar must have real computed geometry",
    );
    let (Some(panel_rect), Some(bar_rect)) = (panel_rect, bar_rect) else {
        return;
    };
    assert!(
        panel_rect.contained_in(&bar_rect),
        "the contextual panel root ({:?}..{:?}) must be CONTAINED inside the bottom bar \
         ({:?}..{:?}) — GTW-726: the context window is part of the bottom HUD, never a \
         free-floating box over the map. Its top edge (min.y={}) must not sit above the bar's top \
         edge (min.y={}).",
        panel_rect.min,
        panel_rect.max,
        bar_rect.min,
        bar_rect.max,
        panel_rect.min.y,
        bar_rect.min.y,
    );
}

#[test]
fn contextual_panel_updates_mutate_in_place() {
    let mut app = battle_running_app();
    spawn_throw_actor(&mut app, 5, 5, 0);

    hover_cell(&mut app, 15, 15);
    app.update();
    let root = single_with::<ContextualPanelRoot>(&mut app);
    let button = single_with::<ThrowGrenadeButton>(&mut app);
    assert!(
        root.is_some() && button.is_some(),
        "the panel root + Throw button must exist once offered",
    );
    let (Some(root), Some(button)) = (root, button) else {
        return;
    };
    assert_eq!(
        app.world().get::<Visibility>(button).copied(),
        Some(Visibility::Visible),
        "an offered Throw shows its button",
    );
    assert_eq!(
        app.world().get::<Node>(button).map(|node| node.display),
        Some(Display::Flex),
        "a shown button reserves its layout row (Display::Flex)",
    );

    clear_hover(&mut app);
    app.update();
    assert_eq!(
        single_with::<ContextualPanelRoot>(&mut app),
        Some(root),
        "the panel root entity must persist while hidden (mutate-in-place, not despawn)",
    );
    assert_eq!(
        single_with::<ThrowGrenadeButton>(&mut app),
        Some(button),
        "the Throw button entity must persist while hidden (mutate-in-place, not despawn)",
    );
    assert_eq!(
        app.world().get::<Visibility>(button).copied(),
        Some(Visibility::Hidden),
        "an un-offered Throw hides its button",
    );
    assert_eq!(
        app.world().get::<Node>(button).map(|node| node.display),
        Some(Display::None),
        "a hidden button COLLAPSES its layout row (Display::None) so the column stays in the bar",
    );

    hover_cell(&mut app, 15, 15);
    app.update();
    assert_eq!(
        single_with::<ContextualPanelRoot>(&mut app),
        Some(root),
        "the panel root entity must be the SAME across the offer on->off->on cycle (no respawn)",
    );
    assert_eq!(
        single_with::<ThrowGrenadeButton>(&mut app),
        Some(button),
        "the Throw button entity must be the SAME across the cycle (no respawn)",
    );
    assert_eq!(
        app.world().get::<Node>(button).map(|node| node.display),
        Some(Display::Flex),
        "the reshown button reserves its layout row again (Display::Flex) — a mutate, not respawn",
    );
}
