//! That every act the panel offers reaches the reply, under its own act and target.

use bevy::{ecs::system::RunSystemOnce as _, prelude::*};
use gdtf_battle_input::contextual::{
    EnterEmplacementAct, ExecuteAct, ExitEmplacementAct, MeleeAct, OpenDoorAct, ShoveAct,
    StabilizeAct, ThrowGrenadeAct,
};
use gdtf_battle_sim::{
    acts::MeleeTarget,
    prelude::{Cell, CellLevel, Level},
};

use crate::{
    dev::net_qa::{
        commands::read::battle_offers::OfferedActs,
        wire::{
            cell::CellLevelNet,
            offer::{ContextualActNet, ContextualOfferNet, OfferTargetNet},
            token::{DoorToken, EmplacementToken, GangerToken},
        },
    },
    states::running::game::battlescape::contextual_panel::ContextualOffer,
};

/// One entity per act, all distinct, so an offer landing under the wrong act is visible.
fn target(raw: u32) -> Entity {
    Entity::from_raw_u32(raw).unwrap_or(Entity::PLACEHOLDER)
}

fn grenade_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 6), Level::new(0))
}

/// A world where all eight contextual acts are on offer at once.
fn a_world_offering_every_act() -> World {
    let mut world = World::new();
    world.insert_resource(ContextualOffer::<ExecuteAct>::new(Some(target(11))));
    world.insert_resource(ContextualOffer::<StabilizeAct>::new(Some(target(12))));
    world.insert_resource(ContextualOffer::<MeleeAct>::new(Some(MeleeTarget::Ganger(
        target(13),
    ))));
    world.insert_resource(ContextualOffer::<ShoveAct>::new(Some(target(14))));
    world.insert_resource(ContextualOffer::<OpenDoorAct>::new(Some(target(15))));
    world.insert_resource(ContextualOffer::<EnterEmplacementAct>::new(Some(target(
        16,
    ))));
    world.insert_resource(ContextualOffer::<ExitEmplacementAct>::new(Some(target(17))));
    world.insert_resource(ContextualOffer::<ThrowGrenadeAct>::new(
        Some(grenade_cell()),
    ));
    world
}

/// What all eight offers look like on the wire, in the order the reply sorts them into.
fn every_offer() -> Vec<ContextualOfferNet> {
    vec![
        ContextualOfferNet::new(
            ContextualActNet::Execute,
            OfferTargetNet::Ganger(GangerToken::new(target(11).to_bits())),
        ),
        ContextualOfferNet::new(
            ContextualActNet::Stabilize,
            OfferTargetNet::Ganger(GangerToken::new(target(12).to_bits())),
        ),
        ContextualOfferNet::new(
            ContextualActNet::Melee,
            OfferTargetNet::Ganger(GangerToken::new(target(13).to_bits())),
        ),
        ContextualOfferNet::new(
            ContextualActNet::Shove,
            OfferTargetNet::Ganger(GangerToken::new(target(14).to_bits())),
        ),
        ContextualOfferNet::new(
            ContextualActNet::OpenDoor,
            OfferTargetNet::Door(DoorToken::new(target(15).to_bits())),
        ),
        ContextualOfferNet::new(
            ContextualActNet::EnterEmplacement,
            OfferTargetNet::Emplacement(EmplacementToken::new(target(16).to_bits())),
        ),
        ContextualOfferNet::new(
            ContextualActNet::ExitEmplacement,
            OfferTargetNet::Emplacement(EmplacementToken::new(target(17).to_bits())),
        ),
        ContextualOfferNet::new(
            ContextualActNet::ThrowGrenade,
            OfferTargetNet::Cell(CellLevelNet::from_sim(grenade_cell())),
        ),
    ]
}

/// Collect the offers the same way the handler does: through the real system param.
fn offered(world: &mut World) -> Vec<ContextualOfferNet> {
    match world.run_system_once(|offered: OfferedActs| offered.collect()) {
        Ok(offers) => offers,
        Err(fault) => unreachable!("collecting offers is a plain read-only system: {fault:?}"),
    }
}

#[test]
fn every_offered_act_reaches_the_reply_with_its_own_target() {
    let mut world = a_world_offering_every_act();
    let offers = offered(&mut world);

    assert_eq!(
        offers,
        every_offer(),
        "each of the eight contextual acts has its own entry carrying its own target, so \
         dropping one act or crossing two targets shows up here",
    );
}

#[test]
fn an_act_the_panel_is_not_offering_has_no_entry() {
    let mut world = a_world_offering_every_act();
    world.insert_resource(ContextualOffer::<ShoveAct>::new(None));
    let offers = offered(&mut world);

    let want: Vec<ContextualOfferNet> = every_offer()
        .into_iter()
        .filter(|offer| offer.act != ContextualActNet::Shove)
        .collect();
    assert_eq!(
        offers, want,
        "withdrawing one offer removes that entry and leaves the other seven alone",
    );
}

#[test]
fn a_host_without_the_contextual_panel_offers_nothing() {
    let mut world = World::new();
    let offers = offered(&mut world);

    assert!(
        offers.is_empty(),
        "with none of the panel's resources present the read still answers, with an empty \
         list: {offers:?}",
    );
}
