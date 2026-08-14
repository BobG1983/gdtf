//! How one capture finished.

use crate::path::CapturePath;

/// Result of one capture request, for a host to map onto its own reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureOutcome {
    /// A decodable PNG landed at this path.
    Landed(CapturePath),
    /// No PNG landed inside the poll budget.
    TimedOut(CapturePath),
}
