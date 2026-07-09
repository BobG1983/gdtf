//! GTW-599 no-bare-types conformance guard — mechanically enforces
//! `.claude/rules/no-bare-types.md` on every `cargo test --workspace` run,
//! mirroring the `module_layout` clause-7 suite (tracked-tree walk via
//! `git ls-files`, a plain-text exemption registry honored with stale-entry
//! failure, clippy-style diagnostics).
//!
//! It parses every tracked PRODUCTION `.rs` file (test bands excluded — see
//! `tree`) with `syn` and walks each struct field, enum-variant field, and fn
//! signature (params + returns), flagging any bare `u32`/`f32`/`bool`/`usize`/
//! `String`/`IVec2`/`Vec3`/`Duration`/… used as a domain value. The structural
//! allowlist encodes EXACTLY rules 4/5 plus the Q4 marker-parameterized-generic
//! carve-out (see `walk`): the single non-`PhantomData` inner field of a
//! tuple-STRUCT newtype (enum variant fields are NOT newtypes), a bare
//! param/return matching the enclosing newtype's OWN inner type
//! (constructor/accessor boundary, rule 5), and trait-`impl` signatures (rule 4
//! — trait DEFINITION signatures are checked). Everything else — including
//! `#[derive(Serialize/Deserialize)]` wire shapes, `ShaderType` GPU uniforms,
//! bare `usize`/`bool` the checker cannot prove is an index/predicate, and all
//! pre-existing debt — is tracked line-by-line in
//! `.claude/rules/no-bare-types-exemptions.txt` (documented false positives +
//! known-violation-pending-follow-up), not exempted structurally.
//!
//! The dylint alternative is RECORDED in GTW-599, not built: this stays inside
//! `cargo dtest` with zero toolchain changes.

mod conformance;
mod diagnostics;
mod fixtures;
mod registry;
mod syntax;
mod tree;
mod types;
mod walk;
