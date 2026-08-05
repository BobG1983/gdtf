//! How one capture finished.

use super::aim::CaptureAimDetail;
use crate::path::CapturePath;

/// Result of one capture request, for a host to map onto its own reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureOutcome {
    /// A decodable PNG landed at this path.
    Landed(CapturePath),
    /// Refused before spawning: nothing renders into the image it would read.
    Refused(CaptureAimDetail),
    /// No PNG landed inside the poll budget.
    TimedOut(CapturePath),
}
