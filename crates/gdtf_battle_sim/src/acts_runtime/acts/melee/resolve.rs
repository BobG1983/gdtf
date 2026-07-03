//! The per-target melee **resolvers** — the two arm helpers
//! [`dispatch_melee`](super::dispatch_melee) branches into (GTW-508 C6 — split out of the
//! dispatch file to keep each concern under the code-health size cap):
//!
//! - [`resolve_ganger_melee`] — the contested §7 opposed-Fight path (GTW-506/507): gate
//!   8-adjacency + opposing faction + alive + LOS, spend the fight-mode TU, run the §7 → §5 →
//!   §6 synthesis onto the target ([`strike_with_target`]), emit [`MeleeResolved`] on a connect.
//! - [`resolve_structure_melee`] — the UNCONTESTED §7 cover-smash path (GTW-508): gate ONLY
//!   8-adjacency (LOS to an adjacent structure is trivial), spend the same TU, and apply
//!   multiplied (`mult_max`) weapon damage through the EXISTING cover ledger — NO opposed roll,
//!   NO RNG draw; fire the EXISTING [`CoverDestroyed`] signal on a lethal smash.
//!
//! Pure dispatch glue over the already-landed verbs: no combat math is reimplemented here —
//! [`resolve_ganger_melee`] composes [`resolve_melee_strike`](crate::melee::resolve_melee_strike)
//! and [`resolve_structure_melee`] composes
//! [`resolve_structural_melee`](crate::melee::resolve_structural_melee). Param-only
//! (`bevy-traps.md` #7 — no `&mut World`); the queries the resolvers borrow are the SAME disjoint
//! [`super`] query types the system owns.

use bevy::prelude::{Entity, MessageWriter, Query, With};

use super::{MeleeGeomQuery, MeleeGrids, MeleeTargetQuery};
use crate::{
    acts::request::{MeleeResolved, ShoveRequested},
    armor::{PieceArmorMut, Wears, WornBy},
    cover::CoverEvent,
    downed_acts::is_8_adjacent,
    ganger::{
        Facing, Faction, Fight, LifeState, Luck, Position, Stance, Toughness, Tu, effective_luck,
        effective_toughness,
    },
    los::{Observer, PeekOffset, Target, has_los},
    melee::{
        Combatants, MeleeStrike, MeleeWeaponHit, resolve_melee_strike, resolve_structural_melee,
    },
    metric::CellLevel,
    occupancy_sync::CoverDestroyed,
    resolve_and_apply::{StruckPiece, TargetGanger},
    rng::{FightRng, SeverityRng, ShotRng},
    tu::spend_tu,
    tuning::CombatTuning,
    weapon::DamageType,
};

/// The attacker's snapshotted gating reads + the resolved wielded-weapon view the two
/// per-target melee resolvers ([`resolve_ganger_melee`] / [`resolve_structure_melee`]) share
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
    /// The weapon's [`DamageType`] — the presenter [`MeleeResolved`] strike-glyph role/color.
    pub(super) strike_damage_type: DamageType,
    /// The wielded melee weapon's [`Shove`](crate::weapon::Shove) tag (GTW-525) — `true`
    /// KNOCKS BACK the target one cell on a CONNECTING strike (the auto-shove hook writes a
    /// `ShoveRequested` after the connect; a miss or a non-`shove` weapon writes nothing).
    pub(super) shove:              crate::weapon::Shove,
}

/// The three seeded draw streams the §7 / §4 / §6 ganger synthesis advances, threaded by
/// `&mut` into [`resolve_ganger_melee`] — a transparent borrow bundle of the named stream
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

