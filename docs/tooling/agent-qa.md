# Agent QA — driving the game over MCP

A language-model harness can drive a running battle end-to-end: launch the game,
navigate it into a battle, inject player intents, read the resulting combat
events, and capture the presenter frame. This is the QA control channel built
across the QA-net ticket run (the GTW-694 epic). It has two processes and one
shared wire contract:

- **The game**, built with the `net_qa` feature, opens a loopback TCP listener
  that services typed requests against the live battle.
- **The MCP host** (`gdtf_qa_mcp`) speaks a hand-rolled JSON-RPC 2.0 subset on
  stdin/stdout to the model harness and forwards each tool call to the game over
  that loopback socket.
- **`gdtf_qa_protocol`** is the bevy-free crate both halves link — the typed
  request/response envelope, the read-model DTOs, and the framing codec.

The channel is a **dev-only** affordance: it never compiles into a release build,
and even a `net_qa`-enabled build stays inert until an environment variable opts
it in.

## The launch recipe

The MCP host launches the game as a child process when the harness calls the
`launch_game` tool. The launch is owned by the `CargoSpawner` in
`bins/gdtf_qa_mcp/src/lifecycle/spawn.rs` (behind the `GameSpawner` trait, so
tests can substitute a stub). It runs:

```bash
cargo run -p grimdark_turfwar --features dynamic_linking,net_qa
```

with two environment variables set on the child:

- `GDTF_NET_QA=1` — opts the build into the QA control channel (the runtime gate,
  below).
- `GDTF_NET_QA_PORT=<port>` — the loopback port the game listens on and the host
  then connects to.

The variable names are the `NET_QA_ENV` / `NET_QA_PORT_ENV` constants in
`spawn.rs`. The child's stdout is discarded (the host's own stdout is the
JSON-RPC channel and must stay clean); its stderr is captured by `ProcessChild`
for the failure tail. The process is managed by the `GameManager`
(`bins/gdtf_qa_mcp/src/lifecycle/manager.rs`): `launch_game` starts it and waits
for it to answer before returning its port and pid, and `stop_game` (plus stdin
EOF) stops it so the game never outlives the host.

You do not have to go through the MCP host — the same recipe run by hand from a
shell brings the channel up identically, and a client can connect to the port
directly.

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

## The tool vocabulary

The MCP host exposes twelve tools, enumerated once by the `ToolName` enum in
`bins/gdtf_qa_mcp/src/mcp/tools/name.rs`. Ten forward to the game (each maps 1:1
onto a `QaRequest`, resolved by `build_request` in
`bins/gdtf_qa_mcp/src/mcp/call/build/request.rs`); two are host-local and never
touch the wire.

| Tool | Kind | Maps to | Arguments |
| --- | --- | --- | --- |
| `send_input` | forward | `QaRequest::Inject` | `intent` (a `NetIntent` as a JSON object or a compact-RON string) |
| `query_state` | forward | `QaRequest::GetBattleState` | none |
| `get_output` | forward | `QaRequest::GetOutput` | `max` (optional cap on events drained) |
| `take_screenshot` | forward | `QaRequest::TakeScreenshot` | `name` (optional file stem) |
| `screenshot_after` | forward | `QaRequest::ScreenshotAfter` | `intent`, `frame_delay`, optional `name` |
| `app_flow` | forward | `QaRequest::GetAppFlow` | none |
| `start_battle` | forward | `QaRequest::StartBattle` | `situation`, optional `seed` |
| `stepper_control` | forward | `QaRequest::StepperControl` | `command` (`"Next"` / `"Skip"` / `{"Auto": {"running": true}}`) |
| `activate_menu_item` | forward | `QaRequest::ActivateMenuItem` | `token` (from `app_flow`'s `menu.items[].token`) |
| `focus_control` | forward | `QaRequest::FocusControl` | `command` (`{"ActivateTarget": <token>}` / `"Activate"` / `{"Step": "Next"}` / `{"Focus": <token>}`) |
| `launch_game` | host-local | — (starts the child via `GameManager`) | `port` (optional) |
| `stop_game` | host-local | — (stops the child via `GameManager`) | none |

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

**Host ↔ game — framed RON over a loopback TCP socket.** For each forwarding
tool call the host connects to the game over an `Ipv4Addr::LOCALHOST` TCP stream
(port from `GDTF_NET_QA_PORT`, default `7616` — `GamePort::from_env` in
`bins/gdtf_qa_mcp/src/game.rs`) and exchanges typed messages using the shared
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
