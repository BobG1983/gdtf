# Agent QA — driving the game over MCP

A language-model harness can drive the game and the content editor from the
outside: launch a child process, read the list of commands that host publishes,
run one, read what the child printed, and stop it. WHAT a host can be asked to do
is that published list, read at run time — not a fixed tool set. See
[qa-commands.md](qa-commands.md) for how a command is added. This is the QA
control channel. It has these processes and one shared wire contract:

- **The game**, in debug builds, opens a loopback TCP listener
  that publishes its own command list and runs a named command against the live
  app.
- **The content editor**, in debug builds, opens a SECOND
  loopback listener on its own port, publishing its own command list. It is a
  separate process from the game and can run at the same time.
- **The MCP host** (`gdtf_qa_mcp`) speaks a hand-rolled JSON-RPC 2.0 subset on
  stdin/stdout to the model harness and forwards each tool call to the game or
  the editor over that loopback socket. ONE host binary manages BOTH children:
  it holds a link and a lifecycle manager per host, so a game on
  `7616` and an editor on `7617` can be up and driven at the same time.
- **`gdtf_qa_protocol`** is the bevy-free crate they all link — `crates/gdtf_app`
  for the game, `crates/gdtf_content_editor` for the editor, and `bins/gdtf_qa_mcp`
  for the host. It carries the typed request/response message shapes, the command
  DTOs, the framing codec, and the values both ends have to agree on — the ports
  and the two timeouts.

