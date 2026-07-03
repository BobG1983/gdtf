//! The shared pooled-draw walk (GTW-568): ONE
//! [`draw_pool`](crate::overlays::pool::draw_pool) helper for every pooled overlay draw
//! — reuse-in-order, lazy (may-decline) growth, surplus-hide — with the
//! `set_if_neq` visibility flips owned by the helper so an unchanged frame re-dirties
//! nothing. See the helper's doc for the pooled-overlay convention and the deliberate
//! `overlays/highlight` exclusion (message-driven retention semantics).

mod draw;

#[cfg(test)]
mod test;

pub use draw::draw_pool;
