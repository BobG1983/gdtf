//! Test helpers and re-exports of scene markers under `test-support`.

use bevy::{
    app::App,
    asset::AssetServer,
    ecs::system::{Commands, Res},
    input::InputPlugin,
    state::{
        app::{AppExtStates, StatesPlugin},
        state::State,
    },
};
use gdtf_battle_sim::tuning::GangerStatTuning;
pub use gdtf_ui::UiPlugin;

/// Current [`AppState`] on the app.
#[must_use]
pub fn app_state(app: &App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// Whether load has finished and the app is in Intro or Running.
#[must_use]
pub fn load_released(app: &App) -> bool {
    matches!(app_state(app), AppState::Intro | AppState::Running)
}

/// Insert load-gate resources when no asset server is present (headless tests).
pub fn seed_load_gate(asset_server: Option<Res<AssetServer>>, mut commands: Commands) {
    if asset_server.is_none() {
        commands.insert_resource(GangerStatTuning::default());
    }
    seed_load_fallbacks(asset_server, commands);
}

#[cfg(debug_assertions)]
pub use crate::dev::net_qa::{
    BattleActivity, BattleModel, BattleScreen, GameFacts, GameFactsParam, NET_QA_PROTOCOL_VERSION,
    NET_QA_SERVER_NAME, NetQaPlugin, PresenterReadiness, StepperActivity,
    assert_game_command_set_is_conformant, game_command_names, net_qa_hello_facts,
    shorten_wait_budget,
};
#[cfg(feature = "dev_tools")]
pub use crate::dev::procgen_stepper::{
    AutoRunning, AutoStepDelay, PendingStepCommand, ProcgenStepperActive, ProcgenStepperPlugin,
    StepCommand, draw_schematic,
};
#[cfg(feature = "dev_tools")]
pub use crate::states::running::options::test_support::{
    ProcgenStepperToggle, ProcgenStepperValueLabel,
};
pub use crate::states::{
    AfterMathState, AppState, BattleScapeState, GameState, LoadedSituation, RunningState,
    ScenesPlugin,
    running::{
        game::battlescape::{
            BottomBarRoot,
            action_bar::test_support::{
                AimToggleButton, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
                ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
                StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton,
                StanceStandingButton,
            },
            battle_running::test_support::BattleRunningComplete,
            combat_log::test_support::{CombatLogLine, CombatLogRoot},
            contextual_panel::test_support::{
                ContextualPanelRoot, EnterEmplacementButton, ExecuteButton, ExitEmplacementButton,
                MeleeButton, OpenDoorButton, ShoveButton, StabilizeButton, ThrowGrenadeButton,
            },
            generation::{
                battle_sim::ResolvedBattleSeed, loading_screen::test_support::LoadingScreenRoot,
                test_support::GenerationComplete,
            },
            inspect_panel::test_support::{
                InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
                InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
            },
            select_cycle::test_support::{SelectCycleRoot, SelectNextButton, SelectPrevButton},
            stat_block::test_support::{
                StatFaction, StatHpBar, StatHpLabel, StatInjuryLine, StatInjuryList, StatName,
                StatPortrait, StatStance, StatTuBar, StatTuLabel, StatWoundLine, StatWoundList,
                StatWoundsPips, portrait_index_for_name,
            },
            status_panel::stability_readout::test_support::StabilityBar,
            weapon_panel::test_support::{
                AimLabel, AimPanel, CombinedWeaponPanel, ReloadButton, WeaponContent, WeaponImage,
                WeaponItemButton, WeaponItemPanel, WeaponMagazineText, WeaponNameText,
                WeaponPanelRoot,
            },
        },
        menu::test_support::{
            BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton,
            StartBattleRequested,
        },
        options::test_support::{
            ContinueButton, OptionsScreenRoot, OptionsTitle, SoundToggle, SoundValueLabel,
        },
    },
    seed_load_fallbacks,
};

/// Register states, scenes, and UI for a headless test app.
pub fn register_headless(app: &mut App) {
    app.add_plugins(StatesPlugin);
    app.init_state::<AppState>();
    app.add_plugins(InputPlugin);
    app.add_plugins(ScenesPlugin);
    app.add_plugins(UiPlugin);
}

/// Register scenes and UI on an app that already has a default plugin stack.
pub fn register_scenes_with_default_plugins(app: &mut App) {
    app.init_state::<AppState>();
    app.add_plugins(ScenesPlugin);
    app.add_plugins(UiPlugin);
}
