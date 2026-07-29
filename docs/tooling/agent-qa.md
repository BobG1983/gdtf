# Agent QA — driving the game over MCP

A language-model harness can drive a running battle end-to-end: launch the game,
navigate it into a battle, inject player intents, read the resulting combat
events, and capture the presenter frame. This is the QA control channel built
across the QA-net ticket run (the GTW-694 epic). It has three processes and one
shared wire contract:

- **The game**, built with the `net_qa` feature, opens a loopback TCP listener
  that services typed requests against the live battle.
- **The content editor**, built with ITS `net_qa` feature, opens a SECOND
  loopback listener on its own port, servicing the editor query family
  (GTW-804 / GTW-805). It is a separate process from the game and can run at the
  same time.
- **The MCP host** (`gdtf_qa_mcp`) speaks a hand-rolled JSON-RPC 2.0 subset on
  stdin/stdout to the model harness and forwards each tool call to the game or
  the editor over that loopback socket. ONE host binary manages BOTH children
  (GTW-808): it holds a link and a lifecycle manager per host, so a game on
  `7616` and an editor on `7617` can be up and driven at the same time.
- **`gdtf_qa_protocol`** is the bevy-free crate all three link — `crates/gdtf_app`
  for the game, `crates/gdtf_content_editor` for the editor, and `bins/gdtf_qa_mcp`
  for the host. It carries the typed request/response envelope, the read-model
  DTOs, and the framing codec.

The channel is a **dev-only** affordance: it never compiles into a release build,
and even a `net_qa`-enabled build stays inert until an environment variable opts
it in.

## The launch recipe — the game

The MCP host launches the game as a child process when the harness calls the
`launch_game` tool. The launch is owned by the `CargoSpawner` in
`bins/gdtf_qa_mcp/src/lifecycle/spawn.rs` (behind the `ChildSpawner` trait, so
tests can substitute a stub). WHAT it launches is the call's own recipe — a
`LaunchSpec` (`bins/gdtf_qa_mcp/src/lifecycle/launch/`) of four typed values:
package, features, working directory, environment overrides (GTW-875). Each one
the call omits falls back to the default game recipe, so a bare `launch_game`
still runs:

```bash
cargo run -p grimdark_turfwar --features dynamic_linking,net_qa
```

in the MCP host's own directory. A call that names them runs what it names —
this launches a `dev_tools` build of a git worktree, with a dev gate set:

```json
{"package": "grimdark_turfwar",
 "features": ["dynamic_linking", "net_qa", "dev_tools"],
 "working_dir": "/Users/you/dev/gdtf-some-worktree",
 "env": {"GDTF_BATTLE_SEED": "42"}}
```

`working_dir` is the one that decides WHICH CHECKOUT is under test: without it
the build comes from whatever directory the host itself runs in, which is how a
QA pass gets reported against code that is not the code under review. A
`working_dir` that is not an existing directory is rejected, never silently
dropped.

Whatever the recipe, two environment variables are set on the child LAST, so a
recipe's own `env` can never displace them:

- `GDTF_NET_QA=1` — opts the build into the QA control channel (the runtime gate,
  below).
- `GDTF_NET_QA_PORT=<port>` — the loopback port the game listens on and the host
  then connects to.

The variable names come from the launch recipe's `QaChannel`
(`bins/gdtf_qa_mcp/src/lifecycle/launch/channel.rs`), which carries one pair per
host — `GDTF_NET_QA` / `GDTF_NET_QA_PORT` for the game, `GDTF_EDITOR_NET_QA` /
`GDTF_EDITOR_NET_QA_PORT` for the editor — so the spawner needs no per-host
branch. The child's stdout is discarded (the host's own stdout is the
JSON-RPC channel and must stay clean); its stderr is captured by `ProcessChild`
for the failure tail. The process is managed by the `HostManager`
(`bins/gdtf_qa_mcp/src/lifecycle/manager.rs`): `launch_game` starts it and waits
for it to answer before returning its port and pid — plus the package, features,
and resolved directory it was built from — and `stop_game` (plus stdin EOF) stops
it so the game never outlives the host.

One game runs at a time, and the manager remembers the recipe that built it:

- The same recipe again is ensure-style `already_running`, reporting the running
  child's package, features, and directory (not the ones this call asked for).