The channel is a **dev-only** affordance: QA modules compile under
`debug_assertions` and always listen in debug builds. There is no `net_qa` cargo
feature and no environment arming variable. Release builds never contain the
listener (see [the debug-build gate](#the-debug-build-gate)). Matches
[verification.md](../../.claude/rules/verification.md).

## The launch recipe — the game

The MCP host launches the game as a child process when the harness calls the
`launch` tool. The launch is owned by the `CargoSpawner` in
`bins/gdtf_qa_mcp/src/lifecycle/spawn.rs` (behind the `ChildSpawner` trait, so
tests can substitute a stub). WHAT it launches is the call's own recipe — a
`LaunchSpec` (`bins/gdtf_qa_mcp/src/lifecycle/launch/`) of these typed values:
package, features, working directory, environment overrides. Each one
the call omits falls back to the default game recipe, so a bare `launch`
still runs:

```bash
cargo run -p grimdark_turfwar --features dynamic_linking,dev_tools
```

in the MCP host's own directory. A call that names them runs what it names —
this launches a `dev_tools` build of a git worktree, with a dev gate set:

```json
{"package": "grimdark_turfwar",
 "features": ["dynamic_linking", "dev_tools"],
 "working_dir": "/Users/you/dev/gdtf-some-worktree",
 "env": {"GDTF_BATTLE_SEED": "42"}}
```

`working_dir` is the one that decides WHICH CHECKOUT is under test: without it
the build comes from whatever directory the host itself runs in, which is how a
QA pass gets reported against code that is not the code under review. A
`working_dir` that is not an existing directory is rejected, never silently
dropped.

The spawner does **not** inject `GDTF_NET_QA` / `GDTF_NET_QA_PORT` (or the
editor pair) onto the child. Those names remain as historical `QaChannel` labels
in launch types for host identity only; debug hosts listen on fixed protocol
ports without arming. BOTH of the child's output streams are piped and drained into one ring by
`ProcessChild` — the host's own stdout is the JSON-RPC channel, so the child's
cannot share it. The `logs` tool reads the tail of that ring, and a failed launch
reports it as the diagnosis. The process is managed by the `HostManager`
(`bins/gdtf_qa_mcp/src/lifecycle/manager.rs`): `launch` starts it and waits
for it to answer before returning its port and pid — plus the package, features,
and resolved directory it was built from — and `stop` (plus stdin EOF) stops
it so the game never outlives the host.

One game runs at a time, and the manager remembers the recipe that built it:

- The same recipe again is ensure-style `already_running`, reporting the running
  child's package, features, and directory (not the ones this call asked for).
- A DIFFERENT recipe is REJECTED, naming what is actually running. Call
  `stop` first. Answering "already running" to a request for another
  checkout would hand back a success for a build that was never started.

The record lasts only as long as the process. Every 60 seconds the host asks the
OS whether each recorded child's pid is still running and drops the record when
it is not, so a closed game window does not leave `launch` answering
`already_running` with a pid nothing listens on. Until the sweep runs the stale
record is still there, so for up to a minute after the child exits `launch` can
still answer `already_running`; `stop` clears it at once. `logs` for that host
answers `not_running` once the record is gone, so read the child's tail before
the sweep drops it. The interval is `sweep_interval` on each host's
`LifecycleConfig` (`bins/gdtf_qa_mcp/src/lifecycle/config.rs`); the sweep is the
`SweepClock` in `bins/gdtf_qa_mcp/src/lifecycle/sweep.rs` calling
`reap_dead_child` on the `HostManager`, and the liveness answer comes from the
`ChildLiveness` trait (`bins/gdtf_qa_mcp/src/lifecycle/liveness.rs`), so a test
injects the answer instead of killing a real process.

QA evidence goes through the MCP host. Hand-writing a socket client against this
port is not an accepted route — that path tested the wrong tree once and is how
to get it wrong again. Running the recipe by hand from a shell is fine for
bringing the editor or game up; driving it is the host's job. If a needed tool
is missing, that is a defect to file, not a reason to bypass the host.

## The launch recipe — the content editor

`launch(host="editor")` is the editor's launch: the same host, the same
`CargoSpawner`, the same recipe arguments — with the editor's own defaults.
A bare `launch(host="editor")` runs:

```bash
cargo run -p gdtf_content_editor_bin --features dynamic_linking,file_watcher
```

in the MCP host's own directory. No arming env vars are set on the child; a
debug editor always opens its QA channel on `7617`. The historical
`GDTF_EDITOR_NET_QA` / `GDTF_EDITOR_NET_QA_PORT` names are not read by the
editor and are not required for launch.

**Warm the build first.** The editor's `dynamic_linking,dev_tools` combination is
one nothing else in the repo produces — `cargo dbuild` builds the GAME binary,
and the workspace checks never link an editor binary — so the first
`launch(host="editor")` against a given checkout is usually a COLD BUILD of the whole
editor. Two things follow:

- The editor's boot timeout is **600 seconds**, not the game's 180
  (`bins/gdtf_qa_mcp/src/lifecycle/config.rs`).
- Run `cargo edbuild` in the checkout you are about to QA before calling
  `launch(host="editor")`, and the launch answers in seconds instead.

If a launch does time out, the failure is not a bare "timed out": it names the
build as the likely cause, prints the exact `cargo build -p … --features …`
warm-up command for the recipe it ran, and carries the child's output tail — both
streams, interleaved — which shows how far the build got.

The editor can also be brought UP by hand — driving it still goes through the
MCP host, per the rule above. It is a separate binary package
(`bins/gdtf_content_editor/Cargo.toml`, package `gdtf_content_editor_bin`) with
its own default port, and running it by hand opens the same channel that launch
opens.

```bash
cargo run -p gdtf_content_editor_bin --features dynamic_linking,file_watcher
```

`cargo edrun` (`.cargo/config.toml`) is the same command as an alias, and
`cargo edbuild` builds without running. Both include `file_watcher` (hot-reload
RON).

A debug editor build always opens the channel on **`7617`**, the
`DEFAULT_EDITOR_PORT` in `crates/gdtf_content_editor/src/net_qa/config.rs` —
deliberately one above the game's `7616`, so both hosts can be up at once without
fighting for a socket. `GDTF_EDITOR_NET_QA` and `GDTF_EDITOR_NET_QA_PORT` are read
by nothing; the MCP host still sets them and the editor ignores them.

When the channel comes up the editor writes this line to **stderr** at `info`
level (`crates/gdtf_content_editor/src/net_qa/plugin.rs`), carrying the port it
actually bound:

```text
editor net_qa: ON (dev) — loopback QA control channel listening
```

To confirm the port is genuinely held by the editor process rather than by a
shell wrapper, check it while the editor is up:

```bash
lsof +c 0 -nP -iTCP:7617 -sTCP:LISTEN
```

`+c 0` is required, not decoration: macOS `lsof` truncates the `COMMAND` column
to 9 characters by default, so the same listener prints as `gdtf_cont` — a
prefix that cannot be told apart from any other `gdtf_cont*` process. `+c 0`
turns the limit off and prints the name in full.

The `COMMAND` column must read `gdtf_content_editor`. If the bind fails the
editor logs `editor net_qa: failed to bind the loopback listener — QA channel
OFF` at `error` level and carries on with no channel, so an absent listener is
always visible in the log rather than silent.

The `Hello` handshake never reaches the editor's drain at all: the shared
transport's listener thread answers it from the `HelloFacts` the editor passed
to `run_listener` (`editor_hello_facts()` in
`crates/gdtf_content_editor/src/net_qa/config.rs`), which is where the server
name `gdtf-editor-net-qa` — how a client tells the two hosts apart — now comes
from.

The editor itself answers the two command-layer requests (`route_editor_requests`
in `crates/gdtf_content_editor/src/net_qa/router/route.rs`): a `Catalogue`, with
its own command list under its own host name, and a `Run`, by admitting the named
command — or answering `Unknown`, which lists what it does offer, for a name it
does not know. That list is `EDITOR_COMMANDS` in
`crates/gdtf_content_editor/src/net_qa/commands/set.rs` and today it holds
`editor.phase`, `editor.last_save`, `editor.validation`,
`editor.families`, `editor.session`, `editor.draft`,
`editor.set_mode`, `editor.new`, `editor.load`, `editor.save`,
`editor.set_field`, `editor.list_op`, `editor.select_theme`,
`editor.toggle_terrain`, `editor.set_default_floor`, `editor.map`,
`editor.set_grid_size`, `editor.select_tile`, `editor.set_level`,
`editor.paint` and `editor.select_injury_tab`.
Capture is not wired here yet: `crates/gdtf_content_editor/Cargo.toml` does not
depend on `gdtf_screenshot`, and the QA plugin registers no capture systems. The
shared pipeline in `crates/gdtf_screenshot/src/capture/` is the path the editor
will capture through, once the capture command that fills its queue exists.
The wire shape, the framing, and the protocol version are the game's — see
[the protocol sketch](#the-protocol-sketch) — so one client library speaks to
both.

## The debug-build gate

One gate, at compile time, so the listener cannot reach a shipped binary: the
plugin wiring site `crates/gdtf_app/src/dev/plugin.rs` is `cfg(debug_assertions)`,
so a release build never contains the server code. The editor mirrors it at
`crates/gdtf_content_editor/src/app.rs`.

There is no cargo feature and no runtime arming. `net_qa` used to be a feature on
both hosts, and `GDTF_NET_QA` used to arm it at run time; both were removed. What
is left keeps the old names but reads nothing: `net_qa_enabled()`
(`crates/gdtf_app/src/dev/net_qa/env.rs`) returns a literal `true`, and
`NetQaPlugin::from_env` / `NetQaEditorPlugin::from_env` take their port from a
constant.

The listen **interface** is never configurable — hardcoded to
`Ipv4Addr::LOCALHOST` — and the **port** is a constant too: `GAME_QA_PORT`
(`7616`) and `EDITOR_QA_PORT` (`7617`) in `crates/gdtf_qa_protocol/src/ports.rs`,
wrapped as `DEFAULT_PORT` and `DEFAULT_EDITOR_PORT` in each host's
`net_qa/config.rs`. One constant per host is why the two never contend for a
socket.

## The tool vocabulary

The MCP host's tools are enumerated once by the `ToolName` enum in
`bins/gdtf_qa_mcp/src/mcp/tools/name.rs`. Some drive a child process — `launch`,
`stop`, `logs` — and the rest carry that child's own command layer: `commands` and
`run`. Every one takes the same optional `host` argument (`"game"` — the default
— or `"editor"`), so which child a call reaches is the CALL's to say rather than
the tool's. `resolve_host` in `bins/gdtf_qa_mcp/src/mcp/courier/handle.rs`
resolves it, and a `host` value naming neither child is rejected as invalid
params rather than quietly defaulting, so a typo cannot drive the wrong process.

**The two command tools name no command.** What a host can be asked to do is read
from the host itself — `commands` returns its live catalogue, `run` calls one
entry by name — so adding a command to a host adds nothing here, needs no courier
rebuild, and needs no MCP reconnect. That is the whole point of the design; see
[the command guide](qa-commands.md), which records why the shape is what it is.

| Tool | Kind | Maps to | Arguments |
| --- | --- | --- | --- |
| `launch` | host-local | — (starts the child via that host's `HostManager`) | all optional: `host`, `port`, `package`, `features` (array or comma-separated string), `working_dir`, `env` |
| `stop` | host-local | — (stops the child via that host's `HostManager`) | optional `host` |
| `logs` | host-local | — (reads the child's captured output) | optional `host`, `max_lines` (trailing lines to return) |
| `commands` | forward | `QaRequest::Catalogue` | optional `host`, `command` (name filter), `detail` (`"Summary"` — the default — or `"Full"`, which adds the published RON shapes) |
| `run` | forward | `QaRequest::Run` | `command` (required, from `commands`), `arguments` (optional string of compact RON, default `"()"`), optional `host`, `await_ready` (whole seconds), `capture` (`true` or a file stem) |

Notes an agent relies on:

- **`commands` is the affordance oracle.** Its reply is that host's live
  catalogue: one row per command with its name, a one-line summary, when it
  answers (`Immediate` or `Deferred`), and whether it can run RIGHT NOW in the
  state the host is in. A command that needs a battle reports `Unavailable` with
  the precondition named until one is running. Start every session here, and act
  on what it advertises — it is the truth about the build in front of you, and it
  changes with the host rather than with this document.
- **`detail: "Full"`** adds each row's published argument and reply shapes — the
  RON documents you read to build a `run` call. The host traces them out of the
  command's own `Deserialize` impls, the same impls that decode the wire, so they
  cannot disagree with what it accepts. `arguments` is then a string of compact
  RON shaped by that command's own `schemas.arguments`; a command that takes none
  is `"()"`.
- **`run`'s refusals each self-correct in one round trip.** A name the host
  does not know comes back `Unknown` listing every name it does offer; arguments
  it will not decode come back `BadArguments` carrying the shape they were read
  against and naming the field that broke, and so does a body that decoded but
  named a value the command cannot take; a command that cannot run in this state
  comes back `Unavailable` with the precondition named.
- **`run`'s riders** are `await_ready` (keep re-testing admission for that
  many seconds instead of deciding once) and `capture` (take a screenshot after
  the command has run). `await_ready` holds a call its command refuses right now
  and tests admission again every frame; the moment the command admits, the call
  runs. When the seconds run out the reply is the command's OWN `Unavailable`
  refusal — the one the last test produced — never a rider refusal and never
  `Timeout`. Nothing clamps the budget: a waiting call holds the channel exactly
  as a parked `wait` does, and a budget past the three minutes the socket waits
  for a reply answers `Timeout` from the connection instead. A name the host does
  not know still comes back `Unknown` at once, because waiting cannot make a name
  appear. `capture` runs the command first and appends one PNG attachment to the
  reply it produced; an outcome that is not `Ran` comes straight back with no
  shot taken, and a shot that never lands answers `Timeout` rather than the reply
  without its PNG. Both riders compose on one call: it waits for admission, runs,
  then comes back with the attachment.
- **What the channel answers today.** The GAME host publishes the commands its catalogue lists.
  `app.phase` reports where the app is at every level of its state machine.
  `capture.screenshot` writes a PNG of what the game is showing and attaches it
  to the reply. The shell reads answer before a battle: `settings.read` (the
  Options values), `ui.focus` (the focused widget and the focusable set) and
  `playback.state` (whether the screen has caught up with the act log). The
  battle reads answer inside a battle:
  `battle.roster`, `battle.turn`, `battle.selection`, `battle.offers`,
  `battle.inspect`, `battle.sightline`, `battle.visible`, `battle.cost`,
  `log.read` and `log.omniscient_read`. `battle.roster`, `battle.inspect`,
  `battle.visible`, the `can_see` half of `battle.sightline` and `log.read`
  report only what the player can see; `battle.turn`, `battle.selection`,
  `battle.offers`, `battle.cost` and the `can_engage` half of
  `battle.sightline` read live state on purpose.
  `battle.cost` is the one that quotes an act before it is taken —
  what it would charge in TU and whether the sim would allow it, with nothing in
  the battle moving. `log.omniscient_read` returns the act log unfiltered and
  fully identified: using it is cheating, it is for testing only, and it must
  never back a claim about what a player can see. Each one's precondition and
  reply shape is in [`qa-commands.md`](qa-commands.md).
  `battle.start` and `battle.flee` start and end a battle, each taking the
  same path its button does. `procgen.step` advances staged generation by one
  stage when the procgen stepper owns it. `wait` holds its reply until one of
  named conditions comes true, so an agent can stop polling — the channel
  serves one caller at a time, so a parked `wait` holds it until it answers.
  The `act.*` commands give the orders a player gives. Some are the
  direct orders: `act.select`, `act.select_next`, `act.select_prev`,
  `act.select_clear`, `act.move`, `act.fire`, `act.reload`, `act.set_stance`,
  `act.set_aiming`, `act.set_facing` and `act.end_turn`. The rest are the
  contextual acts the panel offers — `act.melee`, `act.shove`,
  `act.throw_grenade`, `act.open_door`, `act.enter_emplacement`,
  `act.exit_emplacement`, `act.stabilize` and `act.execute`. The panel offers
  exactly one target per family, so seven of them take `()` and act on the offer
  standing or refuse `NoOffer`. `act.melee` names the target it is about to hit
  — the same `MeleeTargetNet` `battle.cost {Melee}` prices, naming back the
  ganger or the cell `battle.offers` reported — and is refused `TargetMismatch`
  when the panel is offering a different one, so it still cannot act on
  anything a press could not. Each takes the same intent bus the
  keybinds and the pointer push onto, so the sim keeps deciding legality; each
  answers the act-log window it opened, which `log.read` then explains.
  `act.fire`, `act.move`, `act.reload`, `act.set_stance` and `act.set_facing`
  answer the sim's own typed reason instead when it turns the act down;
  [`qa-commands.md`](qa-commands.md) lists the reasons per act.
  Name the actor with `act.select` before every act: the game re-selects the
  first living player
  ganger as soon as nothing is selected, so `act.select_clear` is undone on the
  next frame and an act after a clear runs through whoever the game re-picked.
  The `input.*` commands drive the raw input paths for the screens and the
  moments a named act cannot reach: `input.press_key` writes the same keyboard
  message winit writes and resolves a named action through the live keybind
  table, `input.hover` moves the pointer to a pixel and leaves the pixel-to-cell
  projection to the picking system, `input.set_focus`, `input.focus_step` and
  `input.activate` are the focus bridge the menu and the Options screen are
  driven with, and `input.click_cell` takes the game's own left-click decision
  on a cell and names that decision in the reply. The `view.*` commands aim the
  battle view: `view.pan` and
  `view.look_at` move the camera through the one function every camera mover
  writes the transform with, and `view.level_up`, `view.level_down` and
  `view.toggle_full_view` pick the storey it draws down to by the same path the
  level keys and the action bar's level buttons take. `battle.set_fire_mode`
  picks a fire mode on the weapon the selected shooter fires, the same weapon the
  action bar's mode panel sets it on, through the same lookup.
  The EDITOR host publishes twenty-one commands. `editor.phase` reports the phase
  the editor is in, the mode tab open right now — absent while it is still
  loading — and every mode tab in tab-bar order, `editor.last_save` reports what
  the newest save per mode did, and `editor.validation` reports the content
  integrity report. All three answer at every point in the lifecycle.
  `editor.families`, `editor.session`, `editor.draft` and `editor.map` read the
  authoring scene: the registry keys an author can pick, the session and the
  view, the open form's draft as the RON its save would write, and every painted
  cell on one storey of the prefab canvas. `editor.set_mode`, `editor.new`,
  `editor.load`, `editor.save`, `editor.set_field`, `editor.list_op`,
  `editor.select_theme`, `editor.toggle_terrain` and `editor.set_default_floor`
  drive the authoring forms, and `editor.set_grid_size`, `editor.select_tile`,
  `editor.set_level` and `editor.paint` drive the prefab canvas, and
  `editor.select_injury_tab` opens one of the Injury tab's two sub-tabs. All
  eighteen need the authoring scene, so during the editor's Load pass they answer
  `Unavailable { code: WrongState }`. `editor.draft` needs a form tab open on
  top of that, because the default Prefab tab carries no draft,
  `editor.set_field` and `editor.list_op` need a form tab whose draft is in the
  world, because both write the open form's own draft,
  `editor.toggle_terrain` and `editor.set_default_floor` need the Theme tab,
  because the terrain library and the default-floor picker are drawn only there,
  the five canvas commands need the Prefab tab, because the palette, the
  size fields, the level rail and the viewport are drawn only there, and
  `editor.select_injury_tab` needs the Injury tab, because the sub-tab row it
  writes is drawn only there.
  A name it does not know answers `Unknown`, listing what it does offer. The
  editor-host section of [qa-commands.md](qa-commands.md) has the whole list.
- **`logs` is the first thing to try when a launch came up but the app is not
  behaving.** Both of the child's output streams are captured at spawn into one
  buffer, in the order they were written, so the reply is what the process
  actually said. `max_lines` bounds it. A host owning no child answers
  `not_running` rather than an empty log.
- **The two children are independent.** A `stop` aimed at the game does not touch
  the editor and vice versa; each `HostManager` owns one child and enforces
  one-at-a-time for its own host only. Stdin EOF stops both.
- **An MCP host restart leaves the child running, and `stop` says so.** Replacing
  the MCP server process — what every `/mcp` reconnect does — kills the host, not
  its child: the child is spawned into its own process group and survives, still
  listening on the fixed port, while the new host process owns no handle to it. A
  stop with no owned child therefore checks who holds the port before answering:
  a port answering the QA protocol is an ORPHAN, and the reply says
  `orphan_stopped` with the port and pid once it is stopped, or is a tool error
  naming the process when it could not be. Only a port nothing answers on gets
  `not_running`. A `launch` into a held port reports the same orphan and starts
  nothing, rather than racing it for the socket.

## The protocol sketch

Two wire shapes are in play, one per hop.

**Host ↔ harness — JSON-RPC 2.0 over stdio.** The MCP host
(`bins/gdtf_qa_mcp`) speaks a small hand-rolled JSON-RPC 2.0 subset on
stdin/stdout, newline-delimited (one JSON message per line), blocking `std` I/O
throughout — no rmcp, no tokio, no async runtime. The stdio loop is `run_stdio`
in `bins/gdtf_qa_mcp/src/serve.rs`; method dispatch lives under
`bins/gdtf_qa_mcp/src/rpc/`. A reader thread pushes each line onto a channel and
the loop waits on that channel with a timeout, so it keeps its own clock and can
sweep dead children between requests instead of sitting inside a blocking read.
`initialize` and `tools/list` answer before the game is even up (the game
connection is opened lazily on the first forwarding
`tools/call`). The MCP-protocol version echoed at `initialize` defaults to
`2024-11-05` (`bins/gdtf_qa_mcp/src/mcp/initialize.rs`) — this is the MCP
handshake string, distinct from the game wire version below.

**Host ↔ child — framed RON over a loopback TCP socket.** For each forwarding
tool call the host connects to that tool's host over an `Ipv4Addr::LOCALHOST` TCP
stream — the game on port `7616` (`GAME_QA_PORT`) or the editor on `7617`
(`EDITOR_QA_PORT`) — and exchanges typed messages using the shared
`gdtf_qa_protocol` crate. That crate is bevy-free — it links only `serde`, `ron`,
and `bevy_derive` (the `Deref` derive), so neither half pulls in the engine. Its
shape:

- **The message** — `QaRequest` and `QaResponse` in
  `crates/gdtf_qa_protocol/src/message/` (`request.rs` / `response.rs`). The
  requests, the responses and the errors are the whole wire. One
  request in, one response out. A session opens with a `Hello(ProtocolVersion)`
  handshake; the server replies `HelloOk(HelloFacts)` on a version match or a
  `VersionMismatch` error otherwise. The handshake is REQUIRED, not optional:
  the listener thread answers it itself, and until a connection has
  negotiated, every other frame on it is refused `NotNegotiated` without ever
  reaching the host — so a client that sends a request first gets that error, not
  a reply. A frame that does not decode as a `QaRequest` is answered `Malformed`,
  negotiated or not. Both are per CONNECTION: a reconnect must negotiate again,
  which is why `QaClient::ensure_connected` in `bins/gdtf_qa_mcp/src/link.rs`
  sends the `Hello` on every connection it opens.
- **The wire protocol version** — `ProtocolVersion::CURRENT` in
  `crates/gdtf_qa_protocol/src/message/hello.rs`. It is bumped on
  any breaking change to a message shape; negotiation is exact equality with no capability
  handshake.
- **The framing codec** — pure functions in
  `crates/gdtf_qa_protocol/src/framing/`. Each message is a 4-byte big-endian
  `u32` length prefix followed by that many bytes of compact-RON payload.
  `encode()` frames a message; `FrameDecoder` turns an incoming byte stream back
  into `Frame`s (split-read tolerant), capped at `MAX_FRAME_LEN`. The codec does
  no I/O — each side owns its own socket.
- **The two timeouts** — `NetTimeouts` in
  `crates/gdtf_qa_protocol/src/timeouts.rs`, which both hosts run with as
  `NetTimeouts::DEFAULT`. A socket read or write gives up after **5 seconds**,
  which is what stops an idle client holding the channel; the listener then
  waits up to **180 seconds** for the host to answer a forwarded request, and
  sends `Timeout` itself if none arrives. They are separate numbers on purpose:
  a `wait` parked for its two-minute budget has to be able to answer, and the
  command-set conformance check asserts every command's budget expires before
  that 180. The MCP host's own link timeout — `LINK_TIMEOUT`, **200 seconds**,
  in `bins/gdtf_qa_mcp/src/link.rs` — sits outside both, so a slow command comes
  back as a reply rather than a broken socket.

Every public type in `gdtf_qa_protocol` round-trips through compact RON
identically, proven by the crate's per-module round-trip tests.

## The smoke sequence

The sequence an agent runs against either child. Every step is a real call
against the shipped tool surface:

1. `launch` — start the child and wait for its QA channel to answer. Add
   `host: "editor"` for the editor.
2. `commands` — read that host's live catalogue: what it offers, and what is
   available right now.
3. `commands` with `detail: "Full"` and a `command` filter — read one command's
   published argument and reply shapes.
4. `run` with that command and its arguments, written as compact RON — do the
   thing.
5. `logs` — if a step surprised you, read what the child printed.
6. `stop` — stop the child and release the port.

On the game host today step 4 is one of the commands its catalogue lists. `run { command: "app.phase",
arguments: "()" }` answers the state tuple (`app`, `running`, `game`,
`battlescape`, `aftermath`, the nested `None` where they are not live).
`run { command: "capture.screenshot", arguments: "(name: Some(\"menu\"))" }`
writes a PNG and answers with it attached; the `name` is optional. The
shell reads all take `()`: `settings.read` answers whether sound is on,
`ui.focus` answers the focused widget and the focusable set as opaque entity
tokens, and `playback.state` answers `(caught_up: true)` wherever no battle is
playing. The battle reads refuse `Unavailable { code: WrongState }` off
the battle screen. `run { command: "battle.start", arguments: "(seed: Some(42))" }`
leaves the menu and answers once generation has finished, carrying the seed the
battle used; omit the seed and the game resolves its own and reports that.
`run { command: "battle.flee", arguments: "()" }` ends a running battle.
`run { command: "procgen.step", arguments: "()" }` advances staged generation by
one stage, and refuses `NotBuilt` in a build without `dev_tools`.
`run { command: "wait", arguments: "(condition: BattleDecided)" }` holds until
that condition comes true, or answers `Timeout` after two minutes. Steps 2-4 are
what change as commands land; steps 1, 5 and 6 never do.

The two hosts are independent: a game child on `7616` and an editor child on
`7617` are tracked by separate `HostManager`s, one per host, constructed in
`bins/gdtf_qa_mcp/src/serve.rs` and looked up by `HostSet::pair` in
`bins/gdtf_qa_mcp/src/hosts/set.rs`. Each call reaches the child its `host`
argument names, and a `stop` aimed at the editor never touches the game.
