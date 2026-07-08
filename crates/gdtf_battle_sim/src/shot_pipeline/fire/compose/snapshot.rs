//! The shooter read — snapshot the shooter's `Copy` state + economy reads off the
//! queries, before [`fire`](super::super::fire)'s burst loop takes its mutable
//! re-borrows.

use bevy::prelude::Entity;

use super::super::query::{MeleeQuery, MountedQuery, ShooterQuery, WeaponQuery, WieldsQuery};
use crate::{
    aim::Shooter,
    effects::attachments::WeaponBraceBonus,
    ganger::{Aiming, Facing, Luck, Position, Shooting, Stance, Suppressed, Tu, TuMax},
    injuries::{HandsAvailable, InflictedInjuries},
    magazine::Magazine,
    stability::EmplacementStability,
    tuning::CombatTuning,
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, Handedness, Kickback, Stable, WeaponDamage,
        WeaponPunch, WeaponShred, WeaponStats,
    },
};

/// The shooter's `Copy` read state, snapshotted **before** the burst loop so the
/// shooter query is only re-borrowed (mutably, for the `Magazine` decrement) one
/// round at a time.
///
/// Reading every `Copy` shooter stat into one local up front releases the immutable
/// borrow of the shooter query, so the loop's `shooters.get_mut(shooter)` (the
/// per-round magazine decrement) does not overlap a live read borrow — the
/// bevy-expert's borrow discipline for the two-query design. Every field is an
/// owned named domain value (no bare primitive); it is an internal call-site
/// snapshot, not a wrapped domain scalar.
pub(in crate::shot_pipeline::fire) struct ShooterSnapshot {
    pub(super) position:    Position,
    pub(super) facing:      Facing,
    pub(super) stance:      Stance,
    pub(super) aiming:      Aiming,
    pub(super) shooting:    Shooting,
    pub(super) luck:        Luck,
    pub(super) base_spread: BaseSpread,
    pub(super) accuracy:    Accuracy,
    pub(super) kickback:    Kickback,
    pub(super) fatal_bias:  FatalBias,
    pub(super) damage:      WeaponDamage,
    pub(super) punch:       WeaponPunch,
    pub(super) shred:       WeaponShred,
    pub(super) damage_type: DamageType,
    pub(super) stable:      Stable,
    // GTW-549: the weapon's optional per-item WeaponBraceBonus attachment, snapshotted so
    // cone_for / stability_for fold the additive graduated brace bonus into the burst's
    // dispersion cone. `None` = no brace attachment = the zero-identity brace term.
    // `WeaponBraceBonus` is `Copy`, so the snapshot owns it and `weapon_stats` hands out an
    // `Option<&WeaponBraceBonus>` borrow. SUPERSEDES the GTW-542 Scoped / sight_bonus fields.
    pub(super) brace_bonus: Option<WeaponBraceBonus>,
    // GTW-526: the shooter's Suppressed state, snapshotted so cone_for / stability_for
    // widen the burst's dispersion cone while the shooter is pinned. `None` = un-suppressed
    // = the zero-identity suppression term. `Suppressed` is `Copy`, so the snapshot owns it
    // and `shooter_view` hands out an `Option<&Suppressed>` borrow.
    pub(super) suppressed:  Option<Suppressed>,
    // GTW-543: whether the shooter's RESOLVED ranged weapon is a MountedWeapon — `true` when the
    // ganger is MANNING an emplacement and firing its bolted-down gun. Feeds the emplacement
    // stability seam (cone_for / the recoil-recompute stability_for), steadying the deliberately
    // inaccurate mount. `false` (an un-mounted / normal shot) resolves the zero-identity
    // EmplacementStability, so the shot is byte-identical to before the seam engaged.
    pub(super) mounted:     bool,
    // GTW-544: the weapon's optional DamageProfile-over-time (`DotProfile`), snapshotted so
    // `weapon_stats` hands out its `Option<&DotProfile>` borrow to the fold — a penetrating hit
    // from a DOT weapon attaches a `Dot` on the struck ganger. `None` = a non-DOT weapon (no
    // attach, byte-identical). `DotProfile` is `Copy`, so the snapshot owns it.
    pub(super) dot:         Option<crate::weapon::DotProfile>,
}

