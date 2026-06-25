//! The five message-driven battle-lifecycle / outcome types
//! [`BattleSimPlugin`](crate::battle::BattleSimPlugin) registers — the buffered
//! [`Message`]s the app drives the sim with and the sim signals back (E10.5 /
//! GTW-207 / GTW-237).

use bevy::prelude::Message;

use crate::{rng::BattleSeed, situation::Situation};

/// A **setup-battle** trigger — build the battle from `situation`, seeding the
/// model RNG from `seed`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), carrying
/// the act's OWNED payload: the authored [`Situation`] (it is [`Clone`], so the app
/// clones its `LoadedSituation` into the trigger by value) plus the [`BattleSeed`]
/// (a [`Copy`] newtype). A [`Message`] cannot hold a borrow, so both are owned —
/// the type has **no lifetime parameter** and names NO `gdtf_app` type, keeping the
/// sim free of the one-way app dependency.
#[derive(Message, Debug, Clone)]
pub struct SetupBattleRequested {
    /// The authored battlefield to pour into the ECS world — owned by value (the app
    /// clones its `LoadedSituation`, or sends [`Situation::default`] when absent).
    pub situation: Situation,
    /// The seed the battle [`ShotRng`](crate::rng::ShotRng)/[`SeverityRng`](crate::rng::SeverityRng) is built from — threaded
    /// through [`ShotRng::from_root`](crate::rng::ShotRng::from_root), so the same seed
    /// reproduces the same draw stream.
    pub seed:      BattleSeed,
}

impl SetupBattleRequested {
    /// Build a setup-battle trigger for `situation`, seeding the RNG from `seed`.
    #[must_use]
    pub const fn new(situation: Situation, seed: BattleSeed) -> Self {
        Self { situation, seed }
    }
}

/// A **teardown-battle** trigger — remove the battle-lifetime sim resources.
///
/// A buffered [`Message`] the app sends at the battle boundary (its
/// `OnExit(GameState::BattleScape)`). Carries no payload: the teardown removes a
/// fixed set of resources ([`ShotRng`](crate::rng::ShotRng)/[`SeverityRng`](crate::rng::SeverityRng) + the four
/// [`setup_battle`](crate::situation::setup_battle)-inserted grids). A unit-payload
/// struct (not an enum / not a field) — the trigger's identity IS the signal.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TeardownBattleRequested;

/// The **battle-ready** signal — [`setup_battle`](crate::situation::setup_battle)
/// succeeded and the battle-lifetime resources are in the world.
///
/// A buffered [`Message`] the SETUP system writes on `Ok` (and NEVER on `Err` —
/// fail-closed). The app reads it to gate its state advance: it advances out of its
/// generation phase strictly AFTER the sim reports ready, and stays put on a failed
/// or absent signal. A unit-payload struct — the signal's identity IS the message.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleReady;

/// The **battle-won** signal — every enemy gang fielded at setup is out of the fight
/// (all `Dead`/`Downed`) while a player ganger still stands (GTW-237). The win condition
/// is design canon — see the "Battle outcome (win / loss)" beat in `docs/combat/combat.md`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) the
/// [`check_outcome`](crate::battle::check_outcome) census writes the first time the win
/// condition holds (then latches, so it fires at most once per battle). A zero-field unit
/// struct — its identity IS the signal, so (like
/// [`BattleReady`]/[`BattleInProgress`](crate::battle::BattleInProgress)) the no-bare-types
/// rule, which wraps domain *values*, does not apply. This is a SIM SIGNAL only: the sim
/// OWNS the buffer; it does NOT drive any `gdtf_app` state, despawn anything, end the
/// battle, or touch the presenter — the app-side consumer that ends the battle is GTW-239.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleWon;

/// The **battle-lost** signal — every player ganger fielded at setup is out of the fight
/// (all `Dead`/`Downed`), so the player gang is wiped (GTW-237). The loss condition (and
/// the mutual-wipe = loss resolution below) is design canon — see the "Battle outcome
/// (win / loss)" beat in `docs/combat/combat.md`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) the
/// [`check_outcome`](crate::battle::check_outcome) census writes the first time the loss
/// condition holds (then latches, so it fires at most once per battle). A zero-field unit
/// struct — its identity IS the signal, so (like
/// [`BattleReady`]/[`BattleInProgress`](crate::battle::BattleInProgress)) the no-bare-types
/// rule, which wraps domain *values*, does not apply. This is a SIM SIGNAL only: the sim
/// OWNS the buffer; it does NOT drive any `gdtf_app` state, despawn anything, end the
/// battle, or touch the presenter — the app-side consumer that ends the battle is GTW-239.
/// A mutual wipe (last enemy and last player fall together) resolves HERE, not as a win:
/// [`check_outcome`](crate::battle::check_outcome)'s win arm requires a surviving player,
/// so a mutual wipe is a LOSS.
#[derive(Message, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BattleLost;
