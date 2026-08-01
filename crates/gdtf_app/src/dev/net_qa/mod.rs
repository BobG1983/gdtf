//! The DEV-ONLY QA network control channel — the GTW-694 architecture's T3 (GTW-736), now
//! the GAME's command host (GTW-942, GTW-943).
//!
//! A loopback-only (`Ipv4Addr::LOCALHOST`) TCP listener + request drain a coding-agent
//! QA harness drives, speaking the bevy-free wire contract (`gdtf_qa_protocol`). NONE of
//! it is shipping behavior: the whole module compiles ONLY under
//! `cfg(all(debug_assertions, feature = "net_qa"))` (its wiring site in
//! [`crate::dev::plugin`] applies that double gate — it opens a listener), and even then
//! it is inert until `GDTF_NET_QA` is set.
//!
//! # What the channel answers
//!
//! Three requests, and only three: the `Hello` handshake (answered in the listener thread),
//! `Catalogue` (this host's live command list), and `Run` (one of those commands, by name).
//! Everything this host can DO is a command in [`commands`] — a file plus one line in that
//! list — so adding one moves no protocol version, adds no wire variant, and needs no
//! courier change.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! The battle-agnostic half — the loopback listener and its accept loop, the
//! one-client-at-a-time gate ([`Busy`](gdtf_qa_protocol::message::QaError::Busy) on a
//! second client), the request/response channel, the pending queue and its frame-deadline
//! sweep — was lifted in GTW-803 into the shared [`gdtf_net_qa_transport`] crate, so the
//! game and the content editor drive ONE codepath. This module is the game's HOST side of
//! it: the gates, the drain, and the command set.
//!
//! - [`config`] — this server's identity constants + its default listen port.
//! - [`env`] — the `GDTF_NET_QA` / `GDTF_NET_QA_PORT` gates.
//! - [`router`] — the ONE drain of the inbox.
//! - [`present`] — the DEV-ONLY offscreen-capture present path (GTW-764): retargets the world
//!   and UI cameras to an offscreen image the render graph writes every tick, and blits that
//!   image back to the window, so a capture reads pixels independent of window focus or
//!   occlusion (a backgrounded macOS window's swapchain reads back BLACK).
//! - [`screenshot`] — the deferred capture pump: capture the real presenter frame and answer
//!   only after the confined PNG lands on disk.
//! - [`plugin`] — the [`NetQaPlugin`] registration (`from_env` / `with_channels`).
//! - [`wire`] — this host's OWN wire types: the app-phase mirrors a command's reply schema is
//!   derived over, and the act / key / token / pointer vocabulary later commands take as
//!   arguments (GTW-942, GTW-943).
//! - [`facts`] — [`GameFacts`](facts::GameFacts), the per-frame facts every command reads,
//!   and the `SystemParam` that samples them once a frame (GTW-942).
//! - [`commands`] — the ONE command list, its registration walk, and the commands
//!   themselves. Adding a command is a file plus one line in that list; it moves no
//!   protocol version and adds no wire variant (GTW-942).

mod commands;
mod config;
mod env;
mod facts;
mod plugin;
mod present;
mod router;
mod screenshot;
mod wire;

// `NetQaPlugin` is the item the binary consumes (via the dev aggregate plugin,
// `crate::dev::plugin`), so it re-exports in BOTH configurations at the `test-support`
// visibility flip the item itself uses (`support_item!` in `plugin`): `pub` under
// `test-support` (the `test_support` ledger needs it), `pub(crate)` otherwise —
// `unreachable_pub`-clean either way.
crate::support_use!(plugin::NetQaPlugin;);

// The TEST surface is consumed ONLY through the `test_support` ledger (the GTW-736
// integration suite). The binary never names these, so re-exporting them in a
// non-`test-support` build would be an unused `pub(crate) use`; gate the re-export to the
// same feature, `pub` because the ledger needs it. The transport's own types are NOT
// re-exported here (GTW-803): they belong to `gdtf_net_qa_transport`, and a test that needs
// one imports it from there — presenting another crate's items as this module's API would
// erase the split at the public surface (module-layout Rule 7).
// GTW-942: the two conformance entry points, which run `gdtf_qa_command::test_support`'s
// per-host assertions over the REAL `GAME_COMMANDS` slice and report the names it publishes.
// The slice itself is NOT exported — its type names this host's internal facts and wire
// vocabulary, and two assertions are not a reason to put that on the crate's public surface.
#[cfg(feature = "test-support")]
pub use commands::{assert_game_command_set_is_conformant, game_command_names};
#[cfg(feature = "test-support")]
pub use config::{
    NET_QA_PROTOCOL_VERSION, SERVER_NAME as NET_QA_SERVER_NAME, hello_facts as net_qa_hello_facts,
};
// GTW-740: the capture pump's confinement-directory + poll-budget config Resources and its
// queue payload, exposed so the capture suite can inject a temp directory + a tiny budget and
// enqueue a capture through the real queue. Widened through `screenshot`'s own `support_use!`
// re-export; gated to `test-support` like the rest.
#[cfg(feature = "test-support")]
pub use screenshot::{QaShotDir, ScreenshotPayload, ShotPollBudget};
