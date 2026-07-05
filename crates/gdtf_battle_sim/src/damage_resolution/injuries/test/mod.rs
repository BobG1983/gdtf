//! Vocabulary tests for the GTW-435 injury types — the [`InflictedInjuries::gain`]
//! ledger folding (C3) and the `.injury.ron` deserialize (C4).
//!
//! These exercise the REAL public API on the real types (no stubs): `gain` folds
//! through the actual [`StatDeltaLedger`] / [`BleedAfflicted`] accumulators, and the
//! RON test parses through the actual `serde` derive. Per the loader-tests rule, the
//! RON test asserts STRUCTURE (kinds resolve, the shape round-trips), never specific
//! tunable magnitudes.

mod support;

mod category;
mod def_parse;
mod hands;
mod ledger;
mod movement_factor;
mod roll;
