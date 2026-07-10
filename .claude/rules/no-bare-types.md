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
   into a collection you own. Those are not domain values. The same reasoning
   admits a small, CLOSED set of **convention carve-outs** — signatures that
   are a language / std-library convention rather than a domain value, so their
   bare type carries no meaning to name. Each is phrased narrowly enough that
   the mechanical checker's predicate is its DIRECT translation; a near-miss
   that does not meet the exact wording stays flagged. These four, and only
   these, are exempt beyond the inner-field / trait-impl allowances:
   - **(a) Std-container method signatures.** The collection-surface trio, where
     a type stands in for a `std` collection: `fn is_empty(&self) -> bool`;
     `fn len(&self) -> usize` **only when the same inherent `impl` also declares
     that** `fn is_empty(&self) -> bool` (the pairing `clippy::len_without_is_empty`
     requires — a `len` with NO `is_empty` sibling stays flagged); and a
     `fn contains…(&self, …) -> bool` **that takes at least one reference
     parameter** (the borrowed key/value whose membership it answers). The
     `usize` count and `bool` answer are named by std convention here, not as a
     domain quantity — this is the delegating trio every registry-family wrapper
     stamps over the foundation `Registry`.
   - **(b) Constructor / accessor scalar boundary of a coordinate/vector
     newtype** — the generalisation of rule 5's own-inner allowance to a newtype
     whose inner is a multi-component `glam` type. Inside the inherent `impl` of
     a newtype wrapping a `glam` vector (`IVec2`/`IVec3`/`IVec4`, `UVec2`/`UVec3`/
     `UVec4`, `Vec2`/`Vec3`/`Vec4`, `Quat`), a bare parameter or return whose
     type is that vector's COMPONENT scalar — `i32` for the `IVec*` family, `u32`
     for `UVec*`, `f32` for the float vectors and `Quat` — is the raw-value
     boundary of the wrapped coordinate: `Cell::new(i32, i32)`,
     `SimPos::new(f32, f32, f32)`, a component accessor. A scalar that does NOT
     match the inner's component type, or a scalar on a newtype whose inner is
     NOT a multi-component vector, stays flagged.
   - **(c) Provable own-collection index.** Rule 4's "indices into a collection
     you own", made mechanically PROVABLE: an inherent method named `index` or
     `idx` returning `usize` on a type whose inherent `impl` also declares an
     associated `const` of fixed-size-array type (`[_; N]`) — the collection it
     indexes (`BodyPart::index()` over `BodyPart::ALL`). An `index`/`idx` method
     on a type with no such owned array stays flagged.
   - **(d) Named hash digest.** A function whose name contains `hash`
     (case-insensitive) returning a bare unsigned integer — a raw digest value,
     plumbing by its very name. Cast and sampling helpers whose names do NOT say
     `hash` are NOT covered: wrap their result or carry them as tracked debt.
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
plus the Q4 marker-parameterized-generic carve-out AND the four **convention
carve-outs** of rule 4 (GTW-722), each predicate a direct translation of its
clause: (a) the std-container trio `is_empty`/`len`/`contains…` (the `len`
allowance gated on an `is_empty` sibling in the same inherent `impl`, the
`contains…` allowance gated on a reference parameter); (b) a coordinate/vector
newtype's constructor/accessor scalar boundary — a bare param/return matching
the inner `glam` vector's COMPONENT scalar (`IVec*`→`i32`, `UVec*`→`u32`,
float-vec/`Quat`→`f32`); (c) a provable own-collection index — an
`index`/`idx` `-> usize` method on a type whose inherent `impl` declares an
associated fixed-size-array `const`; (d) a named hash digest — a fn whose name
contains `hash` returning a bare unsigned integer. A near-miss that fails the
exact wording (a `len` with no `is_empty`, a scalar on a non-coordinate
newtype, an `index` on a type with no owned array, a cast/sampling helper whose
name does not say `hash`) is STILL flagged. NOTHING else is exempted in
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

The registry is **SHRINK-ONLY** (GTW-704, user ruling 2026-07-09). SECTION 2
(the GTW-599 baseline of known pre-existing violations) is capped by a ceiling
pinned INSIDE the suite (`ceiling.rs`, `SECTION_2_CEILING`): a run whose live
SECTION-2 count EXCEEDS the pin FAILS, so no new bare type may be baselined away
— wrap it in a newtype instead. Any commit may LOWER the pin (each burn-down
wave's land does); it is raised only with explicit user approval. SECTION 1
(documented false positives) is exempt from the ceiling, but its additions stay
user-approval-only, so growth past its pinned baseline prints a loud, non-failing
reminder. When an edit drifts a baselined line, the suite's opt-in re-key mode
(`NO_BARE_TYPES_REGEN=1 cargo test … regenerate_section_2_keys`) rewrites
SECTION 2's keys from the live scan — preserving each entry's why-text, refusing
to add net-new entries (an unpairable live violation is printed and left out),
and never touching SECTION 1.

`/gate`'s `design-gate` audit additionally treats a bare domain type as a
violation, even if it compiles — and treats a `pub`/`pub(crate)`/`pub(super)`
newtype inner field (rule 5) as the same violation: the inner must be private,
reachable only through `Deref`/`DerefMut`/a constructor/a named accessor.
Wrapping a value is also how `docs/glossary.md` vocabulary becomes code — see
`design-fidelity.md`.
