//! The MCP tool registry — the tools this bridge exposes (GTW-741, GTW-745, GTW-749,
//! GTW-766).
//!
//! Eight tools ([`SendInput`](ToolName::SendInput) …
//! [`StepperControl`](ToolName::StepperControl)) map 1:1 onto a
//! [`QaRequest`](gdtf_qa_protocol::envelope::QaRequest) forwarded to a running game; two more
//! ([`LaunchGame`](ToolName::LaunchGame) / [`StopGame`](ToolName::StopGame)) are host-local —
//! they start and stop the game process itself and never reach the wire. The [`ToolName`]
//! enum is the single place the tool set is enumerated: `tools/list` walks it, and
//! [`ToolName::from_wire`] resolves a `tools/call` name.

use serde_json::{Value, json};

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
    /// Launch the game as a child process and wait for it to answer — host-local, no
    /// wire request.
    LaunchGame,
    /// Stop the running game child — host-local, no wire request.
    StopGame,
}

/// Every tool, in listing order — the one enumeration `tools/list` and any future
/// registry walk read.
const ALL: &[ToolName] = &[
    ToolName::SendInput,
    ToolName::QueryState,
    ToolName::GetOutput,
    ToolName::TakeScreenshot,
    ToolName::ScreenshotAfter,
    ToolName::AppFlow,
    ToolName::StartBattle,
    ToolName::StepperControl,
    ToolName::ActivateMenuItem,
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
            Self::LaunchGame => "launch_game",
            Self::StopGame => "stop_game",
        }
    }

    /// Resolve a `tools/call` name to its [`ToolName`], or `None` for an unknown tool.
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        ALL.iter().copied().find(|tool| tool.wire_name() == name)
    }

    /// A one-line human description of what the tool does.
    const fn description(self) -> &'static str {
        match self {
            Self::SendInput => {
                "Inject one battle intent (a NetIntent) into the running game as the \
                 selected ganger. Argument `intent` is a NetIntent as a JSON object or a \
                 compact-RON string. Besides the act intents (Fire / Move / SetStance / \
                 Reload / EndTurn / the contextual acts / Select* / Level*), this also \
                 drives keyboard-shaped behaviour for QA: {\"PressKey\": {\"key\": \
                 {\"Key\": \"Tab\"}}} taps a physical key (or {\"key\": {\"Action\": \
                 \"SelectClear\"}} taps whatever key a named keybind is on); {\"Hover\": \
                 {\"at\": {\"x\": 120, \"y\": 48}}} moves the mouse to a window pixel; \
                 {\"SetFocus\": {\"target\": <entity-token>}} points UI input focus at an \
                 entity. Each routes through the same windowing-input path local input uses."
            }
            Self::QueryState => {
                "Read a compact curated battle snapshot (gangers, terrain, fog, selection, \
                 turn) as JSON. Terrain is summarized (grid dims plus sparse doors and \
                 emplacements), and fog is summarized as visible/explored CELL COUNTS, not \
                 per-cell lists — so the snapshot stays a fixed, small size on any map. No \
                 arguments."
            }
            Self::GetOutput => {
                "Drain buffered combat events as JSON. Optional argument `max` caps how \
                 many events are drained."
            }
            Self::TakeScreenshot => {
                "Capture a screenshot of the running game and return it as an image. \
                 Optional argument `name` picks the file stem."
            }
            Self::ScreenshotAfter => {
                "Inject one battle intent (a NetIntent), then capture a screenshot \
                 `frame_delay` frames after it queues, returning the image — the \
                 frame-exact way to catch a transient effect (a muzzle flash, an impact \
                 flash) mid-animation, since a request/response round-trip cannot land on \
                 a specific frame itself. Also the way to capture keyboard-driven behaviour: \
                 pass a raw-input intent (e.g. {\"PressKey\": {\"key\": {\"Key\": \"Tab\"}}} \
                 or a {\"Hover\": ...} / {\"SetFocus\": ...}) and screenshot the frames after \
                 to see focus / hover move. Argument `intent` is a NetIntent as a JSON \
                 object or a compact-RON string. Argument `frame_delay` is the frame \
                 count to wait (0 captures on the very next frame). If the intent is \
                 rejected (e.g. an unoffered target), NO screenshot is taken and the tool \
                 reports the rejection reason instead. Optional argument `name` picks the \
                 file stem."
            }
            Self::AppFlow => {
                "Read the app-lifecycle snapshot (which AppState, whether a battle is \
                 running, and an `available` list of the request kinds the game will \
                 service right now) as JSON. Call this first and act only on what its \
                 `available` list advertises: the battle-only requests (query_state, \
                 send_input, get_output) are absent until a battle is running. This also \
                 answers 'what menu am I on, what can I click': on a menu the `menu` field \
                 carries its `id` and `items` ({token, label, enabled}) — pass an item's \
                 `token` to activate_menu_item; `menu` is null off a menu. No arguments."
            }
            Self::StartBattle => {
                "Start a battle from a situation, taking a freshly launched game from \
                 the menu into a running battle — the step that makes the battle-only \
                 tools (query_state, send_input, get_output) available. Argument \
                 `situation` is the situation name to start; the game currently ships \
                 one, \"skirmish\", and rejects any other name. Optional argument `seed` \
                 pins the procgen RNG so a run is reproducible; omit it for a \
                 game-chosen seed. Only accepted once the game has booted through to the \
                 menu: call `app_flow` first and wait until it reports `state: \"Running\"` \
                 with `battle_active: false` — sent earlier (while `state` is still Init, \
                 Load, or Intro) the request is rejected with the same BadRequest an \
                 unknown situation gets. The reply is the app-flow snapshot as of the \
                 moment the request was ACCEPTED, so it still reports \
                 `battle_active: false` — generating the battle takes a moment. Poll \
                 `app_flow` until `battle_active` is true before calling the battle-only \
                 tools."
            }
            Self::StepperControl => {
                "Drive the DEV-ONLY procgen load-time stepper over the wire: advance one \
                 stage (Next), toggle Auto free-run on/off, or skip to completion (Skip). \
                 Argument `command` is the string \"Next\" or \"Skip\", or an object \
                 {\"Auto\": {\"running\": true}} to start (or {\"running\": false} to stop) \
                 Auto free-run — as a JSON value or a compact-RON string. Serviceable ONLY \
                 while a procgen-stepper drive is actually in flight (during a battle's \
                 Generation, with the stepper engaged via GDTF_PROCGEN_STEPPER); otherwise \
                 it is rejected StepperInactive. Poll `app_flow` and act only when its \
                 `available` list includes StepperControl."
            }
            Self::ActivateMenuItem => {
                "Activate (click) one enumerated menu item by reference. Read the current \
                 menu from `app_flow`'s `menu.items`, then pass an item's `token` here to \
                 activate it through the game's real focus-activation path (as if pressing \
                 Enter on that button). A token that no longer names a live, listed menu \
                 item is rejected StaleToken; a disabled item's activation is a no-op."
            }
            Self::LaunchGame => {
                "Launch the game as a child process with the net_qa control channel \
                 enabled, then wait for it to answer before returning its port and pid. \
                 If a game is already running, returns the existing one (no second \
                 launch). Optional argument `port` picks the loopback port."
            }
            Self::StopGame => {
                "Stop the running game child (graceful SIGTERM, then SIGKILL fallback) and \
                 reap it. Returns a typed result when there is nothing to stop. No \
                 arguments."
            }
        }
    }

    /// The JSON-Schema for this tool's `arguments` object.
    fn input_schema(self) -> Value {
        match self {
            Self::SendInput => json!({
                "type": "object",
                "properties": {
                    "intent": {
                        "description": "A NetIntent, as a JSON object (e.g. \
                         {\"Reload\": null}) or a compact-RON string (e.g. \"Reload\")."
                    }
                },
                "required": ["intent"]
            }),
            Self::QueryState | Self::AppFlow | Self::StopGame => json!({
                "type": "object",
                "properties": {}
            }),
            Self::LaunchGame => json!({
                "type": "object",
                "properties": {
                    "port": { "type": "integer", "minimum": 0, "maximum": 65535,
                              "description": "Loopback port for the game's net_qa \
                               listener; omit for the default." }
                }
            }),
            Self::GetOutput => json!({
                "type": "object",
                "properties": {
                    "max": { "type": "integer", "minimum": 0,
                             "description": "Cap on events drained; omit to drain all." }
                }
            }),
            Self::TakeScreenshot => json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string",
                              "description": "Screenshot file stem; omit for a chosen name." }
                }
            }),
            Self::ScreenshotAfter => json!({
                "type": "object",
                "properties": {
                    "intent": {
                        "description": "A NetIntent, as a JSON object (e.g. \
                         {\"Reload\": null}) or a compact-RON string (e.g. \"Reload\")."
                    },
                    "frame_delay": { "type": "integer", "minimum": 0,
                                     "description": "Frames to wait, after the intent \
                                      queues, before capturing (0 = the very next frame)." },
                    "name": { "type": "string",
                              "description": "Screenshot file stem; omit for a chosen name." }
                },
                "required": ["intent", "frame_delay"]
            }),
            Self::StartBattle => json!({
                "type": "object",
                "properties": {
                    "situation": { "type": "string",
                                   "description": "The situation to start. The game \
                                    currently ships one, \"skirmish\"." },
                    "seed": { "type": "integer", "minimum": 0,
                              "description": "Deterministic procgen seed; omit for a \
                               game-chosen seed." }
                },
                "required": ["situation"]
            }),
            Self::StepperControl => json!({
                "type": "object",
                "properties": {
                    "command": {
                        "description": "A stepper command: the string \"Next\" or \"Skip\", \
                         or an object {\"Auto\": {\"running\": true}} to start / stop Auto \
                         free-run. Accepts a JSON value or a compact-RON string."
                    }
                },
                "required": ["command"]
            }),
            Self::ActivateMenuItem => json!({
                "type": "object",
                "properties": {
                    "token": { "type": "integer", "minimum": 0,
                               "description": "The menu item's numeric token, read from \
                                `app_flow`'s `menu.items[].token`." }
                },
                "required": ["token"]
            }),
        }
    }

    /// This tool's `tools/list` descriptor: name, description, and input schema.
    fn descriptor(self) -> Value {
        json!({
            "name": self.wire_name(),
            "description": self.description(),
            "inputSchema": self.input_schema(),
        })
    }
}

