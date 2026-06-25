//! [`rederive_stats_on_tuning_change`] — the GTW-384 LIVE re-derivation: when the
//! [`GangerStatTuning`] resource changes (the app-side hot-reload overwrites it on a
//! `stat_tuning.ron` edit, GTW-374 pattern), re-derive every spawned ganger's computed
//! stats and CLAMP the current pools to the new maxes — never reset a damaged pool to
//! full mid-battle.
//!
//! Lives in the sim because it touches ganger ENTITIES (the derivation result lands on
//! their components); the asset plumbing stays in the app, which overwrites the
//! `GangerStatTuning` resource on a file Modified — this system reacts to the resulting
//! resource change (`resource_changed` run-condition), so the sim never depends on the
//! asset system (the one-way dep direction holds, and there is no `&mut World`).

use bevy::{
    ecs::change_detection::DetectChanges,
    prelude::{Mut, Query, Res},
};

use crate::{
    ganger::{
        Aim, Bottle, Cool, Fight, GangerAttributes, Grit, Hp, HpMax, Luck, Morale, Reactions,
        Reflexes, Shooting, Speed, Strength, Toughness, Tu, TuMax, Wounds, WoundsMax, derive_stats,
    },
    tuning::GangerStatTuning,
};

/// The read-side attribute set of one ganger, queried for the re-derivation (the eight
/// authored direct attributes). Nested into a sub-tuple to keep the outer query arity
/// under Bevy's 16-element `QueryData` tuple cap.
type AttributeRead<'w> = (
    &'w Speed,
    &'w Aim,
    &'w Strength,
    &'w Toughness,
    &'w Reflexes,
    &'w Cool,
    &'w Grit,
    &'w Luck,
);

/// The write-side derived-skill components (the `f32` skill stats — never pools, so
/// they are overwritten wholesale on a re-derive).
type SkillWrite<'w> = (
    Mut<'w, Shooting>,
    Mut<'w, Fight>,
    Mut<'w, Reactions>,
    Mut<'w, Morale>,
);

/// The write-side pool + max components (the maxes are overwritten; the current pools
/// are CLAMPED to the new maxes — never reset to full).
type PoolWrite<'w> = (
    Mut<'w, Tu>,
    Mut<'w, TuMax>,
    Mut<'w, Hp>,
    Mut<'w, HpMax>,
    Mut<'w, Wounds>,
    Mut<'w, WoundsMax>,
    Mut<'w, Bottle>,
);

/// `Update`: when the [`GangerStatTuning`] resource CHANGES (the app-side hot-reload
/// overwrites it on a `stat_tuning.ron` edit, GTW-374 pattern), re-derive every spawned
/// ganger's computed stats from its eight authored attributes × the NEW tuning, updating
/// the derived skill stats + the MAX components and CLAMPING the current pools to the new
/// maxes (GTW-384 — mirroring the GTW-374 combat hot-reload's clamp-not-reset).
///
/// The attributes are the slowly-changing raw potential — they are NOT touched here
/// (only an authored situation / use-improvement edits them); this re-derives the stats
/// that depend on them through the tuning weights. The skill stats
/// ([`Shooting`] / [`Fight`] / [`Reactions`] / [`Morale`]) are overwritten wholesale
/// (they are not pools). The MAX components ([`TuMax`] / [`HpMax`] / [`WoundsMax`]) take
/// the new derived value; the CURRENT pools ([`Tu`] / [`Hp`] / [`Wounds`]) are clamped
/// to the new max via `min`, so a ganger already damaged keeps its damage rather than
/// snapping back to full. [`Bottle`] takes the freshly derived value (the dormant
/// psychological life pool has no mid-battle damage path yet, so it tracks its max).
///
/// Self-guarding (`bevy-traps.md` #1, the GTW-374 combat-redrive precedent): it takes
/// the tuning as an [`Option`]al [`Res`] and returns early when ABSENT (a `MinimalPlugins`
/// harness that adds `BattleSimPlugin` without the persistent `Load`-tier
/// `GangerStatTuning` does not panic) AND only acts when the resource is `is_changed()`
/// (the change-detection gate — it re-derives ONLY on a tuning edit, not every frame).
/// Doing the existence + change check INSIDE the system (not as a `Res`-param run
/// condition) avoids a run-condition `Res` param validating eagerly against an absent
/// resource (`bevy-traps.md` #3).
///
/// Param-only (`bevy-traps.md` #7): an `Option<Res<GangerStatTuning>>` (read) + one
/// `Query` over the attribute + derived components (write). The query nests its component
/// groups into sub-tuples to stay under Bevy's 16-element `QueryData` arity cap.
pub fn rederive_stats_on_tuning_change(
    tuning: Option<Res<GangerStatTuning>>,
    mut gangers: Query<(AttributeRead, SkillWrite, PoolWrite)>,
) {
    // Absent (pre-load) → nothing to re-derive; unchanged → not a tuning edit, skip.
    let Some(tuning) = tuning else {
        return;
    };
    if !tuning.is_changed() {
        return;
    }
    for ((speed, aim, strength, toughness, reflexes, cool, grit, luck), skills, pools) in
        &mut gangers
    {
        // Reconstruct the attribute record from the ganger's components and re-derive.
        let attributes = GangerAttributes {
            speed:     *speed,
            aim:       *aim,
            strength:  *strength,
            toughness: *toughness,
            reflexes:  *reflexes,
            cool:      *cool,
            grit:      *grit,
            luck:      *luck,
        };
        let derived = derive_stats(&attributes, &tuning);

        let (mut shooting, mut fight, mut reactions, mut morale) = skills;
        // Skill stats are not pools — overwrite wholesale.
        *shooting = derived.shooting;
        *fight = derived.fight;
        *reactions = derived.reactions;
        *morale = derived.morale;

        let (mut tu, mut tu_max, mut hp, mut hp_max, mut wounds, mut wounds_max, mut bottle) =
            pools;
        // MAXES take the new derived ceiling; CURRENT pools are CLAMPED to the new max
        // (`min`), never reset to full — a damaged ganger keeps its damage mid-battle
        // (the GTW-374 clamp-not-reset contract).
        *tu_max = derived.tu_max;
        *tu = Tu::new((*tu).min(*derived.tu_max));
        *hp_max = derived.hp_max;
        *hp = Hp::new((*hp).min(*derived.hp_max));
        *wounds_max = derived.wounds_max;
        *wounds = Wounds::new((*wounds).min(*derived.wounds_max));
        // Bottle is dormant (no mid-battle drain path yet), so it tracks its derived max.
        *bottle = derived.bottle;
    }
}