- A DIFFERENT recipe is REJECTED, naming what is actually running. Call
  `stop_game` first. Answering "already running" to a request for another
  checkout would hand back a success for a build that was never started.

QA evidence goes through the MCP host. Hand-writing a socket client against this
port is not an accepted route — GTW-875 records a QA pass that did exactly that
and thereby tested the wrong tree. Running the recipe by hand from a shell is
fine for bringing the editor or game up (and is how GTW-878's evidence is
produced); driving it is the host's job. If a needed tool is missing, that is a
defect to file, not a reason to bypass the host.

## The launch recipe — the content editor

`launch_editor` is the editor's `launch_game` (GTW-808): the same host, the same
`CargoSpawner`, the same five recipe arguments — with the editor's own defaults.
A bare `launch_editor` runs:

```bash
cargo run -p gdtf_content_editor_bin --features dynamic_linking,net_qa
```

in the MCP host's own directory, with `GDTF_EDITOR_NET_QA=1` and
`GDTF_EDITOR_NET_QA_PORT=7617` set on the child LAST (so a recipe's own `env`
can never displace them). WHICH two variables a launch sets is part of the
recipe — the `QaChannel` in
`bins/gdtf_qa_mcp/src/lifecycle/launch/channel.rs` — which is how one spawner
serves both hosts with no per-host branch. It is NOT a caller argument: setting
`GDTF_NET_QA` on an editor child would leave the editor inert with no listener
on the port the launcher is about to probe.

**Warm the build first.** The editor's `dynamic_linking,net_qa` combination is
one nothing else in the repo produces — `cargo dbuild` builds the GAME binary,
and the workspace checks never link an editor binary — so the first
`launch_editor` against a given checkout is usually a COLD BUILD of the whole
editor. Two things follow:

- The editor's boot timeout is **600 seconds**, not the game's 180
  (`bins/gdtf_qa_mcp/src/lifecycle/config.rs`).
- Run `cargo edqabuild` in the checkout you are about to QA before calling
  `launch_editor`, and the launch answers in seconds instead.

If a launch does time out, the failure is not a bare "timed out": it names the
build as the likely cause, prints the exact `cargo build -p … --features …`
warm-up command for the recipe it ran, and carries the child's stderr tail (which
shows how far the build got).

The editor can also be brought UP by hand — driving it still goes through the
MCP host, per the rule above. It is a separate binary package
(`bins/gdtf_content_editor/Cargo.toml`, package `gdtf_content_editor_bin`) with
its own `net_qa` feature, its own two environment variables, and its own default
port, and running it by hand opens the same channel `launch_editor` opens.
GTW-878 added that binary feature; before it, the editor's listener existed only
inside workspace-wide `cargo` checks and no launchable editor could open a port
at all.

```bash
GDTF_EDITOR_NET_QA=1 GDTF_EDITOR_NET_QA_PORT=7617 \
  cargo run -p gdtf_content_editor_bin --features dynamic_linking,net_qa
```

`cargo edqarun` (`.cargo/config.toml`) is the same command as an alias, and
`cargo edqabuild` builds without running. The plain `edrun` / `edbuild` aliases
deliberately leave `net_qa` off — a normal dev editor should not pay the
feature's compile cost.

- `GDTF_EDITOR_NET_QA=1` — opts the build into the editor's QA control channel.
  Truthy is `1` / `true` / `yes` / `on`, trimmed and case-insensitive
  (`crates/gdtf_content_editor/src/net_qa/env.rs`); anything else, including
  unset, leaves the editor inert.
- `GDTF_EDITOR_NET_QA_PORT=<port>` — the loopback port. Unset or unparseable
  falls back to **`7617`**, the `DEFAULT_EDITOR_PORT` in
  `crates/gdtf_content_editor/src/net_qa/config.rs` — deliberately one above the
  game's `7616`, so both hosts can be up at once without fighting for a socket.

The names are the editor's OWN, not the game's: setting `GDTF_NET_QA` does
nothing to the editor, and pointing both hosts at one variable would switch them
both on at one port.

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

