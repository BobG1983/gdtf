//! The start-battle navigation request — the one message BOTH the menu's
//! Battlescape button and the dev-only `net_qa` network consumer produce, and the
//! one system that applies it (GTW-742).
//!
//! Before this ticket, the menu's Battlescape button wrote
//! [`NextState<RunningState>`] directly, so the network QA channel had no way to
//! start a battle without duplicating that write. This module introduces the
//! minimal shared request: a [`StartBattleRequested`] message and its single
//! consumer [`apply_start_battle`]. Both the local button
//! ([`mouse_button_actions`](super::systems::mouse_button_actions) /
//! [`focus_activated_actions`](super::systems::focus_activated_actions)) and the
//! network path
//! (`crate::dev::net_qa::start_battle::drive_start_battle`) produce this ONE
//! message, and [`apply_start_battle`] performs the actual `Menu → Game`
//! transition — so both paths share one truth, mirroring how battle acts route
//! through `PendingActIntent` rather than raw writes.
//!
//! This is deliberately the minimal surface — start-battle navigation only. A
//! general menu-navigation vocabulary (every button as an enumerable request) is
//! future scope (GTW-747), not this ticket.

use bevy::prelude::*;
use gdtf_battle_sim::rng::BattleSeed;

use crate::states::RunningState;

/// A request to leave the main menu and begin a battle — the shared navigation
/// message the local Battlescape button and the network QA consumer both write.
///
/// Carries an optional [`BattleSeed`]: the local button always passes `None` (the
/// battle uses the normal `resolve_root_seed` env-var / wall-clock path), while the
/// network `StartBattle` path passes `Some(seed)` to pin the procgen RNG for a
/// reproducible QA run. [`apply_start_battle`] installs the pinned seed (when
/// present) as the `Res<BattleSeed>` override the Generation slice's
/// `request_battle_setup` already reads, then performs the state transition.
///
/// `pub(crate)` so the dev-only `net_qa` consumer can write the SAME message the
/// button writes; a build without `net_qa` still uses it end-to-end (the button
/// writes it, [`apply_start_battle`] reads it).
#[derive(Message, Debug, Clone)]
pub(crate) struct StartBattleRequested {
    /// The seed to pin the battle's RNG to, or `None` for the normal
    /// `resolve_root_seed` path.
    seed: Option<BattleSeed>,
}

impl StartBattleRequested {
    /// Build a start-battle request, pinning `seed` when the caller supplied one.
    pub(crate) const fn new(seed: Option<BattleSeed>) -> Self {
        Self { seed }
    }

    /// The pinned seed, if any ([`BattleSeed`] is `Copy`, so this reads without
    /// consuming).
    pub(crate) const fn seed(&self) -> Option<BattleSeed> {
        self.seed
    }
}

/// Applies every [`StartBattleRequested`]: install the pinned [`BattleSeed`]
/// override (when present) and request the `Menu → Game` transition (GTW-742).
///
/// The single consumer of the shared navigation message — the local button and the
/// network QA path both write it, and this system is the ONE place the actual
/// state transition happens, so neither path writes `NextState` itself.
///
/// Registered `run_if(in_state(RunningState::Menu))` and ordered AFTER the button
/// action systems ([`super::plugin`]), so a button press writing the message in the
/// menu is applied the SAME update (bevy-traps rule 3). The message is only ever
/// written while at the menu (the button systems are menu-gated; the network
/// consumer accepts a `StartBattle` only at the menu), so a menu-gated consumer
/// never misses one.
pub(in crate::states::running::menu) fn apply_start_battle(
    mut requests: MessageReader<StartBattleRequested>,
    mut next: ResMut<NextState<RunningState>>,
    mut commands: Commands,
) {
    for request in requests.read() {
        // Install the pinned seed BEFORE the descent: `request_battle_setup` reads
        // this `Res<BattleSeed>` override at OnEnter(Generation) — many frames from
        // now — and bypasses `resolve_root_seed`, so the QA-requested seed drives
        // procgen + the per-subsystem RNG streams.
        if let Some(seed) = request.seed() {
            commands.insert_resource(seed);
        }
        next.set(RunningState::Game);
    }
}
