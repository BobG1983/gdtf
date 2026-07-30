//! The top-level protocol envelope — [`QaRequest`] / [`QaResponse`] and their reply
//! payloads (GTW-734).
//!
//! The outermost wire types: every exchange is one [`QaRequest`] in and one
//! [`QaResponse`] out. One concern per file — the [`hello`] handshake
//! ([`ProtocolVersion`] / [`HelloFacts`]),
//! the inject [`receipt`], the [`menu`]-activation receipt, the [`focus`]-control command +
//! receipt, the [`error`] vocabulary, the
//! [`screenshot`] result, the DEV [`stepper`]-control command + receipt, and the
//! [`request`] / [`response`] enums themselves.

pub mod error;
pub mod focus;
pub mod hello;
pub mod menu;
pub mod receipt;
pub mod request;
pub mod response;
pub mod screenshot;
pub mod stepper;

pub use error::QaError;
pub use focus::{FocusCommandNet, FocusControlReceipt, FocusStepNet};
pub use hello::{HelloFacts, ProtocolVersion, ServerNameNet};
pub use menu::MenuActivationReceipt;
pub use receipt::{InjectReceipt, RejectReason};
pub use request::QaRequest;
pub use response::QaResponse;
pub use screenshot::{CaptureAimNet, ScreenshotAfterResult, ScreenshotPathNet, ScreenshotResult};
pub use stepper::{AutoRunNet, StepperCommandNet, StepperReceipt};

#[cfg(test)]
mod test;
