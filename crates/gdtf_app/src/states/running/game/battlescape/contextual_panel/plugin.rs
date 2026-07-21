//! The contextual-panel scene-plugin (GTW-294; generic per-act registration since
//! GTW-571).
//!
//! Registers the battle-scoped contextual cluster in the battlescape neighborhood,
//! beside the action-bar / bottom-bar / weapon-panel plugins:
//!
//! - **Set vocabulary** — [`configure_contextual_panel_sets`] configures the panel's
//!   `SystemSet`s ONCE (P5): the Update chain `Offer -> Toggle -> Press` (a press reads
//!   the SAME update's freshly-scanned offer), `Offer.before(pick_hovered_cell)` (the
//!   consumers-read-the-hover-before-the-picker-rewrites-it idiom), `Press.before(`the
//!   input layer's [`ContextualActSystems::Drain`](gdtf_battle_input::contextual::ContextualActSystems)`)`
//!   (the Q5 same-frame press -> `*Requested` edge), and the `OnEnter` chain
//!   `Root -> Buttons -> Order`.
//! - **Lifecycle** (mirrors the sibling action bar / bottom bar) —
//!   `spawn_contextual_panel` (the root box) `OnEnter(BattleScapeState::BattleRunning)`,
//!   the per-act generic button spawns after it, the deterministic
//!   `order_contextual_buttons` re-parent last, and `despawn_contextual_panel`
//!   `OnExit(BattleScapeState::BattleRunning)` — the panel exists only during the live
//!   tactical layer.
//! - **Per-act registration** — ONE
//!   [`add_contextual_act_button`](ContextualPanelActAppExt::add_contextual_act_button)
//!   line per contextual act stamps the act's offer resource + generic spawn / toggle /
//!   press systems over its descriptor (GTW-571 C1; see
//!   `docs/authoring/contextual-act-recipe.md` for the add-an-act recipe).
//! - **Root visibility** — the act-agnostic `sync_panel_root_visibility` pass shows the
//!   panel box iff ANY act button is visible, ordered `.after` the per-act toggles.
//!
//! View-only — it owns no sim/input state; it reads the input selection + writes the
//! per-act [`PendingContextualIntents`](gdtf_battle_input::contextual::PendingContextualIntents)
//! queues, never a sim component directly (P8 — dispatch is sim-side).

use bevy::prelude::*;
use gdtf_battle_input::contextual::{
    EnterEmplacementAct, ExecuteAct, ExitEmplacementAct, MeleeAct, OpenDoorAct, ShoveAct,
    StabilizeAct, ThrowGrenadeAct,
};
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::{
        bottom_bar::{despawn_bottom_bar, spawn_bottom_bar},
        contextual_panel::{
            acts,
            registrar::{
                ContextualPanelActAppExt, ContextualPanelSpawnSystems, ContextualPanelSystems,
                configure_contextual_panel_sets,
            },
            systems::{
                despawn_contextual_panel, order_contextual_buttons, spawn_contextual_panel,
                sync_panel_root_visibility,
            },
        },
    },
};

/// The contextual-panel scene-plugin — configures the panel's set vocabulary, spawns /
/// despawns the bottom-right cluster on the `BattleRunning` boundary, registers each
/// contextual act's button slice through the compile-time registrar (ONE line per act),
/// and runs the act-agnostic root-visibility pass gated on the live-battle witness
/// (GTW-294 / GTW-571).
pub(in crate::states::running::game::battlescape) struct ContextualPanelPlugin;

impl Plugin for ContextualPanelPlugin {
    fn build(&self, app: &mut App) {
        // The SystemSet vocabulary — configured ONCE (bevy-traps.md #5 / GTW-571 P5).
        configure_contextual_panel_sets(app);

        app.add_systems(
            OnEnter(BattleScapeState::BattleRunning),
            (
                // Ordered `.after(spawn_bottom_bar)` (GTW-726): the bottom-bar root must exist so
                // `spawn_contextual_panel` can parent the panel box INSIDE it (the stance-panel
                // precedent). Both run on the same `OnEnter` boundary, so the order must be
                // explicit (`bevy-traps.md` #3).
                spawn_contextual_panel
                    .in_set(ContextualPanelSpawnSystems::Root)
                    .after(spawn_bottom_bar),
                // The deterministic slot-order re-parent — after every per-act button
                // spawn (the set chain), so the column order never depends on system
                // scheduling (bevy-traps.md #3).
                order_contextual_buttons.in_set(ContextualPanelSpawnSystems::Order),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            // Ordered `.before(despawn_bottom_bar)` (GTW-726): the panel root is a CHILD of the
            // bar, so it must be despawned (and unlinked from the bar's children) before the bar's
            // own recursive despawn runs — otherwise both would target it (a double despawn).
            despawn_contextual_panel.before(despawn_bottom_bar),
        )
        .add_systems(
            Update,
            // The panel root shows iff ANY act button is visible — ordered after the
            // per-act toggles so it reads the SAME update's visibility writes.
            sync_panel_root_visibility
                .after(ContextualPanelSystems::Toggle)
                .run_if(resource_exists::<BattleInProgress>),
        );

        // ONE registration line per contextual act (GTW-571 C1) — each stamps the
        // generic spawn / toggle / press systems over the act's descriptor and wires
        // its bespoke offer scan into the Offer set.
        app.add_contextual_act_button::<ExecuteAct, _>(acts::execute::offer_execute)
            .add_contextual_act_button::<StabilizeAct, _>(acts::stabilize::offer_stabilize)
            .add_contextual_act_button::<MeleeAct, _>(acts::melee::offer_melee)
            .add_contextual_act_button::<ShoveAct, _>(acts::shove::offer_shove)
            .add_contextual_act_button::<OpenDoorAct, _>(acts::open_door::offer_open_door)
            .add_contextual_act_button::<EnterEmplacementAct, _>(
                acts::enter_emplacement::offer_enter_emplacement,
            )
            .add_contextual_act_button::<ExitEmplacementAct, _>(
                acts::exit_emplacement::offer_exit_emplacement,
            )
            .add_contextual_act_button::<ThrowGrenadeAct, _>(
                acts::throw_grenade::offer_throw_grenade,
            );
    }
}
