//! The shared attacker snapshot + stream-borrow bundles the two per-target melee
//! arms take — assembled once per request by the dispatch, handed to a resolver.

use bevy::prelude::Entity;

use crate::{
    ganger::{Facing, Faction, Fight, Luck, Position, Stance, Tu},
    melee::MeleeWeaponHit,
    rng::{FightRng, SeverityRng, ShotRng},
    weapon::DamageType,
};

/// The attacker's snapshotted gating reads + the resolved wielded-weapon view the two
/// per-target melee resolvers ([`resolve_ganger_melee`](super::ganger::resolve_ganger_melee) / [`resolve_structure_melee`](super::structure::resolve_structure_melee)) share
/// — factored out of [`dispatch_melee`](super::dispatch_melee)'s per-request loop so each arm
/// is a focused helper (GTW-508 C6 — keeping the melee dispatch under the size cap; the
/// [`strike_with_target`] split precedent).
///
/// A transparent borrow/`Copy` bundle of the already-named domain newtypes (no bare
/// primitive): the attacker's [`Position`] / [`Stance`] / [`Facing`] / [`Fight`] /
/// [`Faction`] / [`Luck`] geometry snapshot, its [`Entity`], the [`MeleeWeaponHit`]
/// borrow-view, the per-strike [`Tu`] cost, and the weapon's [`DamageType`] (the presenter
/// strike-glyph role). Assembled ONCE per request before the target branch. `pub(super)` —
/// the dispatch module assembles it and hands it to a resolver.
pub(super) struct AttackerSnapshot<'a> {
    /// The attacking ganger's [`Entity`] — the `&mut Tu` fetch target.
    pub(super) entity:             Entity,
    /// The attacker's snapshotted [`Position`] — the 8-adjacency + LOS-observer read.
    pub(super) position:           Position,
    /// The attacker's snapshotted [`Stance`] — the LOS-observer eye read.
    pub(super) stance:             Stance,
    /// The attacker's snapshotted [`Facing`] — the LOS-observer facing read.
    pub(super) facing:             Facing,
    /// The attacker's snapshotted effective [`Fight`] — the §7 `Fight_attacker`.
    pub(super) fight:              Fight,
    /// The attacker's [`Faction`] — the opposing-faction gate (ganger arm only).
    pub(super) faction:            Faction,
    /// The attacker's [`Luck`] — the §6 shooter-Luck nasty-wound term.
    pub(super) luck:               Luck,
    /// The wielded melee weapon's §5/§6 stats — the resolved [`MeleeWeaponHit`] borrow-view.
    pub(super) weapon:             MeleeWeaponHit<'a>,
    /// The primary fight-mode flat [`Tu`] cost the swing spends (saturating).
    pub(super) tu_cost:            Tu,
    /// The weapon's [`DamageType`] — the presenter [`MeleeResolved`](crate::acts::request::MeleeResolved) strike-glyph role/color.
    pub(super) strike_damage_type: DamageType,
    /// The wielded melee weapon's [`Shove`](crate::weapon::Shove) tag (GTW-525) — `true`
    /// KNOCKS BACK the target one cell on a CONNECTING strike (the auto-shove hook writes a
    /// `ShoveRequested` after the connect; a miss or a non-`shove` weapon writes nothing).
    pub(super) shove:              crate::weapon::Shove,
}

/// The three seeded draw streams the §7 / §4 / §6 ganger synthesis advances, threaded by
/// `&mut` into [`resolve_ganger_melee`](super::ganger::resolve_ganger_melee) — a transparent borrow bundle of the named stream
/// resources so the resolver's signature stays under clippy's argument-count gate (the
/// structural arm takes none — a cover-smash is RNG-free). `pub(super)` — the dispatch module
/// borrows the owned `ResMut` streams into it.
pub(super) struct MeleeStreams<'a> {
    /// The §7 opposed-Fight stream — two draws per resolve.
    pub(super) fight:    &'a mut FightRng,
    /// The §4 body-part-roll stream — one draw per resolve.
    pub(super) shot:     &'a mut ShotRng,
    /// The §6 severity-roll stream — one draw per connecting resolve.
    pub(super) severity: &'a mut SeverityRng,
}
