//! The battlescape focus-navigation scene-plugin (GTW-782).
//!
//! Wires the keyboard focus-navigation into the battlescape HUD: the typed-`Keybinds` →
//! framework-message [`bridge`](super::bridge) (live only while a panel holds focus), the
//! [`topology`](super::topology) rebuild that keeps the Tab chain current over the
//! dynamically shown/hidden panel buttons, and the [`outline`](super::outline) focus
//! highlight. The [`PanelNavOrder`](gdtf_battle_input::PanelNavOrder) markers themselves
//! are attached by the individual panels' spawn systems (action bar / weapon panel /
//! contextual panel).

use bevy::{input_focus::directional_navigation::DirectionalNavigationMap, prelude::*};
use gdtf_battle_input::Keybinds;
use gdtf_battle_sim::prelude::BattleInProgress;
use gdtf_ui::focus_nav::FocusNavSystems;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::focus_nav::{
        bridge::{apply_focus_cancel, bridge_panel_focus_nav},
        outline::paint_focus_outline,
        topology::{clear_panel_nav_topology, rebuild_panel_nav_topology},
    },
};

/// The focus-navigation scene-plugin — registers the panel-focus bridge, the Tab-chain
/// topology rebuild, and the focus outline, all gated on the live-battle witness, plus the
/// `OnExit(BattleRunning)` nav-map teardown.
pub(in crate::states::running::game::battlescape) struct GameBattleScapeFocusNavScenePlugin;

impl Plugin for GameBattleScapeFocusNavScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                // Keep the Tab chain current over the live shown/enabled button set, and
                // feed the framework nav messages — both BEFORE `apply_navigation`
                // (`FocusNavSystems::Apply`) so a Tab / arrow this frame reads the fresh
                // chain and moves focus the same frame (`bevy-traps.md` #3).
                rebuild_panel_nav_topology.before(FocusNavSystems::Apply),
                bridge_panel_focus_nav.before(FocusNavSystems::Apply),
                // Escape's cancel clears focus AFTER the bridge raised it this frame.
                apply_focus_cancel.after(bridge_panel_focus_nav),
                // The focus ring follows the resolved focus.
                paint_focus_outline,
            )
                // Gated on the live-battle witness AND the focus framework being present:
                // the systems take `DirectionalNavigationMap` / `InputFocus` / the nav
                // message buffers (all installed by `UiPlugin`'s `FocusNavPlugin`) and
                // `Keybinds` (loaded by battle time), so a headless harness without the UI
                // plugin / a bound keybind table (which some battle-scoped panel tests are)
                // stays inert rather than panicking on a missing resource (`bevy-traps.md`
                // #1). `DirectionalNavigationMap` is the witness that the whole focus
                // framework — its resources and message buffers — is installed.
                .run_if(
                    resource_exists::<BattleInProgress>
                        .and_then(resource_exists::<DirectionalNavigationMap>)
                        .and_then(resource_exists::<Keybinds>),
                ),
        )
        // Drop the (global) nav edges when the battle's live layer ends, so stale
        // button-entity edges never poison the next menu's chain.
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            clear_panel_nav_topology,
        );
    }
}