impl ShooterSnapshot {
    /// Assemble a transient [`WeaponStats`] borrow-view over this snapshot's weapon
    /// stats — the read-shape the §1/§6 readers ([`cone_for`](crate::aim::cone_for) / [`resolve_and_apply`](crate::resolve_and_apply::resolve_and_apply))
    /// take, built from the snapshotted components (the query-based equivalent of
    /// [`WeaponBundle::stats`](crate::weapon::WeaponBundle::stats)).
    pub(super) const fn weapon_stats(&self) -> WeaponStats<'_> {
        WeaponStats {
            base_spread: &self.base_spread,
            accuracy:    &self.accuracy,
            kickback:    &self.kickback,
            fatal_bias:  &self.fatal_bias,
            damage:      &self.damage,
            punch:       &self.punch,
            shred:       &self.shred,
            damage_type: &self.damage_type,
            stable:      &self.stable,
            // GTW-549: borrow the snapshotted per-item brace bonus (None = no brace attachment).
            brace_bonus: self.brace_bonus.as_ref(),
            // GTW-544: borrow the snapshotted DOT profile (None = a non-DOT weapon).
            dot:         self.dot.as_ref(),
        }
    }

    /// Assemble a transient [`Shooter`] borrow-view over this snapshot's ganger state
    /// — the read-shape [`cone_for`](crate::aim::cone_for) / [`stability_for`](crate::aim::stability_for) take.
    pub(super) const fn shooter_view(&self) -> Shooter<'_> {
        Shooter {
            stance:     &self.stance,
            aiming:     &self.aiming,
            position:   &self.position,
            facing:     &self.facing,
            // GTW-526: borrow the snapshotted Suppressed state (None = un-suppressed).
            suppressed: self.suppressed.as_ref(),
        }
    }

    /// The GTW-543 emplacement stability term for this shot — the tunable
    /// [`EmplacementStabilityBonus`](crate::tuning::EmplacementStabilityBonus) when the shooter's
    /// resolved ranged weapon is a [`MountedWeapon`](crate::weapon::MountedWeapon) (the ganger is
    /// MANNING an emplacement), else [`EmplacementStability::none`] (the zero identity).
    ///
    /// Resolved ONCE here so the two burst-round reads — [`cone_for`](crate::aim::cone_for) (cone width) and the
    /// recoil-recompute [`stability_for`](crate::aim::stability_for) — feed the SAME emplacement term, keeping the cone and
    /// its recoil damping consistent within a round (the GTW-542 `sight_stability` shared-term
    /// precedent). An un-mounted shot resolves the zero identity, so its stability score — and
    /// thus its cone — is byte-identical to before the seam engaged (the pure-additive property).
    pub(super) fn emplacement_stability(&self, tuning: &CombatTuning) -> EmplacementStability {
        if self.mounted {
            EmplacementStability::new(*tuning.cone_stability.emplacement_stability_bonus)
        } else {
            EmplacementStability::none()
        }
    }
}

