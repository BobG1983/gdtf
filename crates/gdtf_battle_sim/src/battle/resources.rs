//! The three battle-lifetime resources the lifecycle systems insert/remove: the
//! [`BattleInProgress`] gate witness (GTW-212), the [`PlayerFaction`] control seed
//! (GTW-226), and the roster-grounded [`BattleRoster`] (GTW-237).

use bevy::{
    platform::collections::HashSet,
    prelude::{Deref, Resource},
};

use crate::ganger::Faction;

/// The **battle-active** witness — present exactly while a battle is in progress, the
/// gate the [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) band's
/// bundled runtime keys off (GTW-212).
///
/// A zero-sized [`Resource`] marker (a unit struct): it carries NO domain value, so the
/// no-bare-types rule — which wraps *values* in named newtypes — does not apply; the
/// tag's identity IS the signal. The setup system inserts it on a successful
/// [`setup_battle`](crate::situation::setup_battle) (the same `Ok` path that emits
/// [`BattleReady`](crate::battle::BattleReady), NEVER on `Err`) and the teardown system
/// removes it on [`TeardownBattleRequested`](crate::battle::TeardownBattleRequested), so it
/// spans the exact battle-active window.
///
/// It is the EXPLICIT replacement for the incidental "gate on
/// [`resource_exists`](bevy::prelude::resource_exists)`::<`[`OccupancyGrid`](crate::occupancy::OccupancyGrid)`>`"
/// proxy E10.5 used: keying the
/// [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) `run_if` on a
/// purpose-built "in progress" tag decouples the gate's INTENT from any one battle
/// resource. [`OccupancyGrid`](crate::occupancy::OccupancyGrid) is still a
/// [`setup_battle`](crate::situation::setup_battle) resource (inserted on setup, removed on
/// teardown) — it is simply no longer the gate witness. Because the tag is inserted/removed
/// at the same points [`OccupancyGrid`](crate::occupancy::OccupancyGrid) effectively was,
/// the gated span — and the runtime's behavior — is unchanged; only the witness is now
/// intentional.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BattleInProgress;

/// The **player faction** — which gang the human player controls; every other
/// [`Faction`] is the enemy.
///
/// A named newtype [`Resource`] over the [`Faction`] gang index (no-bare-types:
/// it carries a domain value — which gang the player runs — so it is a real named
/// newtype over [`Faction`], NOT the bare [`Faction`] and NOT a zero-sized marker
/// like [`BattleInProgress`]). The derived [`Deref`] reads the inner [`Faction`]
/// back (never a hand-written `impl Deref`).
///
/// It is the single source of truth the later manual-play slices read: control-gating
/// lets the player act only on this gang's gangers, and the victory census polls every
/// gang that is NOT this one. Seeded data-driven from
/// [`Situation::player_faction`](crate::situation::Situation) on setup.
///
/// **Lifetime tracks [`BattleInProgress`]:** the setup system inserts it on the same
/// successful-setup `Ok` path that inserts [`BattleInProgress`] (NEVER on `Err` —
/// fail-closed) and the teardown system removes it alongside, so it is present for
/// exactly the battle-active window. Any later `Res<PlayerFaction>` reader must
/// therefore be gated on
/// [`resource_exists`](bevy::prelude::resource_exists)`::<`[`BattleInProgress`]`>` (the
/// identical window) or take `Option<Res<_>>`, or it panics when the resource is absent
/// (`bevy-traps.md` #1).
#[derive(Resource, Deref, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlayerFaction(Faction);

impl PlayerFaction {
    /// Build the player-faction resource from the gang the player controls.
    ///
    /// The public constructor (house style) so the setup can build a `PlayerFaction`
    /// from an authored [`Situation::player_faction`](crate::situation::Situation)
    /// without reaching the private field.
    #[must_use]
    pub const fn new(faction: Faction) -> Self {
        Self(faction)
    }
}

/// The **battle roster** — the set of [`Faction`]s that fielded ≥1 ganger at setup
/// (GTW-237).
///
/// A named newtype [`Resource`] (no-bare-types: it carries a domain value — which gangs
/// took the field — so it wraps the fielded-factions set in a real named type, NOT a bare
/// [`HashSet`]). The collection it owns is a [`HashSet`]`<`[`Faction`]`>` ([`Faction`] is
/// [`Copy`]/[`Eq`]/[`Hash`]). Private inner; the small accessors the census needs read it
/// back (never a hand-written `impl Deref`, because the census asks roster QUESTIONS, not
/// for the raw set).
///
/// It is the source of the "are there enemy gangers?" / "was the player fielded?"
/// EXISTENCE facts the [`check_outcome`](crate::battle::check_outcome) census splits on —
/// grounded in the gangs FIELDED at setup, NOT in a per-frame entity scan. The earlier
/// victory-only design fired on "≥1 non-player entity is in the query this frame", which
/// couples the win to dead gangers PERSISTING as entities (true today — the sim has no
/// despawn-on-death — but it would silently break if despawn is ever added). Grounding
/// existence in the roster makes the win/loss robust regardless of whether dead gangers are
/// ever despawned: the live [`LifeState`](crate::ganger::LifeState) scan answers only "is
/// any of them still up?".
///
/// **Lifetime tracks [`BattleInProgress`]:** captured on the same successful-setup `Ok`
/// path that inserts [`BattleInProgress`] + [`PlayerFaction`] (NEVER on `Err` —
/// fail-closed) and removed alongside on teardown, so it is present for exactly the
/// battle-active window. The
/// [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate)-band census
/// therefore reads its `Res<BattleRoster>` panic-free (the band is gated on
/// [`resource_exists`](bevy::prelude::resource_exists)`::<`[`BattleInProgress`]`>` — the
/// identical window; `bevy-traps.md` #1).
///
/// A faction SET, not per-faction counts — sufficient for the binary win/loss existence
/// checks. If a later procgen-roster slice (GTW-243) needs counts, it can enrich.
///
/// Grounding existence in the fielded roster (not a live scan) is part of the battle-outcome
/// canon — see the "Battle outcome (win / loss)" beat in `docs/combat/combat.md`.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct BattleRoster(HashSet<Faction>);

impl BattleRoster {
    /// Build the battle roster from the [`Faction`]s that fielded a ganger.
    ///
    /// The public constructor (house style) so the setup can build a `BattleRoster` from
    /// the situation's gangers (each [`GangerSpawn::faction`](crate::situation::GangerSpawn))
    /// without reaching the private field.
    #[must_use]
    pub fn new(factions: impl IntoIterator<Item = Faction>) -> Self {
        Self(factions.into_iter().collect())
    }

    /// Whether the `player` gang was fielded at setup — the players-fielded existence
    /// fact the loss census splits on.
    #[must_use]
    pub fn has_player(&self, player: Faction) -> bool {
        self.0.contains(&player)
    }

    /// Whether ≥1 fielded faction is NOT the `player` gang — the enemies-fielded existence
    /// fact the win census splits on. An empty enemy roster (player-only / no fielded
    /// faction) is `false`, so the census never wins on a degenerate roster.
    #[must_use]
    pub fn has_enemy_of(&self, player: Faction) -> bool {
        self.0.iter().any(|&faction| faction != player)
    }
}
