//! The syn parse + AST walk. `scan_source` parses one file and walks it with a
//! [`Walker`] that flags bare domain types in struct/enum fields and fn
//! signatures, honoring EXACTLY the structural allowlist of rules 4/5 of
//! `.claude/rules/no-bare-types.md` (plus the Q4 marker-generic carve-out):
//!
//! - the single non-`PhantomData` field of a tuple-**struct** newtype is exempt
//!   (rule 5, plus Q4: a generic marker newtype like `Meters<T>(f32,
//!   PhantomData<T>)` still counts as a newtype), see [`is_newtype`]. Enum
//!   variant fields are NOT newtypes — a single-field tuple variant is a
//!   variant, not a newtype, so its bare field is flagged;
//! - fn signatures inside a trait `impl` are exempt (rule 4 "trait-impl
//!   signatures"); a trait DEFINITION's signatures are NOT exempt (rule 4 names
//!   trait-*impl* signatures);
//! - inside a newtype's OWN inherent `impl`, a bare param/return that MATCHES
//!   the newtype's inner type is exempt — the constructor/accessor raw-value
//!   boundary of rule 5; any OTHER bare type in that `impl` is still flagged;
//! - `#[cfg(test)]` modules are skipped (their scaffolding is not domain data).
//!
//! Bevy system params (`Commands`/`Query`/`Res`/`ResMut`/`MessageWriter`/…) and
//! owned-collection indices need no special handling: the former are not flagged
//! bare types, and a bare `usize`/`bool` the checker cannot prove is an
//! index/predicate is flagged and tracked in the exemption registry.

use std::collections::HashMap;

use syn::{
    Fields, FnArg, ImplItemFn, ItemEnum, ItemFn, ItemImpl, ItemMod, ItemStruct, ReturnType,
    TraitItemFn, Type, spanned::Spanned, visit::Visit,
};

use crate::{
    syntax::{collect_newtypes, flagged_type, has_cfg_test, is_newtype, segment_head},
    types::{ColumnNumber, LineNumber, PositionKind, RepoPath, TypeName, Violation},
};

/// A file that syn could not parse (non-fatal — reported, never panics).
pub(crate) struct ParseError {
    /// The offending file.
    pub(crate) path:    RepoPath,
    /// syn's error message.
    pub(crate) message: String,
}

/// Parse `src` as a Rust file and return every bare-type violation, or a
/// [`ParseError`] if it does not parse.
pub(crate) fn scan_source(path: &RepoPath, src: &str) -> Result<Vec<Violation>, ParseError> {
    let file = syn::parse_file(src).map_err(|err| ParseError {
        path:    path.clone(),
        message: err.to_string(),
    })?;
    let newtypes = collect_newtypes(&file);
    let mut walker = Walker {
        path,
        newtypes: &newtypes,
        trait_impl_depth: 0,
        newtype_impl_inner: Vec::new(),
        test_cfg_depth: 0,
        violations: Vec::new(),
    };
    walker.visit_file(&file);
    Ok(walker.violations)
}

/// The AST walker — carries the enclosing-context state the allowlist needs.
struct Walker<'a> {
    /// The file being walked (for violation coordinates).
    path:               &'a RepoPath,
    /// The tuple-struct newtypes declared in this file, mapped to the flagged
    /// inner type each wraps (if any).
    newtypes:           &'a HashMap<String, Option<TypeName>>,
    /// Depth of enclosing trait `impl`s (>0 ⇒ fn sigs exempt — rule 4).
    trait_impl_depth:   usize,
    /// Stack of enclosing newtype inherent `impl`s — each frame is the flagged
    /// inner type of the newtype whose `impl` we are in (`None` if its inner is
    /// not itself a flagged bare type). A bare sig type matching the top frame
    /// is exempt (the newtype's constructor/accessor boundary, rule 5).
    newtype_impl_inner: Vec<Option<TypeName>>,
    /// Depth of enclosing `#[cfg(test)]` modules (>0 ⇒ skip).
    test_cfg_depth:     usize,
    /// Accumulated violations.
    violations:         Vec<Violation>,
}

