use gdtf_battle_sim::{
    effects::attachments::{AimDelta, AttachmentEffect},
    magazine::Magazine,
    weapon::{
        Accuracy, FireMode, FireModeSpec, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, Silenced,
    },
};

use super::harness::*;

const fn granted_burst_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Full,
        ModeConeMult::new(1.7),
        ModeTuPercent::new(0.6),
        ModeShots::new(6),
    )
}


#[test]
fn gain_fire_mode_effect_adds_a_fire_mode() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base_modes = base_app
        .world()
        .get::<FireMode>(base_weapon)
        .map(|modes| modes.len());
    let (gain_app, gain_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::GainFireMode(granted_burst_mode())]);
    let gain_modes = gain_app
        .world()
        .get::<FireMode>(gain_weapon)
        .map(|modes| modes.len());
    let (Some(base_modes), Some(gain_modes)) = (base_modes, gain_modes) else {
        unreachable!("both weapons carry a FireMode selector");
    };
    assert_eq!(
        gain_modes,
        base_modes + 1,
        "a GainFireMode attachment adds one mode to the weapon's selector \
         (gained {gain_modes} = baseline {base_modes} + 1)",
    );
    let has_granted = gain_app
        .world()
        .get::<FireMode>(gain_weapon)
        .is_some_and(|modes| modes.iter().any(|m| *m == granted_burst_mode()));
    assert!(
        has_granted,
        "the added mode is the GainFireMode payload (the granted Full-auto mode)",
    );
}


#[test]
fn aim_effect_raises_accuracy() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app.world().get::<Accuracy>(base_weapon).map(|a| **a);
    let (aim_app, aim_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::Aim(AimDelta::new(0.6))]);
    let aimed = aim_app.world().get::<Accuracy>(aim_weapon).map(|a| **a);
    let (Some(base), Some(aimed)) = (base, aimed) else {
        unreachable!("both weapons carry Accuracy");
    };
    assert!(
        aimed > base,
        "an Aim attachment raises the weapon's Accuracy (aimed {aimed} > baseline {base}) — a \
         sight boosts AIM, not stability",
    );
}


#[test]
fn extra_ammo_effect_grows_the_magazine() {
    let (base_app, base_weapon) = spawn_lone_player_weapon(Vec::new());
    let base = base_app
        .world()
        .get::<Magazine>(base_weapon)
        .map(|m| m.size().get());
    let (drum_app, drum_weapon) =
        spawn_lone_player_weapon(vec![AttachmentEffect::ExtraAmmo(MagazineSize::new(12))]);
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
fn empty_attachments_spawn_with_no_effects() {
    let (app, weapon) = spawn_lone_player_weapon(Vec::new());
    let world = app.world();
    assert!(
        world.get::<Silenced>(weapon).is_none(),
        "an un-attached weapon has NO Silenced sibling",
    );
    assert_eq!(
        world.get::<FireMode>(weapon).map(|modes| modes.len()),
        Some(1),
        "an un-attached weapon keeps its single authored fire mode (no GainFireMode applied)",
    );
    assert_eq!(
        world.get::<Accuracy>(weapon).map(|a| **a),
        Some(5.0),
        "an un-attached weapon keeps its authored Accuracy (no Aim effect applied)",
    );
}
