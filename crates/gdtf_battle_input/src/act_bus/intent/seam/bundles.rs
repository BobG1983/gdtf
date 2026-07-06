//! The clippy-arg-count `SystemParam` bundles the drain consumes: the `*Requested` writers +
//! the GTW-458 selection-cycle reads.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    acts::{
        EndTurnRequested, FireRequested, MoveRequested, ReloadRequested, SetAimingRequested,
        SetFacingRequested, SetStanceRequested,
    },
    battle::PlayerFaction,
    prelude::{Faction, Position},
};

use crate::selection::cell_order_key;

/// The `*Requested` act [`MessageWriter`]s [`dispatch_act_intents`](super::dispatch_act_intents) emits onto,
/// grouped into ONE [`SystemParam`] so the drain's parameter list stays under
/// clippy's argument-count gate (the sim's `BattleGridsParam` precedent).
///
/// Grouping the cohesive emission writers into one param keeps
/// [`dispatch_act_intents`](super::dispatch_act_intents) at five parameters; the drain arms call
/// `acts.fire.write(..)` / `acts.stance.write(..)` etc. A transparent system-param
/// bundle of framework [`MessageWriter`]s — not itself a wrapped domain scalar. The
/// CONTEXTUAL acts' writers are NOT here — each per-act generic drain in
/// [`crate::contextual`] holds its own `MessageWriter<A::Requested>` (GTW-571), so
/// adding a contextual act never widens this bundle. The
/// [`SetFacingRequested`] writer ([`facing`](Self::facing)) is REUSED by BOTH the
/// keyboard [`ActIntent::FacingCycle`](super::ActIntent::FacingCycle) arm and the right-click
/// [`ActIntent::Turn`](super::ActIntent::Turn) arm (one set-facing message type, one sim dispatch).
#[derive(SystemParam)]
pub struct ActWriters<'w> {
    /// The FIRE-act writer — the left-click FIRE surface's drained message.
    pub(super) fire:     MessageWriter<'w, FireRequested>,
    /// The MOVE-act writer — the left-click MOVE branch's drained message (GTW-238).
    pub(super) movement: MessageWriter<'w, MoveRequested>,
    /// The set-stance writer — the stance-cycle key's drained message.
    pub(super) stance:   MessageWriter<'w, SetStanceRequested>,
    /// The set-aiming writer — the aim-toggle key's drained message.
    pub(super) aiming:   MessageWriter<'w, SetAimingRequested>,
    /// The set-facing writer — the facing-cycle key's AND the right-click turn-to-face
    /// surface's drained message (GTW-238 reuses it for [`ActIntent::Turn`](super::ActIntent::Turn)).
    pub(super) facing:   MessageWriter<'w, SetFacingRequested>,
    /// The reload-act writer — the weapon panel Reload button's drained message
    /// (GTW-275).
    pub(super) reload:   MessageWriter<'w, ReloadRequested>,
    /// The end-turn writer — the action-bar End-Turn button's drained message (GTW-309).
    /// A fieldless turn signal: the drain emits the unit [`EndTurnRequested`] verbatim.
    pub(super) end_turn: MessageWriter<'w, EndTurnRequested>,
}

/// The READ-ONLY world the [`ActIntent::SelectNext`](super::ActIntent::SelectNext) / [`ActIntent::SelectPrev`](super::ActIntent::SelectPrev) cycle arms
/// read, grouped into ONE [`SystemParam`] so [`dispatch_act_intents`](super::dispatch_act_intents) stays under clippy's
/// argument-count gate (GTW-458 — the [`ActWriters`] precedent).
///
/// Bundles the player faction the cycle gates on and the read-only
/// `Query<(`[`Entity`]`, &`[`Faction`]`, &`[`Position`]`)>` over every ganger, so the drain
/// can build the deterministic player-faction order on demand. Optional reads
/// (`Option<Res<PlayerFaction>>`) so the drain stays valid when no battle has inserted the
/// faction yet — the cycle arms then no-op (`bevy-traps.md` #1). A transparent system-param
/// bundle, not itself a wrapped domain scalar.
#[derive(SystemParam)]
pub struct SelectionCycleReads<'w, 's> {
    /// The faction the player controls — the cycle considers ONLY gangers whose own
    /// [`Faction`] equals this (enemies excluded). `Option` so the arm no-ops pre-battle.
    player:  Option<Res<'w, PlayerFaction>>,
    /// Every ganger's `(`[`Entity`]`, &`[`Faction`]`, &`[`Position`]`)` — read-only, the
    /// cycle filters to the player faction and sorts by [`cell_order_key`].
    gangers: Query<'w, 's, (Entity, &'static Faction, &'static Position)>,
}

impl SelectionCycleReads<'_, '_> {
    /// The player-faction gangers SORTED ascending by the deterministic [`cell_order_key`]
    /// `(z, y, x)` order — the exact order the battle-start auto-select picks the first of, so
    /// `Next`/`Prev` step ONE shared order (GTW-458). Enemies are excluded. Returns an empty
    /// vec when the player faction is absent (pre-battle) or the player has no gangers.
    pub(super) fn ordered_player_gangers(&self) -> Vec<Entity> {
        let Some(player) = self.player.as_ref() else {
            return Vec::new();
        };
        // `***player`: `&Res` → `Res<PlayerFaction>` → `PlayerFaction` → `Faction` (its inner).
        let player_faction: Faction = ***player;
        let mut ordered: Vec<(Entity, Position)> = self
            .gangers
            .iter()
            // `**faction` reads the ganger's `Faction` through `&&Faction`.
            .filter(|(_, faction, _)| **faction == player_faction)
            .map(|(entity, _, position)| (entity, *position))
            .collect();
        ordered.sort_by_key(|(_, position)| cell_order_key(position));
        ordered.into_iter().map(|(entity, _)| entity).collect()
    }
}
