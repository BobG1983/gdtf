//! GTW-736: the DEV-ONLY QA `net_qa` transport and the GAME's command host.
//!
//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate
//! doc so the doc survives a feature-off build — the `procgen_stepper` suite precedent)
//! compiles the whole dir-form suite to an empty crate without the feature — the
//! `crate::dev::net_qa` module it exercises does not exist there. CI's test step names the
//! feature (GTW-883), so this suite runs there rather than collecting nothing.
//!
//! GTW-943 cut the suite down with the wire: the request-per-feature files went with the
//! requests they drove. What is left drives the three requests the channel still answers.
//!
//! - [`commands`] drives the GAME's REAL command layer (GTW-942) over a REAL `TcpStream`: the
//!   one-entry catalogue with its derived schemas, `app.phase` answering the five-level state
//!   tuple, and the three refusals a `Run` can come back with — an unknown name, an argument
//!   the command does not declare, and a rider this build has not implemented.
//! - [`app_phase_depth`] answers `app.phase` from INSIDE a real battle (GTW-942), so the four
//!   nested levels are asserted live rather than as the `null`s the menu legitimately reports:
//!   each reported level is compared against the app's own `State<…>` resource.
//! - [`command_set`] pins what no request can see (GTW-942): the two per-host conformance
//!   assertions over the REAL `GAME_COMMANDS` slice, the set being exactly `app.phase`, and
//!   that exactly ONE system drains `Res<NetInbox>`, source and schedule alike.
//! - [`deadline`] (GTW-943 retargeted) pins that an admitted `Run` is answered by its
//!   command's own handler and never falls through to the transport's frame-deadline
//!   `Timeout` sweep over `PendingQueue<CommandCall<C>>`.
//! - [`battle_fixture`] owns the menu-resting app the deep-phase case descends into a
//!   battle, through the menu's own `StartBattleRequested`.
//! - [`socket_support`] owns the client half both real-listener suites share: the headless
//!   game with its listener bound, the framed client, and the frame pump; [`command_exchange`]
//!   adds the one step above it the command cases share — negotiate, then send and collect.
//! - [`hello_socket`] drives the GAME's REAL listener (GTW-940) over a REAL `TcpStream` via
//!   [`NetQaPlugin::listening`](gdtf_app::test_support::NetQaPlugin): the handshake carries
//!   the GAME's own version and server name, and the pre-handshake refusal.
#![cfg(all(debug_assertions, feature = "net_qa"))]

mod app_phase_depth;
mod battle_fixture;
mod command_exchange;
mod command_set;
mod commands;
mod deadline;
mod hello_socket;
mod socket_support;
