use std::sync::mpsc::Receiver;

use bevy::prelude::*;
use gdtf_battle_input::{PendingActIntent, SelectedFireMode, SelectedShooter};
use gdtf_battle_sim::tuning::CombatTuning;
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
use crate::dev::net_qa::{commands::act::fire::ActFireArgs, wire::act::ActReply};

/// One cell to shoot at, written in `act.fire`'s own argument shape.
const AT: &str = "(at:(cell:(x:1,y:1),level:0))";

/// Which resources a case leaves out of the world before the shot is claimed.
#[derive(Clone, Copy)]
struct Loaded {
    mode:   bool,
    tuning: bool,
}

/// An app holding `act.fire`'s own claim and settle systems, a selected shooter, and nothing else.
fn fire_app(loaded: Loaded) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<PendingQueue<CommandCall<ActFire>>>();
    app.init_resource::<DeferredReplies<ActFire>>();
    app.init_resource::<PendingActIntent>();
    if loaded.mode {
        app.init_resource::<SelectedFireMode>();
    }
    if loaded.tuning {
        app.init_resource::<CombatTuning>();
    }
    app.configure_sets(
        Update,
        ActCommandSystems::Settle.after(ActCommandSystems::Claim),
    );
    ActFire::register_handler(&mut app);
    let shooter = app.world_mut().spawn_empty().id();
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
fn a_shot_with_no_fire_mode_loaded_refuses_missing_model_naming_it() {
    let mut app = fire_app(Loaded {
        mode:   false,
        tuning: true,
    });

    let reply_rx = fire_once(&mut app);

    let reply = reply_rx.try_recv().ok();
    assert_eq!(
        refusal(reply.as_ref())
            .as_ref()
            .map(|(code, note)| (*code, note.contains("SelectedFireMode"))),
        Some((UnavailableCode::MissingModel, true)),
        "a host with no SelectedFireMode cannot build the shot, so it must refuse MissingModel \
         naming that resource — never an accepted window, which reads as a shot the sim declined; \
         got {reply:?}",
    );
}

#[test]
fn a_shot_with_no_combat_tuning_loaded_refuses_missing_model_naming_it() {
    let mut app = fire_app(Loaded {
        mode:   true,
        tuning: false,
    });

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
fn a_declined_shot_on_a_loaded_host_still_answers_an_empty_window() {
    let mut app = fire_app(Loaded {
        mode:   true,
        tuning: true,
    });

    let reply_rx = fire_once(&mut app);

    let reply = reply_rx.try_recv().ok();
    let Some(QaResponse::Outcome(CommandOutcome::Ran { reply: body, .. })) = &reply else {
        unreachable!("a loaded host answers the shot rather than refusing it, got {reply:?}");
    };
    let Ok(decoded) = ron::de::from_str::<ActReply>(body.as_str()) else {
        unreachable!("`act.fire` must answer its declared reply shape, got {body:?}");
    };
    let ActReply::Accepted {
        from_seq, to_seq, ..
    } = decoded
    else {
        unreachable!(
            "a shooter with no weapon is declined by the sim, not by the QA layer, so the reply \
             is still an accepted window: {decoded:?}"
        );
    };
    assert_eq!(
        from_seq, to_seq,
        "a shot the sim declines logs nothing, so its window is empty — this is the answer the \
         missing-resource refusal must stay distinguishable from",
    );
}
