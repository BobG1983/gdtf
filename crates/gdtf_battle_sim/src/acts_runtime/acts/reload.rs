//! The **reload** dispatch — drain each buffered [`ReloadRequested`] message and
//! refill the actor's magazine, charging the weapon's per-weapon reload TU cost
//! (GTW-275).
//!
//! The real, TU-costed `ReloadAct`: the cost is the actor's OWN
//! [`Magazine::reload_tu`](crate::magazine::Magazine::reload_tu) (a per-WEAPON number
//! authored in the weapon `.ron`, USER DIRECTIVE 2026-06-17 — NOT a global tuning
//! leaf). It mirrors the [`set_stance`](crate::posture::set_stance) /
//! [`set_facing`](crate::posture::set_facing) pattern: GATE first (alive +
//! affordable), then [`spend_tu`](crate::tu::spend_tu) and
//! [`refill`](crate::magazine::Magazine::refill). It fetches the actor's components via
//! a Bevy query (`bevy-traps.md` #7 — no `&mut World`).

use bevy::{
    ecs::query::With,
    prelude::{Entity, Message, MessageReader, MessageWriter, Query},
};

use crate::{
    acts::request::ReloadRequested,
    ganger::{LifeState, Tu},
    magazine::Magazine,
    tu::{can_spend_tu, spend_tu},
    weapon::{WieldedBy, Wields},
};

/// The user-facing OUTCOME of a single [`dispatch_reload`] of one
/// [`ReloadRequested`] message — the three real, presenter-visible branches
/// (GTW-312).
///
/// A `Copy` enum (a domain value, not a bare type — no-bare-types rule): the
/// presenter classifies a [`ReloadResult`] by this variant to pop the matching
/// floating-combat-text. It deliberately has NO "Empty" / "out of ammo" variant:
/// the GTW-275 reload model carries no ammo reserve
/// ([`Magazine::refill`](crate::magazine::Magazine::refill) always succeeds, mags
/// spawn full), so an empty-reserve outcome cannot occur.
///
/// The internal guard branches (a not-[`Alive`](crate::ganger::LifeState::Alive)
/// actor, a missing queried component) emit NO [`ReloadResult`] at all — a
/// dead / absent ganger never issues a reload intent — so they have no variant
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadOutcome {
    /// The success branch — the actor could afford the magazine's `reload_tu`, so
    /// it spent the TU and refilled the magazine to full.
    Reloaded,
    /// The already-full no-op branch — the magazine was already full, so the act
    /// was a no-op and charged NO TU (the FLAGGED GTW-275 "redundant act = no
    /// charge" choice; see [`dispatch_reload`]).
    AlreadyFull,
    /// The can't-afford branch — the actor's [`Tu`] could not pay the magazine's
    /// own [`reload_tu`](crate::magazine::Magazine::reload_tu), so the reload was
    /// silently rejected: no TU spent, no refill.
    NoTu,
}

/// A reload act RESOLVED — its `actor` and the [`ReloadOutcome`] that befell it
/// (GTW-312).
///
/// Emitted by [`dispatch_reload`] exactly once per drained [`ReloadRequested`]
/// that reaches one of the three real outcome branches (NOT the internal guard
/// skips). The presenter (and any reactive sim system) reads this to react to the
/// reload moment — mirroring the [`ArmorBroken`](crate::armor_wear::ArmorBroken)
/// presenter-visible signal.
///
/// A buffered Bevy **message** (`#[derive(Message)]`), NOT the observer `Event`
/// API (`bevy-traps.md` #4: Bevy 0.18 renamed buffered `Event`/`EventReader` to
/// `Message`/`MessageReader`), so it is written with
/// [`bevy::prelude::MessageWriter`] and read with
/// [`bevy::prelude::MessageReader`]. The payload is named — [`ReloadOutcome`] is a
/// domain enum (no-bare-types); [`Entity`] is Bevy framework plumbing (the only
/// bare type the no-bare-types rule permits — an entity handle, not a domain
/// value).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReloadResult {
    /// The ganger whose reload was resolved — the actor that issued the intent.
    pub actor:   Entity,
    /// Which of the three real reload outcomes befell the `actor`.
    pub outcome: ReloadOutcome,
}

impl ReloadResult {
    /// Build a reload-result signal for `actor`'s reload that resolved to
    /// `outcome`.
    #[must_use]
    pub const fn new(actor: Entity, outcome: ReloadOutcome) -> Self {
        Self { actor, outcome }
    }
}

