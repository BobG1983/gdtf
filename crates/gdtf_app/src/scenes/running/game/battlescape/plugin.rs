use bevy::prelude::*;
use gdtf_battle_sim::BattleInProgress;

use crate::{
    scenes::running::game::battlescape::{
        GameBattleScapeActionBarScenePlugin, GameBattleScapeAfterMathScenePlugin,
        GameBattleScapeAnimateInScenePlugin, GameBattleScapeAnimateOutScenePlugin,
        GameBattleScapeBattleRunningScenePlugin, GameBattleScapeGenerationScenePlugin,
        GameBattleScapeHoverPanelScenePlugin, GameBattleScapeStatusPanelScenePlugin,
        GameBattleScapeWeaponPanelScenePlugin, systems::*,
    },
    states::{BattleScapeState, GameState},
};

pub(in crate::scenes) struct GameBattleScapeScenePlugin;

impl Plugin for GameBattleScapeScenePlugin {
    fn build(&self, app: &mut App) {
        add_states(app);
        add_plugins(app);
        add_systems(app);
    }
}

fn add_systems(app: &mut App) {
    app.add_systems(OnEnter(GameState::BattleScape), print_on_enter)
        .add_systems(OnExit(GameState::BattleScape), print_on_exit)
        // GTW-216: the SHARED world-camera lifecycle. The presenter exposes these as
        // `pub` param-only systems but cannot name `GameState` (it has no `gdtf_app`
        // dep), so the app registers them on the `GameState::BattleScape` boundary —
        // the same span as `request_battle_teardown` — so the camera survives the whole
        // `Generation → AnimateIn → BattleRunning → AnimateOut → AfterMath` walk and is
        // torn down only when the battle is left.
        .add_systems(
            OnEnter(GameState::BattleScape),
            gdtf_battle_presenter::spawn_world_camera,
        )
        .add_systems(
            OnExit(GameState::BattleScape),
            gdtf_battle_presenter::despawn_world_camera,
        )
        // GTW-271: confine the world render to a central viewport sub-rect. The app owns the
        // UI layout, so it MEASURES the panel roots' `ComputedNode` sizes and writes the
        // presenter's `WorldCamera` viewport (the panels live in the margins around the map).
        // Gated on the `BattleInProgress` live-battle witness — the SAME witness the
        // action-bar / status-panel / input gate on (`bevy-traps.md` #1) — so it runs only
        // when the camera + panels exist. The idempotent every-frame write subsumes AC7's
        // resize + battle-start recompute (it runs every live frame). The presenter's
        // pan/clamp READ the viewport rect, so this writer needs no explicit ordering against
        // them (a one-frame settle is harmless for a camera rect).
        .add_systems(
            Update,
            set_world_viewport.run_if(resource_exists::<BattleInProgress>),
        );
}

fn add_plugins(app: &mut App) {
    app.add_plugins(GameBattleScapeGenerationScenePlugin)
        .add_plugins(GameBattleScapeAnimateInScenePlugin)
        .add_plugins(GameBattleScapeBattleRunningScenePlugin)
        .add_plugins(GameBattleScapeAnimateOutScenePlugin)
        .add_plugins(GameBattleScapeAfterMathScenePlugin)
        // The GTW-48 presenter seam (GTW-215): the VIEW that mirrors the sim. Its
        // `build` runs here when the scene plugins register; the default mode
        // builds the TopDown renderer (GTW-217 renamed it from CP437; it loads the
        // sprite atlases but draws no sprite yet — that is S4/S5/S6).
        .add_plugins(gdtf_battle_presenter::BattlePresenterPlugin::default())
        // The GTW-48 S7 input seam (GTW-221): the HEAD of the
        // `input -> presenter -> sim` chain. Its `build` runs here beside the
        // presenter; its picking + hover-highlight systems run in `Update` gated
        // on the sim's `BattleInProgress` witness, so the cursor is inert until a
        // live battle. It reads the presenter's `WorldCamera` + px/level interface
        // and writes `HoveredCell` + one highlight sprite; it emits NO act (S8).
        .add_plugins(gdtf_battle_input::GdtfBattleInputPlugin)
        // The GTW-48 S9 / 222c action-bar (GTW-228): a themed `gdtf_ui` button surface
        // on the GTW-120 UI camera. It spawns/despawns on the `BattleScapeState::
        // BattleRunning` boundary and its button-action system writes the SAME 222a
        // `PendingActIntent` seam the input crate's keyboard surface writes — buttons +
        // keys are PARALLEL surfaces over the ONE drain. It deps `gdtf_ui` (the spawn
        // helpers) + `gdtf_battle_input` (the intent seam), both already on the app's
        // edge; the chain stays acyclic.
        .add_plugins(GameBattleScapeActionBarScenePlugin)
        // The GTW-252 status HUD panel: a themed `gdtf_ui` panel on the GTW-120 UI
        // camera showing the selected player ganger's vitals. Like the action-bar it
        // spawns/despawns on the `BattleScapeState::BattleRunning` boundary; its repaint
        // system reads `Res<SelectedShooter>` (the input crate's selection seam) + the
        // sim's on-entity vital components and is gated on the `BattleInProgress`
        // witness. UI/view only — no sim/input change, no act. It deps `gdtf_ui` (the
        // spawn helpers) + `gdtf_battle_input` (the selection seam), both already on the
        // app's edge; the chain stays acyclic.
        .add_plugins(GameBattleScapeStatusPanelScenePlugin)
        // The GTW-274 hover-inspect panel: the twin of the status panel, anchored top-right.
        // Same `BattleRunning` lifecycle + `BattleInProgress` gate; its repaint reads the
        // input crate's `HoveredCell` + the sim's `OccupancyGrid` / `CoverLedger` / vital
        // components and renders the shared stat block (hovered ganger) or an object block
        // (hovered wall / cover). UI/view only — no sim/input change, no act.
        .add_plugins(GameBattleScapeHoverPanelScenePlugin)
        // The GTW-275 weapon panel: bottom-left, the selected ganger's weapon (graphic
        // placeholder + name + magazine cur/max) + a LIVE Reload button + throwable
        // placeholders. Same `BattleRunning` lifecycle + `BattleInProgress` gate; its repaint
        // reads `Res<SelectedShooter>` + the sim's `WeaponName` / `Magazine` components, and
        // its Reload button WRITES the input crate's act-intent seam (→ `ReloadRequested` →
        // the sim's `dispatch_reload`). It deps `gdtf_ui` (spawn helpers) + `gdtf_battle_input`
        // (selection + intent seam), both already on the app's edge; the chain stays acyclic.
        .add_plugins(GameBattleScapeWeaponPanelScenePlugin);
}

fn add_states(app: &mut App) {
    app.add_sub_state::<BattleScapeState>();
}
