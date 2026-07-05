//! The `dispatch_melee` system — drain the buffered request, snapshot + gate,
//! resolve the wielded melee weapon, and branch to the per-target arm.

use bevy::prelude::{MessageReader, MessageWriter, Query, With};

use super::{
    MeleeFacts, MeleeGrids, MeleeRngs,
    ganger::resolve_ganger_melee,
    queries::{MeleeGeomQuery, MeleeTargetQuery, MeleeWeaponQuery},
    snapshot::{AttackerSnapshot, MeleeStreams},
    structure::resolve_structure_melee,
};
use crate::{
    acts::request::{MeleeRequested, MeleeResolved, MeleeTarget, ShoveRequested},
    armor::{PieceArmorMut, Wears, WornBy},
    fire::{MeleeQuery, WieldsQuery},
    ganger::Tu,
    melee::MeleeWeaponHit,
    occupancy_sync::CoverDestroyed,
    weapon::DamageType,
};

/// **Dispatch** buffered [`MeleeRequested`] messages — the LIVE melee act (GTW-507).
///
/// For each request the dispatch:
///
/// 1. **Snapshots** the attacker's + target's `Copy` reads off the geometry query
///    (position / stance / facing / Fight / faction / Luck), releasing the read borrow before
///    any mutation. An attacker or target missing the read components is skipped (fail-closed,
///    no panic).
/// 2. **Gates** the strike, REUSING the landed predicates verbatim (reject = no spend, no
///    strike, no signal — never a panic):
///    - [`is_8_adjacent`](crate::downed_acts::is_8_adjacent) — same-storey Moore-8 reach (the §7
///      close-combat range this slice; reach > 1 is a future refinement);
///    - the target is [`LifeState::is_active`](crate::ganger::LifeState::is_active) (an ALIVE — incl. Downed — opposing ganger; a
///      corpse is no target);
///    - `attacker.faction != target.faction` — an OPPOSING ganger (no friendly melee);
///    - [`has_los`](crate::los::has_los) — a clear sight line attacker → target over the SAME
///      voxel geometry the sim fires through (`docs/combat/resolution.md` §2 clearance), built
///      exactly as `reaction_trigger` builds it (the corpse `is_dead` pass-through reused).
/// 3. **Resolves the weapon** — `attacker → Wields → the MeleeWeapon entity`
///    ([`Wields::melee_weapon`](crate::weapon::Wields::melee_weapon), excluding the ranged
///    weapon via the [`MeleeQuery`] probe), reading its §5 damage stats + primary
///    [`FightMode`](crate::weapon::FightMode) TU cost. An attacker wielding no melee weapon is skipped (fail-closed).
/// 4. **Spends** the wielded melee weapon's primary fight-mode flat TU off the attacker's
///    `&mut Tu` (saturating; the act is taken whether or not the strike connects — a swing
///    costs TU regardless, mirroring the ranged `fire()` TU charge).
/// 5. **Runs** [`resolve_melee_strike`](crate::melee::resolve_melee_strike) — the §7
///    opposed-Fight → §5 damage (× the §7 margin
///    multiplier) → §6 wound synthesis (GTW-506 core + the §4/§5/§6 pieces, reused verbatim),
///    folding the connecting hit onto the target's `&mut` surfaces in place and emitting the
///    existing wound/injury signals (via [`apply_hit`](crate::apply_hit::apply_hit) +
///    `InflictedWounds`). The three injected streams (`FightRng` / `ShotRng` / `SeverityRng`)
///    are the only entropy.
/// 6. On a **connect**, emits ONE [`MeleeResolved`] carrying the target's struck `(cell, level)`
///    plus the weapon's [`DamageType`] (the presenter's strike-glyph FX role/color). A MISS
///    (the opposed roll lost) deals no damage and emits nothing.
///
/// # Melee-vs-STRUCTURE (GTW-508)
///
/// When the [`MeleeTarget`] is a [`Structure`](MeleeTarget::Structure) cell instead of a
/// ganger, the dispatch resolves an UNCONTESTED cover-smash: it gates only 8-adjacency to the
/// struck cell (LOS to an immediately-adjacent structure is trivially satisfied — no spurious
/// LOS block), spends the same wielded fight-mode TU, and calls
/// [`resolve_structural_melee`](crate::melee::resolve_structural_melee) — which applies
/// multiplied (`mult_max`, FORK 4a) weapon damage through the EXISTING
/// [`CoverLedger::deplete_cover`](crate::cover::CoverLedger::deplete_cover). There is **NO
/// opposed Fight roll and NO [`FightRng`](crate::rng::FightRng) draw** (nor any `ShotRng` / `SeverityRng` draw) — a
/// structure is inert. On a lethal smash ([`CoverEvent::Destroyed`](crate::cover::CoverEvent::Destroyed))
/// it emits the EXISTING
/// [`CoverDestroyed`] signal (the GTW-386 FX fires verbatim); on either outcome it emits a
/// [`MeleeResolved`] strike-glyph at the struck cell. A cell with no ledger entry lazy-seeds at
/// its authored `max_hp` (the ledger owns that), so a strike on an unauthored cell still
/// resolves defined bookkeeping (never a panic).
///
/// Ordered in [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems) by
/// [`SimActsPlugin`](crate::acts::SimActsPlugin). Param-only (`bevy-traps.md` #7 — no
/// `&mut World`); the queries are disjoint (see each `type`'s doc). The `ResMut<FightRng>` /
/// `ResMut<ShotRng>` / `ResMut<SeverityRng>` are the drawing streams (never `Res` — drawing
/// advances the cursor; the `rng::streams` binding constraint).
#[expect(
    clippy::too_many_arguments,
    reason = "the melee dispatch needs the request reader, the disjoint geometry / attacker-Tu \
              / target-surfaces ganger queries, the two armor relationship queries, the wields \
              + melee-marker + melee-weapon-stat relationship queries, the grouped grids+tuning \
              (MeleeGrids) + draw streams (MeleeRngs) bundles, and the MeleeResolved + \
              MeleeStruck + CoverDestroyed + ShoveRequested writers — each a distinct Bevy \
              SystemParam (the dispatch_fire argument-count carve-out)"
)]
pub fn dispatch_melee(
    mut requests: MessageReader<MeleeRequested>,
    geom: MeleeGeomQuery,
    mut tu_q: Query<&mut Tu>,
    mut targets: MeleeTargetQuery,
    // Worn-armor relationship queries (the struck piece resolution, GTW-323 / ADR-0004) —
    // disjoint from the ganger queries (a different component / a different entity set).
    wears: Query<&Wears>,
    mut pieces: Query<PieceArmorMut, With<WornBy>>,
    // The wielded-weapon relationship — `attacker → Wields → the melee weapon entity`.
    wields: WieldsQuery,
    melee: MeleeQuery,
    weapons: MeleeWeaponQuery,
    // The LOS gate's read grids + tuning, grouped (MeleeGrids) so the system stays under
    // Bevy's 16-param limit. `cover` is `ResMut` (the GTW-508 cover-smash spends it).
    mut grids: MeleeGrids,
    // The three draw streams the §7 / §4 / §6 synthesis advances, grouped (MeleeRngs).
    rngs: MeleeRngs,
    mut resolved: MessageWriter<MeleeResolved>,
    // GTW-572: the grouped fact writers — a CONNECTING ganger strike writes one MeleeStruck
    // { attacker, target, hp_damage } (the combat log's melee-damage line) and, on a
    // protecting→broken wear crossing, one ArmorBroken; a miss / structural smash writes
    // neither (`bevy-traps.md` #4).
    mut facts: MeleeFacts,
    // The GTW-508 cover-destroyed bridge — a lethal cover-smash writes the EXISTING signal the
    // presenter's `read_cover_destroyed` FX (GTW-386) already reacts to (`bevy-traps.md` #4).
    mut cover_destroyed: MessageWriter<CoverDestroyed>,
    // GTW-525: the weapon-tag auto-shove bridge — a connecting ganger strike with a `shove`
    // weapon writes an internal ShoveRequested (ShoveSource::Weapon) `dispatch_shove` drains
    // this same frame (it is ordered `.after(dispatch_melee)`); a miss / non-`shove` weapon
    // writes nothing (`bevy-traps.md` #4).
    mut shoves: MessageWriter<ShoveRequested>,
    // GTW-547: the terminal-death bridge — a melee strike that KILLS a ganger, or a lethal
    // cover-smash, writes an OnDeathOccurred so `resolve_on_death` fans the dead source's
    // on-death effect (`bevy-traps.md` #4). Threaded into both per-target resolvers.
    mut deaths: MessageWriter<crate::on_death::OnDeathOccurred>,
) {
    // `bevy-traps.md` #1: without all three seeded streams no strike can resolve a draw — fail
    // closed (no panic) rather than reading an absent battle-lifetime resource. In the real app
    // all three are sim-set (inserted at setup), so this never bails there (the `reaction_trigger`
    // `Option<ResMut<ReactionRng>>` precedent). The structural (cover-smash) path takes NO draw,
    // but the streams are always present in a live battle, so gating both arms here is harmless.
    let (Some(mut fight_rng), Some(mut shot_rng), Some(mut severity_rng)) =
        (rngs.fight, rngs.shot, rngs.severity)
    else {
        return;
    };
    for request in requests.read() {
        // (1) Snapshot the attacker's gating reads (Copy), releasing the read borrow before the
        //     mutations. A missing read component fails the strike (fail-closed). Shared by both
        //     the ganger-vs-ganger and the melee-vs-structure paths.
        let Ok((&atk_pos, &atk_stance, &atk_facing, &atk_fight, &atk_faction, &atk_luck)) =
            geom.get(request.attacker)
        else {
            continue;
        };

        // (2) Resolve the wielded MELEE weapon (`attacker → Wields → the MeleeWeapon entity`,
        //     excluding the ranged weapon), and read its §5 stats + primary fight-mode TU —
        //     shared by both paths. An attacker wielding no melee weapon cannot strike
        //     (fail-closed).
        let Some(weapon_entity) = wields
            .get(request.attacker)
            .ok()
            .and_then(|w| w.melee_weapon(|entity| melee.get(entity).is_ok()))
        else {
            continue;
        };
        let Ok((damage, punch, shred, damage_type, fatal_bias, fight_mode, shove)) =
            weapons.get(weapon_entity)
        else {
            continue;
        };
        let weapon = MeleeWeaponHit {
            damage,
            punch,
            shred,
            damage_type,
            fatal_bias,
        };
        // The primary fight-mode flat TU cost (the wielded weapon's first authored mode, or a
        // total structural default — `FightMode::primary` never panics).
        let tu_cost = Tu::new(u8::try_from(*fight_mode.primary().tu_cost).unwrap_or(u8::MAX));
        let strike_damage_type: DamageType = *damage_type;

        // The attacker's snapshotted gating reads + the resolved weapon/TU, bundled for the
        // per-arm resolvers (below) so the branch stays a two-line dispatch.
        let attacker = AttackerSnapshot {
            entity: request.attacker,
            position: atk_pos,
            stance: atk_stance,
            facing: atk_facing,
            fight: atk_fight,
            faction: atk_faction,
            luck: atk_luck,
            weapon,
            tu_cost,
            strike_damage_type,
            shove: *shove,
        };
        match request.target {
            // ── Ganger-vs-ganger (GTW-506/507) — the contested opposed-Fight path (unchanged). ──
            MeleeTarget::Ganger(target_entity) => resolve_ganger_melee(
                &attacker,
                target_entity,
                &geom,
                &mut tu_q,
                &mut targets,
                &wears,
                &mut pieces,
                &mut grids,
                MeleeStreams {
                    fight:    &mut fight_rng,
                    shot:     &mut shot_rng,
                    severity: &mut severity_rng,
                },
                &mut resolved,
                &mut facts,
                &mut shoves,
                &mut deaths,
            ),

            // ── Melee-vs-structure (GTW-508) — the UNCONTESTED cover-smash path. ──
            MeleeTarget::Structure(at) => resolve_structure_melee(
                &attacker,
                at,
                &mut tu_q,
                &mut grids,
                &mut resolved,
                &mut cover_destroyed,
                &mut deaths,
            ),
        }
    }
}
