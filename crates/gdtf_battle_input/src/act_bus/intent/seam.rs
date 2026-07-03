//! The buffered intent queue + the ONE drain system both input surfaces feed.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    Aiming, CellLevel, Facing, Faction, PlayerFaction, Position, Stance, StanceKind,
    acts::{
        AimRequest, EndTurnRequested, EnterEmplacementRequested, ExecuteDownedRequested,
        ExitEmplacementRequested, FireRequested, MeleeRequested, MoveRequested, OpenDoorRequested,
        ReloadRequested, SetAimingRequested, SetFacingRequested, SetStanceRequested,
        ShoveRequested, StabilizeDownedRequested, ThrowGrenadeRequested,
    },
};

use crate::{
    SelectedShooter, cycle,
    intent::level::{LevelStep, step_level},
    selection::{CycleDirection, cell_order_key, cycle_player_selection},
};

/// One queued battle intent — the act a press (key OR button) asked for.
///
/// A domain enum (no-bare-types: a queued intent is a named act request, not a bare
/// discriminant). Both input surfaces [`push`](PendingActIntent::push) these; the
/// single [`dispatch_act_intents`] drain interprets them. The no-act variants
/// ([`SelectionClear`](Self::SelectionClear) / [`LevelUp`](Self::LevelUp) /
/// [`LevelDown`](Self::LevelDown)) are 222a's; the act-bearing variants
/// ([`StanceCycle`](Self::StanceCycle) / [`SetStance`](Self::SetStance) /
/// [`AimToggle`](Self::AimToggle) / [`FacingCycle`](Self::FacingCycle)) + the
/// [`Fire`](Self::Fire) variant are filled / added by 222b (GTW-227). (The fire-mode
/// `FireModeCycle` blind-cycle variant was REMOVED in GTW-254; the GTW-265 action-bar
/// then replaced the popup picker with a 3-toggle Mode sub-panel that sets
/// [`SelectedFireMode`](crate::SelectedFireMode) directly — no intent variant.)
///
/// Only [`PartialEq`] (no `Eq` / `Hash`): [`Fire`](Self::Fire) carries an owned
/// [`FireRequested`] whose [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) has `f32`
/// fields, so the enum cannot derive `Eq` / `Hash`
/// (`f32`-field-breaks-container-`Eq`-`Hash`). Nothing keys an `ActIntent` — it is
/// only pushed to / drained from a `Vec` and compared in tests — so `PartialEq`
/// suffices. Not `Copy`: [`Move`](Self::Move) / [`Turn`](Self::Turn) and
/// [`Fire`](Self::Fire) carry owned request payloads, and the enum stays `Clone`
/// (no `Copy`) to keep the variant set uniform.
#[derive(Debug, Clone, PartialEq)]
pub enum ActIntent {
    /// Clear the current ganger selection (no sim act — input-layer state only).
    SelectionClear,
    /// Raise the presenter's [`ActiveLevel`] by one storey (clamped to the top).
    LevelUp,
    /// Lower the presenter's [`ActiveLevel`] by one storey (floored at 0).
    LevelDown,
    /// Toggle the presenter's [`ViewMode`](gdtf_battle_presenter::ViewMode) between
    /// `DownToActive` (draw `0..=active`, the default) and `FullView` (draw ALL storeys —
    /// the UFO full-stack view) (GTW-521). A no-act, presenter-view intent like
    /// [`LevelUp`](Self::LevelUp) / [`LevelDown`](Self::LevelDown): it does NOT touch
    /// [`ActiveLevel`], only flips the view mode; the drain toggles the presenter-owned
    /// [`ViewMode`](gdtf_battle_presenter::ViewMode) resource, honoring the one-way
    /// `input -> presenter` edge (input never reads the presenter's internals). The bound
    /// full-view key pushes it; the terrain draw + ganger visibility re-run on the resulting
    /// [`ViewMode`](gdtf_battle_presenter::ViewMode) change.
    ToggleFullView,
    /// Step the [`SelectedShooter`]'s stance through the authored cycle — drained to
    /// [`SetStanceRequested`] (GTW-227). The KEYBOARD stance-cycle key still pushes this
    /// (a blind step through the [`crate::cycle`] order); the GTW-267 action-bar replaced
    /// its BLIND-cycle BUTTON with three direct-set toggles that push
    /// [`SetStance`](Self::SetStance) instead.
    StanceCycle,
    /// Set the [`SelectedShooter`]'s stance DIRECTLY to the carried [`StanceKind`] —
    /// drained to [`SetStanceRequested`] for that exact posture (GTW-267). The action-bar
    /// Stance 3-toggle sub-panel pushes this (Stand / Kneel / Prone each set their
    /// posture directly, NOT a cycle step). With no selection it is a no-op in the drain.
    SetStance(StanceKind),
    /// Toggle the [`SelectedShooter`]'s aim mode — drained to [`SetAimingRequested`]
    /// (GTW-227).
    AimToggle,
    /// Step the [`SelectedShooter`]'s facing through the authored cycle — drained to
    /// [`SetFacingRequested`] (GTW-227).
    FacingCycle,
    /// FIRE the carried request — drained 1:1 to a [`FireRequested`] (GTW-227). The
    /// left-click FIRE surface runs the shared `can_fire` guard at the WRITE site and
    /// only pushes this when it passes, so the drain emits the payload unconditionally.
    Fire(FireRequested),
    /// MOVE the carried request — drained 1:1 to a [`MoveRequested`] (GTW-238). The
    /// unified left-click surface pushes this when the MOVE branch wins (a player-faction
    /// selection + an empty, in-bounds, unblocked hovered cell); the drain emits the
    /// payload verbatim onto the move writer (the destination's terrain TU cost is the
    /// sim's `dispatch_move` / committed-walk concern, not this layer's).
    Move(MoveRequested),
    /// TURN the carried request — drained 1:1 to a [`SetFacingRequested`] (GTW-238). The
    /// right-click turn-to-face surface pushes this with the
    /// [`Direction`](gdtf_battle_sim::Direction) computed from the actor's cell toward
    /// the hovered cell; the drain emits it onto the SAME facing writer the
    /// [`FacingCycle`](Self::FacingCycle) intent uses (the per-45deg-step turn TU cost is
    /// the sim's facing dispatch, not this layer's).
    Turn(SetFacingRequested),
    /// RELOAD the [`SelectedShooter`]'s weapon — drained to [`ReloadRequested`] for the
    /// selection (GTW-275). The weapon panel's Reload button pushes this; the drain
    /// emits [`ReloadRequested::new(selected)`](ReloadRequested::new) ONLY when a shooter
    /// is selected (a no-op with no selection). The per-weapon `reload_tu` cost is the
    /// sim's reload dispatch, not this layer's.
    Reload,
    /// END the active team's turn — drained 1:1 to a fieldless [`EndTurnRequested`]
    /// (GTW-309). A GLOBAL turn signal like [`SelectionClear`](Self::SelectionClear) /
    /// [`LevelUp`](Self::LevelUp), NOT a per-ganger act: which team's turn is ending lives
    /// in the sim's [`ActiveFaction`](gdtf_battle_sim::ActiveFaction) resource, so it needs
    /// NO [`SelectedShooter`] and carries no payload. The action-bar's End-Turn button
    /// pushes this; the drain emits the unit [`EndTurnRequested`] unconditionally (the
    /// sim's [`dispatch_end_turn`](gdtf_battle_sim::dispatch_end_turn) advances the cycle
    /// and runs the next team's turn-start TU regen).
    EndTurn,
    /// EXECUTE the carried downed `target` — drained to [`ExecuteDownedRequested`] for the
    /// [`SelectedShooter`] as the actor (GTW-294). The carried [`Entity`] is the downed
    /// TARGET; the actor is always the selection. The drain emits
    /// [`ExecuteDownedRequested::new(actor, target)`](ExecuteDownedRequested::new) ONLY when
    /// a shooter is selected (a no-op with no selection); the sim's
    /// [`execute_downed`](gdtf_battle_sim::execute_downed) faction gate (an 8-adjacent alive
    /// ENEMY) is the authoritative check, not this layer's.
    Execute(Entity),
    /// STABILIZE the carried downed `target` — drained to [`StabilizeDownedRequested`] for
    /// the [`SelectedShooter`] as the actor (GTW-294). The carried [`Entity`] is the downed
    /// TARGET; the actor is always the selection. The drain emits
    /// [`StabilizeDownedRequested::new(actor, target)`](StabilizeDownedRequested::new) ONLY
    /// when a shooter is selected (a no-op with no selection); the sim's
    /// [`stabilize_downed`](gdtf_battle_sim::stabilize_downed) faction gate (an 8-adjacent
    /// alive ALLY) is the authoritative check, not this layer's.
    Stabilize(Entity),
    /// MELEE-strike the carried `target` — drained to [`MeleeRequested`] for the
    /// [`SelectedShooter`] as the attacker (GTW-507). The carried [`Entity`] is the opposing
    /// TARGET ganger; the attacker is always the selection. The drain emits
    /// [`MeleeRequested::new(attacker, target)`](MeleeRequested::new) ONLY when a shooter is
    /// selected (a no-op with no selection); the sim's
    /// [`dispatch_melee`](gdtf_battle_sim::dispatch_melee) gate (8-adjacent + clear LOS + an
    /// alive opposing target) is the authoritative check, not this layer's. Bound to the
    /// DEDICATED contextual MELEE button (NOT a left-click overload — the GTW-507 D1 ruling),
    /// mirroring the [`Execute`](Self::Execute) / [`Stabilize`](Self::Stabilize) contextual acts.
    Melee(Entity),
    /// MELEE-SMASH the carried adjacent STRUCTURE cell — drained to
    /// [`MeleeRequested::new_structural`] for the [`SelectedShooter`] as the attacker (GTW-508,
    /// child GTW-37d). The carried [`CellLevel`] is the adjacent Cover / Wall cell the player
    /// aimed the strike at; the attacker is always the selection. The drain emits
    /// [`MeleeRequested::new_structural(attacker, at)`](MeleeRequested::new_structural) ONLY when
    /// a shooter is selected (a no-op with no selection); the sim's
    /// [`dispatch_melee`](gdtf_battle_sim::dispatch_melee) gate (8-adjacency to the cell) is the
    /// authoritative check. The shared act-intent seam routes this structural target the SAME
    /// way it routes a ganger [`Melee`](Self::Melee) target — one melee button, two target kinds.
    /// Bound to the SAME dedicated contextual MELEE button (the GTW-508 C3 extension), which
    /// offers a structure target when no meleeable ganger is in reach but an adjacent structure
    /// is.
    MeleeStructure(CellLevel),
    /// SHOVE the carried `target` — drained to [`ShoveRequested`] for the [`SelectedShooter`] as
    /// the shover (GTW-525). The carried [`Entity`] is the opposing TARGET ganger; the shover is
    /// always the selection. The drain emits
    /// [`ShoveRequested::new(shover, target)`](ShoveRequested::new) — the DELIBERATE, gated,
    /// TU-costed form — ONLY when a shooter is selected (a no-op with no selection); the sim's
    /// [`dispatch_shove`](gdtf_battle_sim::dispatch_shove) gate (8-adjacent + opposing + alive) is
    /// the authoritative check, not this layer's. A DELIBERATE, PURE-DISPLACEMENT act available to
    /// ANY ganger (NO weapon requirement) — it does not roll a strike / deal a wound; the fall, if
    /// any, does the harm. Bound to the DEDICATED contextual SHOVE button (mirroring the
    /// [`Melee`](Self::Melee) / [`Execute`](Self::Execute) / [`Stabilize`](Self::Stabilize)
    /// contextual acts).
    Shove(Entity),
    /// OPEN the carried adjacent CLOSED `door` — drained to [`OpenDoorRequested`] for the
    /// [`SelectedShooter`] as the actor (GTW-315). The carried [`Entity`] is the openable
    /// TARGET door (a door / hatch entity, a raw Bevy handle — framework plumbing, the same
    /// bare-`Entity` payload the [`Execute`](Self::Execute) / [`Stabilize`](Self::Stabilize) /
    /// [`Melee`](Self::Melee) / [`Shove`](Self::Shove) contextual acts carry); the actor is
    /// always the selection. The drain emits
    /// [`OpenDoorRequested::new(actor, door)`](OpenDoorRequested::new) ONLY when a shooter is
    /// selected (a no-op with no selection); the sim's
    /// [`dispatch_open_door`](gdtf_battle_sim::dispatch_open_door) gate (door carries an
    /// [`OpenState`](gdtf_battle_sim::OpenState) that is CLOSED + 8-adjacent to the actor +
    /// affords the [`OpenDoorTu`](gdtf_battle_sim::tuning::OpenDoorTu) leaf) is the authoritative
    /// check, not this layer's, and the SIM spends the TU (one-way `input -> sim` boundary).
    /// F4 PLAYER-ONLY (no enemy door-open this ticket). Bound to the DEDICATED contextual
    /// Open-Door button, which offers a CLOSED door only (the button always OPENS — closing is
    /// not offered), mirroring the other contextual acts.
    OpenDoor(Entity),
    /// ENTER (man) the carried adjacent VACANT `emplacement` — drained to
    /// [`EnterEmplacementRequested`] for the [`SelectedShooter`] as the actor (GTW-543). The carried
    /// [`Entity`] is the weapon-emplacement TARGET (a terrain-piece entity, a raw Bevy handle —
    /// framework plumbing, the same bare-`Entity` payload the [`Execute`](Self::Execute) /
    /// [`OpenDoor`](Self::OpenDoor) contextual acts carry); the actor is always the selection. The
    /// drain emits
    /// [`EnterEmplacementRequested::new(actor, emplacement)`](EnterEmplacementRequested::new) ONLY
    /// when a shooter is selected (a no-op with no selection); the sim's
    /// [`dispatch_enter_emplacement`](gdtf_battle_sim::acts::dispatch_enter_emplacement) gate (the
    /// `emplacement` carries an
    /// [`EmplacementState`](gdtf_battle_sim::EmplacementState) that is VACANT + 8-adjacent to the
    /// actor + affords the [`EnterEmplacementTu`](gdtf_battle_sim::tuning::EnterEmplacementTu) leaf)
    /// is the authoritative check, not this layer's, and the SIM spends the TU (one-way
    /// `input -> sim` boundary). Bound to the DEDICATED contextual Enter button, which offers a
    /// VACANT emplacement only, mirroring the other contextual acts.
    EnterEmplacement(Entity),
    /// EXIT (dismount) the carried `emplacement` the actor is manning — drained to
    /// [`ExitEmplacementRequested`] for the [`SelectedShooter`] as the actor (GTW-543). The carried
    /// [`Entity`] is the weapon-emplacement the actor occupies; the actor is always the selection.
    /// The drain emits
    /// [`ExitEmplacementRequested::new(actor, emplacement)`](ExitEmplacementRequested::new) ONLY when
    /// a shooter is selected (a no-op with no selection); the sim's
    /// [`dispatch_exit_emplacement`](gdtf_battle_sim::acts::dispatch_exit_emplacement) gate (the
    /// `emplacement`'s recorded
    /// [`EmplacementOccupant`](gdtf_battle_sim::EmplacementOccupant) IS the actor + affords the
    /// [`ExitEmplacementTu`](gdtf_battle_sim::tuning::ExitEmplacementTu) leaf) is the authoritative
    /// check. Exit is a SEPARATE TU-costed context action — there is NO force-eject. Bound to the
    /// DEDICATED contextual Exit button, which offers ONLY the emplacement the selection occupies.
    ExitEmplacement(Entity),
    /// THROW (lob) a grenade at the carried `target` CELL — drained to [`ThrowGrenadeRequested`]
    /// for the [`SelectedShooter`] as the thrower (GTW-546, child GTW-41d). The carried
    /// [`CellLevel`] is the cell the grenade is LOBBED at — a cell AT RANGE, not an 8-adjacent
    /// entity, so it mirrors the [`MeleeStructure`](Self::MeleeStructure) `CellLevel` payload
    /// rather than the bare-`Entity` contextual acts. The thrower is always the selection. The
    /// drain emits [`ThrowGrenadeRequested::new(thrower, at)`](ThrowGrenadeRequested::new) ONLY
    /// when a shooter is selected (a no-op with no selection); the sim's
    /// [`dispatch_throw_grenade`](gdtf_battle_sim::acts::dispatch_throw_grenade) gate (the thrower
    /// wields a [`TrajectoryStyle::Arc`](gdtf_battle_sim::TrajectoryStyle) weapon with a loaded
    /// round + affords the [`ThrowTu`](gdtf_battle_sim::tuning::ThrowTu) leaf) is the authoritative
    /// check, not this layer's, and the SIM spends the TU + magazine round (one-way
    /// `input -> sim` boundary). The throw is BLIND — there is NO line-of-sight / facing / arc
    /// gate (a lob need not see its target), so no arc / turn-to-fire runs. Bound to the DEDICATED
    /// contextual Throw button, which is offered when the selection wields an `Arc` weapon and a
    /// target cell is hovered, mirroring the other contextual acts.
    ThrowGrenade(CellLevel),
    /// CYCLE the [`SelectedShooter`] to the NEXT player ganger in `(z, y, x)` order, wrapping
    /// (GTW-458). A SELECTION-layer intent (no sim act), drained directly in
    /// [`dispatch_act_intents`] via [`cycle_player_selection`]: it collects the player-faction
    /// gangers (enemies excluded), sorts them by the SAME [`cell_order_key`] the battle-start
    /// auto-select uses, finds the current selection's index, and `set_selection`s the wrapping
    /// `(i + 1) % n` neighbour. With NO selection it makes the FIRST; an EMPTY player gang is a
    /// no-op. Both the `Tab` key and the on-bar Next button push this through the ONE seam
    /// (ADR-0001 — keys + buttons share one dispatch).
    SelectNext,
    /// CYCLE the [`SelectedShooter`] to the PREVIOUS player ganger in `(z, y, x)` order,
    /// wrapping (GTW-458). The [`SelectNext`](Self::SelectNext) twin in reverse: it
    /// `set_selection`s the wrapping `(i + n - 1) % n` neighbour; with NO selection it makes
    /// the LAST; an EMPTY player gang is a no-op. `Shift+Tab` and the on-bar Prev button push
    /// this through the ONE seam.
    SelectPrev,
}

