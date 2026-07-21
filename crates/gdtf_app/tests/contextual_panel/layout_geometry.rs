//! GTW-726 — the contextual (context) window is a CHILD of the bottom bar, laid out INSIDE it,
//! and its updates mutate in place (never despawn/respawn).
//!
//! The first test drives the [`real_layout_harness`](super::real_layout_harness) — a REAL
//! `bevy_ui` layout pass, against a REAL window size and a REAL loaded font — so it reads ACTUAL
//! computed pixel geometry ([`ComputedNode`] + [`UiGlobalTransform`]) and can assert the panel
//! root's rect is CONTAINED inside the bottom bar's rect (never floating up over the map) — the
//! whole detach bug. The second test drives the faster `MinimalPlugins` `battle_running_app` and
//! pins the mutate-in-place update path: toggling an act offer on -> off -> on hides and reshows
//! the panel while the SAME entities persist (a `Visibility` + `Display` toggle, never a
//! despawn/respawn).

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

/// A small epsilon (physical px): sub-pixel rounding between the layout pass and the rect math.
const EPSILON_PX: f32 = 1.0;

/// Spawns a THROW actor — a ganger with its [`Position`] + [`Faction`], wielding an
/// [`TrajectoryStyle::Arc`] ranged weapon (so the Throw act is offer-able) — at cell `(x, y)`, and
/// SELECTS it. Returns the ganger entity. Mirrors the `throw.rs` spawner.
fn spawn_throw_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app.world_mut().spawn((at(x, y), Faction::new(gang))).id();
    app.world_mut()
        .spawn((WieldedBy::new(actor), TrajectoryStyle::Arc));
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

/// Seeds the [`InspectTarget`] live hovered cell to `(x, y)` on level 0 — the blind Throw's target
/// cell (the cursor-over-cell the picker would write). Re-seeded each frame because the headless
/// picker overwrites `InspectTarget` with no live cursor (the `throw.rs` precedent).
fn hover_cell(app: &mut App, x: i32, y: i32) {
    app.world_mut()
        .insert_resource(InspectTarget::new(Some(CellLevel::new(
            Cell::new(x, y),
            Level::new(0),
        ))));
}

/// Clears the [`InspectTarget`] live hovered cell — no cell under the cursor, so the blind Throw is
/// not offered.
fn clear_hover(app: &mut App) {
    app.world_mut().insert_resource(InspectTarget::new(None));
}

/// An axis-aligned rect from a UI node's REAL computed geometry — its [`ComputedNode::size`]
/// (physical px) centered on its [`UiGlobalTransform`] translation (physical px, per `bevy_ui`'s
/// own over-UI hit-test — both physical, matching units). (The `weapon_panel` `PixelRect`
/// precedent.)
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

    /// Whether `self` is fully contained inside `other` (within [`EPSILON_PX`] on every edge).
    fn contained_in(&self, other: &Self) -> bool {
        self.min.x >= other.min.x - EPSILON_PX
            && self.min.y >= other.min.y - EPSILON_PX
            && self.max.x <= other.max.x + EPSILON_PX
            && self.max.y <= other.max.y + EPSILON_PX
    }
}

/// The single entity carrying marker `M`'s [`ComputedNode`] + [`UiGlobalTransform`] as a
/// [`PixelRect`], or `None` if not exactly one exists.
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

/// GTW-726 — with an act offered (a Throw), the contextual panel root is a CHILD of the bottom bar
/// AND its REAL computed rect is CONTAINED inside the bottom bar's rect: it does not float up over
/// the map viewport. Pin-discriminating: reverting to the pre-GTW-726 top-level root anchored to
/// the window (with eight `Visibility::Hidden` rows reserving layout) grows the box past the bar's
/// top edge onto the map, failing the containment assert.
#[test]
fn contextual_panel_is_contained_inside_the_bottom_bar() {
    let app_opt = real_layout_battle_running_app();
    assert!(
        app_opt.is_some(),
        "the real-layout harness must reach BattleScapeState::BattleRunning with its font loaded",
    );
    let Some(mut app) = app_opt else { return };

    spawn_throw_actor(&mut app, 5, 5, 0);
    // Re-seed the hover each frame (the offer scan reads it before the picker clears it) and settle:
    // the offer -> button/root visibility toggle -> a real bevy_ui layout pass each need a frame.
    for _ in 0..8 {
        hover_cell(&mut app, 15, 15);
        app.update();
    }

    // The root is a CHILD of the bottom bar (not a free-floating top-level overlay).
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

    // Sanity: the offered Throw revealed the panel root (so its rect is a real laid-out box).
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

/// GTW-726 — the update path mutates in place: toggling the Throw offer on -> off -> on hides and
/// reshows the panel WITHOUT despawning it. The SAME root + button entities persist across the
/// whole cycle (asserted by `Entity` id), and the button's `Visibility` + `Node::display` flip
/// together (the collapse that keeps the box inside the bar). Pin-discriminating: a
/// despawn/respawn update path would hand back a fresh `Entity` id on the reshow, failing the
/// persistence asserts.
#[test]
fn contextual_panel_updates_mutate_in_place() {
    let mut app = battle_running_app();
    spawn_throw_actor(&mut app, 5, 5, 0);

    // Offer ON: Throw is revealed.
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

    // Offer OFF: clear the hover — Throw is no longer offered, so the panel hides.
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

    // Offer ON again: the SAME entities reshow — a fresh id here would prove a respawn.
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
