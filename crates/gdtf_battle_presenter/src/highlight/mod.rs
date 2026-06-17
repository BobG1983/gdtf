//! Hover-highlight draw layer: the presenter half of the GTW-251 message-driven
//! hover-highlight seam.
//!
//! Per the repo's message-driven I/O boundary (`docs/` ADR-0001 — the presenter
//! owns ALL sim→view drawing; the sim never reads the presenter) and the user's
//! 2026-06-16 ruling, the highlight is now request-DRIVEN: the input crate
//! ([`gdtf_battle_input`]) EMITS a [`HighlightRequest`] reflecting the cell its
//! cursor picked, and THIS module LISTENS and draws — mirroring how the sim defines
//! the `*Requested` messages that input writes. The presenter, as the CONSUMER,
//! defines the request type (its input API), so the crate edge stays one-way
//! (`input → presenter → sim`, never a cycle): input can name a presenter-defined
//! message, the presenter never names input.
//!
//! GTW-251 MIGRATED the existing mouse hover-highlight here from
//! `gdtf_battle_input` (where `update_hover_highlight` both computed AND drew it).
//! The DRAWING is identical — a single [`HoverHighlight`] sprite sized to one cell,
//! spawned lazily on the first request, moved to [`cell_to_world`](crate::cell_to_world) of
//! the requested cell + shown on [`Some`], hidden on [`None`] — only the TRIGGER moved
//! from a `Res<HoveredCell>` read to a [`MessageReader<HighlightRequest>`](bevy::ecs::message::MessageReader)
//! drain. This is the seam GTW-259 (the gamepad cursor) builds on; this slice adds NO
//! gamepad read, NO new highlight style, and NO sim change.

mod draw;

#[cfg(test)]
mod test;

pub use draw::{HighlightRequest, HoverHighlight, draw_highlight_on_request};