/// **Dispatch** buffered [`ReloadRequested`] messages — drain each and reload the
/// actor's magazine, charging the weapon's per-weapon reload TU cost (GTW-275).
///
/// Queries the actor's `(&mut `[`Magazine`]`, &mut `[`Tu`]`, &`[`LifeState`]`)` and:
///
/// 1. GATES like the other acts — skips (silently, no panic) when the actor is not
///    [`Alive`](crate::ganger::LifeState::Alive), or when its [`Tu`] cannot afford the
///    magazine's own [`reload_tu`](crate::magazine::Magazine::reload_tu)
///    ([`can_spend_tu`]); the rejection is silent, matching the fire / move / stance
///    acts.
/// 2. On success, [`spend_tu`]s the magazine's `reload_tu` then
///    [`refill`](crate::magazine::Magazine::refill)s the magazine to full
///    (`loaded == size`).
///
/// **Already-full behavior (engineer's call, FLAGGED, GTW-275 AC3): a reload of an
/// already-FULL magazine is a NO-OP — no TU is charged.** This mirrors the codebase's
/// "redundant act = no charge" convention ([`set_stance`](crate::posture::set_stance)
/// charges only on a real stance change; [`set_facing`](crate::posture::set_facing)
/// only on a real turn) and is the kinder UX (a misclick on a full magazine is free).
/// The ONLY designed cost is the `reload_tu` spend — no turn-ending semantics are
/// invented.
///
/// REUSES the magazine grouping's own primitives. A message for an actor missing any
/// queried component is skipped (fail-closed, no panic — the `dispatch_set_*`
/// precedent).
///
/// Since GTW-323 slice 2 (ADR-0004) the actor's [`Magazine`] lives on the related
/// **weapon entity**, not the ganger — so this resolves `ganger → Wields → the weapon
/// entity` and reloads the weapon entity's magazine. The ganger's `(&mut `[`Tu`]`,
/// &`[`LifeState`]`, &`[`Wields`]`)` are read off the `actors` query (gangers); the
/// `&mut `[`Magazine`] off the disjoint `weapons` query (`With<`[`WieldedBy`]`>`, the
/// weapon entities). A shooter wielding no weapon — or whose weapon entity is not in the
/// weapon query — is skipped (fail-closed).
pub fn dispatch_reload(
    mut requests: MessageReader<ReloadRequested>,
    mut actors: Query<(&'static mut Tu, &'static LifeState, &'static Wields)>,
    mut weapons: Query<&'static mut Magazine, With<WieldedBy>>,
    mut results: MessageWriter<ReloadResult>,
) {
    for request in requests.read() {
        let Ok((mut tu, &life, wields)) = actors.get_mut(request.actor) else {
            continue;
        };

        // GATE: alive only (a Downed/Dead ganger cannot reload), mirroring the firing
        // act's liveness gate. Internal skip — emits NO ReloadResult (a dead/absent
        // ganger never issues a reload intent).
        if life != LifeState::Alive {
            continue;
        }

        // GTW-323 slice 2: resolve the actor's weapon entity (`ganger → Wields → the
        // weapon entity`) and fetch its magazine. A ganger wielding no weapon (or whose
        // weapon entity is absent from the weapon query) is skipped silently — no
        // ReloadResult (it could not have issued a reload intent).
        let Some(weapon_entity) = wields.weapon() else {
            continue;
        };
        let Ok(mut magazine) = weapons.get_mut(weapon_entity) else {
            continue;
        };

        // Already-full reload is a no-op — no charge (FLAGGED choice; see the fn doc).
        if magazine.is_full() {
            results.write(ReloadResult::new(request.actor, ReloadOutcome::AlreadyFull));
            continue;
        }

        // GATE: affordable — the per-weapon reload_tu must be payable (silent reject
        // when short, matching fire/move/stance).
        let cost = Tu::new(*magazine.reload_tu());
        if !can_spend_tu(&tu, cost) {
            results.write(ReloadResult::new(request.actor, ReloadOutcome::NoTu));
            continue;
        }

        // SUCCESS: charge the reload_tu, then refill the magazine to full.
        spend_tu(&mut tu, cost);
        magazine.refill();
        results.write(ReloadResult::new(request.actor, ReloadOutcome::Reloaded));
    }
}
