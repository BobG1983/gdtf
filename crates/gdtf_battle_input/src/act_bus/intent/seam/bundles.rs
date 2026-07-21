//! The clippy-arg-count `SystemParam` bundles the drain consumes: the `*Requested` writers +
//! the GTW-458 selection-cycle reads.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    acts::{
        EndTurnRequested, FireRequested, MoveRequested, ReloadRequested, SetAimingRequested,
        SetFacingRequested, SetStanceRequested,
    },
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{Faction, Position},
};

use crate::{SelectedShooter, selection::cell_order_key};

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

/// The READ-ONLY world the selection intents read — the
/// [`ActIntent::SelectNext`](super::ActIntent::SelectNext) /
/// [`ActIntent::SelectPrev`](super::ActIntent::SelectPrev) cycle arms AND the GTW-735
/// direct [`ActIntent::Select`](super::ActIntent::Select) arm — grouped into ONE
/// [`SystemParam`] so [`dispatch_act_intents`](super::dispatch_act_intents) stays under clippy's
/// argument-count gate (GTW-458 — the [`ActWriters`] precedent).
///
/// Bundles the player faction the selection gates on, the read-only
/// `Query<(`[`Entity`]`, &`[`Faction`]`, &`[`Position`]`)>` over every ganger (so the drain
/// can build the deterministic player-faction cycle order on demand), and a plain
/// `Query<&`[`Faction`]`>` (so the direct-select arm can resolve ONE token's faction, mirroring
/// [`decide_left_click`](crate::decide_left_click)'s `Query<&Faction>`). Optional reads
/// (`Option<Res<PlayerFaction>>`) so the drain stays valid when no battle has inserted the
/// faction yet — the selection arms then no-op (`bevy-traps.md` #1). A transparent system-param
/// bundle, not itself a wrapped domain scalar.
#[derive(SystemParam)]
pub struct SelectionCycleReads<'w, 's> {
    /// The faction the player controls — the selection considers ONLY gangers whose own
    /// [`Faction`] equals this (enemies excluded). `Option` so the arm no-ops pre-battle.
    player:   Option<Res<'w, PlayerFaction>>,
    /// Every ganger's `(`[`Entity`]`, &`[`Faction`]`, &`[`Position`]`)` — read-only, the
    /// cycle filters to the player faction and sorts by [`cell_order_key`].
    gangers:  Query<'w, 's, (Entity, &'static Faction, &'static Position)>,
    /// Every ganger's `&`[`Faction`] keyed by [`Entity`] — read-only, the direct-select
    /// [`ActIntent::Select`](super::ActIntent::Select) gate resolves ONE token's faction through
    /// it (mirroring [`decide_left_click`](crate::decide_left_click)'s `Query<&Faction>`). A
    /// second IMMUTABLE `Faction` read alongside `gangers` (read-read never conflicts, so no
    /// B0001), so the token lookup does not require the target to also carry a [`Position`].
    factions: Query<'w, 's, &'static Faction>,
    /// Every ganger's `&`[`LifeState`] keyed by [`Entity`] — read-only, the GTW-729 Alive gate
    /// BOTH selection paths share: the direct-select
    /// [`ActIntent::Select`](super::ActIntent::Select) refuses a non-Alive token, and the
    /// Prev/Next cycle SKIPS non-Alive gangers (they never enter the ordered gang). A third
    /// IMMUTABLE read alongside `gangers` / `factions` (read-read never conflicts, so no B0001).
    lifes:    Query<'w, 's, &'static LifeState>,
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
            // `**faction` reads the ganger's `Faction` through `&&Faction`. GTW-729: SKIP any
            // ganger that is not Alive — a Downed / Dead ganger never enters the cycle order, so
            // Prev/Next steps straight past it to the next Alive one (never landing on it).
            .filter(|(entity, faction, _)| **faction == player_faction && self.is_alive(*entity))
            .map(|(entity, _, position)| (entity, *position))
            .collect();
        ordered.sort_by_key(|(_, position)| cell_order_key(position));
        ordered.into_iter().map(|(entity, _)| entity).collect()
    }

    /// Resolves a DIRECT actor-selection token ([`ActIntent::Select`](super::ActIntent::Select))
    /// to the [`SelectedShooter`] to write, applying the SAME player-faction gate
    /// [`decide_left_click`](crate::decide_left_click)'s SELECT clause enforces
    /// (`faction == player`) — a faithful mirror, NOT a variant of it.
    ///
    /// Returns `Some(SelectedShooter::new(entity))` iff `entity` is a LIVE ganger whose
    /// [`Faction`] equals the [`PlayerFaction`], else `None` — FAIL-CLOSED: a dead / despawned
    /// token (the `factions` query lookup errors), a non-ganger entity (no [`Faction`]), an
    /// ENEMY-faction ganger, or a pre-battle absent [`PlayerFaction`] all refuse the selection,
    /// with NO panic (the deny-lints forbid `unwrap`/`expect`). The caller (the drain) then
    /// writes the returned selection only on a real change (the `set_selection`
    /// change-detection hygiene the cycle arms share).
    pub(super) fn select_target(&self, entity: Entity) -> Option<SelectedShooter> {
        // `***player`: `&Res` → `Res<PlayerFaction>` → `PlayerFaction` → `Faction` (its inner).
        let player_faction: Faction = ***self.player.as_ref()?;
        // A dead / despawned token (or a non-ganger without `Faction`) errors the lookup → None.
        let faction = self.factions.get(entity).ok()?;
        // GTW-729: a Downed / Dead ganger is NEVER a selectable ACTOR — refuse it even if it is a
        // player-faction ganger (it stays a stabilize / execute TARGET, never the SelectedShooter).
        // The SAME `faction == player` gate `decide_left_click`'s SELECT clause enforces, PLUS the
        // Alive gate.
        (*faction == player_faction && self.is_alive(entity)).then(|| SelectedShooter::new(entity))
    }

    /// Whether `entity` is a SELECTABLE ACTOR by life-state (GTW-729): it is
    /// [`Alive`](LifeState::Alive), or carries no [`LifeState`] at all.
    ///
    /// FAIL-OPEN on an absent [`LifeState`]: a real ganger ALWAYS carries one (seeded at spawn,
    /// defaulting to [`Alive`](LifeState::Alive)), and a Downed / Dead ganger ALWAYS carries a
    /// non-Alive [`LifeState`] — so a missing component can only mean Alive (a focused test
    /// fixture spawning a bare faction marker). Never a panic: a query miss reads as selectable.
    fn is_alive(&self, entity: Entity) -> bool {
        match self.lifes.get(entity) {
            Ok(life) => *life.is_active(),
            Err(_) => true,
        }
    }
}
