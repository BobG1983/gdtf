use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::MeleeButton;
use gdtf_battle_input::{SelectedShooter, contextual::ContextualActSystems};
use gdtf_battle_sim::{
    acts::{MeleeRequested, MeleeTarget, melee_tu_cost},
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Facing, Tu},
    metric::{Cell, CellLevel, Level},
    prelude::{Direction, Faction, Position, Stance, StanceKind},
    weapon::{FightMode, FightModeKind, FightModeSpec, MeleeWeapon, Strikes, TuCost, WieldedBy},
};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

/// One swing mode, so the weapon the actor wields quotes a cost through `melee_tu_cost`.
fn a_swing() -> FightMode {
    FightMode::new(vec![FightModeSpec::new(
        FightModeKind::Swing,
        TuCost::new(5),
        Strikes::new(1),
    )])
}

/// What one strike with the wielded weapon charges, from the sim's own cost helper.
fn strike_cost() -> Tu {
    melee_tu_cost(&a_swing())
}

fn spawn_melee_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app
        .world_mut()
        .spawn((
            at(x, y),
            Faction::new(gang),
            Stance::new(StanceKind::Standing),
            Facing::new(Direction::East),
            strike_cost(),
        ))
        .id();
    app.world_mut()
        .spawn((WieldedBy::new(actor), MeleeWeapon, a_swing()));
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

fn add_melee_probe(app: &mut App) {
    app.init_resource::<MessageProbe<MeleeRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<MeleeRequested>.after(ContextualActSystems::Drain),
    );
}

fn melees(app: &App) -> Vec<MeleeRequested> {
    probed::<MeleeRequested>(app)
}