/// The shooter read result — the `Copy` [`ShooterSnapshot`], the wielded
/// [`weapon`](ShooterReads::weapon) entity (whose [`Magazine`] the burst decrements),
/// and the [`FireActor`](crate::magazine::FireActor)-shaping economy reads.
///
/// Bundles the values [`read_shooter`] hands back so [`fire`](super::super::fire) can validate
/// the act (the `(Tu, TuMax, Aiming, Magazine)` economy), charge the up-front TU, and —
/// across the burst loop — re-borrow the weapon entity to spend a round per fired
/// iteration. Every field is an owned named domain value or a Bevy [`Entity`] handle
/// (framework plumbing); the bundle itself is a transparent call-site record, not a
/// wrapped domain scalar.
pub(in crate::shot_pipeline::fire) struct ShooterReads {
    /// The shooter's `Copy` ganger-state + weapon-stat snapshot.
    pub(in crate::shot_pipeline::fire) snapshot:        ShooterSnapshot,
    /// The wielded weapon entity — the burst re-borrows its [`Magazine`] per round.
    pub(in crate::shot_pipeline::fire) weapon:          Entity,
    /// The shooter's current TU pool ([`can_fire`](crate::magazine::can_fire) reads it).
    pub(in crate::shot_pipeline::fire) tu:              Tu,
    /// The shooter's TU ceiling ([`mode_tu_cost`](crate::magazine::mode_tu_cost) reads it).
    pub(in crate::shot_pipeline::fire) tu_max:          TuMax,
    /// Whether the shooter is aiming (the ×1.5 TU premium toggle).
    pub(in crate::shot_pipeline::fire) aiming:          Aiming,
    /// The weapon's ammo state, snapshotted for the affordability / burst-clamp reads.
    pub(in crate::shot_pipeline::fire) magazine:        Magazine,
    /// The wielded weapon's [`Handedness`] (GTW-443) — fed to the [`FireActor`](crate::magazine::FireActor)
    /// hand-count gate.
    pub(in crate::shot_pipeline::fire) handedness:      Handedness,
    /// The shooter's [`HandsAvailable`] (GTW-443), FOLDED from its injury ledger (an
    /// absent ledger = the uninjured two-hands default) — fed to the
    /// [`FireActor`](crate::magazine::FireActor) hand-count gate.
    pub(in crate::shot_pipeline::fire) hands_available: HandsAvailable,
}

