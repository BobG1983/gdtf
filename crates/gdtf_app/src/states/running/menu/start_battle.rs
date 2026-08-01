//! The start-battle navigation request — the one message every caller that starts a
//! battle writes, and the one system that applies it (GTW-742).
//!
//! Before this ticket, the menu's Battlescape button wrote
//! [`NextState<RunningState>`] directly, so the network QA channel had no way to
//! start a battle without duplicating that write. This module introduces the
//! minimal shared request: a [`StartBattleRequested`] message and its single
//! consumer [`apply_start_battle`]. The local button
//! ([`mouse_button_actions`](super::systems::mouse_button_actions) /
//! [`focus_activated_actions`](super::systems::focus_activated_actions)) writes this ONE
//! message and [`apply_start_battle`] performs the actual `Menu → Game` transition, so a
//! later `net_qa` start-battle command joins by writing the same message rather than
//! duplicating the write — mirroring how battle acts route through `PendingActIntent`.
//! Today the button and the integration suites are the only writers; the QA request that
//! used to be one went with GTW-943's cut.
//!
//! This is deliberately the minimal surface — start-battle navigation only. A
//! general menu-navigation vocabulary (every button as an enumerable request) is
//! future scope (GTW-747), not this ticket.

use bevy::prelude::*;
use gdtf_battle_sim::rng::BattleSeed;

use crate::states::RunningState;

crate::support_item! {
    /// A request to leave the main menu and begin a battle — the shared navigation
    /// message the local Battlescape button and a test drive both write.
    ///
    /// Carries an optional [`BattleSeed`]: the button always passes `None` (the battle uses
    /// the normal `resolve_root_seed` env-var / wall-clock path), and a caller pinning the
    /// procgen RNG for a reproducible run passes `Some(seed)`. `apply_start_battle` installs
    /// the pinned seed (when present) as the `Res<BattleSeed>` override the Generation
    /// slice's `request_battle_setup` already reads, then performs the state transition.
    ///
    /// The visibility flips to `pub` under `test-support` (the `test_support` ledger
    /// re-exports it, so a suite can descend the real menu → battle path by writing the same
    /// message the button writes) and `pub(crate)` otherwise.
    #[derive(Message, Debug, Clone)]
    struct StartBattleRequested {
        /// The seed to pin the battle's RNG to, or `None` for the normal
        /// `resolve_root_seed` path.
        seed: Option<BattleSeed>,
    }
}

impl StartBattleRequested {
    crate::support_item! {
        /// Build a start-battle request, pinning `seed` when the caller supplied one.
        #[must_use]
        const fn new(seed: Option<BattleSeed>) -> Self {
            Self { seed }
        }
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
/// The single consumer of the shared navigation message — every writer goes through it, and
/// this system is the ONE place the actual state transition happens, so no writer sets
/// `NextState` itself.
///
/// Registered `run_if(in_state(RunningState::Menu))` and ordered AFTER the button
/// action systems ([`super::plugin`]), so a button press writing the message in the
/// menu is applied the SAME update (bevy-traps rule 3). The message is only ever
/// written while at the menu — the button systems are menu-gated — so a menu-gated
/// consumer never misses one.
pub(in crate::states::running::menu) fn apply_start_battle(
    mut requests: MessageReader<StartBattleRequested>,
    mut next: ResMut<NextState<RunningState>>,
    mut commands: Commands,
) {
    for request in requests.read() {
        // Install the pinned seed BEFORE the descent: `request_battle_setup` reads
        // this `Res<BattleSeed>` override at OnEnter(Generation) — many frames from
        // now — and bypasses `resolve_root_seed`, so the caller's pinned seed drives
        // procgen + the per-subsystem RNG streams.
        if let Some(seed) = request.seed() {
            commands.insert_resource(seed);
        }
        next.set(RunningState::Game);
    }
}
