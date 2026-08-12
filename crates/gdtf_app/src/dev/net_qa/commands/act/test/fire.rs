use std::sync::mpsc::Receiver;

use bevy::prelude::*;
use gdtf_battle_input::{PendingActIntent, SelectedShooter};
use gdtf_battle_sim::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    tuning::CombatTuning,
};
use gdtf_net_qa_transport::{PendingQueue, Responder};
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CommandCall, DeferredReplies},
};
use gdtf_qa_protocol::{
    command::{CommandOutcome, UnavailableCode},
    message::QaResponse,
};

use super::super::{fire::ActFire, sets::ActCommandSystems};
use crate::dev::net_qa::{
    commands::act::fire::ActFireArgs,
    wire::{act::ActReply, refusal::ShotRefusalNet},
};

/// One cell to shoot at, written in `act.fire`'s own argument shape.
const AT: &str = "(at:(cell:(x:1,y:1),level:0))";

/// Whether the case leaves the tuning in the world before the shot is claimed.
#[derive(Clone, Copy)]
struct Loaded {
    tuning: bool,
}

/// An app holding `act.fire`'s own claim and settle systems, a selected shooter, and nothing else.
fn fire_app(loaded: Loaded) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<PendingQueue<CommandCall<ActFire>>>();
    app.init_resource::<DeferredReplies<ActFire>>();
    app.init_resource::<PendingActIntent>();
    if loaded.tuning {
        app.init_resource::<CombatTuning>();
    }
    app.configure_sets(
        Update,
        ActCommandSystems::Settle.after(ActCommandSystems::Claim),
    );
    ActFire::register_handler(&mut app);
    let shooter = app
        .world_mut()
        .spawn((
            LifeState::Alive,
            Tu::new(10),
            TuMax::new(10),
            Aiming::new(false),
        ))
        .id();
    app.insert_resource(SelectedShooter::new(shooter));
    app
}

/// Queue one `act.fire` call and run the frame that claims and settles it.
fn fire_once(app: &mut App) -> Receiver<QaResponse> {
    let Ok(args) = ron::de::from_str::<ActFireArgs>(AT) else {
        unreachable!("`act.fire`'s own argument shape must decode {AT}");
    };
    let (responder, reply_rx) = Responder::channel();
    app.world_mut()
        .resource_mut::<PendingQueue<CommandCall<ActFire>>>()
        .push_new(CommandCall::<ActFire>::new(args), responder);
    app.update();
    reply_rx
}

/// The code and note of an `Unavailable` outcome, or nothing when the reply is a different shape.
fn refusal(reply: Option<&QaResponse>) -> Option<(UnavailableCode, String)> {
    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = reply? else {
        return None;
    };
    Some((*code, note.as_str().to_owned()))
}

#[test]
fn a_shot_with_no_combat_tuning_loaded_refuses_missing_model_naming_it() {
    let mut app = fire_app(Loaded { tuning: false });

    let reply_rx = fire_once(&mut app);

    let reply = reply_rx.try_recv().ok();
    assert_eq!(
        refusal(reply.as_ref())
            .as_ref()
            .map(|(code, note)| (*code, note.contains("CombatTuning"))),
        Some((UnavailableCode::MissingModel, true)),
        "a host with no CombatTuning cannot cost the shot, so it must refuse MissingModel naming \
         that resource — never an accepted window, which reads as a shot the sim declined; got \
         {reply:?}",
    );
}

#[test]
fn a_shooter_with_no_weapon_answers_the_no_firing_weapon_reason() {
    let mut app = fire_app(Loaded { tuning: true });

    let reply_rx = fire_once(&mut app);

    let reply = reply_rx.try_recv().ok();
    let Some(QaResponse::Outcome(CommandOutcome::Ran { reply: body, .. })) = &reply else {
        unreachable!("a loaded host answers the shot rather than refusing it, got {reply:?}");
    };
    let Ok(decoded) = ron::de::from_str::<ActReply>(body.as_str()) else {
        unreachable!("`act.fire` must answer its declared reply shape, got {body:?}");
    };
    assert_eq!(
        decoded,
        ActReply::FireRefused {
            reason: ShotRefusalNet::NoFiringWeapon,
        },
        "a shooter holding nothing that fires never reaches the sim, so the reply names that \
         reason — this is the answer the missing-resource refusal must stay distinguishable from",
    );
}
