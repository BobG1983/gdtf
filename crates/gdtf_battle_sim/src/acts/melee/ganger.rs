//! The contested §7 opposed-Fight melee arm — gate, spend, synthesize onto the
//! struck ganger, and emit the connect signals (GTW-506/507).

use bevy::prelude::{Entity, MessageWriter, Query, With};

use super::{
    MeleeFacts, MeleeWorld,
    emit::{MeleeConnectSignals, emit_connect_signals},
    queries::{MeleeGeomQuery, MeleeTargetQuery},
    snapshot::{AttackerSnapshot, MeleeStreams},
};
use crate::{
    acts::{
        downed::is_8_adjacent,
        request::{MeleeResolved, ShoveRequested},
    },
    armor::{PieceArmorMut, Wears, WornBy},
    ganger::{LifeState, Luck, Toughness, Tu, effective_luck, effective_toughness},
    injuries::{InjuryRegistry, InjuryTables},
    los::{Observer, PeekOffset, Target, has_los},
    melee::{Combatants, MeleeStrike, MeleeStrikeEnv, MeleeWeaponHit, resolve_melee_strike},
    metric::CellLevel,
    resolve_and_apply::{StruckPiece, TargetGanger},
    tu::spend_tu,
};

/// Resolve ONE ganger-vs-ganger melee request — the contested §7 opposed-Fight path
/// (GTW-506/507), extracted from [`dispatch_melee`](super::dispatch_melee)'s target branch
/// (GTW-508 C6 — file size cap). Gates 8-adjacency + opposing faction + alive + LOS, spends the
/// fight-mode TU, runs the §7 → §5 → §6 → §8 synthesis onto the target, and — on a connect —
/// hands the frozen verdict to [`emit_connect_signals`].
///
/// Fail-closed at every gate (a rejected strike returns early — no spend, no strike, no
/// signal, never a panic), exactly as the inlined arm did.
#[expect(
    clippy::too_many_arguments,
    reason = "the ganger arm threads the attacker snapshot, the target entity, the disjoint \
              geometry / attacker-Tu / target-surfaces queries, the two armor relationship \
              queries, the grouped world reads, the four draw streams, and the \
              MeleeResolved + ShoveRequested writers and the grouped fact writers \
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
    world: &mut MeleeWorld,
    streams: MeleeStreams<'_>,
    resolved: &mut MessageWriter<MeleeResolved>,
    facts: &mut MeleeFacts,
    shoves: &mut MessageWriter<ShoveRequested>,
    deaths: &mut MessageWriter<crate::effects::on_death::OnDeathOccurred>,
) {
    let Ok((&tgt_pos, &tgt_stance, _, &tgt_fight, &tgt_faction, _)) = geom.get(target_entity)
    else {
        return;
    };

    // Gate — 8-adjacency + opposing faction (the cheap reads first).
    if !*is_8_adjacent(attacker.position, tgt_pos) || attacker.faction == tgt_faction {
        return;
    }

    // The target's liveness — an ALIVE (incl. Downed) opposing ganger only; a corpse is no
    // target. Read off the target query (the SINGLE LifeState read path), snapshotted.
    let Ok((_, _, &tgt_life, _, &tgt_toughness, &tgt_luck, tgt_injuries)) =
        targets.get(target_entity)
    else {
        return;
    };
    if !*tgt_life.is_active() {
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
        stair_eye_offset: world.occupancy.stair_eye_offset_at(&attacker.position),
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
    // (defaults `false`). The cover ledger is read through `&*world.cover` (a `ResMut` derefs to
    // `&CoverLedger`).
    let is_dead = |entity: Entity| {
        targets
            .get(entity)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
    };
    let sighted = has_los(
        &observer,
        &los_target,
        &world.occupancy,
        &world.surface,
        &world.cover,
        &world.tuning,
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

    // GTW-821: the §8 injury content, with EMPTY fallbacks for an asset-less harness (no Load
    // flow → no InjuryTables / InjuryRegistry) so the roll always has valid content to sample —
    // it then takes its one draw and rolls nothing (the dispatch_fire / dispatch_shove
    // fallback precedent), never a panic.
    let empty_tables = InjuryTables::default();
    let empty_registry = InjuryRegistry::default();
    let tables: &InjuryTables = world.tables.as_deref().unwrap_or(&empty_tables);
    let registry: &InjuryRegistry = world.registry.as_deref().unwrap_or(&empty_registry);

    // Run the §7 → §5 → §6 → §8 synthesis onto the target. The verb owns the §4 part roll, so
    // the struck piece cannot be keyed by part before it runs; `strike_with_target` resolves the
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
        MeleeStrikeEnv {
            tuning: &world.tuning,
            tables,
            registry,
            fight_rng: streams.fight,
            shot_rng: streams.shot,
            severity_rng: streams.severity,
            injury_rng: streams.injury,
        },
    );

    // On a connect, emit the strike's output signals (the presenter glyph, the GTW-572 facts,
    // the GTW-821 injury bridge, the GTW-547 death gate, the GTW-525 shove). A miss deals no
    // damage and emits nothing (the §7 connect gate).
    if *strike.connect {
        // The target's own (cell, level) key — Position derefs to CellLevel (the
        // old decompose-then-recompose was the identity on every real key).
        let at: CellLevel = *tgt_pos;
        emit_connect_signals(
            attacker,
            target_entity,
            at,
            strike,
            targets,
            MeleeConnectSignals {
                resolved,
                facts,
                shoves,
                deaths,
            },
        );
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
              the grouped strike environment (tuning + injury content + the four draw \
              streams) — the irreducible input set resolve_melee_strike takes; bundling \
              further would only hide the access set"
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
    env: MeleeStrikeEnv<'_>,
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
            connect:   crate::melee::Connected::new(false),
            severity:  crate::severity::Severity::None,
            hp_damage: crate::resolve_hit::HpDamage::new(0),
            wear:      crate::armor_wear::ArmorWearOutcome::Unaffected,
            injury:    None,
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
        env,
    )
}
