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

`/gate`'s `design-gate` audit treats a bare domain type as a violation, even if
it compiles — and treats a `pub`/`pub(crate)`/`pub(super)` newtype inner field
(rule 5) as the same violation: the inner must be private, reachable only through
`Deref`/`DerefMut`/a constructor/a named accessor. Wrapping a value is also how
`docs/glossary.md` vocabulary becomes code — see `design-fidelity.md`.
