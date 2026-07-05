//! The contested §7 opposed-Fight melee arm — gate, spend, synthesize onto the
//! struck ganger, and emit the connect signals (GTW-506/507).

use bevy::prelude::{Entity, MessageWriter, Query, With};

use super::{
    MeleeFacts, MeleeGrids,
    queries::{MeleeGeomQuery, MeleeTargetQuery},
    snapshot::{AttackerSnapshot, MeleeStreams},
};
use crate::{
    acts::request::{MeleeResolved, MeleeStruck, ShoveRequested},
    armor::{PieceArmorMut, Wears, WornBy},
    armor_wear::ArmorWearOutcome,
    downed_acts::is_8_adjacent,
    ganger::{LifeState, Luck, Toughness, Tu, effective_luck, effective_toughness},
    los::{Observer, PeekOffset, Target, has_los},
    melee::{Combatants, MeleeStrike, MeleeWeaponHit, resolve_melee_strike},
    metric::CellLevel,
    resolve_and_apply::{StruckPiece, TargetGanger},
    rng::{FightRng, SeverityRng, ShotRng},
    tu::spend_tu,
    tuning::CombatTuning,
};

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
              MeleeResolved + ShoveRequested writers and the grouped GTW-572 fact writers \
              (MeleeFacts) — the irreducible per-arm access set (the strike_with_target \
              precedent); bundling further would only hide the access set"
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
    facts: &mut MeleeFacts,
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

        // GTW-572: the NUMBER-BEARING melee fact — the connecting strike's applied HP loss,
        // with both combatants, so the combat log can phrase a melee-damage line. The
        // strike-glyph signal above stays cell+damage-type only (the GTW-507 FX contract).
        facts.struck.write(MeleeStruck::new(
            attacker.entity,
            target_entity,
            strike.hp_damage,
        ));

        // GTW-572: a protecting→broken wear crossing (surfaced on the strike verdict — it
        // was previously computed and dropped inside the verb) emits the SAME ArmorBroken
        // fact a ranged break does, so a melee break pops and logs identically.
        if let ArmorWearOutcome::Broke(broken) = strike.wear {
            facts.breaks.write(broken);
        }

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
            connect:   false,
            severity:  crate::severity::Severity::None,
            hp_damage: crate::resolve_hit::HpDamage::new(0),
            wear:      crate::armor_wear::ArmorWearOutcome::Unaffected,
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