/// Build the `tools/list` result — the descriptor for every registered tool.
#[must_use]
pub fn tools_list_result() -> Value {
    let tools: Vec<Value> = ALL.iter().map(|tool| tool.descriptor()).collect();
    json!({ "tools": tools })
}

#[cfg(test)]
mod tests {
    use super::{ToolName, tools_list_result};

    /// `tools/list` advertises exactly the eleven implemented tools — the nine forwarding
    /// tools plus the two lifecycle tools.
    ///
    /// `start_battle` is asserted PRESENT: the game has serviced `QaRequest::StartBattle`
    /// since T9 (GTW-742), but no client tool sent it, so an agent could never reach a
    /// battle over the wire and the battle-only tools stayed unavailable forever. That
    /// gap survived a green suite because the only coverage was game-side (GTW-760).
    /// `stepper_control` is asserted PRESENT for the SAME reason (GTW-766): the game
    /// services `QaRequest::StepperControl`, so a missing client tool would be an
    /// unreachable half. `activate_menu_item` is asserted PRESENT for the SAME reason
    /// (GTW-787): the game services `QaRequest::ActivateMenuItem`, so a missing client tool
    /// would leave menu clicks unreachable over the wire.
    #[test]
    fn lists_every_tool_including_start_battle() {
        let result = tools_list_result();
        let Some(tools) = result["tools"].as_array() else {
            unreachable!("tools/list result carries a `tools` array");
        };
        assert_eq!(tools.len(), 11);
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        assert!(names.contains(&"send_input"));
        assert!(names.contains(&"query_state"));
        assert!(names.contains(&"get_output"));
        assert!(names.contains(&"take_screenshot"));
        assert!(names.contains(&"screenshot_after"));
        assert!(names.contains(&"app_flow"));
        assert!(names.contains(&"launch_game"));
        assert!(names.contains(&"stop_game"));
        assert!(names.contains(&"start_battle"));
        assert!(names.contains(&"stepper_control"));
        assert!(names.contains(&"activate_menu_item"));
    }

    /// Every wire name round-trips through `from_wire`, and an unknown name resolves to
    /// `None`.
    #[test]
    fn wire_names_round_trip() {
        for tool in [
            ToolName::SendInput,
            ToolName::QueryState,
            ToolName::GetOutput,
            ToolName::TakeScreenshot,
            ToolName::ScreenshotAfter,
            ToolName::AppFlow,
            ToolName::StartBattle,
            ToolName::StepperControl,
            ToolName::ActivateMenuItem,
            ToolName::LaunchGame,
            ToolName::StopGame,
        ] {
            assert_eq!(ToolName::from_wire(tool.wire_name()), Some(tool));
        }
        assert_eq!(ToolName::from_wire("nope"), None);
    }
}
