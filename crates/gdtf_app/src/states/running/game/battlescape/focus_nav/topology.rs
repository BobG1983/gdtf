//! GTW-782: the inter-panel focus-crossing TOPOLOGY — the every-frame rebuild of the
//! battlescape's Tab traversal chain over the currently-shown, enabled panel buttons, and
//! its battle-scoped teardown.
//!
//! ## How this differs from the menu's static column
//!
//! The menu ([`spawn_menu`](crate::states::running::menu)) wires its focus chain ONCE on
//! spawn: a fixed, always-present column of buttons joined by South edges, cleared on
//! menu exit. The battlescape HUD is a materially different shape — MULTIPLE independently
//! shown/hidden panels (the action bar, the weapon panel's Reload, and the 0–8 dynamically
//! offered contextual buttons), each of which appears, disappears, or greys out
//! (`DisabledButton`) as the battle plays. A single static wiring would strand edges on
//! hidden / despawned buttons.
//!
//! So instead of a one-shot wiring this rebuilds the chain EACH FRAME from the live set:
//! every button wearing [`PanelNavOrder`] that is currently shown and enabled is gathered,
//! sorted by its stable [`PanelNavOrder`] ordinal (entity id as the tie-break, matching the
//! contextual panel's own `order_contextual_buttons`), and joined into ONE left-to-right
//! chain by EAST edges (so Tab / Right = East steps forward, Shift+Tab / Left = West steps
//! back). Rebuilding is cheap (≤ ~15 buttons) and always reflects the current visibility,
//! so a button that hides/greys this frame simply drops out of next frame's chain.
//!
//! The chain uses purely HORIZONTAL (East/West) edges on purpose: the presenter binds the
//! arrow keys AND WASD to camera panning every battle frame, and the focus framework's
//! own built-in keyboard bridge maps Up/Down/W/S to vertical navigate requests — a
//! vertical battlescape chain would let those camera-pan keys also walk focus. A
//! horizontal-only chain leaves the vertical navigate requests with no edge to follow (a
//! no-op), so camera panning and Tab focus-nav never collide.

use bevy::{
    input_focus::directional_navigation::DirectionalNavigationMap, math::CompassOctant, prelude::*,
};
use gdtf_battle_input::PanelNavOrder;
use gdtf_ui::DisabledButton;

/// The Tab-chain ordinal band the ACTION BAR's buttons occupy — Level + / Level − /
/// End Turn / Flee take `ACTION_BAR_NAV_BASE + 0..=3`, so they lead the left-to-right
/// traversal (the top-centre bar reads first).
pub(in crate::states::running::game::battlescape) const ACTION_BAR_NAV_BASE: u16 = 0;

/// The Tab-chain ordinal band the WEAPON PANEL's Reload button occupies — after the action
/// bar, before the contextual cluster.
pub(in crate::states::running::game::battlescape) const WEAPON_NAV_BASE: u16 = 100;

/// The Tab-chain ordinal band the CONTEXTUAL PANEL's act buttons occupy — the contextual
/// cluster comes last, each button offset by its stable `PanelSlot` ordinal so the Tab
/// order within the cluster matches its top-to-bottom column order.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_NAV_BASE: u16 = 200;

/// Rebuilds the battlescape's focus-navigation Tab chain over the currently-shown, enabled
/// panel buttons (GTW-782).
///
/// Gathers every [`PanelNavOrder`] button that is not [`Visibility::Hidden`] and not a
/// [`DisabledButton`], sorts by `(PanelNavOrder, Entity)` (the deterministic order the
/// contextual panel's own child-ordering uses), clears the
/// [`DirectionalNavigationMap`], and joins the chain by EAST edges via
/// [`add_edges`](DirectionalNavigationMap::add_edges) (which also lays the reverse West
/// edges) — so Tab / Right steps East (forward), Shift+Tab / Left steps West (back).
///
/// It reads the button's OWN [`Visibility`] component (set to `Hidden` by the contextual
/// per-act toggle when not offered and by the weapon panel when the weapon has no
/// magazine, left `Inherited` = shown for the always-present bar buttons), so a hidden
/// button drops out of the chain the frame it hides. Registered in `Update` gated on the
/// live-battle witness and ordered `.before(FocusNavSystems::Apply)` so a navigate raised
/// this frame reads the fresh chain (`bevy-traps.md` #3). During battle this system is the
/// sole writer of the global map, so clearing + rebuilding it wholesale each frame is
/// correct and ordering-independent (the menu is torn down; its edges were cleared on
/// menu exit).
pub(in crate::states::running::game::battlescape) fn rebuild_panel_nav_topology(
    mut nav_map: ResMut<DirectionalNavigationMap>,
    buttons: Query<(Entity, &PanelNavOrder, &Visibility, Has<DisabledButton>)>,
) {
    let mut chain: Vec<(PanelNavOrder, Entity)> = buttons
        .iter()
        .filter(|(_, _, visibility, disabled)| {
            !matches!(**visibility, Visibility::Hidden) && !disabled
        })
        .map(|(entity, order, ..)| (*order, entity))
        .collect();
    chain.sort_by_key(|(order, entity)| (*order, *entity));
    let ordered: Vec<Entity> = chain.into_iter().map(|(_, entity)| entity).collect();

    nav_map.clear();
    // `add_edges` needs at least two nodes to lay an edge; a 0/1-button chain is just an
    // empty map (a lone focusable has no neighbour to Tab to — a no-op, not an error).
    if ordered.len() >= 2 {
        nav_map.add_edges(&ordered, CompassOctant::East);
    }
}

/// Clears the battlescape's focus-navigation edges on `OnExit(BattleRunning)` (GTW-782).
///
/// The panel buttons are despawned by the panels' own `OnExit` cleanup, but the
/// [`DirectionalNavigationMap`] is a GLOBAL resource (not state-scoped), so its edges —
/// keyed by the now-dead button entities — would linger and could poison the next menu's
/// chain (the menu's `spawn_menu` adds edges WITHOUT clearing first). Clearing wholesale on
/// battle exit mirrors the menu's own `clear_nav_map` and keeps the map clean across the
/// menu ⇆ battle boundary. Ordering-independent (a wholesale `clear`, not a per-entity
/// `remove` that would need the entities still alive).
pub(in crate::states::running::game::battlescape) fn clear_panel_nav_topology(
    mut nav_map: ResMut<DirectionalNavigationMap>,
) {
    nav_map.clear();
}
