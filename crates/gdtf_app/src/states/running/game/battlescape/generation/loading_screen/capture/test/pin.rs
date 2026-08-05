use bevy::{prelude::*, state::app::StatesPlugin};
use gdtf_screenshot::{CaptureCompletions, CapturePath, CaptureQueue, PollCap};

use super::super::{register::wire_loading_capture, systems::pin_generation_until_shot};
use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::loading_screen::components::LoadingScreenRoot,
};

const TEST_POLL_BUDGET: u32 = 2;

const DRIVE_UPDATES: u32 = 64;

// The pin stops writing on the frame after the completion lands; the state applies on the next.
const RELEASE_FRAMES: u32 = 4;

// Stands in for the FixedUpdate system that advances Generation once the map is built.
fn ask_to_leave_the_loading_screen(mut next: ResMut<NextState<BattleScapeState>>) {
    next.set(BattleScapeState::AnimateIn);
}

fn loading_screen_app() -> (App, tempfile::TempDir) {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let path = CapturePath::new(tmp.path().join("loading.png"));
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(StatesPlugin);
    app.init_state::<BattleScapeState>();
    wire_loading_capture(&mut app, path);
    app.insert_resource(PollCap::new(TEST_POLL_BUDGET));
    app.add_systems(
        Update,
        ask_to_leave_the_loading_screen.before(pin_generation_until_shot),
    );
    app.world_mut().spawn(LoadingScreenRoot);
    (app, tmp)
}

fn state_of(app: &App) -> BattleScapeState {
    *app.world().resource::<State<BattleScapeState>>().get()
}

fn capture_is_in_flight(app: &App) -> bool {
    !app.world().resource::<CaptureQueue<()>>().is_idle()
}

fn capture_finished(app: &App) -> bool {
    !app.world().resource::<CaptureCompletions<()>>().is_empty()
}

#[test]
fn the_loading_screen_holds_while_its_shot_is_still_in_flight() {
    let (mut app, _tmp) = loading_screen_app();

    app.update();
    assert!(
        capture_is_in_flight(&app),
        "test setup: the loading screen must queue its shot on the first update, or the pin \
         below proves nothing",
    );

    let mut held_frames = 0;
    while capture_is_in_flight(&app) && held_frames < DRIVE_UPDATES {
        app.update();
        held_frames += 1;
        assert_eq!(
            state_of(&app),
            BattleScapeState::Generation,
            "something asked to advance to AnimateIn on frame {held_frames} while the loading \
             shot was still in flight; the pin must override it until the PNG lands or the \
             capture gives up",
        );
    }
    assert!(
        held_frames > 1,
        "test setup: the capture must stay in flight for more than one frame, or the loop above \
         asserts nothing",
    );
}

#[test]
fn the_pin_releases_once_the_shot_finishes_and_not_before() {
    let (mut app, _tmp) = loading_screen_app();

    for _ in 0..DRIVE_UPDATES {
        app.update();
        if capture_finished(&app) {
            break;
        }
        assert_eq!(
            state_of(&app),
            BattleScapeState::Generation,
            "the pin must hold until the capture FINISHES — requesting it is not enough",
        );
    }
    assert!(
        capture_finished(&app),
        "the capture must finish within {DRIVE_UPDATES} frames; headless it can only time out",
    );

    for _ in 0..RELEASE_FRAMES {
        app.update();
        if state_of(&app) == BattleScapeState::AnimateIn {
            return;
        }
    }
    unreachable!(
        "with its shot finished the loading screen must stop pinning Generation within \
         {RELEASE_FRAMES} frames and let the app move on; it is still {:?}",
        state_of(&app),
    );
}
