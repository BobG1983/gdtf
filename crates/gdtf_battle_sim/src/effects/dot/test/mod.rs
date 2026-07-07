//! The GTW-544 DOT runtime unit tests, grouped by concern (the bleed `test/` dir
//! precedent): the shared headless-app fixtures live in [`support`], the per-round
//! [`tick_dot`](super::tick_dot) drain / decrement-or-REMOVE expiry (GTW-643) /
//! DOT-kills pins in [`tick`], and the [`apply_dot`](super::apply_dot)
//! refresh-not-stack + once-per-span start-fact pins in [`apply`].

mod support;

mod apply;
mod tick;
