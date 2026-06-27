//! The LIVE re-derivation systems — the ONE projector helper [`rederive_one`] and the
//! two systems that drive it: [`rederive_stats_on_tuning_change`] (the GTW-384
//! `stat.tuning.ron` hot-reload path) and [`rederive_stats_on_injury_change`] (the
//! GTW-436 injury-ledger path).
//!
//! Both systems re-derive a spawned ganger's computed stats from its eight authored
//! attributes × the [`GangerStatTuning`] weights × its [`InflictedInjuries`] ledger
//! (the GTW-405 modifier layer), overwriting the derived skill stats + the MAX
//! components and CLAMPING the current pools to the new maxes (`min`) — never resetting
//! a damaged pool to full mid-battle (the GTW-374 clamp-not-reset contract).
//!
//! The two systems differ ONLY in their trigger (one on `resource_changed::<GangerStatTuning>`,
//! the other on `Changed<InflictedInjuries>`); they share the [`rederive_one`]
//! projection so the two paths can never diverge. The projection RE-SUMS the ledger's
//! deltas every time it runs (never applied-once), so a `stat.tuning.ron` hot-reload
//! RE-APPLIES the injury deltas by construction rather than wiping them — the GTW-405
//! single-source-of-truth invariant.
//!
//! Lives in the sim because it touches ganger ENTITIES (the derivation result lands on
//! their components); the asset plumbing stays in the app, which overwrites the
//! [`GangerStatTuning`] resource on a file Modified — these systems react to the
//! resulting resource / component change, so the sim never depends on the asset system
//! (the one-way dep direction holds, and there is no `&mut World`).

use bevy::{
    ecs::change_detection::DetectChanges,
    prelude::{Changed, Mut, Query, Res},
};

use crate::{
    ganger::{
        Aim, Bottle, Cool, Fight, GangerAttributes, Grit, Hp, HpMax, Luck, Morale, Reactions,
        Reflexes, Shooting, Speed, Strength, Toughness, Tu, TuMax, Wounds, WoundsMax,
        injury_projection::derive_stats_with_injuries,
    },
    injuries::InflictedInjuries,
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
/// they are overwritten wholesale on a re-derive, with the GTW-436 skill-layer deltas
/// folded in).
type SkillWrite<'w> = (
    Mut<'w, Shooting>,
    Mut<'w, Fight>,
    Mut<'w, Reactions>,
    Mut<'w, Morale>,
);

/// The write-side pool + max components (the maxes are overwritten with the GTW-436
/// pool-layer deltas folded in; the current pools are CLAMPED to the new maxes — never
/// reset to full).
type PoolWrite<'w> = (
    Mut<'w, Tu>,
    Mut<'w, TuMax>,
    Mut<'w, Hp>,
    Mut<'w, HpMax>,
    Mut<'w, Wounds>,
    Mut<'w, WoundsMax>,
    Mut<'w, Bottle>,
);

/// **The ONE projection helper** — re-derive one ganger's computed stats from its base
/// attributes × `tuning` × its injury `ledger` (the GTW-405 modifier layer), writing
/// the derived skill stats + the MAX components and CLAMPING the current pools to the
/// new maxes (GTW-384 clamp-not-reset; GTW-436 injury layer).
///
/// Called from BOTH re-derive systems so the tuning-change and injury-change paths can
/// never diverge — they differ only in their trigger, not their projection. The
/// projection RE-SUMS the ledger's deltas (via
/// [`derive_stats_with_injuries`](crate::ganger::injury_projection::derive_stats_with_injuries)),
/// never applies-once, so the GTW-405 single-source-of-truth invariant holds: every
/// stored stat is `f(BaseAttributes, GangerStatTuning, InflictedInjuries)`.
///
/// The skill stats ([`Shooting`] / [`Fight`] / [`Reactions`] / [`Morale`]) are
/// overwritten wholesale (they are not pools). The MAX components
/// ([`TuMax`] / [`HpMax`] / [`WoundsMax`]) take the new docked-derived value; the
/// CURRENT pools ([`Tu`] / [`Hp`] / [`Wounds`]) are clamped to the new max via `min`,
/// so a ganger already damaged keeps its damage rather than snapping back to full (and
/// because a pool `Modify` floors the new max at `1`, the clamp can never itself reduce
/// a live current pool to `0`). [`Bottle`] takes the freshly derived value (the dormant
/// psychological life pool has no mid-battle damage path yet, so it tracks its max).
fn rederive_one(
    base: &GangerAttributes,
    tuning: &GangerStatTuning,
    ledger: &InflictedInjuries,
    skills: SkillWrite,
    pools: PoolWrite,
) {
    let derived = derive_stats_with_injuries(base, tuning, ledger);

    let (mut shooting, mut fight, mut reactions, mut morale) = skills;
    // Skill stats are not pools — overwrite wholesale (the derived skill already carries
    // the GTW-436 skill-layer delta).
    *shooting = derived.shooting;
    *fight = derived.fight;
    *reactions = derived.reactions;
    *morale = derived.morale;

    let (mut tu, mut tu_max, mut hp, mut hp_max, mut wounds, mut wounds_max, mut bottle) = pools;
    // MAXES take the new docked-derived ceiling; CURRENT pools are CLAMPED to the new max
    // (`min`), never reset to full — a damaged ganger keeps its damage mid-battle (the
    // GTW-374 clamp-not-reset contract).
    *tu_max = derived.tu_max;
    *tu = Tu::new((*tu).min(*derived.tu_max));
    *hp_max = derived.hp_max;
    *hp = Hp::new((*hp).min(*derived.hp_max));
    *wounds_max = derived.wounds_max;
    *wounds = Wounds::new((*wounds).min(*derived.wounds_max));
    // Bottle is dormant (no mid-battle drain path yet), so it tracks its derived max.
    *bottle = derived.bottle;
}

