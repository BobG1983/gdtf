//! [`GAME_COMMANDS`] — the ONE list of commands this host offers (GTW-942).

use gdtf_qa_command::command::ErasedCommand;
use gdtf_qa_protocol::envelope::ServerNameNet;

use super::read::AppPhase;
use crate::dev::net_qa::{config::SERVER_NAME, facts::GameFacts};

/// Every command the GAME host offers, in the order its catalogue lists them.
///
/// The ONE list. The catalogue walks it, admission scans it, and
/// [`register_game_commands`](super::register_game_commands) registers it — so a command
/// added here is advertised, admissible and wired by that single edit, and there is no
/// second list to fall out of step with.
///
/// The compiler enforces the host coupling: an entry whose
/// [`QaCommand::Facts`](gdtf_qa_command::command::QaCommand::Facts) is not [`GameFacts`]
/// fails to coerce HERE, at the literal, before anything runs — so the editor's commands
/// cannot be listed in the game's set by accident.
///
/// It is NOT exported from the crate, even under `test-support`: its type names
/// [`GameFacts`], which names this host's whole internal wire vocabulary. The suite reaches
/// it through the `conformance` sibling instead, which is compiled only under the
/// `test-support` feature.
pub(in crate::dev::net_qa) const GAME_COMMANDS: &[&dyn ErasedCommand<GameFacts>] = &[&AppPhase];

/// The name this host answers a `Catalogue` under — the same name its handshake reports.
///
/// Read from [`SERVER_NAME`] rather than restated, so a client that identified the host by
/// its `HelloOk` sees the identical name on its catalogue.
#[must_use]
pub(in crate::dev::net_qa) fn game_host_name() -> ServerNameNet {
    ServerNameNet::new(SERVER_NAME.to_owned())
}
