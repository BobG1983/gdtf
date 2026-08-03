//! QA command catalogue: names, schemas, availability, outcomes, and run options.

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
