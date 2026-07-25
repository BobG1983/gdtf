//! Round-trip pins for the read-model DTOs + exhaustive witnesses for their enums
//! (GTW-734; split into per-concern files per module-layout when GTW-802 added the focus
//! handout).
//!
//! ## Members (one concern per file)
//!
//! - [`battle`] — the whole-battle aggregate fixtures + their round-trips.
//! - [`fog`] — the fixed-size fog handout.
//! - [`appflow`] — the app-flow snapshot + the request-kind / app-state witnesses.
//! - [`menu`] — the folded menu enumeration handout.
//! - [`focus`] — the folded focus-navigable control enumeration handout.
//! - [`ganger`] — the ganger-card enum witnesses (life state, severity, body part).
//! - [`editor`] — the editor query family: the per-topic views + their enum witnesses.

mod appflow;
mod battle;
mod editor;
mod focus;
mod fog;
mod ganger;
mod menu;
