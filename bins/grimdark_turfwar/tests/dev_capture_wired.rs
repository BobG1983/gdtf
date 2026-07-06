//! GTW-590 regression pin on the ROOT-CAUSE seam of the dead QA capture: the
//! `dynamic_linking` dev flow must compile the `dev_capture` affordance in.
//!
//! The pinned GTW-590 symptom: `GDTF_AUTOBATTLE=1 GDTF_CAPTURE_PATH=... cargo run -p
//! grimdark_turfwar --features dynamic_linking` reached `BattleRunning`, ran 300+
//! frames, and produced ZERO PNGs and ZERO capture log lines — because `dev_capture`
//! was a separate opt-in feature the invocation never enabled, so the capture module
//! (and its env-var reads) was never compiled. Invisible to the whole green suite.
//!
//! The fix folds `gdtf_app/dev_capture` into this binary's `dynamic_linking` feature
//! (`bins/grimdark_turfwar/Cargo.toml`). This test runs under the SAME feature the
//! dev/gate suite enables (`grimdark_turfwar/dynamic_linking`), so `cargo dtest` goes
//! red if the fold is ever dropped. Under the CI static suite (no `dynamic_linking`)
//! the file compiles to an empty test crate — the fold is a dev-flow property.
#![cfg(feature = "dynamic_linking")]

/// The `dynamic_linking` dev flow compiles the capture affordance in: with this
/// binary's `dynamic_linking` feature on (a debug build — tests are), `gdtf_app`
/// must report the `crate::app::capture` module compiled
/// ([`gdtf_app::dev_capture_compiled`]).
#[test]
fn dynamic_linking_dev_flow_compiles_dev_capture_in() {
    assert!(
        gdtf_app::dev_capture_compiled(),
        "grimdark_turfwar/dynamic_linking must fold gdtf_app/dev_capture in (GTW-590): \
         without it the pinned QA capture invocation silently ignores every \
         GDTF_CAPTURE_* env var and writes no PNGs",
    );
}
