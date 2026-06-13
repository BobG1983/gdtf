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
it compiles. Wrapping a value is also how `docs/glossary.md` vocabulary becomes
code — see `design-fidelity.md`.
