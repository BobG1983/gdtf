//! Cross-surface world-readback probes + ganger/report fixtures shared by the
//! `fx_draw` concern modules.

use bevy::{
    app::App, prelude::Text2d, sprite::Sprite, text::TextColor, transform::components::Transform,
};
use gdtf_battle_presenter::{FloatingCombatText, FxFlash};
use gdtf_battle_sim::{
    armor::BodyPart,
    ganger::{Aiming, Facing},
    matchup::Matchup,
    prelude::{
        Cell, CellLevel, Direction, Faction, Level, LifeState, Position, Stance, StanceKind,
    },
    resolve_and_apply::{AppliedDamage, HitReport},
    resolve_coarse::ShotKind,
    resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage},
    severity::Severity,
    test_support::GangerEntityBuilder,
};

/// Spawns a ganger entity carrying a `Position` at `cell`/`level` plus `Wounds(wounds)`, and
/// returns its `Entity` (the readers look it up by that entity).
pub(crate) fn wounded_ganger(
    app: &mut App,
    cell: Cell,
    level: Level,
    wounds: u8,
) -> bevy::ecs::entity::Entity {
    GangerEntityBuilder::new()
        .at(CellLevel::new(cell, level))
        .wounds(wounds)
        .spawn(app.world_mut())
}

/// Counts the `FxFlash` entities currently in the world.
pub(crate) fn fx_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&FxFlash>();
    q.iter(app.world()).count()
}

/// The (translation, atlas index) of the SINGLE `FxFlash` sprite — asserts exactly one exists.
/// Returns `None` if zero or more than one flash exists (the caller asserts `Some`).
pub(crate) fn single_flash(app: &mut App) -> Option<(bevy::math::Vec3, Option<usize>)> {
    let mut q = app.world_mut().query::<(&FxFlash, &Sprite, &Transform)>();
    let mut found: Option<(bevy::math::Vec3, Option<usize>)> = None;
    for (_, sprite, transform) in q.iter(app.world()) {
        if found.is_some() {
            // More than one flash — the caller wants exactly one.
            return None;
        }
        found = Some((
            transform.translation,
            sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
        ));
    }
    found
}

/// Spawns a REAL sim ganger (the components `spawn_ganger_sprites` queries — `Position`,
/// `Faction`, `Facing`, `Stance`, `Aiming`, `LifeState`, the GTW-631 appearance-classifier
/// input set) at `cell`/`level` and drives one `update()` so the
/// presenter's real spawn system builds its sprite and registers the `sim Entity -> sprite
/// Entity` link in `GangerSprites`. Returns the sim `Entity` (the `ShotKind::Ganger`
/// payload). `BattleInProgress` must already be resident (the spawn gate).
pub(crate) fn spawn_sim_ganger_with_sprite(
    app: &mut App,
    cell: Cell,
    level: Level,
) -> bevy::ecs::entity::Entity {
    let at = CellLevel::new(cell, level);
    let sim = app
        .world_mut()
        .spawn((
            Position::new(at),
            Faction::new(0),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            LifeState::Alive,
        ))
        .id();
    // Drive the real spawn_ganger_sprites system (gated on CharacterRoles + TopDownAtlases +
    // BattleInProgress, all resident after settle) so the presenter sprite + GangerSprites
    // mapping exist before the shot is fired.
    app.update();
    sim
}

/// The `(text, alpha-1 color)` of every live `FloatingCombatText` pop — the rendered string and
/// its `TextColor` (full-alpha at spawn, before the first fade tick). Unordered.
pub(crate) fn fct_pops(app: &mut App) -> Vec<(String, bevy::prelude::Color)> {
    let mut q = app
        .world_mut()
        .query::<(&FloatingCombatText, &Text2d, &TextColor)>();
    q.iter(app.world())
        .map(|(_, text, color)| ((**text).clone(), color.0))
        .collect()
}

/// Whether the live pops contain a pop with exactly `text` whose color's RGB matches `color`'s
/// (alpha-agnostic, since the pop fades — but at spawn, pre-tick, it is still full alpha).
pub(crate) fn has_fct_pop(
    pops: &[(String, bevy::prelude::Color)],
    text: &str,
    color: bevy::prelude::Color,
) -> bool {
    let want = color.to_srgba();
    pops.iter().any(|(t, c)| {
        let got = c.to_srgba();
        t == text
            && (got.red - want.red).abs() < 0.001
            && (got.green - want.green).abs() < 0.001
            && (got.blue - want.blue).abs() < 0.001
    })
}

/// A ganger-hit `HitReport` for `part` with `hp` HP loss / `pen` penetration / `severity` tier
/// / `life_after` state, struck on `struck` — the report the FCT reader classifies.
/// (Not `const`: the GTW-573 ganger verdict is boxed, and `Box::new` is not const.)
pub(crate) fn ganger_hit_report(
    struck: bevy::ecs::entity::Entity,
    part: BodyPart,
    hp: i32,
    pen: i32,
    severity: Severity,
    life_after: LifeState,
) -> HitReport {
    HitReport {
        kind:    ShotKind::Ganger(struck),
        verdict: gdtf_battle_sim::resolve_and_apply::HitVerdict::Ganger(Box::new(
            gdtf_battle_sim::resolve_and_apply::GangerVerdict {
                target: struck,
                part,
                applied: AppliedDamage {
                    matchup: Matchup::Neutral,
                    hit: HitResult {
                        penetrating: PenetratingDamage::new(pen),
                        hp_damage:   HpDamage::new(hp),
                        wear:        IntegrityWear::new(0),
                    },
                    severity,
                    life_after,
                    wear: gdtf_battle_sim::armor_wear::ArmorWearOutcome::Unaffected,
                },
                injury: None,
                dot_applied: None,
            },
        )),
    }
}

/// The number of live `FloatingCombatText` pops currently in the world.
pub(crate) fn fct_pop_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&FloatingCombatText>();
    q.iter(app.world()).count()
}

/// The `(text, world y)` of every live `FloatingCombatText` pop — the rendered string and its
/// `Transform`'s world `y`. A pop's spawn `y` is `cell_to_world(cell, level).y` shifted DOWN by
/// `stack_slot × STACK_STEP_PX`, so the `y` is a direct readout of the pop's stacking slot when
/// read on its spawn frame (before `animate_floating_text` has risen it). Unordered.
pub(crate) fn fct_pops_with_y(app: &mut App) -> Vec<(String, f32)> {
    let mut q = app
        .world_mut()
        .query::<(&FloatingCombatText, &Text2d, &Transform)>();
    q.iter(app.world())
        .map(|(_, text, transform)| ((**text).clone(), transform.translation.y))
        .collect()
}

/// The world `y` of the (first) live pop whose rendered string equals `text`, or `None`.
pub(crate) fn pop_y_for(pops: &[(String, f32)], text: &str) -> Option<f32> {
    pops.iter().find(|(t, _)| t == text).map(|(_, y)| *y)
}

/// The number of live FCT pops whose rendered string equals `text` (any color) — distinguishes
/// "exactly one pop was spawned" from "none / many".
pub(crate) fn pop_count_for(pops: &[(String, bevy::prelude::Color)], text: &str) -> usize {
    pops.iter().filter(|(t, _)| t == text).count()
}
