//! The syn parse + AST walk. `scan_source` parses one file and walks it with a
//! [`Walker`] that flags bare domain types in struct/enum fields and fn
//! signatures, honoring EXACTLY the structural allowlist of rules 4/5 of
//! `.claude/rules/no-bare-types.md` (plus the Q4 marker-generic carve-out and
//! the four GTW-722 convention carve-outs, see [`crate::conventions`]):
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
//!   boundary of rule 5; carve-out (b) generalises this to the inner `glam`
//!   vector's COMPONENT scalar; any OTHER bare type in that `impl` is still
//!   flagged;
//! - the convention carve-outs (a) std-container `is_empty`/`len`/`contains…`,
//!   (c) a provable own-collection `index`/`idx`, and (d) a named hash digest
//!   exempt the specific RETURN they name — decided by [`crate::conventions`]
//!   from the enclosing inherent `impl`'s facts (an [`ImplFrame`]);
//! - `#[cfg(test)]` modules are skipped (their scaffolding is not domain data).
//!
//! Bevy system params (`Commands`/`Query`/`Res`/`ResMut`/`MessageWriter`/…) are
//! not flagged bare types, and a bare `usize`/`bool` the checker cannot prove is
//! an index/predicate is flagged and tracked in the exemption registry.

use std::collections::HashMap;

use syn::{
    Fields, FnArg, ImplItemFn, ItemEnum, ItemFn, ItemImpl, ItemMod, ItemStruct, ReturnType,
    TraitItemFn, Type, spanned::Spanned, visit::Visit,
};

use crate::{
    conventions::{
        declares_is_empty, declares_owned_array, glam_component_scalar, return_is_conventional,
    },
    syntax::{collect_newtypes, flagged_type, has_cfg_test, is_newtype, segment_head},
    types::{ColumnNumber, LineNumber, PositionKind, RepoPath, TypeName, Violation},
};

/// The facts about an enclosing inherent `impl` the convention carve-outs read.
/// One frame is pushed per inherent `impl` (trait impls are wholly exempt via
/// [`Walker::trait_impl_depth`] and get no frame).
struct ImplFrame {
    /// The bare types exempt as this newtype's raw-value boundary (rule 5 + the
    /// carve-out (b) `glam` component scalar) — matched against params AND
    /// returns. Empty for a non-newtype inherent `impl`.
    exempt_scalars:  Vec<TypeName>,
    /// The `impl` declares `is_empty(&self) -> bool` — gates the `len` carve-out.
    has_is_empty:    bool,
    /// The `impl` declares a fixed-size-array `const` — proves the `index`
    /// carve-out's owned collection.
    has_owned_array: bool,
}

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
        impl_frames: Vec::new(),
        test_cfg_depth: 0,
        violations: Vec::new(),
    };
    walker.visit_file(&file);
    Ok(walker.violations)
}

/// The AST walker — carries the enclosing-context state the allowlist needs.
struct Walker<'a> {
    /// The file being walked (for violation coordinates).
    path:             &'a RepoPath,
    /// The tuple-struct newtypes declared in this file, mapped to the flagged
    /// inner type each wraps (if any).
    newtypes:         &'a HashMap<String, Option<TypeName>>,
    /// Depth of enclosing trait `impl`s (>0 ⇒ fn sigs exempt — rule 4).
    trait_impl_depth: usize,
    /// Stack of enclosing inherent `impl` frames — the newtype/std-container/
    /// owned-array facts the convention carve-outs read (see [`ImplFrame`]).
    impl_frames:      Vec<ImplFrame>,
    /// Depth of enclosing `#[cfg(test)]` modules (>0 ⇒ skip).
    test_cfg_depth:   usize,
    /// Accumulated violations.
    violations:       Vec<Violation>,
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

    /// Flag bare params + return of the fn `name`d by `sig`, honoring the
    /// convention carve-outs against the innermost enclosing inherent `impl`
    /// frame (if any). A param/return matching the frame's exempt scalars is the
    /// newtype's raw-value / coordinate-component boundary (rules 5 + carve-out
    /// (b)); a RETURN the [`return_is_conventional`] predicate names (carve-outs
    /// a/c/d) is skipped too.
    fn flag_signature(&mut self, sig: &syn::Signature, name: &str) {
        // Snapshot the enclosing-`impl` facts BEFORE the `&mut self` record
        // calls below (the frame borrows `self.impl_frames`).
        let frame = self.impl_frames.last();
        let exempt_scalars: Vec<TypeName> =
            frame.map(|f| f.exempt_scalars.clone()).unwrap_or_default();
        let has_is_empty = frame.is_some_and(|f| f.has_is_empty);
        let has_owned_array = frame.is_some_and(|f| f.has_owned_array);
        for input in &sig.inputs {
            let FnArg::Typed(pat) = input else {
                continue; // `self` receiver — not a typed param
            };
            if let Some(found) = flagged_type(&pat.ty)
                && !exempt_scalars.contains(&found)
            {
                self.record(&pat.ty, found, PositionKind::FnParam);
            }
        }
        if let ReturnType::Type(_, ty) = &sig.output
            && let Some(found) = flagged_type(ty)
            && !exempt_scalars.contains(&found)
            && !return_is_conventional(name, &found, sig, has_is_empty, has_owned_array)
        {
            self.record(ty, found, PositionKind::FnReturn);
        }
    }

    /// Build the [`ImplFrame`] for an inherent `impl`: its newtype raw-value /
    /// `glam`-component exempt scalars (carve-out (b) + rule 5) and the
    /// std-container / owned-array facts carve-outs (a) and (c) read.
    fn build_impl_frame(&self, node: &ItemImpl) -> ImplFrame {
        let inner = segment_head(&node.self_ty)
            .and_then(|(head, _)| self.newtypes.get(&head))
            .cloned()
            .flatten();
        let mut exempt_scalars = Vec::new();
        if let Some(inner) = inner {
            if let Some(component) = glam_component_scalar(&inner) {
                exempt_scalars.push(component);
            }
            exempt_scalars.push(inner);
        }
        ImplFrame {
            exempt_scalars,
            has_is_empty: declares_is_empty(&node.items),
            has_owned_array: declares_owned_array(&node.items),
        }
    }
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
        if is_trait_impl {
            // Trait-impl signatures are wholly exempt (rule 4); no frame needed.
            self.trait_impl_depth += 1;
            syn::visit::visit_item_impl(self, node);
            self.trait_impl_depth -= 1;
            return;
        }
        self.impl_frames.push(self.build_impl_frame(node));
        syn::visit::visit_item_impl(self, node);
        self.impl_frames.pop();
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if self.signatures_checked() {
            self.flag_signature(&node.sig, &node.sig.ident.to_string());
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        if self.signatures_checked() {
            self.flag_signature(&node.sig, &node.sig.ident.to_string());
        }
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast TraitItemFn) {
        // A trait DEFINITION's signatures are a domain-modeling choice (rule 4
        // exempts trait-*impl* signatures only), so they are checked.
        if self.signatures_checked() {
            self.flag_signature(&node.sig, &node.sig.ident.to_string());
        }
        syn::visit::visit_trait_item_fn(self, node);
    }
}
