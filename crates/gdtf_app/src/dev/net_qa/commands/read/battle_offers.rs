use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::contextual::{
    EnterEmplacementAct, ExecuteAct, ExitEmplacementAct, MeleeAct, OpenDoorAct, ShoveAct,
    StabilizeAct, ThrowGrenadeAct,
};
use gdtf_battle_sim::acts::MeleeTarget;
use gdtf_net_qa_transport::PendingQueue;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, QaCommandSystems, take_calls},
};
use gdtf_qa_protocol::command::{CommandAvailability, CommandName, CommandSummary, CommandTiming};
use serde::{Deserialize, Serialize};

use super::availability::presenter_is_ready;
use crate::{
    dev::net_qa::{
        facts::GameFacts,
        wire::{
            cell::CellLevelNet,
            offer::{ContextualActNet, ContextualOfferNet, OfferTargetNet},
            token::{DoorToken, EmplacementToken, GangerToken},
        },
    },
    states::running::game::battlescape::contextual_panel::{
        registrar::ContextualPanelSystems, seam::ContextualOffer,
    },
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BattleOffersArgs {}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BattleOffersReply {
    offers: Vec<ContextualOfferNet>,
}

pub(crate) struct BattleOffers;

impl QaCommand for BattleOffers {
    type Args = BattleOffersArgs;
    type Facts = GameFacts;
    type Parked = ();
    type Reply = BattleOffersReply;

    const NAME: CommandName = CommandName::from_static("battle.offers");
    const SUMMARY: CommandSummary = CommandSummary::from_static(
        "Read the contextual buttons the panel is offering this frame, each with the target it \
         would act on. These are the same per-act resources the buttons read, not a second \
         computation, so an offer here is a button on screen.",
    );
    const TIMING: CommandTiming = CommandTiming::Immediate;

    fn availability(facts: &GameFacts) -> CommandAvailability {
        presenter_is_ready(*facts)
    }

    fn register_handler(app: &mut App) {
        app.add_systems(
            Update,
            handle_battle_offers
                .after(QaCommandSystems::Claim)
                .after(ContextualPanelSystems::Toggle),
        );
    }
}

/// The per-act offer resources the contextual panel writes each frame.
#[derive(SystemParam)]
pub(super) struct OfferedActs<'w> {
    execute:           Option<Res<'w, ContextualOffer<ExecuteAct>>>,
    stabilize:         Option<Res<'w, ContextualOffer<StabilizeAct>>>,
    melee:             Option<Res<'w, ContextualOffer<MeleeAct>>>,
    shove:             Option<Res<'w, ContextualOffer<ShoveAct>>>,
    open_door:         Option<Res<'w, ContextualOffer<OpenDoorAct>>>,
    enter_emplacement: Option<Res<'w, ContextualOffer<EnterEmplacementAct>>>,
    exit_emplacement:  Option<Res<'w, ContextualOffer<ExitEmplacementAct>>>,
    throw_grenade:     Option<Res<'w, ContextualOffer<ThrowGrenadeAct>>>,
}

impl OfferedActs<'_> {
    /// Every act the panel is offering now, sorted by act.
    #[must_use]
    pub(super) fn collect(&self) -> Vec<ContextualOfferNet> {
        let mut offers: Vec<ContextualOfferNet> = Vec::new();
        push(
            &mut offers,
            ContextualActNet::Execute,
            self.execute.as_deref().and_then(ganger),
        );
        push(
            &mut offers,
            ContextualActNet::Stabilize,
            self.stabilize.as_deref().and_then(ganger),
        );
        push(
            &mut offers,
            ContextualActNet::Melee,
            self.melee.as_deref().and_then(melee),
        );
        push(
            &mut offers,
            ContextualActNet::Shove,
            self.shove.as_deref().and_then(ganger),
        );
        push(
            &mut offers,
            ContextualActNet::OpenDoor,
            self.open_door.as_deref().and_then(door),
        );
        push(
            &mut offers,
            ContextualActNet::EnterEmplacement,
            self.enter_emplacement.as_deref().and_then(emplacement),
        );
        push(
            &mut offers,
            ContextualActNet::ExitEmplacement,
            self.exit_emplacement.as_deref().and_then(emplacement),
        );
        push(
            &mut offers,
            ContextualActNet::ThrowGrenade,
            self.throw_grenade.as_deref().and_then(cell),
        );
        offers.sort_unstable_by_key(|offer| offer.act);
        offers
    }
}

fn push(
    offers: &mut Vec<ContextualOfferNet>,
    act: ContextualActNet,
    target: Option<OfferTargetNet>,
) {
    if let Some(target) = target {
        offers.push(ContextualOfferNet::new(act, target));
    }
}

pub(super) fn ganger<A>(offer: &ContextualOffer<A>) -> Option<OfferTargetNet>
where
    A: gdtf_battle_input::contextual::ContextualAct<Target = Entity>,
{
    offer
        .target()
        .map(|entity| OfferTargetNet::Ganger(GangerToken::new(entity.to_bits())))
}

pub(super) fn door<A>(offer: &ContextualOffer<A>) -> Option<OfferTargetNet>
where
    A: gdtf_battle_input::contextual::ContextualAct<Target = Entity>,
{
    offer
        .target()
        .map(|entity| OfferTargetNet::Door(DoorToken::new(entity.to_bits())))
}

pub(super) fn emplacement<A>(offer: &ContextualOffer<A>) -> Option<OfferTargetNet>
where
    A: gdtf_battle_input::contextual::ContextualAct<Target = Entity>,
{
    offer
        .target()
        .map(|entity| OfferTargetNet::Emplacement(EmplacementToken::new(entity.to_bits())))
}

pub(super) fn cell<A>(offer: &ContextualOffer<A>) -> Option<OfferTargetNet>
where
    A: gdtf_battle_input::contextual::ContextualAct<Target = gdtf_battle_sim::prelude::CellLevel>,
{
    offer
        .target()
        .map(|at| OfferTargetNet::Cell(CellLevelNet::from_sim(at)))
}

pub(super) fn melee(offer: &ContextualOffer<MeleeAct>) -> Option<OfferTargetNet> {
    offer.target().map(|target| match target {
        MeleeTarget::Ganger(entity) => OfferTargetNet::Ganger(GangerToken::new(entity.to_bits())),
        MeleeTarget::Structure(at) => OfferTargetNet::Cell(CellLevelNet::from_sim(at)),
    })
}

fn handle_battle_offers(
    offered: OfferedActs,
    mut queue: ResMut<PendingQueue<CommandCall<BattleOffers>>>,
) {
    if queue.is_empty() {
        return;
    }
    let offers = offered.collect();
    for (_args, responder) in take_calls::<BattleOffers>(&mut queue) {
        responder.answer(&BattleOffersReply {
            offers: offers.clone(),
        });
    }
}
