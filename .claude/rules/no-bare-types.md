---
paths: ["**/*.rs"]
---

# No bare types — every domain value gets a named newtype

Why this rule exists: a field or signature of `u32`/`f32`/`String` tells you
nothing and lets the compiler wave through nonsense — passing hit points where
time units belong, a column where a row was meant. A name is a contract. In
gdtf, domain values carry their meaning **in the type**, never in a comment or a
variable name bolted onto a bare primitive.

## The rule

1. **No bare Rust/std type as a domain value** — not as a struct field, a
   function parameter or return, nor a `Component`/`Resource`/`Event` payload.
   This covers primitives (`u32`, `f32`, `bool`, `usize`), `String`, and even
   `glam`/std types (`IVec2`, `Vec3`, `Duration`) when they carry domain meaning.
2. **If you think you want a bare type, you don't.** Wrap it: a tuple-struct
   newtype named from `docs/glossary.md`, implementing `Deref` (and `DerefMut`
   only where the inner value is mutated through it) to the wrapped type.
3. **Distinct concepts get distinct newtypes** even over the same inner type:
   `Hp` and `Tu` are both `u32` and are never interchangeable.
4. The ONLY bare types that remain are the inner field of a newtype itself, and
   **framework plumbing you cannot wrap** — Bevy system params (`Commands`,
   `Query`, `Res`/`ResMut`, `EventWriter`), trait-impl signatures, and indices
   into a collection you own. Those are not domain values.
5. **The newtype's inner field is PRIVATE — never `pub`, `pub(crate)`, or
   `pub(super)`.** Construct it through a `new` (or named) constructor, read it
   through the derived `Deref`, mutate it through `DerefMut` (added only where the
   value is mutated) or a named setter. A `pub`/`pub(crate)`/`pub(super)` inner
   leaks the wrapper open to `Self(x)` tuple construction and `x.0 = …` field
   writes from outside the defining module — bypassing the constructor/accessor
   contract the newtype exists to enforce, so the name stops being a guarantee.
   Why: a private inner makes the type the *only* place the raw value is touched,
   so invariants (clamping, validation, defaults) can never be sidestepped.

## Example

```rust
use std::ops::Deref;

/// A ganger's current hit points.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hp(u32);

// NO:  struct Ganger { hp: u32, name: String }
// YES: struct Ganger { hp: Hp, name: GangerName }
```

## Enforcement

A `syn`-based conformance test enforces this rule MECHANICALLY on every
`cargo dtest` run — the no-bare-types suite
(`crates/gdtf_test_utils/tests/no_bare_types/`, dir-form target
`no_bare_types`, GTW-599) parses every tracked PRODUCTION `.rs` file, walks
each struct/enum field and fn signature (params + returns), and FAILS on any
bare `u32`/`f32`/`bool`/`usize`/`String`/`IVec2`/`Vec3`/`Duration`/… used as a
domain value, printing a clippy-style `file:line:col` diagnostic per hit. The
STRUCTURAL allowlist encodes rules 4/5 exactly — the single non-`PhantomData`
inner field of a tuple-STRUCT newtype (an enum variant is not a newtype), a
param/return matching the enclosing newtype's OWN inner type (its
constructor/accessor boundary, rule 5), and trait-`impl` signatures (rule 4;
trait DEFINITION signatures ARE checked, being a domain-modeling choice) —
plus the Q4 marker-parameterized-generic carve-out. NOTHING else is exempted in
code: `#[derive(Serialize/Deserialize)]` wire shapes serialize newtypes fine
and are NOT carved out, `ShaderType` GPU uniforms (bare `f32`/`Vec2` for WGSL
layout) are NOT carved out, and a bare `usize`/`bool` the checker cannot prove
is a collection index or a predicate answer is flagged. Test bands are out of
scope (their scaffolding values are not domain data). Every residual exception
— documented FALSE POSITIVES (framework plumbing that genuinely cannot be
wrapped, e.g. a `ShaderType` uniform) and KNOWN VIOLATIONS pending a follow-up
ticket — lives line-by-line in `.claude/rules/no-bare-types-exemptions.txt`,
and the test fails on any stale entry — mirroring the `module_layout` clause-7
guard.

`/gate`'s `design-gate` audit additionally treats a bare domain type as a
violation, even if it compiles — and treats a `pub`/`pub(crate)`/`pub(super)`
newtype inner field (rule 5) as the same violation: the inner must be private,
reachable only through `Deref`/`DerefMut`/a constructor/a named accessor.
Wrapping a value is also how `docs/glossary.md` vocabulary becomes code — see
`design-fidelity.md`.