/// The shared intent QUEUE — the buffered seam both input surfaces write.
///
/// A named newtype over a `Vec<ActIntent>` (no-bare-types: a pending-intent queue is
/// a domain value; the inner `Vec` is the collection-of-domain-values carve-out),
/// owned by `gdtf_battle_input` and made `pub` so the 222b keyboard systems AND the
/// 222c `gdtf_app` buttons both reach it across the legal `gdtf_app ->
/// gdtf_battle_input` edge. `init_resource`-d by
/// [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin) (its [`Default`] is the
/// empty queue) and DRAINED every update by [`dispatch_act_intents`], so a buffered
/// intent is acted on exactly once.
#[derive(Resource, Debug, Default)]
pub struct PendingActIntent(Vec<ActIntent>);

impl PendingActIntent {
    /// Queue `intent` to be drained by [`dispatch_act_intents`] next time it runs.
    ///
    /// The single write-point both surfaces call — a key system or a `gdtf_app`
    /// button system pushes the intent the press maps to. Buffered (not applied
    /// inline) so the ONE drain system is the only place an intent takes effect.
    pub fn push(&mut self, intent: ActIntent) {
        self.0.push(intent);
    }

    /// Take and clear every queued intent — the drain's read.
    ///
    /// Returns the buffered intents in push order and leaves the queue empty, so an
    /// intent is acted on exactly once. `pub(crate)` — only the in-crate drain
    /// consumes the queue; external surfaces only [`push`](Self::push).
    pub(crate) fn drain(&mut self) -> Vec<ActIntent> {
        core::mem::take(&mut self.0)
    }

