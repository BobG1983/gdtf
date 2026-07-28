//! The per-tool human description a client reads to learn what a tool does and how to
//! sequence it (the discoverability contract of ADR 0007).

use crate::mcp::tools::name::ToolName;

impl ToolName {
    /// A one-line human description of what the tool does.
    pub(super) const fn description(self) -> &'static str {
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
                 entity. Each routes through the same windowing-input path local input \
                 uses. This tool needs a running battle; off-battle, drive the UI with \
                 focus_control instead."
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
            Self::ScreenshotAfter => Self::SCREENSHOT_AFTER_DESCRIPTION,
            Self::AppFlow => Self::APP_FLOW_DESCRIPTION,
            Self::StartBattle => Self::START_BATTLE_DESCRIPTION,
            Self::StepperControl => {
                "Drive the DEV-ONLY procgen load-time stepper over the wire: advance one \
                 stage (Next), toggle Auto free-run on/off, or skip to completion (Skip). \
                 Argument `command` is the string \"Next\" or \"Skip\", or an object \
                 {\"Auto\": {\"running\": true}} to start (or {\"running\": false} to stop) \
                 Auto free-run — as a JSON value or a compact-RON string. Serviceable ONLY \
                 while a procgen-stepper drive is actually in flight (during a battle's \
                 Generation, with the stepper engaged). Engage it first: open the Options \
                 screen and turn on its dev-only procgen-stepper toggle (it defaults to off, \
                 and it takes effect for the NEXT battle generation); otherwise \
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
            Self::FocusControl => Self::FOCUS_CONTROL_DESCRIPTION,
            Self::LaunchGame => Self::LAUNCH_GAME_DESCRIPTION,
            Self::StopGame => {
                "Stop the running game child (graceful SIGTERM, then SIGKILL fallback) and \
                 reap it. Returns a typed result when there is nothing to stop. No \
                 arguments."
            }
        }
    }

    /// The `launch_game` description — it names all four recipe arguments, because a
    /// caller that cannot see them falls back to hand-rolled tooling (GTW-875).
    const LAUNCH_GAME_DESCRIPTION: &'static str = "Launch the game as a child process with the net_qa control channel \
         enabled, then wait for it to answer before returning its port and pid, \
         plus the package, features, and directory it was built from. If a game \
         built from the SAME recipe is already running, returns that child (no \
         second launch), naming its package, features, and directory; if one \
         built from a DIFFERENT recipe is running, the call is rejected and says \
         what is actually up — call stop_game first. Optional argument `port` picks the \
         loopback port. Optional `package` picks the cargo package (default \
         grimdark_turfwar). Optional `features` picks the cargo features, as an \
         array or a comma-separated string (default dynamic_linking,net_qa) — add \
         dev_tools to reach the dev-only features such as the procgen stepper. \
         Optional `working_dir` picks WHICH CHECKOUT is built: pass a git worktree \
         path to QA that tree instead of whatever directory this host runs in. \
         Optional `env` is an object of extra environment variables for the child, \
         e.g. {\"GDTF_BATTLE_SEED\": \"42\"}.";

    /// The `screenshot_after` description — pulled out of the `match` so no single arm
    /// pushes [`description`](Self::description) past the clippy `too_many_lines` ceiling.
    const SCREENSHOT_AFTER_DESCRIPTION: &'static str = "Inject one battle intent (a NetIntent), then capture a screenshot \
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
         file stem.";

    /// The `app_flow` description — the poll-first entry point, so it names BOTH token
    /// handouts a client acts on (`menu.items` and `focus.focusables`).
    const APP_FLOW_DESCRIPTION: &'static str = "Read the app-lifecycle snapshot (which AppState, whether a battle is \
         running, and an `available` list of the request kinds the game will \
         service right now) as JSON. Call this first and act only on what its \
         `available` list advertises: the battle-only requests (query_state, \
         send_input, get_output) are absent until a battle is running. This also \
         answers 'what menu am I on, what can I click': on a menu the `menu` field \
         carries its `id` and `items` ({token, label, enabled}) — pass an item's \
         `token` to activate_menu_item; `menu` is null off a menu. And on ANY \
         focus-navigable screen (the Options screen, the menu, the battlescape HUD \
         panels) the `focus` field carries `focused` plus `focusables` ({token, \
         label, kind, enabled, checked}) — pass a `token` to focus_control to move \
         focus to, or click, that control; `focus` is null on a screen with no \
         focus chain. No arguments.";

    /// The `start_battle` description — the navigation step and its polling contract.
    const START_BATTLE_DESCRIPTION: &'static str = "Start a battle from a situation, taking a freshly launched game from \
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
         tools.";

    /// The `focus_control` description — the whole off-battle UI-drive workflow, spelled
    /// out because discoverability from the tool list alone is the point (ADR 0007).
    const FOCUS_CONTROL_DESCRIPTION: &'static str = "Drive the focus of ANY focus-navigable screen: move focus, point focus at \
         a control, or activate (click) the focused one. Unlike send_input this \
         works OFF-BATTLE, so it is how you drive the Options screen, the menu, or \
         any other bevy_ui screen. Read the controls from `app_flow`'s \
         `focus.focusables`; each row carries `token`, `label`, `kind` (Button / \
         Checkbox / Other), `enabled`, and — for a checkbox — its current \
         `checked` value. Argument `command` is a JSON value or a compact-RON \
         string: {\"ActivateTarget\": <token>} points focus at that control and \
         clicks it; \"Activate\" clicks whatever is focused; {\"Step\": \"Next\"} \
         (or \"Prev\" / \"Left\" / \"Right\") moves focus one control; {\"Focus\": \
         <token>} points focus without clicking. Everything runs through the real \
         focus/input path — a Step writes the same navigate message an arrow key \
         writes, and an activation emits a real Enter keypress at the focused \
         control. A token that is not a currently listed focusable is rejected \
         StaleToken. The effect lands on the FOLLOWING frame, so re-read `app_flow` \
         to confirm (a checkbox reports its new `checked` value there).";
}