The editor answers four request kinds (`route_editor_requests` in
`crates/gdtf_content_editor/src/net_qa/router.rs`): `Hello` version negotiation
— replying with the server name `gdtf-editor-net-qa`, which is how a client
tells the two hosts apart — the ADR 0007 editor query pair
`GetEditorQueryOptions` and `QueryEditor`, and `TakeScreenshot` (GTW-880), which
the drain does not answer itself: it queues onto the shared transport's
`PendingQueue` and the capture pump in
`crates/gdtf_content_editor/src/net_qa/screenshot/` replies once the PNG has
landed on disk. Every other kind is rejected
`BadRequest` immediately rather than left to hang. The wire shape, the framing,
and the protocol version are the game's — see
[the protocol sketch](#the-protocol-sketch) — so one client library speaks to
both.

## The feature + env double gate

The server is gated twice, so it cannot leak into a shipped binary and cannot
open a port by accident:

1. **The `net_qa` cargo feature** must be enabled at build time. The binary
   feature `net_qa = ["gdtf_app/net_qa"]` lives in
   `bins/grimdark_turfwar/Cargo.toml`; it chains to the library feature
   `net_qa = ["dep:gdtf_qa_protocol", "dep:gdtf_screenshot", "dep:image"]` in
   `crates/gdtf_app/Cargo.toml`. Only that feature pulls in the wire-contract
   crate, the self-capture crate, and image encoding. A build without it never
   links them.

2. **The compile gate** at the plugin wiring site,
   `crates/gdtf_app/src/dev/plugin.rs`, is
   `cfg!(all(debug_assertions, feature = "net_qa"))` — the server code is only
   compiled in a debug build with the feature on. A release build never contains
   it, because it opens a listener.

3. **The runtime env gate** keeps even a `net_qa` debug build inert by default.
   `crates/gdtf_app/src/dev/net_qa/env.rs` reads `GDTF_NET_QA`: `net_qa_enabled()`
   treats `1` / `true` / `yes` / `on` (trimmed, case-insensitive) as enabled and
   anything else — including unset — as disabled. `NetQaPlugin::from_env`
   (`crates/gdtf_app/src/dev/net_qa/plugin/net_qa_plugin.rs`) registers nothing unless that
   check passes.

The listen **interface** is never configurable — it is hardcoded to
`Ipv4Addr::LOCALHOST`. Only the **port** is read from the environment:
`port_from_env()` parses `GDTF_NET_QA_PORT` as a `u16`, or falls back to
`DEFAULT_PORT` (`7616`) defined in `crates/gdtf_app/src/dev/net_qa/config.rs`.

The editor mirrors all three gates with its own names: the binary feature
`net_qa = ["gdtf_content_editor/net_qa"]` in
`bins/gdtf_content_editor/Cargo.toml` chains to
`net_qa = ["dep:gdtf_qa_protocol", "dep:gdtf_net_qa_transport", "dep:image"]` in
`crates/gdtf_content_editor/Cargo.toml`; the wiring site
`crates/gdtf_content_editor/src/app.rs` is
`cfg(all(debug_assertions, feature = "net_qa"))`; and
`NetQaEditorPlugin::from_env` reads `GDTF_EDITOR_NET_QA`. That binary feature is
pinned by a conformance test —
`crates/gdtf_test_utils/tests/binary_feature_passthrough/` fails the suite if a
binary over a `net_qa`-bearing library stops declaring the passthrough, which is
exactly how the editor's listener spent GTW-804 and GTW-805 unreachable from any
launchable process.

## The tool vocabulary

The MCP host exposes sixteen tools, enumerated once by the `ToolName` enum in
`bins/gdtf_qa_mcp/src/mcp/tools/name.rs`. Twelve forward to a running child (each
maps 1:1 onto a `QaRequest`, resolved by `build_request` in
`bins/gdtf_qa_mcp/src/mcp/call/build/request.rs`); four are host-local and never
touch the wire. Every tool names a default host (`ToolName::host`), so an
editor tool travels over the editor's link and drives the editor's lifecycle —
the two children are never confused for one another. One tool —
`take_screenshot`, the only one both children can serve — is aimed per call
instead: `ToolName::accepts_host_argument` lets its optional `host` argument
(`"game"` / `"editor"`) pick the child, `resolve_host` in
`bins/gdtf_qa_mcp/src/mcp/call/handle.rs` resolves it, and a `host` value naming
neither child is rejected as invalid params rather than quietly defaulting, so a
typo cannot screenshot the wrong process (GTW-880).

| Tool | Kind | Maps to | Arguments |
| --- | --- | --- | --- |
| `send_input` | forward | `QaRequest::Inject` | `intent` (a `NetIntent` as a JSON object or a compact-RON string) |
| `query_state` | forward | `QaRequest::GetBattleState` | none |
| `get_output` | forward | `QaRequest::GetOutput` | `max` (optional cap on events drained) |
| `take_screenshot` | forward | `QaRequest::TakeScreenshot` | `name` (optional file stem), `host` (optional `"game"` — the default — or `"editor"`) |
| `screenshot_after` | forward | `QaRequest::ScreenshotAfter` | `intent`, `frame_delay`, optional `name` |
| `app_flow` | forward | `QaRequest::GetAppFlow` | none |
| `start_battle` | forward | `QaRequest::StartBattle` | `situation`, optional `seed` |
| `stepper_control` | forward | `QaRequest::StepperControl` | `command` (`"Next"` / `"Skip"` / `{"Auto": {"running": true}}`) |
| `activate_menu_item` | forward | `QaRequest::ActivateMenuItem` | `token` (from `app_flow`'s `menu.items[].token`) |
| `focus_control` | forward | `QaRequest::FocusControl` | `command` (`{"ActivateTarget": <token>}` / `"Activate"` / `{"Step": "Next"}` / `{"Focus": <token>}`) |
| `launch_game` | host-local | — (starts the child via the game's `HostManager`) | all optional: `port`, `package`, `features` (array or comma-separated string), `working_dir`, `env` |
| `stop_game` | host-local | — (stops the child via the game's `HostManager`) | none |
| `get_editor_query_options` | forward (editor) | `QaRequest::GetEditorQueryOptions` | none |
| `query_editor` | forward (editor) | `QaRequest::QueryEditor` | `topic` (`"Readiness"` / `"Mode"` / `"Session"` / `"Draft"` / `"Validation"`) |
| `launch_editor` | host-local | — (starts the editor child via the editor's `HostManager`) | all optional: `port`, `package`, `features`, `working_dir`, `env` |
| `stop_editor` | host-local | — (stops the editor child via the editor's `HostManager`) | none |

Notes an agent relies on:

- **`app_flow` is the affordance oracle.** Its reply carries an `available` list
  of the request kinds the game will service right now. The battle-only tools
  (`query_state`, `send_input`, `get_output`) are absent until a battle is
  running, so poll `app_flow` first and act on what it advertises. It also carries
  the two token handouts an agent clicks through: `menu` (`id` plus `items` of
  `{token, label, enabled}`) on a menu, and `focus` (`focused` plus `focusables`
  of `{token, label, kind, enabled, checked}`) on any focus-navigable screen.
- **`start_battle` is the navigation step** that carries a cold-launched game
  from the menu into a battle. The game currently ships one situation,
  `skirmish`, and rejects any other name; the optional `seed` pins the procgen
  RNG for a reproducible run. Generating the battle takes a moment — poll
  `app_flow` until it reports the battle is active before calling the battle-only
  tools.
- **`send_input`** injects one `NetIntent` as the selected ganger. The intent
  vocabulary lives in `crates/gdtf_qa_protocol/src/intent/net_intent.rs` and
  includes `Fire`, `Move`, `SetStance`, `SetFacing`, `Reload`, `EndTurn`,
  `Melee`, `Select`, `SelectNext`, and more.
- **`screenshot_after`** injects an intent and then captures a screenshot a fixed
  number of frames later — the frame-exact way to catch a transient effect (a
  muzzle flash) that a request/response round-trip cannot land on itself.
- **`focus_control` drives the UI OFF-BATTLE** (GTW-802), which `send_input`
  cannot: `send_input` is serviced only while a battle is running and caught up.
  Read the controls from `app_flow`'s `focus.focusables` — the enumeration is read
  off the game's OWN `DirectionalNavigationMap`, so any screen that is
  keyboard-navigable at all (the Options screen, the menu, the battlescape HUD
  panels) is listed with no per-screen tagging. `{"ActivateTarget": <token>}`
  points focus at a control and clicks it; `"Activate"` clicks whatever holds
  focus; `{"Step": "Next" | "Prev" | "Left" | "Right"}` moves focus; `{"Focus":
  <token>}` points focus without clicking. Everything runs through the real path —
  a step writes the same navigate message an arrow key writes, and an activation
  emits a real `Enter` keypress at the focused control, so the game's own bridges
  and first-party widget observers react exactly as they do to a player. A token
  that is not a currently listed focusable is rejected `StaleToken`. The effect
  lands on the FOLLOWING frame, so re-read `app_flow` to confirm (a checkbox
  reports its new `checked` value there).
- **`get_editor_query_options` is the editor's affordance oracle**, the exact
  counterpart of `app_flow` on the game side. Its reply carries `readiness`
  (`"Load"` while the editor's asset pass runs, `"Editing"` once the authoring
  scene is up) and `topics` — the topics the editor will answer RIGHT NOW, each
  with a one-line description. Poll it after `launch_editor` until `readiness`
  reads `"Editing"`, then ask `query_editor` only for a topic it lists. During
  `Load` the topics backed by an `Editing`-only resource (`Mode`, `Session`,
  `Draft`) are absent and asking for one is rejected `BadRequest`, never
  answered with a fabricated empty view; `Readiness` and `Validation` are
  answered from the first frame, because `ContentIntegrityReport` is
  `init_resource`'d while the app is built
  (`crates/gdtf_content_editor/src/load/injuries.rs`), so the resource
  `crates/gdtf_content_editor/src/net_qa/snapshot/topics.rs` tests for already
  exists — the `Load`-phase topic list is asserted against the real editor app
  over its real listener in `crates/gdtf_content_editor/tests/net_qa_editor_query/`.
  That test is the evidence for every `Load`-phase claim in this bullet — the
  absent `Mode` / `Session` / `Draft` topics, the `BadRequest` rejection, and
  `Readiness` / `Validation` answering from the first frame. It is not a
  stand-in: it runs the editor's own `MapEditorPlugin` and `NetQaEditorPlugin`,
  binds a real loopback listener, and speaks the real framing codec over a real
  `TcpStream`. Its client connects BEFORE the app runs a single frame, which is
  what makes the `Load` observation deterministic.

  Do not try to reproduce it by polling a launched editor. The asset pass ends
  in a few frames, sooner than an MCP client's next request arrives — across
  four `launch_editor` cycles the first `get_editor_query_options` reported
  `Editing` every time. Polling for `Load` is a race the test already wins
  deterministically (GTW-902).
- **The two children are independent.** `stop_game` does not touch the editor
  and `stop_editor` does not touch the game; each `HostManager` owns one child
  and enforces one-at-a-time for its own host only. Stdin EOF stops both.

## The protocol sketch

Two wire shapes are in play, one per hop.

**Host ↔ harness — JSON-RPC 2.0 over stdio.** The MCP host
(`bins/gdtf_qa_mcp`) speaks a small hand-rolled JSON-RPC 2.0 subset on
stdin/stdout, newline-delimited (one JSON message per line), blocking `std` I/O
throughout — no rmcp, no tokio, no async runtime. The stdio loop is `run_stdio`
in `bins/gdtf_qa_mcp/src/serve.rs`; method dispatch lives under
`bins/gdtf_qa_mcp/src/rpc/`. `initialize` and `tools/list` answer before the game
is even up (the game connection is opened lazily on the first forwarding
`tools/call`). The MCP-protocol version echoed at `initialize` defaults to
`2024-11-05` (`bins/gdtf_qa_mcp/src/mcp/initialize.rs`) — this is the MCP
handshake string, distinct from the game wire version below.

**Host ↔ child — framed RON over a loopback TCP socket.** For each forwarding
tool call the host connects to that tool's host over an `Ipv4Addr::LOCALHOST` TCP
stream — the game on `GDTF_NET_QA_PORT` (default `7616`) or the editor on
`GDTF_EDITOR_NET_QA_PORT` (default `7617`), resolved by `QaHost::port_from_env`
in `bins/gdtf_qa_mcp/src/hosts/host.rs` and carried by the `QaClient` in
`bins/gdtf_qa_mcp/src/link.rs` — and exchanges typed messages using the shared
`gdtf_qa_protocol` crate. That crate is bevy-free — it links only `serde`, `ron`,
and `bevy_derive` (the `Deref` derive), so neither half pulls in the engine. Its
shape:

- **The envelope** — `QaRequest` and `QaResponse` in
  `crates/gdtf_qa_protocol/src/envelope/` (`request.rs` / `response.rs`). One
  request in, one response out. A session opens with a `Hello(ProtocolVersion)`
  handshake; the server replies `HelloOk(HelloFacts)` on a version match or a
  `VersionMismatch` error otherwise.
- **The wire protocol version** — `ProtocolVersion::CURRENT` in
  `crates/gdtf_qa_protocol/src/envelope/hello.rs`, currently `4`. It is bumped on
  any breaking envelope change; negotiation is exact equality with no capability
  handshake.
- **The framing codec** — pure functions in
  `crates/gdtf_qa_protocol/src/framing/`. Each message is a 4-byte big-endian
  `u32` length prefix followed by that many bytes of compact-RON payload.
  `encode()` frames a message; `FrameDecoder` turns an incoming byte stream back
  into `Frame`s (split-read tolerant), capped at `MAX_FRAME_LEN`. The codec does
  no I/O — each side owns its own socket.

Every public type in `gdtf_qa_protocol` round-trips through compact RON
identically, proven by the crate's per-module round-trip tests.

## The validated smoke sequence

The epic capstone (GTW-743) ran this sequence live against the real game over
MCP, with seed `4242`:

1. `launch_game` — start the game child and wait for it to answer.
2. `app_flow` — read the lifecycle snapshot and its `available` list.
3. `start_battle` with `{ situation: "skirmish", seed: 4242 }` — navigate into a
   battle.
4. `query_state` — read the whole battle snapshot.
5. `send_input` with a `Select` intent — pick a ganger.
6. `send_input` with a `Move` intent.
7. `send_input` with a `Fire` intent.
8. `get_output` — drain the combat events.
9. `take_screenshot` — capture the presenter frame.

All nine calls answered. `get_output` returned five real events — a
`MoveCompleted`, two reaction `ShotFired`s, an `Injury`, and the player's
`ShotFired` — with correct identities, and the screenshot showed the real
presenter frame.

## The smoke sequence — the content editor

The editor's equivalent of the sequence above. Like that one, this is a
transcript of calls that were actually made — run against `develop` at
`154936d7`, with the replies below copied from what came back.

Run `cargo edqabuild` in the checkout first, then:

1. `launch_editor` — starts the child and waits for it to answer. Returned
   `status: "launched"`, `pid`, `port: 7617`, `package:
   "gdtf_content_editor_bin"`, `features: "dynamic_linking,net_qa"`. Confirmed
   independently: `ps` showed `target/debug/gdtf_content_editor` at that pid and
   `lsof` showed it listening on `127.0.0.1:7617`.
2. `get_editor_query_options` — `readiness: "Editing"` with all five topics
   (`Readiness`, `Mode`, `Session`, `Draft`, `Validation`). The `Load`→`Editing`
   transition is the editor's own (`crates/gdtf_content_editor/src/plugin.rs`);
   nothing over the wire drives it, and it completes too fast to observe here —
   see the polling note above.
3. `query_editor` once per topic, all five answered:
   - `Readiness` → `"Editing"`
   - `Mode` → `active: "Prefab"`, `label: "PREFAB"`, `tab_index: 2`
   - `Session` → theme and resolved `default_floor` UUIDs, `grid_size` 60x60x8,
     `selected_tile: null`
   - `Draft` → `mode: "Prefab"`, fields `painted_cells: "0"` and
     `current_level: "Some(CurrentEditLevel(Level(0)))"`
   - `Validation` → `checks_complete: true`, `findings: []`
4. `stop_editor` — `status: "stopped"` with the pid. Confirmed independently:
   the process was reaped and port 7617 released.

Every value above matches what
`crates/gdtf_content_editor/tests/net_qa_editor_query/` already asserts
in-process, which is why that test is the evidence for the `Load` phase rather
than a launched process.

The two hosts are independent: a `launch_game` child on `7616` and a
`launch_editor` child on `7617` are tracked by separate `HostManager`s, one per
host, constructed in `bins/gdtf_qa_mcp/src/serve.rs` and looked up by
`HostSet::pair` in `bins/gdtf_qa_mcp/src/hosts/set.rs`. Each tool reaches its own
child and `stop_editor` never touches the game.
