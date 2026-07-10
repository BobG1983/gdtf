//! The four **convention carve-outs** of rule 4 (GTW-722) — pure syn/string
//! predicates the walker consults to exempt signatures that are a language /
//! std-library convention rather than a domain value. Each predicate is a
//! DIRECT translation of its clause in `.claude/rules/no-bare-types.md`; a
//! near-miss that fails the exact wording is deliberately NOT matched (and so
//! stays flagged). No mutable state — [`crate::walk`] owns the walk and the
//! enclosing-`impl` bookkeeping these predicates read.

use syn::{FnArg, ImplItem, Signature, Type};

use crate::{syntax::flagged_type, types::TypeName};

/// The COMPONENT scalar of a `glam` multi-component vector type name, if `inner`
/// names one — carve-out (b). `IVec*`→`i32`, `UVec*`→`u32`, the float vectors
/// and `Quat`→`f32`. `None` for a non-`glam`/scalar inner (so a scalar-inner
/// newtype gets NO extra component exemption beyond rule 5's exact inner).
pub(crate) fn glam_component_scalar(inner: &TypeName) -> Option<TypeName> {
    let scalar = if inner.is("IVec2") || inner.is("IVec3") || inner.is("IVec4") {
        "i32"
    } else if inner.is("UVec2") || inner.is("UVec3") || inner.is("UVec4") {
        "u32"
    } else if inner.is("Vec2") || inner.is("Vec3") || inner.is("Vec4") || inner.is("Quat") {
        "f32"
    } else {
        return None;
    };
    Some(TypeName::new(scalar))
}

/// Whether a signature's first input is a `self` receiver of ANY form (`self` /
/// `&self` / `&mut self` / `self: Box<Self>`) — carve-out (c)'s wording ("an
/// inherent method named `index`/`idx`") sets no receiver form, so `index(self)`
/// by value counts.
fn has_self_receiver(sig: &Signature) -> bool {
    matches!(sig.inputs.first(), Some(FnArg::Receiver(_)))
}

/// Whether a signature's first input is an IMMUTABLE-reference receiver (`&self`
/// exactly — `reference = Some`, `mutability = None`). Carve-out (a) quotes the
/// exact signatures `fn is_empty(&self) -> bool` / `fn len(&self) -> usize` /
/// `fn contains…(&self, …) -> bool`, so a receiver-form near-miss (`&mut self`,
/// `self` by value, `self: Box<Self>`) does NOT meet the wording and stays
/// flagged (GTW-722 fix — the trio gate and the `is_empty` sibling check both
/// use this, not [`has_self_receiver`]).
fn has_shared_ref_receiver(sig: &Signature) -> bool {
    matches!(
        sig.inputs.first(),
        Some(FnArg::Receiver(recv)) if recv.reference.is_some() && recv.mutability.is_none()
    )
}

/// Whether a signature has at least one typed (non-receiver) parameter of a
/// reference type — the `contains…(&self, &key)` gate of carve-out (a).
fn has_reference_param(sig: &Signature) -> bool {
    sig.inputs.iter().any(|arg| match arg {
        FnArg::Typed(pat) => matches!(pat.ty.as_ref(), Type::Reference(_)),
        FnArg::Receiver(_) => false,
    })
}

/// Whether an inherent `impl`'s items declare the std-container sibling
/// `fn is_empty(&self) -> bool` — the pairing carve-out (a) gates a `len` on.
pub(crate) fn declares_is_empty(items: &[ImplItem]) -> bool {
    items.iter().any(|item| {
        let ImplItem::Fn(method) = item else {
            return false;
        };
        method.sig.ident == "is_empty"
            && has_shared_ref_receiver(&method.sig)
            && returns_flagged(&method.sig).is_some_and(|ret| ret.is("bool"))
    })
}

/// Whether an inherent `impl`'s items declare an associated `const` of
/// fixed-size-array type (`[_; N]`) — the OWNED collection carve-out (c) proves
/// an `index`/`idx` method against.
pub(crate) fn declares_owned_array(items: &[ImplItem]) -> bool {
    items.iter().any(|item| match item {
        ImplItem::Const(konst) => matches!(konst.ty, Type::Array(_)),
        _ => false,
    })
}

/// Whether a fn's flagged RETURN type `ret` is exempt as a convention carve-out
/// — the std-container trio (a), the provable own-collection index (c), and the
/// named hash digest (d). The receiver / reference-param facts are read from
/// `sig`; `has_is_empty_sibling` and `has_owned_array` are the enclosing inherent
/// `impl`'s facts (both `false` for a free fn). Carve-out (b), the coordinate
/// component scalar, is a per-newtype exempt-set check the walker applies to
/// params AND returns and so is NOT decided here.
pub(crate) fn return_is_conventional(
    name: &str,
    ret: &TypeName,
    sig: &Signature,
    has_is_empty_sibling: bool,
    has_owned_array: bool,
) -> bool {
    // (a) std-container method signatures — the wording quotes `&self` exactly,
    // so an immutable-reference receiver is REQUIRED (a receiver-form near-miss
    // like `&mut self` / `self` stays flagged).
    if has_shared_ref_receiver(sig) {
        if name == "is_empty" && ret.is("bool") {
            return true;
        }
        if name == "len" && has_is_empty_sibling && ret.is("usize") {
            return true;
        }
        if name.starts_with("contains") && has_reference_param(sig) && ret.is("bool") {
            return true;
        }
    }
    // (c) provable own-collection index — the wording sets no receiver form, so
    // any `self` receiver (incl. `index(self)` by value) counts.
    if has_self_receiver(sig)
        && (name == "index" || name == "idx")
        && has_owned_array
        && ret.is("usize")
    {
        return true;
    }
    // (d) named hash digest.
    name_says_hash(name) && is_unsigned(ret)
}

/// Whether a fn name says it computes a hash (case-insensitive `hash` substring)
/// — carve-out (d).
fn name_says_hash(name: &str) -> bool {
    name.to_ascii_lowercase().contains("hash")
}

/// Whether `ret` names a bare unsigned integer — the digest type of carve-out (d).
fn is_unsigned(ret: &TypeName) -> bool {
    ["u8", "u16", "u32", "u64", "u128", "usize"]
        .iter()
        .any(|unsigned| ret.is(unsigned))
}

/// The flagged type of a signature's return position, if any (peeling the same
/// references / `Option` layers the walker's [`flagged_type`] does).
fn returns_flagged(sig: &Signature) -> Option<TypeName> {
    match &sig.output {
        syn::ReturnType::Type(_, ty) => flagged_type(ty),
        syn::ReturnType::Default => None,
    }
}
