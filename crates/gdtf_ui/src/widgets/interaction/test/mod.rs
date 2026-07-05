//! Tests for the theme-derived interaction layer and the hover→focus bridge,
//! split by concern: shared fixtures, the `theme_interaction` swaps + exclusion
//! filters, the focus bridge, the GTW-280 deactivation repaint, and the
//! GTW-147 theme-reload repaint.

mod support;

mod deactivation;
mod focus;
mod theme;
mod theme_reload;
