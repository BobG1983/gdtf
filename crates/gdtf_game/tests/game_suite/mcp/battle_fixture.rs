use std::sync::mpsc;

use bevy::{
    app::App,
    ecs::{entity::Entity, relationship::RelationshipTarget},
    state::state::State,
};
use cobalt_mcp_host::{IncomingRequest, Responder};
use cobalt_mcp_protocol::{
    command::{CommandArgsRon, CommandName, CommandOutcome, RunOptions},
    message::{McpRequest, McpResponse, RunCommand},
};
use cobalt_test_utils::{MinimalTestAppBuilder, advance_until};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_presenter::playback::PlaybackCursor;
use gdtf_battle_sim::{
    act_log::ActLog,
    battle::PlayerFaction,
    effects::fields::FieldDefRegistry,
    ganger::{Faction, LifeState},
    situation::Situation,
    test_support::{
        fixtures, test_armor_registry, test_gang_registry, test_melee_weapon_registry,
        test_weapon_registry,
    },
    tuning::{CombatTuning, GangerStatTuning},
    weapon::{FireMode, MeleeWeapon, MountedWeapon, WieldedBy, Wields},
};
use gdtf_content_families::situation::LoadedSituation;
use gdtf_game::test_support::{
    AppState, BattleScapeState, McpPlugin, PreplacedGangers, RunningState, StartBattleRequested,
};
use gdtf_ui::theme::default_theme;

use crate::mcp::socket_support::{FIXTURE_SEED, NO_DEFERRAL_DEADLINE};

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Hand one request to the router's inbox and keep the channel its reply will arrive on.
pub(crate) fn send(
    tx: &mpsc::Sender<IncomingRequest>,
    request: McpRequest,
) -> mpsc::Receiver<McpResponse> {
    let (responder, reply_rx) = Responder::channel();
    let sent = tx.send(IncomingRequest::new(request, responder));
    assert!(sent.is_ok(), "the router inbox must be open");
    reply_rx
}

/// A plain `Run` request for `name` with `arguments` and no riders.
pub(crate) fn run_request(name: &'static str, arguments: &str) -> McpRequest {
    McpRequest::Run(RunCommand::with_options(
        CommandName::from_static(name),
        CommandArgsRon::new(arguments.to_owned()),
        RunOptions::default(),
    ))
}

pub(crate) fn menu_app_with_mcp() -> (App, mpsc::Sender<IncomingRequest>) {
    menu_app_with_situation(fixtures::two_ganger())
}

/// The same harness on a chosen battlefield, for a case the shipped two-ganger one cannot tell.
pub(crate) fn menu_app_with_situation(
    built: (Situation, Vec<gdtf_battle_sim::situation::PlacedGanger>),
) -> (App, mpsc::Sender<IncomingRequest>) {
    let (situation, placements) = built;
    let mut app =
        MinimalTestAppBuilder::new_with_scene_support(gdtf_game::test_support::register_headless)
            .starting_in(AppState::Running)
            .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(GangerStatTuning::default());
    app.world_mut().insert_resource(test_weapon_registry());
    app.world_mut()
        .insert_resource(test_melee_weapon_registry());
    app.world_mut().insert_resource(test_armor_registry());
    app.world_mut().insert_resource(FieldDefRegistry::default());
    app.world_mut().insert_resource(test_gang_registry());
    app.world_mut()
        .insert_resource(LoadedSituation::new(situation));
    app.world_mut()
        .insert_resource(PreplacedGangers::new(placements));

    let (tx, rx) = mpsc::channel();
    // Before the plugin builds: its `build` is what registers every command's budget.
    app.insert_resource(NO_DEFERRAL_DEADLINE);
    app.add_plugins(McpPlugin::with_channels(rx));

    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));
    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    (app, tx)
}

