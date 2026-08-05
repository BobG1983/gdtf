//! QA command catalogue: names, shapes, availability, outcomes, and run options.

pub mod availability;
pub mod catalogue;
pub mod name;
pub mod options;
pub mod outcome;
pub mod payload;
pub mod shape;
pub mod timing;

pub use availability::{CommandAvailability, RefusalNote, UnavailableCode};
pub use catalogue::{CommandCatalogue, CommandEntry};
pub use name::{CommandName, CommandSummary};
pub use options::{AwaitBudget, CaptureRider, RunOptions};
pub use outcome::{ArtifactPath, AttachmentKind, CommandOutcome, ReplyAttachment};
pub use payload::{ArgumentFault, CommandArgsRon, CommandReplyRon};
pub use shape::{
    ArgSchemaRon, FALLBACK_SHAPE_TEXT, ReplySchemaRon, RonShape, ShapeBody, ShapeDef, ShapeDoc,
    ShapeField, ShapeFieldName, ShapeName, ShapeVariant, VariantShape, shape_text, shape_trace,
};
pub use timing::CommandTiming;

#[cfg(test)]
mod test;
