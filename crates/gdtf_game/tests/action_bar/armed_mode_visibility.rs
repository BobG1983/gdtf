use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
        ArmorRegistry, ArmorSpec, ArmorType,
    },
    ganger::{Aim, Aiming, GangerName},
    prelude::{Cell, CellLevel, Faction, Level},
    situation::Situation,
    test_support::test_weapon_spec,
    tuning::CombatTuning,
    weapon::{FatalBias, WeaponName, WeaponRegistry, WeaponSpec},
};
use gdtf_content_families::situation::LoadedSituation;
use gdtf_game::test_support::{AppState, ModePanelRoot, ModeSingleButton};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::theme::default_theme;

use super::harness::*;

const PLAYER_WEAPON_KEY: &str = "test-weapon";

const PLAYER_ARMOR_KEY: &str = "test-armor";

const PLAYER_FACTION: u8 = 0;

fn armed_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(PLAYER_WEAPON_KEY.to_owned()),
        WeaponSpec {
            fatal_bias: FatalBias::new(0.0),
            ..test_weapon_spec()
        },
    )])
}

const fn arbitrary_armor() -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(1),
        ArmorIntegrity::new(2),
        ArmorHardness::new(3),
        ArmorType::DEFAULT,
    ))
}

fn armed_armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([(
        ArmorName::new(PLAYER_ARMOR_KEY.to_owned()),
        arbitrary_armor(),
    )])
}

fn armed_player_situation() -> (Situation, gdtf_battle_sim::ganger::GangRegistry) {
    use gdtf_battle_sim::test_support::{GangerSpawnBuilder, SituationBuilder};
    SituationBuilder::new()
        .with_ganger(
            GangerSpawnBuilder::new()
                .at(CellLevel::new(Cell::new(2, 5), Level::new(0)))
                .name(GangerName::new("Alex Mercer".to_owned()))
                .faction(Faction::new(PLAYER_FACTION))
                .aiming(Aiming::new(false))
                .aim(Aim::new(3.0))
                .armor(ArmorName::new(PLAYER_ARMOR_KEY.to_owned()))
                .weapon(WeaponName::new(PLAYER_WEAPON_KEY.to_owned()))
                .build(),
        )
        .build_with_gangs()
}

fn walk_app_with_situation(
    situation: Situation,
    gangs: gdtf_battle_sim::ganger::GangRegistry,
) -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(armed_registry());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::test_support::test_melee_weapon_registry());
    app.world_mut().insert_resource(armed_armor_registry());
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .insert_resource(LoadedSituation::new(situation));
    app
}

fn battle_running_app_with_situation(
    situation: Situation,
    gangs: gdtf_battle_sim::ganger::GangRegistry,
) -> App {
    let mut app = walk_app_with_situation(situation, gangs);
    drive_to_battle_running(&mut app);
    app
}

fn mode_panel_visibility(app: &mut App) -> Option<Visibility> {
    single_with::<ModePanelRoot>(app)
        .and_then(|panel| app.world().get::<Visibility>(panel).copied())
}

#[test]
fn mode_panel_hidden_when_nothing_armed_selected() {
    let mut app = battle_running_app();
    app.update();
    app.update();

    assert_eq!(
        mode_panel_visibility(&mut app),
        Some(Visibility::Hidden),
        "with nothing armed selected, the Mode panel must be Hidden (no empty Mode box)",
    );
}

#[test]
fn mode_panel_visible_when_armed_player_ganger_selected() {
    let (situation, gangs) = armed_player_situation();
    let mut app = battle_running_app_with_situation(situation, gangs);
    app.update();
    app.update();

    assert!(
        app.world()
            .get_resource::<SelectedShooter>()
            .is_some_and(|s| s.is_some()),
        "precondition: the real auto-select must have filled SelectedShooter with the player \
         ganger (no hand-inserted selection)",
    );
    assert_eq!(
        count_with::<ModeSingleButton>(&mut app),
        1,
        "the armed player ganger's single-mode weapon must build one Mode toggle",
    );
    assert_eq!(
        mode_panel_visibility(&mut app),
        Some(Visibility::Visible),
        "with an armed player ganger selected, the Mode panel must be Visible",
    );
}
