//! Per-effect spawn application — each attachment effect lands on the CORRECT stat of the
//! really-spawned weapon entity, and an empty list applies nothing (identity).

use bevy::{app::App, prelude::Entity};
use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect, ReloadTimeScale, WeaponBraceBonus},
    ganger::Direction,
    magazine::Magazine,
    test_support::SituationBuilder,
    weapon::{Accuracy, DamageType, MagazineSize, Silenced, Stable},
};

use super::harness::*;

/// Spawn a lone player wielding the attachment-bearing weapon; return its weapon entity (after
/// the post-spawn application has settled).
fn spawn_lone_player_weapon(effects: Vec<AttachmentEffect>) -> (App, Entity) {
    let mut app = battle_app(effects);
    // A lone player + a distant enemy so setup's win/loss census stays two-sided.
    let situation = SituationBuilder::new()
        .with_gangers([
            player_at(ground(5, 5), Direction::East),
            enemy_at(ground(20, 20)),
        ])
        .build_with_gangs();
    drive_setup(&mut app, situation);
    let Some(player) = ganger_of(&mut app, PLAYER) else {
        unreachable!("setup spawns one player");
    };
    let Some(weapon) = weapon_entity_of(&mut app, player) else {
        unreachable!("the player wields a ranged weapon entity");
    };
    (app, weapon)
}

// ── Each effect applies to the CORRECT stat on the REAL spawn ────────────────────

#[test]
fn silence_effect_spawns_the_silenced_component() {
    let (app, weapon) = spawn_lone_player_weapon(vec![AttachmentEffect::Silence]);
    assert!(
        app.world().get::<Silenced>(weapon).is_some(),
        "a Silence attachment fits the Silenced tag on the weapon entity (post-spawn apply)",
    );
}

#[test]
fn aim_effect_raises_accuracy() {
    // The HEADLINE fix: a sight boosts AIM (Accuracy), not stability. Baseline authored 5.0.
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app.world().get::<Accuracy>(base_weapon).map(|a| **a);
    let (aim_app, aim_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::Aim(AimDelta::new(0.5))]);
    let aimed = aim_app.world().get::<Accuracy>(aim_weapon).map(|a| **a);
    let (Some(base), Some(aimed)) = (base, aimed) else {
        unreachable!("both weapons carry Accuracy");
    };
    assert!(
        aimed > base,
        "an Aim attachment raises the weapon's Accuracy (aimed {aimed} > baseline {base})",
    );
}

#[test]
fn stability_effect_inserts_a_weapon_brace_bonus() {
    // Stability maps to a graduated WeaponBraceBonus component (the brace bonus).
    let (app, weapon) = spawn_lone_player_weapon(vec![AttachmentEffect::Stability(
        WeaponBraceBonus::new(12.0),
    )]);
    assert!(
        app.world().get::<WeaponBraceBonus>(weapon).is_some(),
        "a Stability attachment inserts a WeaponBraceBonus (the §1a brace bonus) on the weapon",
    );
    // The un-braced baseline carries NO such component.
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    assert!(
        base_app
            .world()
            .get::<WeaponBraceBonus>(base_weapon)
            .is_none(),
        "an un-braced weapon carries no WeaponBraceBonus",
    );
}

#[test]
fn extra_ammo_effect_grows_the_magazine_size() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app
        .world()
        .get::<Magazine>(base_weapon)
        .map(|m| m.size().get());
    let (drum_app, drum_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::ExtraAmmo(MagazineSize::new(10))]);
    let drum = drum_app
        .world()
        .get::<Magazine>(drum_weapon)
        .map(|m| m.size().get());
    let (Some(base), Some(drum)) = (base, drum) else {
        unreachable!("both weapons carry a Magazine");
    };
    assert!(
        drum > base,
        "an ExtraAmmo attachment grows the magazine capacity (drum {drum} > baseline {base})",
    );
}

#[test]
fn reload_time_effect_lowers_the_magazine_reload_tu() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app
        .world()
        .get::<Magazine>(base_weapon)
        .map(|m| *m.reload_tu());
    let (fast_app, fast_weapon) = spawn_lone_player_weapon(vec![AttachmentEffect::ReloadTime(
        ReloadTimeScale::new(0.5),
    )]);
    let fast = fast_app
        .world()
        .get::<Magazine>(fast_weapon)
        .map(|m| *m.reload_tu());
    let (Some(base), Some(fast)) = (base, fast) else {
        unreachable!("both weapons carry a Magazine");
    };
    assert!(
        fast < base,
        "a ReloadTime attachment lowers the weapon's reload_tu (fast {fast} < baseline {base})",
    );
}

// ── IDENTITY: an empty attachments list spawns with no attachment effects ─────────

#[test]
fn empty_attachments_spawn_with_no_effects() {
    let (app, weapon) = spawn_lone_player_weapon(Vec::new());
    let world = app.world();
    assert!(
        world.get::<Silenced>(weapon).is_none(),
        "an un-attached weapon has NO Silenced sibling",
    );
    assert!(
        world.get::<WeaponBraceBonus>(weapon).is_none(),
        "an un-attached weapon has NO WeaponBraceBonus",
    );
    // The authored `stable: false` + DamageType + Accuracy are un-rewritten.
    assert_eq!(
        world.get::<Stable>(weapon).copied(),
        Some(Stable::new(false)),
        "an un-attached weapon keeps its authored `stable: false`",
    );
    assert_eq!(
        world.get::<DamageType>(weapon).copied(),
        Some(DamageType::Kinetic),
        "an un-attached weapon keeps its authored DamageType",
    );
    assert_eq!(
        world.get::<Accuracy>(weapon).map(|a| **a),
        Some(5.0),
        "an un-attached weapon keeps its authored Accuracy (no Aim effect applied)",
    );
}
