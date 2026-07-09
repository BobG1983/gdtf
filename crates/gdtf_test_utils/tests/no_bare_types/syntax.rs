//! The pure syn-AST predicates the walker relies on: resolving a type to the
//! flagged bare type it ultimately is, recognising tuple-struct newtypes (and
//! the inner type each wraps), and spotting a `#[cfg(test)]` module. No mutable
//! state — [`crate::walk`] owns the walk.

use std::collections::HashMap;

use syn::{Fields, GenericArgument, PathArguments, Type};

use crate::types::{TypeName, is_flagged};

/// The last path segment of a type after peeling references, plus whether it
/// carries angle-bracketed generic arguments.
pub(crate) fn segment_head(ty: &Type) -> Option<(String, bool)> {
    let inner = match ty {
        Type::Reference(reference) => reference.elem.as_ref(),
        other => other,
    };
    let Type::Path(path) = inner else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let has_args = matches!(segment.arguments, PathArguments::AngleBracketed(_));
    Some((segment.ident.to_string(), has_args))
}

/// Resolve a type to the flagged bare type it ultimately is, if any — peeling
/// references and unwrapping `Option<..>` (one or more levels). Returns `None`
/// for non-flagged types and for anything wrapped in another container
/// (`Vec<_>`, `HashMap<_, _>`, `Handle<_>`, …), which are out of scope.
pub(crate) fn flagged_type(ty: &Type) -> Option<TypeName> {
    let inner = match ty {
        Type::Reference(reference) => reference.elem.as_ref(),
        other => other,
    };
    let Type::Path(path) = inner else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let name = segment.ident.to_string();
    if name == "Option" {
        if let PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
        {
            return flagged_type(inner_ty);
        }
        return None;
    }
    if matches!(segment.arguments, PathArguments::None) && is_flagged(&name) {
        Some(TypeName::new(name))
    } else {
        None
    }
}

/// Whether a type's head segment is `PhantomData` (which does not count toward
/// a newtype's field arity — the Q4 marker-parameterized-generic carve-out).
fn is_phantom_data(ty: &Type) -> bool {
    segment_head(ty).is_some_and(|(name, _)| name == "PhantomData")
}

/// Whether `fields` form a newtype at all (single non-`PhantomData` tuple field).
pub(crate) fn is_newtype(fields: &Fields) -> bool {
    let Fields::Unnamed(unnamed) = fields else {
        return false;
    };
    unnamed
        .unnamed
        .iter()
        .filter(|f| !is_phantom_data(&f.ty))
        .count()
        == 1
}

/// The flagged bare type wrapped by a newtype's single non-`PhantomData` field,
/// if `fields` form a newtype AND that inner is itself a flagged bare type
/// (returns `None` when the inner is another newtype, a container, etc.).
fn newtype_inner(fields: &Fields) -> Option<TypeName> {
    let Fields::Unnamed(unnamed) = fields else {
        return None;
    };
    let mut carried = unnamed.unnamed.iter().filter(|f| !is_phantom_data(&f.ty));
    let field = carried.next()?;
    if carried.next().is_some() {
        return None; // more than one carried field ⇒ not a newtype
    }
    flagged_type(&field.ty)
}

/// Pre-pass: every tuple-struct newtype in the file mapped to the flagged inner
/// type it wraps (or `None` when the inner is not itself a flagged bare type).
/// The walker uses this so a newtype's OWN inherent `impl` may exempt a bare
/// param/return that MATCHES its inner — the constructor/accessor boundary of
/// rule 5 — while still flagging any other bare type in that `impl`.
pub(crate) fn collect_newtypes(file: &syn::File) -> HashMap<String, Option<TypeName>> {
    let mut map = HashMap::new();
    for item in &file.items {
        if let syn::Item::Struct(item_struct) = item
            && is_newtype(&item_struct.fields)
        {
            map.insert(
                item_struct.ident.to_string(),
                newtype_inner(&item_struct.fields),
            );
        }
    }
    map
}

/// Whether an attribute list carries `#[cfg(test)]` — a `cfg(..)` whose meta
/// tokens contain the bare word `test`.
pub(crate) fn has_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let syn::Meta::List(list) = &attr.meta else {
            return false;
        };
        list.tokens
            .to_string()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|tok| tok == "test")
    })
}