impl Walker<'_> {
    /// Whether fn signatures are checked at all in the current context — not
    /// inside a `#[cfg(test)]` module or a trait `impl`. (A newtype's own
    /// `impl` IS checked, with a per-type exemption applied in
    /// [`Walker::flag_signature`].)
    const fn signatures_checked(&self) -> bool {
        self.test_cfg_depth == 0 && self.trait_impl_depth == 0
    }

    /// Record a violation at `ty`'s span.
    fn record(&mut self, ty: &Type, name: TypeName, kind: PositionKind) {
        let start = ty.span().start();
        self.violations.push(Violation {
            path: self.path.clone(),
            line: LineNumber::new(start.line),
            column: ColumnNumber::new(start.column + 1),
            type_name: name,
            kind,
        });
    }

    /// Flag every bare field type in `fields`. When `allow_newtype` is set (a
    /// `struct`), a single-field tuple newtype's inner is exempt (rules 4/5 +
    /// the Q4 carve-out); enum variant fields pass `false` — a single-field
    /// tuple *variant* is not a newtype.
    fn flag_fields(&mut self, fields: &Fields, kind: PositionKind, allow_newtype: bool) {
        if allow_newtype && is_newtype(fields) {
            return;
        }
        for field in fields {
            if let Some(name) = flagged_type(&field.ty) {
                self.record(&field.ty, name, kind);
            }
        }
    }

    /// Flag bare params + return of a fn signature. `exempt_inner` is the inner
    /// type of the enclosing newtype's own `impl` (if any) — a bare param/return
    /// matching it is the newtype's raw-value boundary (rule 5) and is skipped.
    fn flag_signature(&mut self, sig: &syn::Signature, exempt_inner: Option<&TypeName>) {
        for input in &sig.inputs {
            let FnArg::Typed(pat) = input else {
                continue; // `self` receiver — not a typed param
            };
            if let Some(name) = flagged_type(&pat.ty)
                && !matches_inner(&name, exempt_inner)
            {
                self.record(&pat.ty, name, PositionKind::FnParam);
            }
        }
        if let ReturnType::Type(_, ty) = &sig.output
            && let Some(name) = flagged_type(ty)
            && !matches_inner(&name, exempt_inner)
        {
            self.record(ty, name, PositionKind::FnReturn);
        }
    }

    /// The exempt inner type of the innermost enclosing newtype `impl`, if any.
    fn current_newtype_inner(&self) -> Option<&TypeName> {
        self.newtype_impl_inner.last().and_then(Option::as_ref)
    }
}

/// Whether `name` matches the enclosing newtype's inner type (see
/// [`Walker::flag_signature`]).
fn matches_inner(name: &TypeName, exempt_inner: Option<&TypeName>) -> bool {
    exempt_inner.is_some_and(|inner| inner == name)
}

impl<'ast> Visit<'ast> for Walker<'_> {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let is_test = has_cfg_test(&node.attrs);
        if is_test {
            self.test_cfg_depth += 1;
        }
        syn::visit::visit_item_mod(self, node);
        if is_test {
            self.test_cfg_depth -= 1;
        }
    }

    fn visit_item_struct(&mut self, node: &'ast ItemStruct) {
        if self.test_cfg_depth == 0 {
            self.flag_fields(&node.fields, PositionKind::StructField, true);
        }
    }

    fn visit_item_enum(&mut self, node: &'ast ItemEnum) {
        if self.test_cfg_depth == 0 {
            for variant in &node.variants {
                self.flag_fields(&variant.fields, PositionKind::EnumField, false);
            }
        }
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let is_trait_impl = node.trait_.is_some();
        // `Some(inner)` iff `self_ty` names a newtype declared in this file;
        // `inner` is its flagged inner type (or `None`).
        let newtype_inner = if is_trait_impl {
            None
        } else {
            segment_head(&node.self_ty)
                .and_then(|(head, _)| self.newtypes.get(&head))
                .cloned()
        };
        let is_newtype_impl = newtype_inner.is_some();
        if is_trait_impl {
            self.trait_impl_depth += 1;
        }
        if let Some(inner) = newtype_inner {
            self.newtype_impl_inner.push(inner);
        }
        syn::visit::visit_item_impl(self, node);
        if is_newtype_impl {
            self.newtype_impl_inner.pop();
        }
        if is_trait_impl {
            self.trait_impl_depth -= 1;
        }
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if self.signatures_checked() {
            let exempt = self.current_newtype_inner().cloned();
            self.flag_signature(&node.sig, exempt.as_ref());
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        if self.signatures_checked() {
            let exempt = self.current_newtype_inner().cloned();
            self.flag_signature(&node.sig, exempt.as_ref());
        }
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        // A trait DEFINITION's signatures are a domain-modeling choice (rule 4
        // exempts trait-*impl* signatures only), so they are checked.
        if self.signatures_checked() {
            let exempt = self.current_newtype_inner().cloned();
            self.flag_signature(&node.sig, exempt.as_ref());
        }
        syn::visit::visit_trait_item_fn(self, node);
    }
}
