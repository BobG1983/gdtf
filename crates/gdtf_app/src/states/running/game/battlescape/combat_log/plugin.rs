//! The combat-log scene-plugin (GTW-328, slice 3, bottom-left, ABOVE the weapon panel).
//!
//! Registers the battle-scoped combat-text LOG in the battlescape neighborhood, beside the
//! action-bar / weapon-panel / status-panel / presenter / input plugins. The log shows the few
//! most-recent combat events (movement, shot declarations, hit/miss outcomes, damage/wounds,
//! reloads, turn boundaries) as lines that scroll up and fade — UI/view only (it READS the sim's
//! combat-event messages + the ganger names, and writes nothing back into the sim/input).
//!
//! - **RON tuning** — the hot-reloadable [`CombatLogTuning`] table loads through the generic
//!   [`RonAsset<T>`](gdtf_assets::RonAsset) loader the FX-tuning / pan-tuning tables use: a
//!   `Startup` [`load_combat_log_tuning`], an `Update` [`resolve_combat_log_tuning`] gated until
//!   it resolves once, and an unguarded `Update` [`redrive_combat_log_tuning_on_asset_event`] so
//!   a `combat_log.ron` edit re-tunes the log live. The whole RON block is gated on an
//!   [`AssetServer`] existing — `init_ron_asset` PANICS at registration without the asset stack,
//!   so a `MinimalPlugins` headless app skips it (`bevy-traps.md` #1), falling back to
//!   [`CombatLogTuning::default`].
//! - **Lifecycle** (mirrors the sibling weapon panel) — [`spawn_combat_log`]
//!   `OnEnter(BattleScapeState::BattleRunning)`, [`despawn_combat_log`]
//!   `OnExit(BattleScapeState::BattleRunning)`, so the log exists only during the live tactical
//!   layer (NOT the whole `GameState::BattleScape`).
//! - **Update** — [`update_combat_log`] drains the four event-driven sim combat-event messages
//!   PLUS the presenter's per-shot [`ShotImpactResolved`](gdtf_battle_presenter::ShotImpactResolved)
//!   signal (the shot-outcome lines key off it so they appear at each shot's staggered IMPACT, not
//!   on the fire frame — GTW-328), classifies them, and appends lines (FIFO-trimming to the tuned
//!   cap); it runs gated
//!   `run_if(resource_exists::<BattleInProgress>)` (the live-battle witness, `bevy-traps.md` #1).
//!   [`fade_combat_log_lines`] ticks each line's fade clock + despawns finished lines; it runs
//!   unguarded (it self-gates on the lines existing) so a line spawned in the last `BattleRunning`
//!   frame still fades out cleanly.
//!
//! It deps `gdtf_ui` (the spawn helpers + theme), `gdtf_battle_presenter` (the shared
//! `classify_log_event` classifier), `gdtf_battle_sim` (the combat-event messages + ganger
//! names), and `gdtf_assets` (the RON loader) — all already on the app's edge; the chain stays
//! acyclic.

use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::RonAssetAppExt;
use gdtf_battle_presenter::ShotImpactResolved;
use gdtf_battle_sim::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::combat_log::{
        systems::{
            animate_combat_log_height, despawn_combat_log, fade_combat_log_lines,
            slide_combat_log_lines, spawn_combat_log, update_combat_log,
        },
        tuning::{
            CombatLogTuning, CombatLogTuningHandle, load_combat_log_tuning,
            redrive_combat_log_tuning_on_asset_event, resolve_combat_log_tuning,
        },
    },
};

/// The combat-log scene-plugin — loads its hot-reloadable RON tuning, spawns/despawns the log on
/// the `BattleRunning` boundary, and runs its event-drain + fade systems.
pub(in crate::states::running::game::battlescape) struct GameBattleScapeCombatLogScenePlugin;

impl Plugin for GameBattleScapeCombatLogScenePlugin {
    fn build(&self, app: &mut App) {
        register_ron_tuning(app);
        add_systems(app);
    }
}

/// Register the hot-reloadable combat-log RON tuning chain (load / resolve / redrive).
///
/// Gated on an [`AssetServer`] existing: `init_ron_asset` PANICS at registration without the
/// `Assets<T>` machinery, so a `MinimalPlugins` headless app (no asset stack) skips the chain —
/// no load, no panic (`bevy-traps.md` #1); it then runs on [`CombatLogTuning::default`]. Under
/// `DefaultPlugins` the table loads for real. Mirrors the presenter's `register_ron_tables`.
fn register_ron_tuning(app: &mut App) {
    if app.world().get_resource::<AssetServer>().is_none() {
        return;
    }
    app.init_ron_asset::<CombatLogTuning>()
        .add_systems(Startup, load_combat_log_tuning)
        .add_systems(
            Update,
            resolve_combat_log_tuning.run_if(
                resource_exists::<CombatLogTuningHandle>
                    .and_then(not(resource_exists::<CombatLogTuning>)),
            ),
        )
        .add_systems(Update, redrive_combat_log_tuning_on_asset_event);
}

/// Register the combat-log lifecycle + per-frame systems.
fn add_systems(app: &mut App) {
    // GTW-328: the shot-OUTCOME lines drain the presenter's per-shot `ShotImpactResolved` signal
    // (so a burst's lines appear at each staggered impact, not on the fire frame). Register its
    // buffer idempotently here so `update_combat_log`'s `MessageReader` param is always valid even
    // if this plugin builds before the presenter's renderer plugin (`bevy-traps.md` #4 — a
    // MessageReader panics validation without its buffer; `add_message` is idempotent, the
    // presenter registers the same buffer).
    app.add_message::<ShotImpactResolved>();
    app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_combat_log)
        .add_systems(OnExit(BattleScapeState::BattleRunning), despawn_combat_log)
        .add_systems(
            Update,
            // Drain the combat-event messages + append lines, gated on the live-battle witness
            // so it is inert when no battle is live (`bevy-traps.md` #1).
            update_combat_log.run_if(resource_exists::<BattleInProgress>),
        )
        // The fade + slide + height animations run UNGUARDED (each self-gates on its entities
        // existing) so a line spawned in the last BattleRunning frame still fades / settles after
        // the battle ends — the despawn on OnExit(BattleRunning) tears the whole log down anyway,
        // so they are harmless when no log exists (an empty query is a no-op). GTW-328 slice B:
        // `slide_combat_log_lines` eases each line toward its slot, `animate_combat_log_height`
        // lerps the panel height toward its content height — both read the prior frame's
        // `ComputedNode` layout (`ui_layout_system` runs in PostUpdate), a one-frame lag that is
        // imperceptible for a smooth lerp.
        .add_systems(
            Update,
            (
                fade_combat_log_lines,
                slide_combat_log_lines,
                animate_combat_log_height,
            ),
        );
}
