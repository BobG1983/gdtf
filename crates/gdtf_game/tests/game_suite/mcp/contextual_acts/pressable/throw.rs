//! Throw greys out when the pool is short of the quote the sim prices one throw at.

use bevy::{app::App, ecs::entity::Entity, prelude::*};
use cobalt_mcp_protocol::ports::McpPort;
use gdtf_battle_input::{InspectTarget, pick_hovered_cell};
use gdtf_battle_sim::{
    acts::throw_grenade_tu_cost,
    prelude::CellLevel,
    weapon::{MeleeWeapon, TrajectoryStyle, Wields},
};
use gdtf_game::qa_wire::offer::{ContextualActNet, OfferPressableNet};

use super::probe::{OfferAtPool, Pool, from_tuning, offer_at_pool};
use crate::mcp::{
    contextual_acts::scene::{a_neighbour, select_a_player_ganger, settle},
    socket_support::{TestError, TestResult, battle_app_listening},
};

/// The cell a headless host is told to hover, in place of the pointer it has not got.
#[derive(Resource, Deref, Clone, Copy, Debug)]
struct HeldHover(Option<CellLevel>);

impl HeldHover {
    /// Hold this cell under the pointer.
    const fn new(at: Option<CellLevel>) -> Self {
        Self(at)
    }
}

/// Put the held cell back after the real picking system resolved no cursor at all.
fn hold_the_hover(held: Res<HeldHover>, mut target: ResMut<InspectTarget>) {
    target.set_hovered(**held);
}

/// The ranged weapon the shooter holds, found the way the offer scan finds it.
fn gun_of(app: &App, actor: Entity) -> Option<Entity> {
    let world = app.world();
    world
        .get_entity(actor)
        .ok()?
        .get::<Wields>()?
        .ranged_weapon(|entity| {
            world
                .get_entity(entity)
                .is_ok_and(|row| row.contains::<MeleeWeapon>())
        })
}

/// A live battle whose shooter wields an arcing weapon and is holding a hover beside itself.
pub(super) fn an_arcing_weapon_over_a_hovered_cell() -> Result<(App, McpPort, ()), TestError> {
    let (mut app, port) = battle_app_listening()?;
    let (shooter, at) = select_a_player_ganger(&mut app)?;
    let beside = a_neighbour(&app, at)?;
    let Some(gun) = gun_of(&app, shooter) else {
        return Err("a generated battle must arm its gangers with a ranged weapon to throw".into());
    };
    let Ok(mut row) = app.world_mut().get_entity_mut(gun) else {
        return Err("the weapon the world just answered with must still exist".into());
    };
    row.insert(TrajectoryStyle::Arc);
    app.insert_resource(HeldHover::new(Some(beside)));
    app.add_systems(Update, hold_the_hover.after(pick_hovered_cell));
    settle(&mut app);
    Ok((app, port, ()))
}

/// The throw offer, read with the shooter's pool set from `pool`.
fn throw_offer(pool: Pool) -> Result<OfferAtPool, TestError> {
    offer_at_pool(
        ContextualActNet::ThrowGrenade,
        an_arcing_weapon_over_a_hovered_cell,
        from_tuning(throw_grenade_tu_cost),
        pool,
    )
}

#[test]
fn a_pool_below_the_throw_quote_is_offered_as_not_pressable() -> TestResult {
    let probe = throw_offer(Pool::ShortOfTheQuote)?;

    assert!(
        *probe.cost > 0,
        "a throw must cost something or a pool below it cannot exist",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(false),
        "the wire reports the greyed button the panel is showing for a pool one short of the \
         sim's own throw quote: {probe:?}",
    );
    Ok(())
}

#[test]
fn a_pool_that_covers_the_throw_quote_is_offered_as_pressable() -> TestResult {
    let probe = throw_offer(Pool::TheQuote)?;

    assert!(
        *probe.cost > 0,
        "a throw must cost something or this case cannot tell a greyed button from a live one",
    );
    assert_eq!(
        probe.offer.pressable,
        OfferPressableNet::new(true),
        "a pool that covers the quote leaves the throw button pressable: {probe:?}",
    );
    Ok(())
}

#[test]
fn the_throw_fixture_holds_a_hover_a_headless_host_would_otherwise_lose() -> TestResult {
    let (app, _port, ()) = an_arcing_weapon_over_a_hovered_cell()?;

    let held = app
        .world()
        .get_resource::<InspectTarget>()
        .and_then(InspectTarget::hovered);
    assert!(
        held.is_some(),
        "this host has no primary window, so the real picking system resolves no cell and the \
         throw scan would read nothing to aim at unless the fixture holds one",
    );
    Ok(())
}
