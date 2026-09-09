use cobalt_mcp_protocol::{
    command::{CommandOutcome, RunOptions},
    message::McpResponse,
};

use super::{
    command_exchange::{PROCGEN_STEP, exchange, run},
    commands::PROCGEN_STEP_IN_THE_MENU,
    socket_support::{TestResult, game_app_listening},
};

#[cfg(not(feature = "dev_tools"))]
#[test]
fn a_build_without_the_stepper_publishes_procgen_step_and_refuses_it_not_built() -> TestResult {
    use cobalt_mcp_protocol::command::UnavailableCode;

    let reply = exchange(
        game_app_listening,
        run(PROCGEN_STEP, "()", RunOptions::default()),
    )?;
    assert!(
        !matches!(reply, McpResponse::Outcome(CommandOutcome::Unknown { .. })),
        "the published list is the same in every build, so the name must be known: {reply:?}",
    );
    let McpResponse::Outcome(CommandOutcome::Unavailable { code, note }) = reply else {
        unreachable!("without the stepper compiled in there is nothing to step, got {reply:?}");
    };
    assert_eq!(
        code,
        UnavailableCode::NotBuilt,
        "no app state can conjure a stepper this binary does not contain",
    );
    assert!(
        !note.as_str().is_empty(),
        "the refusal says what is missing: {}",
        note.as_str(),
    );
    Ok(())
}

#[test]
fn procgen_step_in_the_menu_is_refused_wrong_state_or_not_built() -> TestResult {
    let reply = exchange(
        game_app_listening,
        run(PROCGEN_STEP, "()", RunOptions::default()),
    )?;
    let McpResponse::Outcome(CommandOutcome::Unavailable { code, .. }) = reply else {
        unreachable!("nothing is generating in the Menu, so the step must be refused: {reply:?}");
    };
    assert_eq!(
        code, PROCGEN_STEP_IN_THE_MENU,
        "a build with the stepper refuses the Menu `WrongState`, and a build without it \
         `NotBuilt` — never any other code",
    );
    Ok(())
}

#[cfg(feature = "dev_tools")]
mod stepping {
    use std::sync::mpsc::{self, Sender};

    use bevy::{app::App, prelude::NextState, state::state::State};
    use cobalt_mcp_host::IncomingRequest;
    use cobalt_mcp_protocol::{command::CommandOutcome, message::McpResponse};
    use cobalt_test_utils::{LoadTestAppBuilder, advance_until};
    use gdtf_battle_sim::procgen::{ProcgenStage, StagedProcgen};
    use gdtf_game::{
        qa_wire::misc::ProcgenStageNet,
        test_support::{AppState, BattleScapeState, McpPlugin, ProcgenStepperPlugin, RunningState},
    };
    use serde::Deserialize;

    use crate::mcp::{
        battle_fixture::{run_request, send},
        command_exchange::PROCGEN_STEP,
    };

    /// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
    const ONE_STEP_A_FRAME: u32 = 1;

    #[derive(Debug, Deserialize)]
    struct StepBody {
        stage: ProcgenStageNet,
    }

    fn running_state(app: &App) -> Option<RunningState> {
        app.world()
            .get_resource::<State<RunningState>>()
            .map(|state| *state.get())
    }

    fn battlescape_state(app: &App) -> Option<BattleScapeState> {
        app.world()
            .get_resource::<State<BattleScapeState>>()
            .map(|state| *state.get())
    }

    /// The real load flow, the real stepper engaged, and the QA router on a test-owned channel.
    fn stepping_app_with_mcp() -> (App, Sender<IncomingRequest>) {
        let mut app =
            LoadTestAppBuilder::new(gdtf_game::test_support::register_scenes_with_default_plugins)
                .starting_in(AppState::Load)
                .build();
        let (tx, rx) = mpsc::channel();
        app.add_plugins(McpPlugin::with_channels(rx));
        app.add_plugins(ProcgenStepperPlugin::with_enabled(true));
        app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
            ONE_STEP_A_FRAME,
        ));
        advance_until(&mut app, |app| {
            running_state(app) == Some(RunningState::Menu)
        });
        app.world_mut()
            .resource_mut::<NextState<RunningState>>()
            .set(RunningState::Game);
        advance_until(&mut app, |app| {
            battlescape_state(app) == Some(BattleScapeState::Generation)
                && app.world().get_resource::<StagedProcgen>().is_some()
        });
        (app, tx)
    }

    fn stage_of(app: &App) -> Option<ProcgenStage> {
        app.world()
            .get_resource::<StagedProcgen>()
            .map(StagedProcgen::stage)
    }

    fn stepped(reply: &McpResponse) -> ProcgenStageNet {
        let McpResponse::Outcome(CommandOutcome::Ran { reply, .. }) = reply else {
            unreachable!("a step the stepper can take must RUN, got {reply:?}");
        };
        let body = reply.as_str();
        let Ok(step) = ron::de::from_str::<StepBody>(body) else {
            unreachable!("the reply body decodes into the published step shape: {body}");
        };
        step.stage
    }

    #[test]
    fn procgen_step_names_the_stage_it_queued_against() {
        let (mut app, tx) = stepping_app_with_mcp();
        let Some(before) = stage_of(&app) else {
            unreachable!("the fixture rests with a staged driver");
        };

        let pending = send(&tx, run_request(PROCGEN_STEP, "()"));
        app.update();
        let Ok(reply) = pending.try_recv() else {
            unreachable!("procgen.step is Immediate and must answer on the frame it is claimed");
        };
        assert_eq!(
            stepped(&reply),
            ProcgenStageNet::from_stage(before),
            "the reply names the stage the driver was on when the step was queued",
        );
    }

    #[test]
    fn stepping_over_the_wire_drives_generation_to_a_running_battle() {
        let (mut app, tx) = stepping_app_with_mcp();
        let mut ran = 0_u32;
        while stage_of(&app).is_some() {
            let pending = send(&tx, run_request(PROCGEN_STEP, "()"));
            app.update();
            if let Ok(McpResponse::Outcome(CommandOutcome::Ran { .. })) = pending.try_recv() {
                ran = ran.saturating_add(1);
            }
        }
        assert!(
            ran > 0,
            "at least one step must have been taken over the wire, or nothing was driven",
        );
        advance_until(&mut app, |app| {
            battlescape_state(app) == Some(BattleScapeState::BattleRunning)
        });
    }
}
