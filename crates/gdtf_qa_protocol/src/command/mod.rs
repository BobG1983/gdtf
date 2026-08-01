//! The command vocabulary — the data a host's command set rides on (GTW-939).
//!
//! The point of this module: **a command is not a wire variant.** Adding a command adds
//! no [`QaRequest`](crate::envelope::QaRequest) variant, no
//! [`QaResponse`](crate::envelope::QaResponse) variant, and no field — it adds a row to a
//! host's [`CommandCatalogue`] and a name a [`Run`](crate::envelope::QaRequest::Run) can
//! carry. One concern per file: a command's identity ([`name`]), the JSON bodies it
//! carries ([`payload`]), its derived JSON Schemas ([`schema`]), the catalogue it is
//! published in ([`catalogue`]), whether it can run right now ([`availability`]), when it
//! answers ([`timing`]), what running it produced ([`outcome`]), and the per-call riders
//! ([`options`]).

pub mod availability;
pub mod catalogue;
pub mod name;
pub mod options;
pub mod outcome;
pub mod payload;
pub mod schema;
pub mod timing;

pub use availability::{CommandAvailability, RefusalNote, UnavailableCode};
pub use catalogue::{CommandCatalogue, CommandEntry};
pub use name::{CommandName, CommandSummary};
pub use options::{AwaitBudget, CaptureRider, RunOptions};
pub use outcome::{ArtifactPath, AttachmentKind, CommandOutcome, ReplyAttachment};
pub use payload::{ArgumentFault, CommandArgsJson, CommandReplyJson};
pub use schema::{ArgSchemaJson, ReplySchemaJson};
pub use timing::CommandTiming;

#[cfg(test)]
mod test;