    /// Whether the queue currently holds no intents (test/inspection helper).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The `*Requested` act [`MessageWriter`]s [`dispatch_act_intents`] emits onto,
/// grouped into ONE [`SystemParam`] so the drain's parameter list stays under
/// clippy's argument-count gate (the sim's `BattleGridsParam` precedent).
///
/// Grouping the cohesive emission writers into one param keeps
/// [`dispatch_act_intents`] at five parameters; the drain arms call
/// `acts.fire.write(..)` / `acts.stance.write(..)` etc. A transparent system-param
/// bundle of framework [`MessageWriter`]s — not itself a wrapped domain scalar. The
/// [`SetFacingRequested`] writer ([`facing`](Self::facing)) is REUSED by BOTH the
/// keyboard [`ActIntent::FacingCycle`] arm and the right-click
/// [`ActIntent::Turn`] arm (one set-facing message type, one sim dispatch).
#[derive(SystemParam)]
pub struct ActWriters<'w> {
    /// The FIRE-act writer — the left-click FIRE surface's drained message.
    fire:              MessageWriter<'w, FireRequested>,
    /// The MOVE-act writer — the left-click MOVE branch's drained message (GTW-238).
    movement:          MessageWriter<'w, MoveRequested>,
    /// The set-stance writer — the stance-cycle key's drained message.
    stance:            MessageWriter<'w, SetStanceRequested>,
    /// The set-aiming writer — the aim-toggle key's drained message.
    aiming:            MessageWriter<'w, SetAimingRequested>,
    /// The set-facing writer — the facing-cycle key's AND the right-click turn-to-face
    /// surface's drained message (GTW-238 reuses it for [`ActIntent::Turn`]).
    facing:            MessageWriter<'w, SetFacingRequested>,
    /// The reload-act writer — the weapon panel Reload button's drained message
    /// (GTW-275).
    reload:            MessageWriter<'w, ReloadRequested>,
    /// The end-turn writer — the action-bar End-Turn button's drained message (GTW-309).
    /// A fieldless turn signal: the drain emits the unit [`EndTurnRequested`] verbatim.
    end_turn:          MessageWriter<'w, EndTurnRequested>,
    /// The execute-downed writer — the downed-target Execute affordance's drained message
    /// (GTW-294). Emitted only for a selected actor over the carried downed target.
    execute:           MessageWriter<'w, ExecuteDownedRequested>,
    /// The stabilize-downed writer — the downed-target Stabilize affordance's drained
    /// message (GTW-294). Emitted only for a selected actor over the carried downed target.
    stabilize:         MessageWriter<'w, StabilizeDownedRequested>,
    /// The melee writer — the contextual Melee button's drained message (GTW-507). Emitted
    /// only for a selected attacker over the carried opposing target; the sim's `dispatch_melee`
    /// gate (8-adjacent + LOS + alive + opposing) is the authoritative check.
    melee:             MessageWriter<'w, MeleeRequested>,
    /// The shove writer — the contextual Shove button's drained message (GTW-525). Emitted only
    /// for a selected shover over the carried opposing target; the sim's `dispatch_shove` gate
    /// (8-adjacent + opposing + alive) is the authoritative check.
    shove:             MessageWriter<'w, ShoveRequested>,
    /// The open-door writer — the contextual Open-Door button's drained message (GTW-315).
    /// Emitted only for a selected actor over the carried adjacent CLOSED door; the sim's
    /// `dispatch_open_door` gate (CLOSED `OpenState` + 8-adjacent + affords `OpenDoorTu`) is
    /// the authoritative check, and the SIM spends the TU (one-way `input -> sim` boundary).
    open_door:         MessageWriter<'w, OpenDoorRequested>,
    /// The enter-emplacement writer — the contextual Enter button's drained message (GTW-543).
    /// Emitted only for a selected actor over the carried adjacent VACANT emplacement; the sim's
    /// `dispatch_enter_emplacement` gate (VACANT `EmplacementState` + 8-adjacent + affords
    /// `EnterEmplacementTu`) is the authoritative check, and the SIM spends the TU (one-way
    /// `input -> sim` boundary).
    enter_emplacement: MessageWriter<'w, EnterEmplacementRequested>,
    /// The exit-emplacement writer — the contextual Exit button's drained message (GTW-543).
    /// Emitted only for a selected actor over the emplacement it occupies; the sim's
    /// `dispatch_exit_emplacement` gate (the emplacement's recorded `EmplacementOccupant` IS the
    /// actor + affords `ExitEmplacementTu`) is the authoritative check, and the SIM spends the TU
    /// (one-way `input -> sim` boundary). Exit is a SEPARATE TU-costed action (NO force-eject).
    exit_emplacement:  MessageWriter<'w, ExitEmplacementRequested>,
    /// The throw-grenade writer — the contextual Throw button's drained message (GTW-546). Emitted
    /// only for a selected thrower over the carried target CELL; the sim's `dispatch_throw_grenade`
    /// gate (wields a `TrajectoryStyle::Arc` weapon + a loaded round + affords `ThrowTu`) is the
    /// authoritative check, and the SIM spends the TU + magazine round (one-way `input -> sim`
    /// boundary). The throw is BLIND — no LOS / facing / arc gate (a lob need not see its target).
    throw:             MessageWriter<'w, ThrowGrenadeRequested>,
}

