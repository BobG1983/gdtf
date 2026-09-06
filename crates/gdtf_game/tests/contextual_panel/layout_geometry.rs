use bevy::{
    ecs::{entity::Entity, query::With},
    math::Vec2,
    prelude::*,
    ui::{ComputedNode, ComputedStackIndex, GlobalZIndex, UiGlobalTransform},
};
use gdtf_battle_input::{InspectTarget, SelectedShooter};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Faction, Level},
    weapon::{TrajectoryStyle, WieldedBy},
};
use gdtf_game::test_support::{
    BottomBarRoot, ContextualPanelRoot, SelectCycleRoot, StancePanelRoot, ThrowGrenadeButton,
    WeaponPanelRoot,
};

use super::{
    actors::{at, magazine_of},
    harness::{battle_running_app, parent_of, single_with, the_only},
    real_layout_harness::real_layout_battle_running_app,
};

const EPSILON_PX: f32 = 1.0;

fn spawn_throw_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app.world_mut().spawn((at(x, y), Faction::new(gang))).id();
    app.world_mut()
        .spawn((WieldedBy::new(actor), TrajectoryStyle::Arc, magazine_of(1)));
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

fn empty_the_magazine(app: &mut App) {
    let weapon = the_only::<TrajectoryStyle>(
        app,
        "exactly one weapon must carry a TrajectoryStyle so emptying it hides Throw",
    );
    if let Ok(mut entity) = app.world_mut().get_entity_mut(weapon) {
        entity.insert(magazine_of(0));
    }
}

fn load_the_magazine(app: &mut App) {
    let weapon = the_only::<TrajectoryStyle>(
        app,
        "exactly one weapon must carry a TrajectoryStyle so loading it shows Throw",
    );
    if let Ok(mut entity) = app.world_mut().get_entity_mut(weapon) {
        entity.insert(magazine_of(1));
    }
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

    fn overlaps(&self, other: &Self) -> bool {
        self.min.x < other.max.x
            && other.min.x < self.max.x
            && self.min.y < other.max.y
            && other.min.y < self.max.y
    }

    fn size(&self) -> Vec2 {
        self.max - self.min
    }
}

/// One panel of the bottom bar, with the entity and the laid-out rect its checks need.
struct BarPanel {
    name:   &'static str,
    entity: Entity,
    rect:   PixelRect,
}

fn bar_panel<M: Component>(app: &mut App, name: &'static str) -> BarPanel {
    let entity = the_only::<M>(
        app,
        &format!("exactly one {name} must exist in the live battle"),
    );
    let rect = pixel_rect_of::<M>(app, &format!("the {name} must have real computed geometry"));
    BarPanel { name, entity, rect }
}

fn stack_index(app: &App, entity: Entity) -> Option<u32> {
    app.world()
        .get::<ComputedStackIndex>(entity)
        .map(|index| **index)
}

/// The one laid-out rect of `M`, failing on `claim` when the marker resolves to any other count.
fn pixel_rect_of<M: Component>(app: &mut App, claim: &str) -> PixelRect {
    let mut query = app
        .world_mut()
        .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<M>>();
    let mut results: Vec<PixelRect> = query
        .iter(app.world())
        .map(|(node, transform)| PixelRect::from_node(node, transform))
        .collect();
    assert_eq!(results.len(), 1, "{claim}; found {} instead", results.len());
    let Some(rect) = results.pop() else {
        unreachable!("the count assertion above leaves exactly one rect")
    };
    rect
}

#[test]
fn contextual_panel_is_contained_inside_the_bottom_bar() {
    let mut app = real_layout_battle_running_app();

    spawn_throw_actor(&mut app, 5, 5, 0);
    for _ in 0..8 {
        hover_cell(&mut app, 15, 15);
        app.update();
    }

    let bar = bar_panel::<BottomBarRoot>(&mut app, "bottom bar");
    let contextual = bar_panel::<ContextualPanelRoot>(&mut app, "contextual panel root");
    assert_eq!(
        app.world().get::<Visibility>(contextual.entity).copied(),
        Some(Visibility::Visible),
        "an offered Throw must reveal the panel root so its geometry is meaningful",
    );

    let panels = [
        bar_panel::<WeaponPanelRoot>(&mut app, "weapon panel"),
        bar_panel::<StancePanelRoot>(&mut app, "stance panel"),
        contextual,
        bar_panel::<SelectCycleRoot>(&mut app, "Next/Prev cluster"),
    ];

    for panel in &panels {
        let size = panel.rect.size();
        assert!(
            size.x > 0.0 && size.y > 0.0,
            "the {} must be laid out with a real size, or every check below passes for the wrong \
             reason. Got {size:?}",
            panel.name,
        );
        assert_eq!(
            parent_of(&app, panel.entity),
            Some(bar.entity),
            "the {} must be a CHILD of the bottom bar, so the bar's row arranges it",
            panel.name,
        );
        assert!(
            panel.rect.contained_in(&bar.rect),
            "the {} ({:?}..{:?}) must be CONTAINED inside the bottom bar ({:?}..{:?}). Every \
             panel is part of the bottom HUD, never a free-floating box over the map. Its top \
             edge (min.y={}) must not sit above the bar's top edge (min.y={}).",
            panel.name,
            panel.rect.min,
            panel.rect.max,
            bar.rect.min,
            bar.rect.max,
            panel.rect.min.y,
            bar.rect.min.y,
        );
    }

    for (index, one) in panels.iter().enumerate() {
        for other in &panels[index + 1..] {
            assert!(
                !one.rect.overlaps(&other.rect),
                "the {} ({:?}..{:?}) must NOT overlap the {} ({:?}..{:?}). The bar's row lays the \
                 four out side by side, so no panel can draw over another",
                one.name,
                one.rect.min,
                one.rect.max,
                other.name,
                other.rect.min,
                other.rect.max,
            );
        }
    }

    let bar_stack = stack_index(&app, bar.entity);
    assert!(
        bar_stack.is_some(),
        "the bottom bar must carry a ComputedStackIndex for the draw-order checks to mean anything",
    );
    for panel in &panels {
        assert!(
            app.world().get::<GlobalZIndex>(panel.entity).is_none(),
            "the {} must carry NO GlobalZIndex. One re-roots it out of the bar's stacking \
             context, which is how the Next/Prev cluster came to draw over the Melee button",
            panel.name,
        );
        let panel_stack = stack_index(&app, panel.entity);
        assert!(
            panel_stack > bar_stack,
            "the {} must draw ABOVE the bar's own background: its ComputedStackIndex \
             ({panel_stack:?}) must be greater than the bar's ({bar_stack:?})",
            panel.name,
        );
    }
}

#[test]
fn contextual_panel_updates_mutate_in_place() {
    let mut app = battle_running_app();
    spawn_throw_actor(&mut app, 5, 5, 0);

    hover_cell(&mut app, 15, 15);
    app.update();
    let root = the_only::<ContextualPanelRoot>(
        &mut app,
        "the panel root must exist once an act is offered",
    );
    let button = the_only::<ThrowGrenadeButton>(
        &mut app,
        "the Throw button must exist once an act is offered",
    );
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

    empty_the_magazine(&mut app);
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

    load_the_magazine(&mut app);
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
