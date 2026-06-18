pub(crate) mod auto_battle;
// The DEV-ONLY screenshot / visual-QA affordance (GTW-297). Double-gated: only compiled
// under the opt-in `dev_capture` feature, and only wired in under `debug_assertions`.
#[cfg(all(debug_assertions, feature = "dev_capture"))]
pub(crate) mod capture;
mod gdtf_app;
pub use gdtf_app::GdtfApp;
