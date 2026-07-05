//! The invalid vertical-link validation abort.

use super::super::support::*;

/// A bad authored vertical link (dangling endpoint) makes `setup_battle` return
/// `Err(DanglingCell)` and spawn NOTHING — validation runs first, aborting the
/// setup before any entity or resource is created (the no-panic contract).
#[test]
fn setup_aborts_on_invalid_vertical_link() {
    let present = key(4, 4, 0);
    let missing = key(4, 4, 1); // never authored
    let link = VerticalLink::new(present, missing, LinkKind::stair());
    let (situation, gangs) = SituationBuilder::new()
        .with_ganger(ganger_at(key(0, 0, 0), 0))
        .slab_at(present) // only `present` authored — `missing` dangles
        .vertical_link(link)
        .build_with_gangs();

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let registry = test_registry();
    let armor = test_armor_registry();
    // GTW-384: setup derives stats from the default stat tuning (irrelevant here — these
    // tests abort BEFORE the spawn loop, but the signature requires the argument).
    let stat_tuning = GangerStatTuning::default();
    // GTW-505: the melee registry holds the `fists` default, so an un-authored ganger's
    // melee weapon resolves (these tests assert the OTHER abort, not MeleeWeaponNotFound).
    let melee = test_melee_weapon_registry();
    let result = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            setup_battle(
                &situation,
                // GTW-396: these tests abort before the terrain-resolution path
                // (vertical-link / weapon / armor errors fire first). Pass terrain: None
                // and the fallback floor cost — no terrain keys are authored in these
                // fixtures, so no registry is needed.
                BattleRegistries::new(&gangs, &registry, &melee, &armor, &stat_tuning, None),
                crate::tuning::CombatTuning::default().move_costs.open,
                &mut commands,
            )
        });

    // The one-shot system ran; the inner setup returned the typed error (now wrapped
    // in BattleSetupError::InvalidLink — GTW-257).
    assert!(result.is_ok(), "the one-shot system must run");
    let Ok(setup_result) = result else {
        return;
    };
    assert_eq!(
        setup_result.err(),
        Some(BattleSetupError::InvalidLink(
            InvalidVerticalLink::DanglingCell { link },
        )),
        "an invalid vertical link must abort setup with the typed error",
    );

    // Nothing was spawned (validation aborted before the spawn loop).
    app.world_mut().flush();
    let world = app.world_mut();
    let mut q = world.query::<&Wears>();
    assert_eq!(
        q.iter(world).count(),
        0,
        "a validation abort must spawn no gangers",
    );
    assert!(
        world.get_resource::<CoverLedger>().is_none(),
        "a validation abort must insert no resources",
    );
}
