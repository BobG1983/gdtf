//! [`ToolName`] — the tool set, its listing order, the wire-name mapping, and which host
//! each tool acts on.

use crate::hosts::QaHost;

/// One MCP tool the bridge exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolName {
    /// Inject one battle intent — maps to `QaRequest::Inject`.
    SendInput,
    /// Read the whole battle snapshot — maps to `QaRequest::GetBattleState`.
    QueryState,
    /// Drain buffered combat events — maps to `QaRequest::GetOutput`.
    GetOutput,
    /// Capture a screenshot — maps to `QaRequest::TakeScreenshot`.
    TakeScreenshot,
    /// Inject one battle intent, then capture a screenshot a fixed number of frames
    /// later — maps to `QaRequest::ScreenshotAfter` (GTW-749).
    ScreenshotAfter,
    /// Read the app-lifecycle snapshot — maps to `QaRequest::GetAppFlow`.
    AppFlow,
    /// Start a battle from a situation — maps to `QaRequest::StartBattle`. The
    /// navigation step that takes a cold-started game from the menu into a battle, so
    /// the battle-only tools become available (GTW-742's required path, reached from
    /// the client half by GTW-760).
    StartBattle,
    /// Drive the DEV procgen load-time stepper (Next / Auto / Skip) — maps to
    /// `QaRequest::StepperControl` (GTW-766).
    StepperControl,
    /// Activate one enumerated menu item by its token — maps to
    /// `QaRequest::ActivateMenuItem` (GTW-787).
    ActivateMenuItem,
    /// Move focus / activate a focused control on ANY focus-navigable screen — maps to
    /// `QaRequest::FocusControl` (GTW-802). Unlike `send_input` it is not battle-gated,
    /// so it is how an off-battle screen (Options) is driven.
    FocusControl,
    /// Launch the game as a child process and wait for it to answer — host-local, no
    /// wire request.
    LaunchGame,
    /// Stop the running game child — host-local, no wire request.
    StopGame,
    /// Ask the running CONTENT EDITOR which query topics it will service right now —
    /// maps to `QaRequest::GetEditorQueryOptions` (GTW-805's discovery half, reachable
    /// from a client since GTW-808).
    GetEditorQueryOptions,
    /// Ask the running CONTENT EDITOR about ONE topic — maps to
    /// `QaRequest::QueryEditor` (GTW-805's read half, reachable since GTW-808).
    QueryEditor,
    /// Launch the content editor as a child process and wait for it to answer —
    /// host-local, no wire request (GTW-808).
    LaunchEditor,
    /// Stop the running content-editor child — host-local, no wire request (GTW-808).
    StopEditor,
    /// Read a host's live command catalogue — maps to `QaRequest::Catalogue` (GTW-942).
    ///
    /// One of the two COURIER tools: it names no command, because a host's vocabulary is
    /// data in the reply rather than a shape written here.
    Commands,
    /// Run one command from a host's catalogue by name — maps to `QaRequest::Run`
    /// (GTW-942). The other courier tool, and equally command-agnostic.
    Run,
}

/// Every tool, in listing order — the one enumeration `tools/list` and any future
/// registry walk read.
pub(super) const ALL: &[ToolName] = &[
    ToolName::SendInput,
    ToolName::QueryState,
    ToolName::GetOutput,
    ToolName::TakeScreenshot,
    ToolName::ScreenshotAfter,
    ToolName::AppFlow,
    ToolName::StartBattle,
    ToolName::StepperControl,
    ToolName::ActivateMenuItem,
    ToolName::FocusControl,
    ToolName::LaunchGame,
    ToolName::StopGame,
    ToolName::GetEditorQueryOptions,
    ToolName::QueryEditor,
    ToolName::LaunchEditor,
    ToolName::StopEditor,
    ToolName::Commands,
    ToolName::Run,
];

impl ToolName {
    /// The MCP wire name a client calls this tool by.
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::SendInput => "send_input",
            Self::QueryState => "query_state",
            Self::GetOutput => "get_output",
            Self::TakeScreenshot => "take_screenshot",
            Self::ScreenshotAfter => "screenshot_after",
            Self::AppFlow => "app_flow",
            Self::StartBattle => "start_battle",
            Self::StepperControl => "stepper_control",
            Self::ActivateMenuItem => "activate_menu_item",
            Self::FocusControl => "focus_control",
            Self::LaunchGame => "launch_game",
            Self::StopGame => "stop_game",
            Self::GetEditorQueryOptions => "get_editor_query_options",
            Self::QueryEditor => "query_editor",
            Self::LaunchEditor => "launch_editor",
            Self::StopEditor => "stop_editor",
            Self::Commands => "commands",
            Self::Run => "run",
        }
    }

    /// Which child process this tool acts on.
    ///
    /// The one place the tool set is partitioned between the two hosts: a forwarding tool
    /// travels over THAT host's link, and a launch / stop tool drives THAT host's
    /// lifecycle. A wildcard-free `match`, so a new tool must state its host rather than
    /// defaulting to the game's link and failing with a mismatched reply (GTW-808).
    ///
    /// This is the host a call takes when it names none. A tool that
    /// [`accepts_host_argument`](Self::accepts_host_argument) may be aimed at the other child
    /// per call; every other tool is fixed to what this returns.
    #[must_use]
    pub const fn host(self) -> QaHost {
        match self {
            Self::SendInput
            | Self::QueryState
            | Self::GetOutput
            | Self::TakeScreenshot
            | Self::ScreenshotAfter
            | Self::AppFlow
            | Self::StartBattle
            | Self::StepperControl
            | Self::ActivateMenuItem
            | Self::FocusControl
            | Self::LaunchGame
            | Self::StopGame
            // The two courier tools default to the game and are AIMED per call
            // (`accepts_host_argument`), because a command layer is a property both hosts
            // have rather than a thing only one of them does (GTW-942).
            | Self::Commands
            | Self::Run => QaHost::Game,
            Self::GetEditorQueryOptions
            | Self::QueryEditor
            | Self::LaunchEditor
            | Self::StopEditor => QaHost::Editor,
        }
    }

    /// Whether a call may AIM this tool at a host with a `host` argument instead of taking
    /// the one [`host`](Self::host) names.
    ///
    /// `take_screenshot` (GTW-880), because both children can capture a frame and one tool
    /// that takes `"game"` or `"editor"` is what lets a QA pass screenshot the editor without
    /// a second, near-identical tool; and the two COURIER tools (GTW-942), because a command
    /// layer is something BOTH hosts have — a `commands` fixed to the game could never read
    /// the editor's catalogue, and duplicating the pair per host is exactly the growth this
    /// design removes.
    ///
    /// Every other tool means something on exactly one host — an `inject` has no meaning in
    /// the editor, a `query_editor` none in the game — so their host stays fixed and a `host`
    /// argument on them is ignored.
    #[must_use]
    pub const fn accepts_host_argument(self) -> bool {
        matches!(self, Self::TakeScreenshot | Self::Commands | Self::Run)
    }

    /// Whether this tool starts or stops a child process instead of forwarding a request
    /// to a running one.
    #[must_use]
    pub const fn is_launch(self) -> bool {
        matches!(self, Self::LaunchGame | Self::LaunchEditor)
    }

    /// Whether this tool stops a child process instead of forwarding a request.
    #[must_use]
    pub const fn is_stop(self) -> bool {
        matches!(self, Self::StopGame | Self::StopEditor)
    }

    /// Resolve a `tools/call` name to its [`ToolName`], or `None` for an unknown tool.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        ALL.iter().copied().find(|tool| tool.wire_name() == name)
    }
}
