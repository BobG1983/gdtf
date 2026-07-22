//! GTW-545 (area-damage fields, child GTW-41f of GTW-41) — the SEEDING path: a situation's
//! authored `fields:` list is resolved against the [`FieldDefRegistry`] catalog and seeded into
//! the live [`FieldRegistry`] resource by the REAL `setup_battle` (the authoritative pour), and
//! the [`Situation`] `.ron` deserializes each field placement + stays unchanged when the
//! list is omitted.
//!
//! The clause contract this covers:
//!
//! - **fields seedable from situation RON**: an authored `fields:` list parses into
//!   [`FieldSpawn`]s and, through `setup_battle`, populates the [`FieldRegistry`] resource with
//!   one placed field per authored cell (PIN-DISCRIMINATING — fails if the seed loop is unwired).
//! - **serde-default byte-identity**: a situation `.ron` that omits `fields:` deserializes to an
//!   empty list (every EXISTING situation stays parse-valid).
//! - **abort-first on a bad key**: an authored field whose KEY is absent from the catalog aborts
//!   `setup_battle` with [`BattleSetupError::FieldNotFound`] and inserts NO [`FieldRegistry`]
//!   (no partial world).
//!
//! The per-round `tick_fields` drain (HP-decrement / immunity skip / expiry / the field-kills
//! gate) is covered END-TO-END by the in-crate unit tests (`effects::fields::test`), which
//! exercise the REAL `tick_fields` against a REAL `OccupancyGrid` + a worn-armor relationship;
//! this file owns the RON-seed path. NO pinned tunable magnitudes.

use bevy::{
    app::App,
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Commands, MinimalPlugins},
    scene::ScenePlugin,
};
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

/// The catalog field-type KEY the fixtures seed.
const TOXIC_KEY: &str = "toxic_waste_pool";

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A single-def [`FieldDefRegistry`] holding a Permanent toxic-pool field under [`TOXIC_KEY`]
/// (magnitudes are ARBITRARY mechanism, never pinned).
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

/// Run `setup_battle` on a fresh `MinimalPlugins` + `AssetPlugin` + `ScenePlugin` app against
/// the given situation + registries (with the field catalog attached), returning the app +
/// the setup `Result` so a test can assert on both the outcome and the seeded resources.
fn run_setup(
    situation: Situation,
    gangs: GangRegistry,
    field_defs: &FieldDefRegistry,
) -> (App, Result<(), BattleSetupError>) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
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
    // `run_system_once` returns Result<SystemOutput, RunSystemError>; unwrap the inner setup
    // Result without a denied `unwrap` on the outer.
    let setup = outcome.unwrap_or(Err(BattleSetupError::FieldNotFound {
        field: FieldKey::new("<run-failed>".to_owned()),
    }));
    (app, setup)
}

/// A minimal two-ganger situation with one authored toxic-pool field, built via the
/// `SituationBuilder` (which synthesizes the gang registry the placements resolve against).
fn situation_with_field(field_cell: CellLevel) -> (Situation, GangRegistry) {
    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(ground(1, 1), 0))
        .with_ganger(ganger_at(ground(2, 1), 1))
        .build_with_gangs();
    // Author one field placement (the builder has no field method — set the pub list directly,
    // exactly what a deserialized `.ron` would produce).
    situation.fields = vec![FieldSpawn::new(
        field_cell,
        FieldKey::new(TOXIC_KEY.to_owned()),
    )];
    (situation, gangs)
}

// === fields seedable from situation RON: setup_battle populates the FieldRegistry. ===

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

// === serde-default byte-identity: a situation omitting `fields:` deserializes to an empty list. ===

#[test]
fn situation_without_fields_deserializes_to_an_empty_list() {
    // A minimal situation `.ron` that omits `fields:` entirely — every field is
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

// === fields seedable from situation RON: an authored `fields:` list parses. ===

#[test]
fn authored_fields_ron_parses_into_field_spawns() {
    // A situation `.ron` authoring a single toxic-pool field at (4, 4, 0).
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

// === abort-first on a bad key: an unknown field key fails FieldNotFound, inserts no registry. ===

#[test]
fn unknown_field_key_aborts_with_field_not_found() {
    let (mut situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(ground(1, 1), 0))
        .with_ganger(ganger_at(ground(2, 1), 1))
        .build_with_gangs();
    // Author a field whose KEY is NOT in the catalog.
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

// === a Turns field seeds with its authored countdown (the duration RON round-trips). ===

#[test]
fn turns_field_def_round_trips_through_ron() {
    // A field def RON authoring a 3-turn electrified floor immune to Plated.
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