/// Resolve ONE ganger-vs-ganger melee request — the contested §7 opposed-Fight path
/// (GTW-506/507), extracted from [`dispatch_melee`](super::dispatch_melee)'s target branch
/// (GTW-508 C6 — file size cap). Gates 8-adjacency + opposing faction + alive + LOS, spends the
/// fight-mode TU, runs the §7 → §5 → §6 synthesis onto the target, and emits [`MeleeResolved`]
/// on a connect.
///
/// Fail-closed at every gate (a rejected strike returns early — no spend, no strike, no
/// signal, never a panic), exactly as the inlined arm did.
#[expect(
    clippy::too_many_arguments,
    reason = "the ganger arm threads the attacker snapshot, the target entity, the disjoint \
              geometry / attacker-Tu / target-surfaces queries, the two armor relationship \
              queries, the grouped grids+tuning bundle, the three draw streams, and the \
              MeleeResolved + ShoveRequested writers — the irreducible per-arm access set (the \
              strike_with_target precedent); bundling further would only hide the access set"
)]
pub(super) fn resolve_ganger_melee(
    attacker: &AttackerSnapshot<'_>,
    target_entity: Entity,
    geom: &MeleeGeomQuery,
    tu_q: &mut Query<&mut Tu>,
    targets: &mut MeleeTargetQuery,
    wears: &Query<&Wears>,
    pieces: &mut Query<PieceArmorMut, With<WornBy>>,
    grids: &mut MeleeGrids,
    streams: MeleeStreams<'_>,
    resolved: &mut MessageWriter<MeleeResolved>,
    shoves: &mut MessageWriter<ShoveRequested>,
    deaths: &mut MessageWriter<crate::on_death::OnDeathOccurred>,
) {
    let Ok((&tgt_pos, &tgt_stance, _, &tgt_fight, &tgt_faction, _)) = geom.get(target_entity)
    else {
        return;
    };

    // Gate — 8-adjacency + opposing faction (the cheap reads first).
    if !is_8_adjacent(attacker.position, tgt_pos) || attacker.faction == tgt_faction {
        return;
    }

    // The target's liveness — an ALIVE (incl. Downed) opposing ganger only; a corpse is no
    // target. Read off the target query (the SINGLE LifeState read path), snapshotted.
    let Ok((_, _, &tgt_life, _, &tgt_toughness, &tgt_luck, tgt_injuries)) =
        targets.get(target_entity)
    else {
        return;
    };
    if !tgt_life.is_active() {
        return;
    }
    // Route the defender's Toughness / Luck through the gate-enforced effective accessors over
    // its injury ledger (an absent ledger = the zero-delta identity), so an injury shifting
    // either stat shifts the §6 severity roll in step (the `fold_ganger_round` precedent).
    let (effective_toughness, effective_luck) = match tgt_injuries {
        Some(ledger) => (
            effective_toughness(tgt_toughness, ledger),
            effective_luck(tgt_luck, ledger),
        ),
        None => (tgt_toughness, tgt_luck),
    };

    // The LOS gate — a clear sight line attacker → target over the SAME voxel geometry the sim
    // fires through. Built exactly as `reaction_trigger` builds the observer/target.
    let observer = Observer {
        position:         &attacker.position,
        stance:           &attacker.stance,
        facing:           &attacker.facing,
        stair_eye_offset: grids.occupancy.stair_eye_offset_at(&attacker.position),
        peek_offset:      PeekOffset::default(),
    };
    let los_target = Target {
        position: &tgt_pos,
        stance:   &tgt_stance,
    };
    // A Dead ganger is a corpse the LOS march flies THROUGH (GTW-317) — the same `is_dead`
    // pass-through `has_los`/`can_see` take. The predicate borrows `targets` IMMUTABLY only for
    // this `has_los` call; it is dropped before the `&mut targets` fold below (NLL), so the
    // immutable + mutable accesses never overlap. An entity absent from `targets` is no corpse
    // (defaults `false`). The cover ledger is read through `&*grids.cover` (a `ResMut` derefs to
    // `&CoverLedger`).
    let is_dead = |entity: Entity| {
        targets
            .get(entity)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
    };
    let sighted = has_los(
        &observer,
        &los_target,
        &grids.occupancy,
        &grids.surface,
        &grids.cover,
        &grids.tuning,
        is_dead,
    );
    if !*sighted {
        return;
    }

    // Spend the fight-mode TU off the attacker (saturating). The swing costs TU whether or not
    // it connects (the ranged `fire()` TU-charge precedent). A missing Tu pool fails the strike
    // (fail-closed).
    let Ok(mut attacker_tu) = tu_q.get_mut(attacker.entity) else {
        return;
    };
    spend_tu(&mut attacker_tu, attacker.tu_cost);

    // Run the §7 → §5 → §6 synthesis onto the target. The verb owns the §4 part roll, so the
    // struck piece cannot be keyed by part before it runs; `strike_with_target` resolves the
    // target's worn protection as the verb's armor input (bare flesh if the target wears
    // nothing) and re-borrows the target's `&mut` surfaces for the fold (the snapshot reads
    // above are released — `get_mut` takes a fresh exclusive borrow).
    let strike = strike_with_target(
        targets,
        wears,
        pieces,
        target_entity,
        Combatants {
            attacker_fight: attacker.fight,
            defender_fight: tgt_fight,
            attacker_luck:  attacker.luck,
        },
        effective_toughness,
        effective_luck,
        attacker.weapon,
        &grids.tuning,
        streams.fight,
        streams.shot,
        streams.severity,
    );

    // On a connect, emit the presenter strike-glyph signal at the target's cell. A miss deals
    // no damage and emits nothing (the §7 connect gate).
    if strike.connect {
        // The target's own (cell, level) key — Position derefs to CellLevel (the
        // old decompose-then-recompose was the identity on every real key).
        let at: CellLevel = *tgt_pos;
        resolved.write(MeleeResolved::new(at, attacker.strike_damage_type));

        // GTW-547: a CONNECTING strike that KILLED the target (its post-fold LifeState is Dead)
        // emits the terminal-death signal at the target's cell so `resolve_on_death` fans the
        // dead ganger's on-death effect. Re-read the target's LifeState off the query AFTER the
        // in-place fold (the SINGLE LifeState read path); a strike that wounded-but-did-not-kill
        // emits nothing.
        if targets
            .get(target_entity)
            .is_ok_and(|(_, _, &life, ..)| life == LifeState::Dead)
        {
            deaths.write(crate::on_death::OnDeathOccurred::new(target_entity, at));
        }

        // GTW-525 C3: a `shove`-tagged weapon KNOCKS BACK the target on a CONNECTING strike
        // (in addition to the damage above). Write the internal weapon-tag ShoveRequested — the
        // connect already gated + charged, so dispatch_shove resolves it un-gated / TU-free
        // (ShoveSource::Weapon). A miss never reaches here (no shove); a non-`shove` weapon
        // writes nothing. dispatch_shove is ordered `.after(dispatch_melee)`, so the same-frame
        // message is consumed this tick.
        if *attacker.shove {
            shoves.write(ShoveRequested::new_weapon(attacker.entity, target_entity));
        }
    }
}