#[test]
fn adjacent_alive_enemy_in_los_offers_melee() {
    let mut app = battle_running_app();
    spawn_melee_actor(&mut app, 5, 5, 0);
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();

    assert!(
        melee_visible(&mut app),
        "an 8-adjacent alive enemy in LOS must reveal the Melee button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Melee must reveal the panel root",
    );
    assert!(
        !execute_visible(&mut app),
        "an ALIVE enemy is no Execute target (Execute needs a DOWNED enemy) -> hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
}

#[test]
fn non_adjacent_or_ally_does_not_offer_melee() {
    let mut app = battle_running_app();
    spawn_melee_actor(&mut app, 5, 5, 0);
    let enemy = spawn_alive_enemy(&mut app, 20, 20, 1);
    spawn_alive_enemy(&mut app, 5, 6, 0);
    app.update();

    assert!(
        !melee_visible(&mut app),
        "a non-adjacent enemy + an adjacent ALLY offer NO melee (the button stays hidden)",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(enemy) {
        *pos = at(6, 6);
    }
    app.update();
    assert!(
        melee_visible(&mut app),
        "moving the alive enemy into 8-adjacency reveals the Melee button (discriminating)",
    );
}

#[test]
fn bare_ground_offers_no_melee_but_a_live_cover_entry_does() {
    let mut app = battle_running_app();
    // The scan answers None when the ledger resource is missing, so it must be present.
    app.world_mut().insert_resource(CoverLedger::new());
    spawn_melee_actor(&mut app, 5, 5, 0);
    app.update();

    assert!(
        !melee_visible(&mut app),
        "an empty cover ledger and no enemy in reach must offer NO melee — bare ground is never \
         a structure target",
    );

    let mut ledger = CoverLedger::new();
    ledger.insert(
        CellLevel::new(Cell::new(6, 5), Level::new(0)),
        CoverEntry::seeded(
            CoverHp::new(30),
            HeightBand::Low,
            ArmorProtection::new(2),
            ArmorHardness::new(1),
        ),
    );
    app.world_mut().insert_resource(ledger);
    app.update();

    assert!(
        melee_visible(&mut app),
        "a LIVE cover entry in an adjacent cell must offer melee, or the bare-ground half above \
         passes vacuously",
    );
}

/// An attacker one TU short of a strike, beside a legal target it can see.
fn an_attacker_one_tu_short() -> (App, Entity) {
    let mut app = battle_running_app();
    add_melee_probe(&mut app);
    let attacker = spawn_melee_actor(&mut app, 5, 5, 0);
    spawn_alive_enemy(&mut app, 6, 6, 1);

    let cost = strike_cost();
    assert!(
        *cost > 0,
        "a strike must cost something or an unaffordable pool cannot exist",
    );
    set_pool(&mut app, attacker, Tu::new(cost.saturating_sub(1)));
    app.update();
    (app, attacker)
}

#[test]
fn a_pool_below_the_strike_cost_still_offers_the_button_greyed_out() {
    let (mut app, _attacker) = an_attacker_one_tu_short();

    assert!(
        melee_visible(&mut app),
        "an unaffordable strike is still OFFERED — the player sees the act and why it is barred",
    );
    assert!(
        button_greyed::<MeleeButton>(&mut app),
        "a pool below the wielded weapon's strike cost greys the button out",
    );
    let (greyed, _idle) = greyed_and_idle_fills(&app);
    assert_eq!(
        button_fill::<MeleeButton>(&mut app),
        Some(greyed),
        "the greyed button is painted the theme's disabled fill",
    );
}

#[test]
fn neither_a_press_nor_a_slot_key_fires_a_greyed_melee() {
    let (mut app, _attacker) = an_attacker_one_tu_short();
    let melee_btn = the_only::<MeleeButton>(
        &mut app,
        "the panel must still offer exactly one Melee button while it is greyed out, or there is \
         nothing for this case to press",
    );

    press_ui_button(&mut app, melee_btn);
    press_digit(&mut app, KeyCode::Digit1);
    app.update();

    assert!(
        melees(&app).is_empty(),
        "a greyed button emits nothing from a mouse press or from its slot key",
    );
}

#[test]
fn raising_the_pool_to_the_cost_re_enables_and_repaints_the_melee_button() {
    let (mut app, attacker) = an_attacker_one_tu_short();
    assert!(
        button_greyed::<MeleeButton>(&mut app),
        "sanity: the button is greyed before the pool is raised",
    );

    set_pool(&mut app, attacker, strike_cost());
    app.update();
    app.update();

    assert!(
        melee_visible(&mut app),
        "a pool that covers the cost keeps the button on screen",
    );
    assert!(
        !button_greyed::<MeleeButton>(&mut app),
        "a pool that covers the cost drops the disabled marker",
    );
    let (greyed, idle) = greyed_and_idle_fills(&app);
    assert_ne!(
        greyed, idle,
        "the theme must paint disabled and idle differently or this case cannot discriminate",
    );
    assert_eq!(
        button_fill::<MeleeButton>(&mut app),
        Some(idle),
        "re-enabling repaints the button back to its enabled fill",
    );

    let melee_btn = the_only::<MeleeButton>(
        &mut app,
        "the panel must offer exactly one Melee button once the pool covers the cost",
    );
    press_ui_button(&mut app, melee_btn);
    app.update();
    assert_eq!(
        melees(&app).len(),
        1,
        "the re-enabled button presses through to exactly one MeleeRequested",
    );
}

#[test]
fn pressing_melee_emits_melee_requested_for_target() {
    let mut app = battle_running_app();
    add_melee_probe(&mut app);
    let attacker = spawn_melee_actor(&mut app, 5, 5, 0);
    let target = spawn_alive_enemy(&mut app, 6, 6, 1);

    app.update();
    assert!(
        melee_visible(&mut app),
        "sanity: the Melee button is offered before the press",
    );
    let melee_btn = the_only::<MeleeButton>(
        &mut app,
        "the panel must offer exactly one Melee button to press",
    );

    press_ui_button(&mut app, melee_btn);
    app.update();

    let emitted = melees(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Melee with a target offered must emit exactly one MeleeRequested",
    );
    assert_eq!(
        emitted[0].attacker, attacker,
        "the attacker is the SelectedShooter",
    );
    assert_eq!(
        emitted[0].target,
        MeleeTarget::Ganger(target),
        "the target is the carried opposing neighbour (the ganger melee form)",
    );
}
