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

pub(crate) fn fx_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&FxFlash>();
    q.iter(app.world()).count()
}

pub(crate) fn single_flash(app: &mut App) -> Option<(bevy::math::Vec3, Option<usize>)> {
    let mut q = app.world_mut().query::<(&FxFlash, &Sprite, &Transform)>();
    let mut found: Option<(bevy::math::Vec3, Option<usize>)> = None;
    for (_, sprite, transform) in q.iter(app.world()) {
        if found.is_some() {
            return None;
        }
        found = Some((
            transform.translation,
            sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
        ));
    }
    found
}

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
    app.update();
    sim
}

pub(crate) fn fct_pops(app: &mut App) -> Vec<(String, bevy::prelude::Color)> {
    let mut q = app
        .world_mut()
        .query::<(&FloatingCombatText, &Text2d, &TextColor)>();
    q.iter(app.world())
        .map(|(_, text, color)| ((**text).clone(), color.0))
        .collect()
}

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

pub(crate) fn fct_pop_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&FloatingCombatText>();
    q.iter(app.world()).count()
}

pub(crate) fn fct_pops_with_y(app: &mut App) -> Vec<(String, f32)> {
    let mut q = app
        .world_mut()
        .query::<(&FloatingCombatText, &Text2d, &Transform)>();
    q.iter(app.world())
        .map(|(_, text, transform)| ((**text).clone(), transform.translation.y))
        .collect()
}

pub(crate) fn pop_y_for(pops: &[(String, f32)], text: &str) -> Option<f32> {
    pops.iter().find(|(t, _)| t == text).map(|(_, y)| *y)
}

pub(crate) fn pop_count_for(pops: &[(String, bevy::prelude::Color)], text: &str) -> usize {
    pops.iter().filter(|(t, _)| t == text).count()
}
