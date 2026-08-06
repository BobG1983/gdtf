//! Which token kind each contextual act's target is minted as.

use bevy::ecs::entity::Entity;
use gdtf_battle_input::contextual::{
    EnterEmplacementAct, ExitEmplacementAct, MeleeAct, OpenDoorAct, ShoveAct, ThrowGrenadeAct,
};
use gdtf_battle_sim::{
    acts::MeleeTarget,
    prelude::{Cell, CellLevel, Level},
};

use crate::{
    dev::net_qa::{
        commands::read::battle_offers::{cell, door, emplacement, ganger, melee},
        wire::{
            cell::CellLevelNet,
            offer::OfferTargetNet,
            token::{DoorToken, EmplacementToken, GangerToken},
        },
    },
    states::running::game::battlescape::contextual_panel::ContextualOffer,
};

fn a_target() -> Entity {
    Entity::from_raw_u32(12).unwrap_or(Entity::PLACEHOLDER)
}

fn a_cell() -> CellLevel {
    CellLevel::new(Cell::new(5, 6), Level::new(0))
}

#[test]
fn a_shove_target_is_a_ganger_token() {
    let offer = ContextualOffer::<ShoveAct>::new(Some(a_target()));

    assert_eq!(
        ganger(&offer),
        Some(OfferTargetNet::Ganger(GangerToken::new(
            a_target().to_bits()
        ))),
        "shove acts on a ganger, so its target is a ganger token",
    );
}

#[test]
fn an_open_door_target_is_a_door_token() {
    let offer = ContextualOffer::<OpenDoorAct>::new(Some(a_target()));

    assert_eq!(
        door(&offer),
        Some(OfferTargetNet::Door(DoorToken::new(a_target().to_bits()))),
        "a door offer names a door, not the ganger standing beside it",
    );
}

#[test]
fn the_emplacement_acts_both_mint_emplacement_tokens() {
    let enter = ContextualOffer::<EnterEmplacementAct>::new(Some(a_target()));
    let exit = ContextualOffer::<ExitEmplacementAct>::new(Some(a_target()));
    let expected = Some(OfferTargetNet::Emplacement(EmplacementToken::new(
        a_target().to_bits(),
    )));

    assert_eq!(
        (emplacement(&enter), emplacement(&exit)),
        (expected, expected),
        "mounting and dismounting both name the emplacement",
    );
}

#[test]
fn a_grenade_target_is_a_cell() {
    let offer = ContextualOffer::<ThrowGrenadeAct>::new(Some(a_cell()));

    assert_eq!(
        cell(&offer),
        Some(OfferTargetNet::Cell(CellLevelNet::from_sim(a_cell()))),
        "a grenade is thrown at a cell, not at an entity",
    );
}

#[test]
fn melee_reports_whichever_of_its_two_targets_is_offered() {
    let at_a_ganger = ContextualOffer::<MeleeAct>::new(Some(MeleeTarget::Ganger(a_target())));
    let at_a_wall = ContextualOffer::<MeleeAct>::new(Some(MeleeTarget::Structure(a_cell())));

    assert_eq!(
        (melee(&at_a_ganger), melee(&at_a_wall)),
        (
            Some(OfferTargetNet::Ganger(GangerToken::new(
                a_target().to_bits()
            ))),
            Some(OfferTargetNet::Cell(CellLevelNet::from_sim(a_cell()))),
        ),
        "melee swings at a ganger or at a structure, and the reply says which",
    );
}

#[test]
fn an_act_with_nothing_on_offer_reports_no_target() {
    let offer = ContextualOffer::<ShoveAct>::new(None);

    assert_eq!(
        ganger(&offer),
        None,
        "an act the panel is not offering has no button and no entry",
    );
}
