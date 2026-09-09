//! Re-exports of scene markers and screen types under `test-support`.

pub use gdtf_ui::UiPlugin;

#[cfg(feature = "mcp")]
pub use crate::dev::mcp::{
    ActCommandSystems, BattleActivity, BattleModel, BattleScreen, ContextualReply, GameFacts,
    GameFactsParam, MCP_PROTOCOL_VERSION, MCP_SERVER_NAME, McpPlugin, PlaybackCatchUp,
    PresenterReadiness, StepperActivity, TurnChangeCount, TurnOwner,
    assert_game_command_set_is_conformant, count_turn_changes, game_command_names, mcp_hello_facts,
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
    AfterMathState, AppState, BattleScapeState, GameState, RunningState, ScenesPlugin,
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
                ContextualPanelRoot, ContextualPanelSystems, EnterEmplacementButton, ExecuteButton,
                ExitEmplacementButton, MeleeButton, OpenDoorButton, ShoveButton, StabilizeButton,
                ThrowGrenadeButton,
            },
            generation::{
                battle_sim::{PreplacedGangers, ResolvedBattleSeed},
                loading_screen::test_support::LoadingScreenRoot,
                test_support::GenerationComplete,
            },
            inspect_panel::test_support::{
                InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
                InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
                ShownOccupancyGrid,
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
