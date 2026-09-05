//! Headless apps built on Bevy's DefaultPlugins (no winit window).

/// Load-path test app builder.
pub mod load;
/// UI-focused test app builder.
pub mod ui;
/// Windowed (render-target) test app builder.
pub mod windowed;
