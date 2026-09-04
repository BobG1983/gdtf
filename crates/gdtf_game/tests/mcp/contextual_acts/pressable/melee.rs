//! Melee greys out for a pool below the wielded weapon's quote, and for no melee weapon at all.

use bevy::{app::App, ecs::entity::Entity};
use cobalt_mcp_protocol::ports::McpPort;
use gdtf_battle_sim::{
    acts::melee_tu_cost,
    ganger::Tu,
    weapon::{FightMode, MeleeWeapon, WieldedBy, Wields},
};
use gdtf_game::qa_wire::offer::{ContextualActNet, OfferPressableNet};

use super::probe::{OfferAtPool, Pool, offer_at_pool, offer_of};
use crate::{
    contextual_acts::{
        neighbour::enemy_beside_the_shooter,
        scene::{settle, shooter},
    },
    socket_support::{TestError, TestResult},
};

/// The melee weapon an actor holds, found the way the panel finds it.
fn melee_weapon_of(app: &App, actor: Entity) -> Option<Entity> {
    let world = app.world();
    world
        .get_entity(actor)
        .ok()?
        .get::<Wields>()?
        .melee_weapon(|entity| {
            world
                .get_entity(entity)
                .is_ok_and(|row| row.contains::<MeleeWeapon>())
        })
}

/// What one strike costs the selected shooter, walked the way the panel walks it.
fn melee_quote(app: &App) -> Option<Tu> {
    let weapon = melee_weapon_of(app, shooter(app)?)?;
    app.world()
        .get_entity(weapon)
        .ok()?
        .get::<FightMode>()
        .map(melee_tu_cost)
}

/// The melee offer against an adjacent enemy, with the shooter's pool set from `pool`.
fn melee_offer(pool: Pool) -> Result<OfferAtPool, TestError> {
    offer_at_pool(
        ContextualActNet::Melee,
        enemy_beside_the_shooter,
        melee_quote,
        pool,
    )
}

/// The same battle with the shooter's melee weapon dropped, so no strike can be priced.
fn a_shooter_wielding_no_melee_weapon_beside_an_enemy() -> Result<(App, McpPort, ()), TestError> {
    let (mut app, port, _placed) = enemy_beside_the_shooter()?;
    let Some(actor) = shooter(&app) else {
        return Err("the fixture must hand over a battle with a shooter selected".into());
    };
    let Some(weapon) = melee_weapon_of(&app, actor) else {
        return Err(
            "a generated battle must arm its gangers with a melee weapon, or dropping one \
             proves nothing"
                .into(),
        );
    };
    let Ok(mut row) = app.world_mut().get_entity_mut(weapon) else {
        return Err("the weapon the world just answered with must still exist".into());
    };
    row.remove::<WieldedBy>();
    settle(&mut app);
    if melee_weapon_of(&app, actor).is_some() {
        return Err("dropping the weapon must leave the shooter wielding no melee weapon".into());
    }
    Ok((app, port, ()))
}

#[test]
fn a_pool_below_the_weapon_quote_is_offered_as_not_pressable() -> TestResult {
    let probe = melee_offer(Pool::ShortOfTheQuote)?;

    assert!(
        *probe.cost > 0,
        "a strike must cost something or a pool below it cannot exist",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(false),
        "the wire reports the greyed button the panel is showing for a pool one short of what \
         the wielded weapon charges: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_pool_that_covers_the_weapon_quote_is_offered_as_pressable() -> TestResult {
    let probe = melee_offer(Pool::TheQuote)?;

    assert!(
        *probe.cost > 0,
        "a strike must cost something or this case cannot tell a greyed button from a live one",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(true),
        "a pool that covers what the weapon charges leaves the melee button pressable: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_shooter_wielding_no_melee_weapon_is_offered_melee_as_not_pressable() -> TestResult {
    let offer = offer_of(
        ContextualActNet::Melee,
        a_shooter_wielding_no_melee_weapon_beside_an_enemy,
    )?;

    assert_eq!(
        offer.pressable,
        OfferPressableNet::new(false),
        "with nothing to swing there is no strike to price, so the button greys out on a full \
         pool — which is the second way this flag goes false: {offer:?}",
    );
    Ok(())
}
