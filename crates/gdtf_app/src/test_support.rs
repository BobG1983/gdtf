//! `#[cfg(feature = "test-support")] pub(crate) mod test_support` submodule in
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

#[must_use]
pub fn app_state(app: &App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

#[must_use]
pub fn load_released(app: &App) -> bool {
    matches!(app_state(app), AppState::Intro | AppState::Running)
}

pub fn seed_load_gate(asset_server: Option<Res<AssetServer>>, mut commands: Commands) {
    if asset_server.is_none() {
        commands.insert_resource(GangerStatTuning::default());
    }
    seed_load_fallbacks(asset_server, commands);
}

#[cfg(all(debug_assertions, feature = "net_qa"))]
pub use crate::dev::net_qa::{
    NET_QA_PROTOCOL_VERSION, NET_QA_SERVER_NAME, NetQaPlugin, QaShotDir, ScreenshotPayload,
    ShotPollBudget, assert_game_command_set_is_conformant, game_command_names, net_qa_hello_facts,
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
            generation::loading_screen::test_support::LoadingScreenRoot,
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

///    `SubStates` type must be registered after its `#[source(...)]` parent (see
pub fn register_headless(app: &mut App) {
    app.add_plugins(StatesPlugin);
    app.init_state::<AppState>();
    app.add_plugins(InputPlugin);
    app.add_plugins(ScenesPlugin);
    app.add_plugins(UiPlugin);
}

pub fn register_scenes_with_default_plugins(app: &mut App) {
    app.init_state::<AppState>();
    app.add_plugins(ScenesPlugin);
    app.add_plugins(UiPlugin);
}