pub(crate) fn request_battle(app: &mut App) {
    // Without a seed the generator falls back to `resolve_root_seed()`, which reads the wall clock.
    app.insert_resource(FIXTURE_SEED);
    app.world_mut()
        .write_message(StartBattleRequested::new(None));
}

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Drive the channel fixture down into a live, running battle.
pub(crate) fn drive_into_battle_running(app: &mut App) {
    request_battle(app);
    app.update();
    advance_until(app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
}

/// Send one command and drive exactly one frame, which is what pins when the effect lands.
pub(crate) fn run_one_frame(
    app: &mut App,
    tx: &mpsc::Sender<IncomingRequest>,
    name: &'static str,
    arguments: &str,
) -> McpResponse {
    let reply = send(tx, run_request(name, arguments));
    app.update();
    let Ok(answer) = reply.try_recv() else {
        unreachable!("`{name}` must answer inside the one frame that follows the send");
    };
    answer
}

/// The RON body of a reply that ran.
pub(crate) fn ran(name: &str, answer: McpResponse) -> String {
    let McpResponse::Outcome(CommandOutcome::Ran { reply, .. }) = answer else {
        unreachable!("`{name}` must RUN in this fixture, got {answer:?}");
    };
    reply.as_str().to_owned()
}

/// The RON body a command's reply decodes into.
pub(crate) fn decoded<T: serde::de::DeserializeOwned>(name: &str, answer: McpResponse) -> T {
    let body = ran(name, answer);
    let Ok(value) = ron::de::from_str::<T>(&body) else {
        unreachable!("`{name}`'s reply must decode into the case's own body type: {body}");
    };
    value
}

/// The shooter the game auto-selected once the battle started.
pub(crate) fn selected_shooter(app: &App) -> Entity {
    let Some(selected) = app
        .world()
        .get_resource::<SelectedShooter>()
        .and_then(|selected| **selected)
    else {
        unreachable!("the fixture auto-selects a player ganger on entering a running battle");
    };
    selected
}

/// Put every one of the player's gangers down, and hand back who was taken away.
///
/// Nobody is selected afterwards and the auto-select has nobody living left to re-pick.
pub(crate) fn down_the_players_gang(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let Some(player) = world.get_resource::<PlayerFaction>().map(|player| **player) else {
        unreachable!("a running battle names the gang the player commands");
    };
    let gang: Vec<Entity> = world
        .iter_entities()
        .filter_map(|entity| (entity.get::<Faction>() == Some(&player)).then_some(entity.id()))
        .collect();
    assert!(
        !gang.is_empty(),
        "the fixture must field a player gang for this case to take it away",
    );
    for entity in &gang {
        if let Ok(mut ganger) = world.get_entity_mut(*entity) {
            ganger.insert(LifeState::Downed);
        }
    }
    *world.resource_mut::<SelectedShooter>() = SelectedShooter::cleared();

    // Downing the gang wrote act-log lines; put the screen on the head so the gate is open.
    let head = world.resource::<ActLog>().head();
    let mut cursor = world.resource_mut::<PlaybackCursor>();
    cursor.reset();
    cursor.jump_to(head);
    gang
}

/// Arm the auto-selected shooter's ranged weapon with `modes`, spawning one if it has none.
pub(crate) fn arm_selected_with_modes(app: &mut App, modes: FireMode) -> Entity {
    let shooter = selected_shooter(app);
    match ranged_weapon_of(app, shooter) {
        Some(weapon) => {
            app.world_mut().entity_mut(weapon).insert(modes);
            weapon
        }
        None => app.world_mut().spawn((WieldedBy::new(shooter), modes)).id(),
    }
}

/// Wield a mounted weapon offering `modes` on the auto-selected shooter.
///
/// This is the state manning an emplacement leaves behind: a mount held on top of the own gun.
pub(crate) fn mount_on_selected(app: &mut App, modes: FireMode) -> Entity {
    let shooter = selected_shooter(app);
    app.world_mut()
        .spawn((WieldedBy::new(shooter), modes, MountedWeapon))
        .id()
}

/// Take every ranged weapon off the auto-selected shooter, leaving it holding no gun.
pub(crate) fn disarm_selected(app: &mut App) {
    let shooter = selected_shooter(app);
    let held: Vec<Entity> = app
        .world()
        .get::<Wields>(shooter)
        .map(|wields| wields.iter().collect())
        .unwrap_or_default();
    for weapon in held {
        if app.world().get::<MeleeWeapon>(weapon).is_none() {
            app.world_mut().entity_mut(weapon).despawn();
        }
    }
    assert!(
        ranged_weapon_of(app, shooter).is_none(),
        "the case needs the selected shooter holding no gun, and despawning what it wields is \
         what leaves it that way",
    );
}

fn ranged_weapon_of(app: &App, shooter: Entity) -> Option<Entity> {
    let held: Vec<Entity> = app
        .world()
        .get::<Wields>(shooter)
        .map(|wields| wields.iter().collect())
        .unwrap_or_default();
    held.into_iter()
        .find(|weapon| app.world().get::<MeleeWeapon>(*weapon).is_none())
}