/// Read the shooter's `Copy` state into a [`ShooterSnapshot`] — its ganger state off the
/// [`ShooterQuery`] and its GTW-200 weapon stats off the related **weapon entity**
/// (`ganger → Wields → the weapon entity`, GTW-323 slice 2) — plus the
/// [`FireActor`](crate::magazine::FireActor)-shaping economy reads, releasing the query
/// borrows before [`fire`](super::super::fire)'s mutable re-borrows.
///
/// Returns `None` when the shooter is not in the shooter query (despawned), wields no
/// RANGED weapon (no [`Wields`](crate::weapon::Wields) collection / it holds only a
/// melee weapon), or the resolved ranged weapon entity is not in the weapon query — so
/// [`fire`](super::super::fire) fails closed in every unarmed/missing case. The economy reads
/// `(Tu, TuMax, Aiming, Magazine)` are what [`can_fire`](crate::magazine::can_fire) /
/// [`mode_tu_cost`](crate::magazine::mode_tu_cost) / [`resolve_and_apply`](crate::resolve_and_apply::resolve_and_apply) consume; the
/// returned [`weapon`](ShooterReads::weapon) entity is the one the burst loop re-borrows
/// to decrement the [`Magazine`] per fired round.
///
/// GTW-505 C5: the ranged weapon is resolved via
/// [`Wields::ranged_weapon`](crate::weapon::Wields::ranged_weapon) over the `melee`
/// [`MeleeQuery`] probe — EXCLUDING the melee weapon the same ganger also wields — so
/// relating a melee weapon never regresses ranged firing.
///
/// GTW-543: the resolution PREFERS a [`MountedWeapon`](crate::weapon::MountedWeapon)-marked
/// wielded entity (the `mounted` [`MountedQuery`] probe) when present — so a ganger MANNING an
/// emplacement fires its bolted-down gun, and reverts to its own carried weapon when it exits
/// (the mount edge despawned). A ganger with no mounted weapon (the common case) falls straight
/// through to the GTW-505 ranged resolution, byte-identical to before. GTW-660: both steps run
/// through the ONE shared preference rule,
/// [`Wields::firing_weapon`](crate::weapon::Wields::firing_weapon) (mounted-first,
/// melee-excluded) — the same fn `dispatch_fire` and the reaction trigger resolve through. The
/// [`mounted`](ShooterSnapshot::mounted) flag on the snapshot records which was resolved so the
/// emplacement stability seam engages for a mounted shot only.
pub(in crate::shot_pipeline::fire) fn read_shooter(
    shooter: Entity,
    shooters: &ShooterQuery,
    wields: &WieldsQuery,
    weapons: &WeaponQuery,
    melee: &MeleeQuery,
    mounted: &MountedQuery,
) -> Option<ShooterReads> {
    let ((position, facing, stance, aiming, shooting, luck, tu_max, suppressed), injuries, tu) =
        shooters.get(shooter).ok()?;
    // GTW-436: read the shooter's Luck through the gate-enforced effective accessor over
    // its injury ledger (an absent ledger = the zero-delta identity), so a `Modify(Luck)`
    // injury shifts the wounds it deals. This is the SINGLE direct-read path for Luck.
    let effective_shooter_luck = match injuries {
        Some(ledger) => crate::ganger::effective_luck(*luck, ledger),
        None => *luck,
    };
    // GTW-443: fold the shooter's available hand count from its injury ledger (an absent
    // ledger = the uninjured two-hands default) for the FireActor hand-count gate.
    let hands_available =
        injuries.map_or_else(HandsAvailable::default, InflictedInjuries::hands_available);
    // Resolve `ganger → Wields → the FIRING weapon entity` (GTW-323 slice 2), then read
    // the GTW-200 weapon stats + magazine off that weapon entity (a different entity
    // than the ganger, so the borrow is disjoint). The resolution is the ONE shared
    // preference rule (`Wields::firing_weapon`, GTW-660): PREFER a MountedWeapon-marked
    // wielded entity (the ganger is manning an emplacement, GTW-543), else the carried
    // ranged gun EXCLUDING the melee weapon the ganger also wields (GTW-505 C5). The
    // resolved entity is mounted-marked ONLY when the mounted-first step won (a carried
    // gun never carries the marker), so the probe re-read recovers the `mounted` flag.
    let wielded = wields.get(shooter).ok()?;
    let weapon = wielded.firing_weapon(
        |entity| mounted.get(entity).is_ok(),
        |entity| melee.get(entity).is_ok(),
    )?;
    let is_mounted = mounted.get(weapon).is_ok();
    let (
        base_spread,
        accuracy,
        kickback,
        fatal_bias,
        damage,
        punch,
        shred,
        damage_type,
        stable,
        brace_bonus,
        handedness,
        magazine,
        dot,
    ) = weapons.get(weapon).ok()?;
    let snapshot = ShooterSnapshot {
        position:    *position,
        facing:      *facing,
        stance:      *stance,
        aiming:      *aiming,
        shooting:    *shooting,
        luck:        effective_shooter_luck,
        base_spread: *base_spread,
        accuracy:    *accuracy,
        kickback:    *kickback,
        fatal_bias:  *fatal_bias,
        damage:      *damage,
        punch:       *punch,
        shred:       *shred,
        damage_type: *damage_type,
        stable:      *stable,
        // GTW-549: copy the weapon's per-item WeaponBraceBonus attachment into the snapshot
        // (None = no brace attachment = zero-identity brace term).
        brace_bonus: brace_bonus.copied(),
        // GTW-526: copy the shooter's Suppressed state into the snapshot (None =
        // un-suppressed = zero-identity suppression term).
        suppressed:  suppressed.copied(),
        // GTW-543: whether the resolved ranged weapon is the emplacement's mounted gun (the
        // ganger is manning it) — engages the emplacement stability seam for this shot.
        mounted:     is_mounted,
        // GTW-544: copy the resolved weapon's DOT profile into the snapshot (None = a non-DOT
        // weapon = no attach). Rides through `weapon_stats` to the fold's DOT-attach decision.
        dot:         dot.copied(),
    };
    Some(ShooterReads {
        snapshot,
        weapon,
        tu: *tu,
        tu_max: *tu_max,
        aiming: *aiming,
        magazine: *magazine,
        handedness: *handedness,
        hands_available,
    })
}
