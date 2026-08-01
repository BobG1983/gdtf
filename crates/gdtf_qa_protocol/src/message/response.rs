//! [`QaResponse`] — the server-to-client response vocabulary (GTW-734, cut to four by
//! GTW-943).

use serde::{Deserialize, Serialize};

use super::{error::QaError, hello::HelloFacts};
use crate::command::{CommandCatalogue, CommandOutcome};

/// A single reply a host's `net_qa` server returns for a
/// [`QaRequest`](crate::message::QaRequest).
///
/// FOUR variants: one per request kind plus the catch-all [`Error`](Self::Error).
/// [`HelloOk`](Self::HelloOk) carries the negotiated handshake facts,
/// [`Catalogue`](Self::Catalogue) the host's live command list,
/// [`Outcome`](Self::Outcome) what running one of those commands produced, and
/// [`Error`](Self::Error) a protocol-level [`QaError`].
///
/// What a command REPLIES with is not here: it is JSON inside
/// [`CommandOutcome::Ran`], shaped by the command's
/// own declared reply type and published as a schema in the catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QaResponse {
    /// The handshake succeeded — the negotiated facts.
    HelloOk(HelloFacts),
    /// The host's live command catalogue — the reply to
    /// [`Catalogue`](crate::message::QaRequest::Catalogue).
    Catalogue(CommandCatalogue),
    /// What running the named command produced — the reply to
    /// [`Run`](crate::message::QaRequest::Run).
    Outcome(CommandOutcome),
    /// A protocol-level error.
    Error(QaError),
}
