//! Unit tests for the top-down renderer plumbing, one file per surface:
//! the px/coordinate projection, the sheet specs, and the sheet-image redrive
//! (plus the process-global tracing capture the redrive's log pin rides).

mod log_capture;
mod projection;
mod redrive;
mod sheets;
