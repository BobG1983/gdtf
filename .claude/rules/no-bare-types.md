---
paths: ["**/*.rs"]
---

# No bare types: every domain value gets a named newtype

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

The type carries the meaning, never a comment and never a variable name.

## The rule

1. No bare Rust/std type as a domain value. Not as a struct or enum field. Not as a
   function parameter or return. Not as a `Component`/`Resource`/`Event`/`Message` payload.
   This covers primitives (`u32`, `f32`, `bool`, `usize`), `String`, and `glam`/std types
   (`IVec2`, `Vec3`, `Duration`) when they carry domain meaning.
2. Write a tuple-struct newtype, named from `docs/glossary.md`, that implements `Deref` to
   the wrapped type. Add `DerefMut` only where the inner value is mutated through it.
3. Distinct concepts get distinct newtypes, even over the same inner type. `Hp` and `Tu`
   are both `u32` and are never interchangeable.
4. The ONLY bare types that remain are the single non-`PhantomData` inner field of a
   tuple-struct newtype itself, and framework plumbing you cannot wrap: Bevy system
   params (`Commands`, `Query`, `Res`/`ResMut`, `EventWriter`), trait-impl signatures,
   and indices into a collection you own. Those are not domain values.

   The same reasoning admits four more exemptions, and only these four. Each one is a
   signature fixed by a language or std-library convention, so its bare type has no domain
   meaning left to name. A near-miss that does not meet the exact wording stays flagged.

   Exemption (a) covers three method signatures on a type that stands in for a `std`
   collection. The first is `fn is_empty(&self) -> bool`. The second is
   `fn len(&self) -> usize`, but only when the same inherent `impl` also declares
   `fn is_empty(&self) -> bool`, the pairing `clippy::len_without_is_empty` requires.
   The third is `fn contains…(&self, …) -> bool` that takes at least one reference parameter,
   the borrowed key or value whose membership it answers. They are the three methods every
   registry wrapper delegates to `foundation::registry::Registry`.

   Exemption (b) applies inside the inherent `impl` of a newtype wrapping a `glam` vector
   (`IVec2`/`IVec3`/`IVec4`, `UVec2`/`UVec3`/`UVec4`, `Vec2`/`Vec3`/`Vec4`, `Quat`), where a
   bare parameter or return may use that vector's COMPONENT scalar. That is `i32` for the
   `IVec*` family, `u32` for `UVec*`, and `f32` for the float vectors and `Quat`. For example:
   `Cell::new(i32, i32)`, `SimPos::new(f32, f32, f32)`, a component accessor.

   Exemption (c) covers an index into a collection the type owns, and only where that
   ownership is PROVABLE. It applies to an inherent method named `index` or `idx` returning
   `usize`, on a type whose inherent `impl` also declares an associated `const` of
   fixed-size-array type (`[_; N]`). That array is the collection being indexed, as
   `BodyPart::index()` indexes `BodyPart::ALL`.

   Exemption (d) covers a named hash digest: a function whose name contains `hash`
   (case-insensitive) and returns a bare unsigned integer. Wrap the result of a cast or
   sampling helper whose name does not say `hash`, or carry it as tracked debt.
5. The newtype's inner field is PRIVATE. Never `pub`, never `pub(crate)`, never
   `pub(super)`. Any of those lets code outside the defining module build it with `Self(x)`
   and write `x.0 = …`. That bypasses the constructor and the accessors, so clamping,
   validation and defaults can be sidestepped. Construct it through a `new` or other named
   constructor. Read it through the derived `Deref`. Mutate it through `DerefMut` or a named
   setter.

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

The `design-gate` review enforces this rule during `/gate`, in the crates audit of the diff.
No mechanical test enforces it.

Through its structure and Bevy lens, the audit treats a bare domain type as a violation even
if it compiles. It treats a public inner field (rule 5) as the same violation.

What the audit lets pass is rules 4 and 5:

- A param or return matching the enclosing newtype's OWN inner type, the type its constructor
  takes and its accessors return.
- Two more pieces of framework plumbing: a `ShaderType` GPU uniform's bare `f32`/`Vec2` for WGSL
  layout, and a `.run_if` run-condition's bare `bool` return.

Test code is out of scope, because its scaffolding values are not domain data.

Wrapping a value is also how `docs/glossary.md` vocabulary becomes code. See
`design-fidelity.md`.