/// The READ-ONLY world the [`ActIntent::SelectNext`] / [`ActIntent::SelectPrev`] cycle arms
/// read, grouped into ONE [`SystemParam`] so [`dispatch_act_intents`] stays under clippy's
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
    fn ordered_player_gangers(&self) -> Vec<Entity> {
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

/// **Dispatch** the queued [`ActIntent`]s — the ONE drain system over the shared seam.
///
/// Drains the [`PendingActIntent`] queue every update (gated on `BattleInProgress` by
/// the plugin) and interprets each intent. The no-act intents are handled directly:
///
/// - [`ActIntent::SelectionClear`] clears [`SelectedShooter`] to `None`.
/// - [`ActIntent::LevelUp`] / [`ActIntent::LevelDown`] step [`ActiveLevel`], clamped
///   to `0..MAX_LEVELS` ([`step_level`]): up saturates at `MAX_LEVELS - 1`, down
///   floors at `0`.
/// - [`ActIntent::ToggleFullView`] flips the presenter-owned
///   [`ViewMode`](gdtf_battle_presenter::ViewMode) between `DownToActive` and `FullView`
///   (GTW-521) — it does NOT touch [`ActiveLevel`]; the presenter's terrain draw + ganger
///   visibility re-run on the [`ViewMode`](gdtf_battle_presenter::ViewMode) change.
///
/// The act-bearing intents (GTW-227) emit the matching
/// `gdtf_battle_sim::acts::*Requested` for the [`SelectedShooter`], reading the
/// actor's CURRENT [`Stance`] / [`Facing`] / [`Aiming`] off the `actors` query and
/// stepping the authored [`crate::cycle`] order:
///
/// - [`ActIntent::StanceCycle`] emits [`SetStanceRequested`] for the
///   next-of-cycle [`StanceKind`] ([`cycle::next_stance`]).
/// - [`ActIntent::SetStance`] emits [`SetStanceRequested`] for the CARRIED
///   [`StanceKind`] directly (GTW-267 — the action-bar 3-toggle stance set).
/// - [`ActIntent::AimToggle`] emits [`SetAimingRequested`] toggling the actor's
///   current [`Aiming`] flag.
/// - [`ActIntent::FacingCycle`] emits [`SetFacingRequested`] for the next-of-cycle
///   [`Direction`](gdtf_battle_sim::Direction) ([`cycle::next_facing`]).
/// - [`ActIntent::Fire`] emits the carried [`FireRequested`] verbatim — the `can_fire`
///   guard already ran at the WRITE site.
/// - [`ActIntent::Move`] emits the carried [`MoveRequested`] verbatim onto the move
///   writer (GTW-238) — the destination's terrain TU cost is the sim's move dispatch.
/// - [`ActIntent::Turn`] emits the carried [`SetFacingRequested`] verbatim onto the SAME
///   facing writer the [`FacingCycle`](ActIntent::FacingCycle) arm uses (GTW-238) — the
///   per-45deg-step turn TU cost is the sim's facing dispatch.
/// - [`ActIntent::Reload`] emits [`ReloadRequested`] for the [`SelectedShooter`]
///   (GTW-275) — a no-op with no selection; the per-weapon `reload_tu` cost is the sim's
///   reload dispatch.
/// - [`ActIntent::EndTurn`] emits the fieldless [`EndTurnRequested`] unconditionally
///   (GTW-309) — a GLOBAL turn signal needing no selection; the sim's `dispatch_end_turn`
///   advances the turn cycle and runs the next team's turn-start TU regen.
/// - [`ActIntent::Execute`] emits [`ExecuteDownedRequested`] with the [`SelectedShooter`] as
///   the actor and the carried downed target (GTW-294) — a no-op with no selection; the
///   sim's `execute_downed` faction gate (an 8-adjacent alive ENEMY) is authoritative.
/// - [`ActIntent::Stabilize`] emits [`StabilizeDownedRequested`] with the [`SelectedShooter`]
///   as the actor and the carried downed target (GTW-294) — a no-op with no selection; the
///   sim's `stabilize_downed` faction gate (an 8-adjacent alive ALLY) is authoritative.
/// - [`ActIntent::Melee`] emits [`MeleeRequested`] with the [`SelectedShooter`] as the
///   attacker and the carried opposing target (GTW-507) — a no-op with no selection; the sim's
///   `dispatch_melee` gate (8-adjacent + clear LOS + an alive opposing target) is authoritative.
/// - [`ActIntent::MeleeStructure`] emits [`MeleeRequested::new_structural`] with the
///   [`SelectedShooter`] as the attacker and the carried adjacent structure cell (GTW-508) —
///   onto the SAME melee writer; a no-op with no selection; the sim's `dispatch_melee`
///   8-adjacency-to-the-cell gate is authoritative.
/// - [`ActIntent::Shove`] emits [`ShoveRequested::new`] (the DELIBERATE form) with the
///   [`SelectedShooter`] as the shover and the carried opposing target (GTW-525) — a no-op with
///   no selection; the sim's `dispatch_shove` gate (8-adjacent + opposing + alive) is
///   authoritative.
/// - [`ActIntent::OpenDoor`] emits [`OpenDoorRequested::new`] with the [`SelectedShooter`] as
///   the actor and the carried adjacent CLOSED door (GTW-315) — a no-op with no selection; the
///   sim's `dispatch_open_door` gate (CLOSED `OpenState` + 8-adjacent + affords `OpenDoorTu`) is
///   authoritative and the SIM spends the TU (F4 player-only; the button offers CLOSED doors only).
/// - [`ActIntent::EnterEmplacement`] emits [`EnterEmplacementRequested::new`] with the
///   [`SelectedShooter`] as the actor and the carried adjacent VACANT emplacement (GTW-543) — a
///   no-op with no selection; the sim's `dispatch_enter_emplacement` gate (VACANT
///   `EmplacementState` + 8-adjacent + affords `EnterEmplacementTu`) is authoritative and the SIM
///   spends the TU.
/// - [`ActIntent::ExitEmplacement`] emits [`ExitEmplacementRequested::new`] with the
///   [`SelectedShooter`] as the actor and the carried occupied emplacement (GTW-543) — a no-op with
///   no selection; the sim's `dispatch_exit_emplacement` gate (the recorded `EmplacementOccupant` IS
///   the actor + affords `ExitEmplacementTu`) is authoritative and the SIM spends the TU (a SEPARATE
///   TU-costed action — NO force-eject).
/// - [`ActIntent::ThrowGrenade`] emits [`ThrowGrenadeRequested::new`] with the [`SelectedShooter`]
///   as the thrower and the carried target CELL (GTW-546) — a no-op with no selection; the sim's
///   `dispatch_throw_grenade` gate (the thrower wields a `TrajectoryStyle::Arc` weapon with a loaded
///   round + affords `ThrowTu`) is authoritative and the SIM spends the TU + magazine round. The
///   throw is BLIND — no LOS / facing / arc gate.
///
/// The GTW-458 SELECTION-CYCLE intents step the [`SelectedShooter`] through the player gang
/// in the shared `(z, y, x)` order ([`cell_order_key`] — the exact order the auto-select
/// picks the first of), WRAPPING, ignoring enemy gangers, and MAKING a first selection from
/// `None`:
///
/// - [`ActIntent::SelectNext`] sets the WRAPPING `(i + 1) % n` neighbour (or the FIRST from
///   no selection); an EMPTY player gang is a no-op.
/// - [`ActIntent::SelectPrev`] sets the WRAPPING `(i + n - 1) % n` neighbour (or the LAST
///   from no selection); an EMPTY player gang is a no-op.
///
/// Both are resolved directly in the drain via [`cycle_player_selection`] over the
/// [`SelectionCycleReads`] bundle, writing [`SelectedShooter`] only on a real change
/// (change-detection hygiene — the `set_selection` precedent).
///
/// With NO [`SelectedShooter`] the posture cycle intents are no-ops (nothing to act on); a
/// cycle intent for a selected entity that lacks the relevant component is skipped
/// (fail-closed, no panic) via the query lookup. Param-only (`bevy-traps.md` #7):
/// [`ResMut`] over the queue + the presenter [`ActiveLevel`] +
/// [`ViewMode`](gdtf_battle_presenter::ViewMode) (the GTW-521 full-view toggle target), a
/// read-only `actors` [`Query`], the [`ActWriters`] message-writer bundle (`bevy-traps.md`
/// #4 — buffered messages), and the [`SelectionCycleReads`] read bundle for the Prev/Next
/// cycle.
/// Registered `.after` the intent WRITERS (`bevy-traps.md` #3) so it drains the same
/// update's pushes.
pub fn dispatch_act_intents(
    mut pending: ResMut<PendingActIntent>,
    mut selected: ResMut<SelectedShooter>,
    mut active_level: ResMut<ActiveLevel>,
    mut view_mode: ResMut<ViewMode>,
    actors: Query<(&Stance, &Facing, &Aiming)>,
    mut acts: ActWriters,
    cycle_reads: SelectionCycleReads,
) {
    for intent in pending.drain() {
        match intent {
            ActIntent::SelectionClear => {
                if selected.is_some() {
                    *selected = SelectedShooter::cleared();
                }
            }
            ActIntent::LevelUp => {
                let next = step_level(**active_level, LevelStep::Up);
                if next != **active_level {
                    *active_level = ActiveLevel::new(next);
                }
            }
            ActIntent::LevelDown => {
                let next = step_level(**active_level, LevelStep::Down);
                if next != **active_level {
                    *active_level = ActiveLevel::new(next);
                }
            }
            ActIntent::ToggleFullView => {
                // GTW-521: flip the presenter-owned ViewMode between DownToActive and FullView,
                // via the presenter's own ViewMode::toggled (GTW-577 C8 — the ONE flip, shared
                // with the editor's toggle surfaces). A no-act presenter-view intent like
                // LevelUp/LevelDown — it does NOT touch ActiveLevel (C5). Assigning through the
                // ResMut marks it changed, so the presenter's terrain draw + ganger visibility
                // re-run on the flip (C3). Always a real change (the two variants differ), so no
                // change-guard is needed.
                let flipped = view_mode.toggled();
                *view_mode = flipped;
            }
            ActIntent::StanceCycle => {
                let Some(actor) = **selected else { continue };
                let Ok((stance, ..)) = actors.get(actor) else {
                    continue;
                };
                let next: StanceKind = cycle::next_stance(**stance);
                acts.stance.write(SetStanceRequested::new(actor, next));
            }
            ActIntent::SetStance(kind) => {
                // Direct set (GTW-267): emit the carried posture for the selection. No
                // need to read the current stance — the action-bar toggle named the exact
                // target posture. With no selection there is nothing to act on.
                let Some(actor) = **selected else { continue };
                acts.stance.write(SetStanceRequested::new(actor, kind));
            }
            ActIntent::AimToggle => {
                let Some(actor) = **selected else { continue };
                let Ok((_, _, aiming)) = actors.get(actor) else {
                    continue;
                };
                acts.aiming
                    .write(SetAimingRequested::new(actor, AimRequest::new(!**aiming)));
            }
            ActIntent::FacingCycle => {
                let Some(actor) = **selected else { continue };
                let Ok((_, facing, ..)) = actors.get(actor) else {
                    continue;
                };
                let next = cycle::next_facing(**facing);
                acts.facing.write(SetFacingRequested::new(actor, next));
            }
            ActIntent::Fire(request) => {
                acts.fire.write(request);
            }
            ActIntent::Move(request) => {
                acts.movement.write(request);
            }
            ActIntent::Turn(request) => {
                acts.facing.write(request);
            }
            ActIntent::Reload => {
                // Emit a reload for the selection only — mirror the SetStance arm
                // (GTW-275). With no selection there is nothing to reload.
                let Some(actor) = **selected else { continue };
                acts.reload.write(ReloadRequested::new(actor));
            }
            ActIntent::EndTurn => {
                // Emit the fieldless turn signal (GTW-309). A GLOBAL act like SelectionClear
                // / LevelUp — no selection needed; the sim's ActiveFaction tracks whose turn
                // is ending, so the drain just writes the unit message unconditionally.
                acts.end_turn.write(EndTurnRequested);
            }
            ActIntent::Execute(_)
            | ActIntent::Stabilize(_)
            | ActIntent::Melee(_)
            | ActIntent::MeleeStructure(_)
            | ActIntent::Shove(_)
            | ActIntent::OpenDoor(_)
            | ActIntent::EnterEmplacement(_)
            | ActIntent::ExitEmplacement(_)
            | ActIntent::ThrowGrenade(_) => {
                // The target-carrying contextual acts share ONE shape — resolve the selected
                // actor then write one `*Requested` (a no-op with no selection); each arm's doc
                // records the sim gate that is the authoritative check. Factored into
                // `emit_selected_act` so this drain stays under clippy's line gate.
                emit_selected_act(&intent, **selected, &mut acts);
            }
            ActIntent::SelectNext => {
                cycle_selection(&mut selected, &cycle_reads, CycleDirection::Next);
            }
            ActIntent::SelectPrev => {
                cycle_selection(&mut selected, &cycle_reads, CycleDirection::Prev);
            }
        }
    }
}

/// Emits the `*Requested` for a TARGET-carrying contextual act, using `selected` as the
/// acting ganger — a no-op when there is no selection (the fail-closed shape shared by every
/// contextual arm).
///
/// The one shape behind [`ActIntent::Execute`] / [`ActIntent::Stabilize`] /
/// [`ActIntent::Melee`] / [`ActIntent::MeleeStructure`] / [`ActIntent::Shove`] /
/// [`ActIntent::OpenDoor`] / [`ActIntent::EnterEmplacement`] / [`ActIntent::ExitEmplacement`] /
/// [`ActIntent::ThrowGrenade`]:
/// resolve the actor from the `SelectedShooter`, then write ONE act
/// message carrying `(actor, carried-target)`. Each arm's variant doc records the SIM gate that
/// is the authoritative check (this layer's offer is advisory). Factored out of
/// [`dispatch_act_intents`] so the drain stays under clippy's function-length gate; any
/// non-target-carrying intent is unreachable here and left untouched.
fn emit_selected_act(intent: &ActIntent, selected: Option<Entity>, acts: &mut ActWriters) {
    let Some(actor) = selected else { return };
    match *intent {
        // GTW-294 — an 8-adjacent alive ENEMY faction gate is the sim's authoritative check.
        ActIntent::Execute(target) => {
            acts.execute
                .write(ExecuteDownedRequested::new(actor, target));
        }
        // GTW-294 — an 8-adjacent alive ALLY faction gate is the sim's authoritative check.
        ActIntent::Stabilize(target) => {
            acts.stabilize
                .write(StabilizeDownedRequested::new(actor, target));
        }
        // GTW-507 — 8-adjacent + clear LOS + an alive opposing target is the sim's check.
        ActIntent::Melee(target) => {
            acts.melee.write(MeleeRequested::new(actor, target));
        }
        // GTW-508 — the SAME melee writer for a structure cell; 8-adjacency-to-the-cell is the check.
        ActIntent::MeleeStructure(at) => {
            acts.melee.write(MeleeRequested::new_structural(actor, at));
        }
        // GTW-525 — the DELIBERATE, TU-costed form; 8-adjacent + opposing + alive is the sim's check.
        ActIntent::Shove(target) => {
            acts.shove.write(ShoveRequested::new(actor, target));
        }
        // GTW-315 — CLOSED `OpenState` + 8-adjacent + affords `OpenDoorTu` is the sim's check.
        ActIntent::OpenDoor(door) => {
            acts.open_door.write(OpenDoorRequested::new(actor, door));
        }
        // GTW-543 — VACANT `EmplacementState` + 8-adjacent + affords `EnterEmplacementTu` is the check.
        ActIntent::EnterEmplacement(emplacement) => {
            acts.enter_emplacement
                .write(EnterEmplacementRequested::new(actor, emplacement));
        }
        // GTW-543 — the recorded `EmplacementOccupant` IS the actor + affords `ExitEmplacementTu`.
        ActIntent::ExitEmplacement(emplacement) => {
            acts.exit_emplacement
                .write(ExitEmplacementRequested::new(actor, emplacement));
        }
        // GTW-546 — wields a `TrajectoryStyle::Arc` weapon + a loaded round + affords `ThrowTu`;
        // BLIND (no LOS / facing / arc gate) — the sim re-gates authoritatively.
        ActIntent::ThrowGrenade(at) => {
            acts.throw.write(ThrowGrenadeRequested::new(actor, at));
        }
        // Not a target-carrying act — the caller only routes the nine variants above here.
        _ => {}
    }
}

/// Steps the [`SelectedShooter`] to the `direction` neighbour in the shared `(z, y, x)`
/// player-gang order (GTW-458), writing only on a real change.
///
/// Collects the deterministic player-faction order from `reads`
/// ([`SelectionCycleReads::ordered_player_gangers`]) and picks the wrapping neighbour with
/// [`cycle_player_selection`] (enemies excluded; first/last-from-`None`; empty gang →
/// no-op). Mirrors the `set_selection` change-detection hygiene — it writes the new
/// selection ONLY when it differs from the current one, so a redundant cycle on a one-ganger
/// gang does not spuriously trip `Changed<SelectedShooter>`.
fn cycle_selection(
    selected: &mut ResMut<SelectedShooter>,
    reads: &SelectionCycleReads,
    direction: CycleDirection,
) {
    let ordered = reads.ordered_player_gangers();
    // `***selected` reads the inner `Option<Entity>` through the Deref chain
    // (`&mut ResMut` → `ResMut` → `SelectedShooter` → `Option<Entity>`).
    let Some(next) = cycle_player_selection(&ordered, ***selected, direction) else {
        // Empty player gang — nothing to cycle to; leave the selection untouched.
        return;
    };
    let next = SelectedShooter::new(next);
    // `**selected` is the whole `SelectedShooter` (one deref for `&mut`, one for `ResMut`'s
    // `DerefMut`). Write only on a real change (the `set_selection` change-detection hygiene).
    if **selected != next {
        **selected = next;
    }
}
