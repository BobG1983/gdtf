//! [`ToolName`] — the tool set, its listing order, and the wire-name mapping.

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
        }
    }

    /// Resolve a `tools/call` name to its [`ToolName`], or `None` for an unknown tool.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        ALL.iter().copied().find(|tool| tool.wire_name() == name)
    }
}