/// Reconstruct a ganger's [`GangerAttributes`] record from its eight queried attribute
/// components — the shared read-assembly both systems feed into [`rederive_one`].
const fn attributes_of(read: &AttributeRead) -> GangerAttributes {
    let (speed, aim, strength, toughness, reflexes, cool, grit, luck) = *read;
    GangerAttributes {
        speed:     *speed,
        aim:       *aim,
        strength:  *strength,
        toughness: *toughness,
        reflexes:  *reflexes,
        cool:      *cool,
        grit:      *grit,
        luck:      *luck,
    }
}

/// `Update`: when the [`GangerStatTuning`] resource CHANGES (the app-side hot-reload
/// overwrites it on a `stat.tuning.ron` edit, GTW-374 pattern), re-derive EVERY spawned
/// ganger's computed stats from its eight authored attributes × the NEW tuning × its
/// injury ledger (GTW-384 + GTW-436), via the shared `rederive_one` projection.
///
/// The attributes are the slowly-changing raw potential — they are NOT touched here
/// (only an authored situation / use-improvement edits them). The injury `ledger` is
/// read OPTIONALLY ([`Option`]`<&`[`InflictedInjuries`]`>`): a ganger with no ledger
/// (the component not yet inserted) re-derives with a zero-delta default — EXACTLY the
/// GTW-384 behavior — so this refactor-to-delegate is behavior-preserving for the
/// no-injury case. A ganger WITH a ledger has its injury deltas RE-APPLIED (not wiped)
/// on the tuning edit — the GTW-405 hot-reload-survival invariant.
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
/// `Query` over the attribute + optional-ledger + derived components (write). The query
/// nests its component groups into sub-tuples to stay under Bevy's 16-element
/// `QueryData` arity cap.
pub fn rederive_stats_on_tuning_change(
    tuning: Option<Res<GangerStatTuning>>,
    mut gangers: Query<(
        AttributeRead,
        Option<&InflictedInjuries>,
        SkillWrite,
        PoolWrite,
    )>,
) {
    // Absent (pre-load) → nothing to re-derive; unchanged → not a tuning edit, skip.
    let Some(tuning) = tuning else {
        return;
    };
    if !tuning.is_changed() {
        return;
    }
    // A ganger with no ledger re-derives against this zero-delta default (the GTW-384
    // identity — no injuries → derive exactly as before).
    let empty = InflictedInjuries::default();
    for (read, ledger, skills, pools) in &mut gangers {
        let base = attributes_of(&read);
        let ledger = ledger.unwrap_or(&empty);
        rederive_one(&base, &tuning, ledger, skills, pools);
    }
}

/// `Update`: when a ganger's [`InflictedInjuries`] ledger CHANGES (a new injury was
/// inflicted — the GTW-437 apply boundary appends a `GainedInjury`, tripping
/// `Changed`), re-derive THAT ganger's computed stats from its base attributes × the
/// tuning × its NOW-updated ledger (GTW-436), via the shared `rederive_one`
/// projection — so a freshly-inflicted injury's deltas land on the derived stats
/// immediately.
///
/// The same projection as [`rederive_stats_on_tuning_change`], driven by a different
/// trigger: the `Changed<InflictedInjuries>` query filter, so only gangers whose ledger
/// changed this tick are re-projected (not every ganger every frame). The ledger is a
/// required (`&`) read here — the filter already restricts the query to gangers that
/// HAVE the component.
///
/// Self-guarding (`bevy-traps.md` #1): the tuning is an `Option<Res<GangerStatTuning>>`
/// read, with an early return when ABSENT (a harness that drives an injury without the
/// persistent-Load `GangerStatTuning` does not panic). Param-only (`bevy-traps.md` #7):
/// an `Option<Res>` + one filtered `Query`, no `&mut World`.
pub fn rederive_stats_on_injury_change(
    tuning: Option<Res<GangerStatTuning>>,
    mut gangers: Query<
        (AttributeRead, &InflictedInjuries, SkillWrite, PoolWrite),
        Changed<InflictedInjuries>,
    >,
) {
    // Absent (pre-load) → no tuning to project against; skip (the injury delta will be
    // applied once a tuning exists and a re-derive runs).
    let Some(tuning) = tuning else {
        return;
    };
    for (read, ledger, skills, pools) in &mut gangers {
        let base = attributes_of(&read);
        rederive_one(&base, &tuning, ledger, skills, pools);
    }
}