/// Resolve ONE melee-vs-structure request — the UNCONTESTED §7 cover-smash path (GTW-508),
/// extracted from [`dispatch_melee`](super::dispatch_melee)'s target branch (GTW-508 C6 — file
/// size cap).
///
/// Gates ONLY 8-adjacency to the struck cell (LOS to an immediately-adjacent structure is
/// trivially satisfied — NO spurious LOS block, the C3 ruling), spends the same fight-mode TU,
/// and calls [`resolve_structural_melee`] — multiplied (`mult_max`, FORK 4a) weapon damage
/// through the EXISTING [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover).
/// There is **NO opposed roll and NO `FightRng`/`ShotRng`/`SeverityRng` draw** — a structure is
/// inert. On a lethal smash it fires the EXISTING [`CoverDestroyed`] signal (the GTW-386 FX);
/// on either outcome it emits the [`MeleeResolved`] strike-glyph (a structure never dodges, so
/// there is no connect gate). Fail-closed on a failed adjacency gate / missing Tu pool.
pub(super) fn resolve_structure_melee(
    attacker: &AttackerSnapshot<'_>,
    at: CellLevel,
    tu_q: &mut Query<&mut Tu>,
    grids: &mut MeleeGrids,
    resolved: &mut MessageWriter<MeleeResolved>,
    cover_destroyed: &mut MessageWriter<CoverDestroyed>,
    deaths: &mut MessageWriter<crate::on_death::OnDeathOccurred>,
) {
    // Gate — 8-adjacency to the struck STRUCTURE cell (reuse `is_8_adjacent` over the attacker's
    // Position vs a Position at the target cell). LOS to an immediately-adjacent structure is
    // trivially satisfied, so NO LOS block is applied (a spurious LOS gate would reject the very
    // cover the attacker stands beside — the C3 ruling).
    if !is_8_adjacent(attacker.position, Position::new(at)) {
        return;
    }

    // The struck cover's prototype — the ledger's stored entry if present, else a lazily-seeded
    // intact wall. `peek` distinguishes an already-registered cell from an unregistered one;
    // either way the smash resolves against a defined entry (the ledger owns the lazy seed). An
    // unregistered cell has no authored HP/armor, so it seeds from the shared no-panic default (a
    // strike on an out-of-bounds / empty cell still resolves defined bookkeeping, never a panic).
    let prototype = grids
        .cover
        .peek(&at)
        .copied()
        .unwrap_or(STRUCTURE_SMASH_FALLBACK);

    // Spend the fight-mode TU off the attacker (saturating) — a swing at a structure costs TU
    // exactly like a swing at a ganger (fail-closed on a missing Tu pool).
    let Ok(mut attacker_tu) = tu_q.get_mut(attacker.entity) else {
        return;
    };
    spend_tu(&mut attacker_tu, attacker.tu_cost);

    // The UNCONTESTED smash — multiplied (mult_max) weapon damage through the EXISTING ledger
    // `deplete_cover`. NO opposed roll, NO FightRng/ShotRng/SeverityRng draw.
    let event = resolve_structural_melee(
        attacker.weapon,
        &prototype,
        at,
        &mut grids.cover,
        &grids.tuning,
    );

    // On a lethal smash, fire the EXISTING cover-destroyed signal (the presenter's GTW-386
    // rubble-burst FX reacts to it verbatim).
    if let CoverEvent::Destroyed(cell) = event {
        cover_destroyed.write(CoverDestroyed::new(cell));
        // GTW-547: a destroyed piece of cover ALSO emits the terminal-death signal (keyed by
        // its cell — cover is not an entity, so Entity::PLACEHOLDER) so `resolve_on_death` fans
        // the cover tile's authored on-death effect (the ranged cover-destroy bridge mirror).
        deaths.write(crate::on_death::OnDeathOccurred::cover(cell));
    }
    // Emit the strike-glyph at the struck structure cell (a structural smash always lands — a
    // structure never dodges — so unlike the ganger path there is no connect gate here).
    resolved.write(MeleeResolved::new(at, attacker.strike_damage_type));
}

