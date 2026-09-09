//! Field seeding from situation setup and on-death leave-field effects.
use bevy::{
    app::App,
    ecs::system::RunSystemOnce,
    prelude::{Commands, MinimalPlugins},
    scene::ScenePlugin,
};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_sim::{
    armor::ArmorType,
    effects::fields::{
        FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
        ImmuneArmorTypes,
    },
    ganger::GangRegistry,
    metric::{Cell, CellLevel, Level},
    situation::{BattleRegistries, BattleSetupError, FieldSpawn, Situation, setup_battle},
    test_support::{
        SituationBuilder, field_turns, ganger_at, test_armor_registry, test_melee_weapon_registry,
        test_terrain_registry, test_weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
    weapon::DamageType,
};

const TOXIC_KEY: &str = "toxic_waste_pool";

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

fn catalog() -> FieldDefRegistry {
    FieldDefRegistry::new([(
        FieldKey::new(TOXIC_KEY.to_owned()),
        FieldDef::new(
            FieldDamage::new(3),
            DamageType::Chem,
            ImmuneArmorTypes::new([ArmorType::Flak, ArmorType::Hazard]),
            FieldDuration::Permanent,
        ),
    )])
}

fn run_setup(
    situation: Situation,
    gangs: GangRegistry,
    field_defs: &FieldDefRegistry,
) -> (App, Result<(), BattleSetupError>) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, unwatched_asset_plugin(), ScenePlugin));
    let weapons = test_weapon_registry();
    let melee = test_melee_weapon_registry();
    let armor = test_armor_registry();
    let stat_tuning = GangerStatTuning::default();
    let terrain = test_terrain_registry();
    let fallback_floor_cost = CombatTuning::default().move_costs.open;
    let field_defs = field_defs.clone();
    let outcome = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                BattleRegistries::new(
                    &gangs,
                    &weapons,
                    &melee,
                    &armor,
                    &stat_tuning,
                    Some(&terrain),
                )
                .with_field_defs(&field_defs),
                fallback_floor_cost,
                &mut commands,
            )
            .map(|_setup| ())
        });
    let setup = outcome.unwrap_or(Err(BattleSetupError::FieldNotFound {
        field: FieldKey::new("<run-failed>".to_owned()),
    }));
    (app, setup)
}

fn situation_with_field(field_cell: CellLevel) -> (Situation, GangRegistry) {
    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(ground(1, 1), 0))
        .with_ganger(ganger_at(ground(2, 1), 1))
        .build_with_gangs();
    situation.fields = vec![FieldSpawn::new(
        field_cell,
        FieldKey::new(TOXIC_KEY.to_owned()),
    )];
    (situation, gangs)
}

#[test]
fn authored_field_seeds_the_field_registry() {
    let field_cell = ground(4, 4);
    let (situation, gangs) = situation_with_field(field_cell);
    let (app, setup) = run_setup(situation, gangs, &catalog());

    assert!(
        setup.is_ok(),
        "setup_battle must succeed on a valid field: {setup:?}"
    );
    let registry = app.world().get_resource::<FieldRegistry>();
    assert!(
        registry.is_some(),
        "setup_battle must insert a FieldRegistry resource on success",
    );
    if let Some(registry) = registry {
        assert_eq!(
            registry.len(),
            1,
            "the one authored field placement seeds exactly one live field",
        );
        assert!(
            registry.field_at(&field_cell).is_some(),
            "the seeded field lands at the authored (cell, level)",
        );
    }
}

#[test]
fn situation_without_fields_deserializes_to_an_empty_list() {
    // `#[serde(default)]`, so this must parse and carry NO fields.
    let ron = "(gangers: [], grid_size: (width: 10, height: 10, levels: 1))";
    let parsed = ron::de::from_str::<Situation>(ron);
    assert!(
        parsed.is_ok(),
        "a situation omitting `fields:` must still deserialize (serde-default): {:?}",
        parsed.as_ref().err(),
    );
    if let Ok(situation) = parsed {
        assert!(
            situation.fields.is_empty(),
            "an omitted `fields:` list defaults to empty (existing situations stay unchanged)",
        );
    }
}

#[test]
fn authored_fields_ron_parses_into_field_spawns() {
    let ron = "(gangers: [], grid_size: (width: 10, height: 10, levels: 1), \
                fields: [(at: (cell: (x: 4, y: 4), level: 0), field: \"toxic_waste_pool\")])";
    let parsed = ron::de::from_str::<Situation>(ron);
    assert!(
        parsed.is_ok(),
        "an authored `fields:` list must deserialize: {:?}",
        parsed.as_ref().err(),
    );
    if let Ok(situation) = parsed {
        assert_eq!(
            situation.fields.len(),
            1,
            "the authored field placement parses into one FieldSpawn",
        );
        assert_eq!(
            situation.fields.first().map(|f| f.at),
            Some(ground(4, 4)),
            "the FieldSpawn carries its authored (cell, level)",
        );
        assert_eq!(
            situation.fields.first().map(|f| (*f.field).clone()),
            Some(TOXIC_KEY.to_owned()),
            "the FieldSpawn carries its authored field-type KEY",
        );
    }
}

#[test]
fn unknown_field_key_aborts_with_field_not_found() {
    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(ground(1, 1), 0))
        .with_ganger(ganger_at(ground(2, 1), 1))
        .build_with_gangs();
    situation.fields = vec![FieldSpawn::new(
        ground(4, 4),
        FieldKey::new("no_such_field".to_owned()),
    )];
    let (app, setup) = run_setup(situation, gangs, &catalog());

    assert!(
        matches!(setup, Err(BattleSetupError::FieldNotFound { .. })),
        "an unknown field key must abort with FieldNotFound: {setup:?}",
    );
    assert!(
        app.world().get_resource::<FieldRegistry>().is_none(),
        "an aborted setup inserts NO FieldRegistry (abort-first, no partial world)",
    );
}

#[test]
fn turns_field_def_round_trips_through_ron() {
    let ron = "(damage: 4, damage_type: Shock, immune_armor_types: [Plated], \
                duration: Turns(3))";
    let parsed = ron::de::from_str::<FieldDef>(ron);
    assert!(
        parsed.is_ok(),
        "a Turns field def must deserialize: {:?}",
        parsed.as_ref().err(),
    );
    if let Ok(def) = parsed {
        assert_eq!(
            def.duration,
            FieldDuration::Turns(field_turns(3)),
            "the authored Turns duration round-trips",
        );
        assert!(
            def.immune_armor_types.contains(&ArmorType::Plated),
            "the authored immune set round-trips",
        );
    }
}
