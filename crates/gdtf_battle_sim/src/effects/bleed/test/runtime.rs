use bevy::prelude::{App, MinimalPlugins};

use super::support::{
    BleedingOut, ENEMY, LifeState, PLAYER, bleed_rate, bleeding_ganger, drain_bleeding, end_turn,
    life_of, live_app, seed_battle_resources, wounds_of,
};
use crate::battle::BattleSimPlugin;

#[test]
fn a_full_round_bleeds_a_live_downed_ganger() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = live_app();
    let downed = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);

    end_turn(&mut app);

    assert_eq!(
        wounds_of(&app, downed),
        start - rate,
        "one full round (a player End Turn) must drain exactly bleed_rate from a live \
         Downed ganger — the bleed-out clock is wired into the turn cycle (GTW-336)",
    );
    assert_eq!(
        life_of(&app, downed),
        LifeState::Downed,
        "a non-lethal round leaves the ganger Downed",
    );

    let bled = drain_bleeding(&mut app);
    assert_eq!(
        bled.iter().filter(|b| b.ganger == downed).count(),
        1,
        "the round must emit exactly one Bleeding for the Downed ganger (the presenter's \
         FCT pop drains this buffer): {bled:?}",
    );
}

#[test]
fn the_clock_ticks_once_per_full_round_not_per_turn_boundary() {
    let rate = bleed_rate();
    let start = rate.saturating_mul(2).saturating_add(10);

    let mut app = live_app();
    let downed = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);

    end_turn(&mut app); 
    assert_eq!(
        wounds_of(&app, downed),
        start - rate,
        "after one full round the drop is exactly one bleed_rate (the enemy-phase start \
         ticks once — NOT once per TurnStarted, which would double it)",
    );

    end_turn(&mut app); 
    assert_eq!(
        wounds_of(&app, downed),
        start - rate * 2,
        "two full rounds drain exactly 2×bleed_rate — once per round, at the enemy phase",
    );
}

#[test]
fn the_live_clock_depletes_a_downed_ganger_to_dead() {
    let rate = bleed_rate();
    let start = rate.saturating_mul(2);

    let mut app = live_app();
    let downed = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);

    end_turn(&mut app);
    assert_eq!(
        life_of(&app, downed),
        LifeState::Downed,
        "the first round is non-lethal (Wounds still remain)",
    );

    end_turn(&mut app);
    assert_eq!(
        wounds_of(&app, downed),
        0,
        "bleeding to the floor leaves Wounds at 0 (saturating, no underflow)",
    );
    assert_eq!(
        life_of(&app, downed),
        LifeState::Dead,
        "the live clock emptying the pool transitions the ganger to Dead (the §9 gate)",
    );
}

#[test]
fn the_live_clock_skips_alive_dead_and_stabilized_gangers() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = live_app();
    let alive = bleeding_ganger(&mut app, PLAYER, LifeState::Alive, start);
    let dead = bleeding_ganger(&mut app, PLAYER, LifeState::Dead, start);
    let downed = bleeding_ganger(&mut app, ENEMY, LifeState::Downed, start);
    let stabilized = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);
    app.world_mut()
        .entity_mut(stabilized)
        .remove::<BleedingOut>();

    end_turn(&mut app);

    assert_eq!(
        wounds_of(&app, downed),
        start - rate,
        "the Downed ganger bleeds"
    );
    assert_eq!(
        wounds_of(&app, alive),
        start,
        "an Alive ganger is up — never bleeds"
    );
    assert_eq!(
        wounds_of(&app, dead),
        start,
        "a Dead ganger is a corpse — never bleeds"
    );
    assert_eq!(
        wounds_of(&app, stabilized),
        start,
        "a stabilized Downed ganger's clock is halted — no drain (Wounds already lost stay \
         lost; it remains Downed)",
    );
    assert_eq!(
        life_of(&app, stabilized),
        LifeState::Downed,
        "the stabilized ganger remains Downed (the clock only halts; it does not revive)",
    );

    let bled = drain_bleeding(&mut app);
    assert_eq!(
        bled.len(),
        1,
        "exactly one Bleeding this round — only the un-stabilized Downed ganger: {bled:?}",
    );
    assert_eq!(
        bled.first().map(|b| b.ganger),
        Some(downed),
        "the single Bleeding carries the un-stabilized Downed ganger",
    );
}

#[test]
fn no_battle_in_progress_means_no_bleed() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    seed_battle_resources(&mut app);

    let downed = bleeding_ganger(&mut app, PLAYER, LifeState::Downed, start);

    end_turn(&mut app);

    assert_eq!(
        wounds_of(&app, downed),
        start,
        "with no BattleInProgress the Simulate band is gated off — the Downed ganger does \
         not bleed (the wiring adds no unconditional work)",
    );
    assert!(
        drain_bleeding(&mut app).is_empty(),
        "no Bleeding is emitted outside a live battle",
    );
}