/// The fallback [`CoverEntry`](crate::cover::CoverEntry) prototype a melee cover-smash resolves
/// against when the struck cell has NO ledger entry yet (an unregistered / unauthored cell) — a
/// no-panic backstop, NOT authored data.
///
/// A registered cell (a real wall / prop) is read through
/// [`CoverLedger::peek`](crate::cover::CoverLedger::peek); this fallback fires ONLY for a strike
/// on a cell that was never registered or hit. It seeds an intact, ARMORLESS piece (protection /
/// hardness `0`, a small `max_hp`, a `Low` band) so the smash resolves defined bookkeeping —
/// [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover) ignores this
/// prototype whenever an authored entry already exists (its `entry_seeded` returns the stored
/// entry unchanged), so authored HP/armor is always honoured; this only backstops an
/// unregistered strike (the ranged `SLAB_FALLBACK_DEFAULTS` precedent). A code-only const — no
/// tuning read, no pinned balance magnitude a test asserts.
const STRUCTURE_SMASH_FALLBACK: crate::cover::CoverEntry = crate::cover::CoverEntry::seeded(
    crate::cover::CoverHp::new(1),
    crate::cover::HeightBand::Low,
    crate::armor::ArmorProtection::new(0),
    crate::armor::ArmorHardness::new(0),
);

/// Fold one melee strike onto the target — resolve the struck worn piece, assemble the
/// [`TargetGanger`] borrow-view, and run [`resolve_melee_strike`] (split out so
/// [`resolve_ganger_melee`] stays under clippy's line gate).
///
/// The §4 part roll lives INSIDE [`resolve_melee_strike`], so the struck piece cannot be keyed
/// by part before the verb runs. This passes the target's worn-armor resolution as a whole
/// through the verb's bare-flesh-vs-armor branch by attaching the target's FIRST worn piece (if
/// any), letting the §5 matchup + §6 wear resolve against it; a target wearing nothing folds to
/// bare flesh. The defender's effective Toughness / Luck (already projected over its injury
/// ledger) ride the [`TargetGanger`]. REUSES the verb verbatim — no combat math here.
#[expect(
    clippy::too_many_arguments,
    reason = "the fold threads the target query + the two armor relationship queries + the \
              target entity + the combatants + the projected Toughness/Luck + the weapon + \
              tuning + the three draw streams — the irreducible input set resolve_melee_strike \
              takes; bundling would only hide the access set"
)]
fn strike_with_target(
    targets: &mut MeleeTargetQuery,
    wears: &Query<&Wears>,
    pieces: &mut Query<PieceArmorMut, With<WornBy>>,
    target_entity: Entity,
    combatants: Combatants,
    toughness: Toughness,
    luck: Luck,
    weapon: MeleeWeaponHit<'_>,
    tuning: &CombatTuning,
    fight_rng: &mut FightRng,
    shot_rng: &mut ShotRng,
    severity_rng: &mut SeverityRng,
) -> MeleeStrike {
    // Resolve the target's struck worn piece view — the FIRST worn piece (if any), read off
    // `target → Wears → the piece entity`. The §6 fold wears its `&mut ArmorIntegrity` in
    // place; a target wearing nothing folds to bare flesh. (The §4 part roll inside the verb
    // owns the location; this resolves the target's worn protection as the verb's armor input.)
    let piece_view = wears
        .get(target_entity)
        .ok()
        .and_then(|worn| worn.pieces().next())
        .and_then(|piece_entity| {
            pieces.get_mut(piece_entity).ok().map(|piece| StruckPiece {
                floor:      *piece.floor,
                protection: *piece.protection,
                hardness:   *piece.hardness,
                armor_type: *piece.armor_type,
                integrity:  piece.integrity.into_inner(),
            })
        });

    let Ok((mut hp, mut wounds, mut life, mut inflicted, ..)) = targets.get_mut(target_entity)
    else {
        return MeleeStrike {
            connect:  false,
            severity: crate::severity::Severity::None,
        };
    };
    resolve_melee_strike(
        combatants,
        weapon,
        TargetGanger {
            hp: &mut hp,
            wounds: &mut wounds,
            life: &mut life,
            piece: piece_view,
            inflicted: &mut inflicted,
            toughness,
            luck,
        },
        target_entity,
        tuning,
        fight_rng,
        shot_rng,
        severity_rng,
    )
}
