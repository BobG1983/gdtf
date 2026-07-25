//! The small TOML-subset manifest reader this guard needs — section bodies,
//! inline path dependencies, and one feature declaration.
//!
//! Deliberately NOT a `toml` dependency: the guard reads six flat, hand-kept
//! manifests — the three under `bins/` plus the one library each names as an
//! inline path dependency — all written in one style (`name = { path = "…" }`
//! dependencies, one-line `feature = ["…"]` declarations), and the sibling
//! guards in this crate are std-only for the same reason.
//!
//! Two shapes fall outside that style, and they behave differently:
//!
//! - A dependency entry the reader does not recognise (a `[dependencies.foo]`
//!   sub-table, say) drops out of the walk, so that pairing goes unchecked. For
//!   the two pairs named in `check.rs`'s `REQUIRED_PAIRS` that is caught — they
//!   are reported UNREACHED, which is exactly the liveness backstop those pairs
//!   exist for. A future binary not named there would go unchecked silently, so
//!   a new binary over a `net_qa`-bearing library is worth adding to that list.
//! - A `[features]` declaration split across lines (`net_qa = [`, then the
//!   entries — the style `crates/gdtf_app/Cargo.toml` uses on the library side)
//!   matches on its left-hand side but never on its value, so it is reported
//!   WRONG rather than skipped. That is a false alarm, but a loud and
//!   self-explaining one: the fix is to write the binary's passthrough on one
//!   line, as all three binaries do today.

/// Collapse a line's runs of whitespace to single spaces, so indentation and
/// padding around the `=` never decide a comparison: `net_qa   =  ["x/net_qa"]`
/// reads as `net_qa = ["x/net_qa"]`.
///
/// It evens out spacing; it does not reformat. `net_qa = [ "x/net_qa" ]` keeps
/// the spaces inside its brackets and is reported WRONG by `check.rs` — the
/// loud false alarm the module doc describes, not a silent skip.
fn normalize(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The non-comment, non-empty, whitespace-normalized lines inside `[section]`.
///
/// A section ends at the next `[header]` line. Only whole-line comments are
/// stripped; the manifests this guard reads put every comment on its own line.
fn section_lines(manifest: &str, section: &str) -> Vec<String> {
    let header = format!("[{section}]");
    let mut inside = false;
    let mut lines = Vec::new();
    for raw in manifest.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            inside = trimmed == header;
            continue;
        }
        if !inside || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        lines.push(normalize(trimmed));
    }
    lines
}

/// The dependency name and its declared relative path, for every inline
/// `name = { path = "…" }` entry in `[dependencies]`.
///
/// Registry dependencies (`serde = { workspace = true }`, `bevy_egui = "0.41"`)
/// carry no path and are skipped — they cannot be a workspace library this
/// guard has anything to say about.
pub(crate) fn path_dependencies(manifest: &str) -> Vec<(String, String)> {
    let mut deps = Vec::new();
    for line in section_lines(manifest, "dependencies") {
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let Some(after) = value.split_once("path =") else {
            continue;
        };
        let Some(rest) = after.1.trim_start().strip_prefix('"') else {
            continue;
        };
        let Some((path, _)) = rest.split_once('"') else {
            continue;
        };
        deps.push((name.to_owned(), path.to_owned()));
    }
    deps
}

/// The whitespace-normalized `[features]` line declaring `feature`, if the
/// manifest declares it at all.
///
/// Matches on the declaration's left-hand side only, so the caller can report
/// a WRONG declaration (a passthrough pointing at the wrong crate) distinctly
/// from a MISSING one.
pub(crate) fn feature_declaration(manifest: &str, feature: &str) -> Option<String> {
    section_lines(manifest, "features")
        .into_iter()
        .find(|line| {
            line.split_once('=')
                .is_some_and(|(lhs, _)| lhs.trim() == feature)
        })
}
