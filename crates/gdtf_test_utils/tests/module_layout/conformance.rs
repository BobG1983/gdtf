//! The clause-7 conformance test — applies the census detectors over the
//! enumerated tree, honors the exemption registry (and fails its stale
//! entries), prints warn/violation lines deterministically, and carries the
//! full violation list in one final assert.

use std::{collections::BTreeSet, fs};

use crate::{
    census::{Band, band, modrs_reasons, pure_wiring, raw_line_count},
    tree::{registry_paths, tracked_rs, workspace_root},
};

/// BLOCK band: a file over this many raw lines must not land.
const BLOCK_LINES: usize = 400;
/// WARN band: a file over this many raw lines is flagged (non-fatal).
const WARN_LINES: usize = 300;

/// A rendered violation line plus its `(-lines, path)` sort key.
struct Violation {
    lines: usize,
    path:  String,
    text:  String,
}

/// The clause-7 conformance guard (see the suite doc in `main.rs`).
#[test]
fn module_layout_conformance() {
    let root = workspace_root();
    let files = tracked_rs(&root);
    let registry = registry_paths(&root);
    let mut violations: Vec<Violation> = Vec::new();
    let mut warns: Vec<(usize, String)> = Vec::new();
    let mut live_exemptions: BTreeSet<String> = BTreeSet::new();
    if files.is_empty() {
        violations.push(Violation {
            lines: 0,
            path:  String::new(),
            text:  format!("no tracked .rs files found under {}", root.display()),
        });
    }
    for path in &files {
        let Ok(bytes) = fs::read(root.join(path)) else {
            continue; // tracked but deleted from the working tree — nothing to measure
        };
        let lines = raw_line_count(&bytes);
        let file_band = band(path);
        let registered = registry.contains(path);
        if file_band == Band::Mod {
            let reasons = modrs_reasons(&String::from_utf8_lossy(&bytes));
            if reasons.is_empty() {
                continue;
            }
            if registered {
                live_exemptions.insert(path.clone());
            } else {
                let text = format!("MODLOGIC {lines:5} {path}  [{}]", reasons.join("; "));
                violations.push(Violation {
                    lines,
                    path: path.clone(),
                    text,
                });
            }
            continue;
        }
        if lines > BLOCK_LINES {
            if registered {
                live_exemptions.insert(path.clone());
                let base = path.rsplit('/').next().unwrap_or(path);
                let crate_root = base == "lib.rs" || base == "main.rs";
                if crate_root && !pure_wiring(&String::from_utf8_lossy(&bytes)) {
                    let text = format!(
                        "BLOCK {} {lines:5} {path}  [registered crate root grew logic — must stay pure wiring]",
                        file_band.label()
                    );
                    violations.push(Violation {
                        lines,
                        path: path.clone(),
                        text,
                    });
                }
            } else {
                let text = format!("BLOCK {} {lines:5} {path}", file_band.label());
                violations.push(Violation {
                    lines,
                    path: path.clone(),
                    text,
                });
            }
        } else if lines > WARN_LINES {
            warns.push((lines, path.clone()));
        }
    }
    for entry in &registry {
        if !live_exemptions.contains(entry) {
            let text = format!("stale exemption — remove {entry}");
            violations.push(Violation {
                lines: 0,
                path: entry.clone(),
                text,
            });
        }
    }
    warns.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    for (lines, path) in &warns {
        eprintln!("warn  {lines:5} {path}");
    }
    violations.sort_by(|a, b| b.lines.cmp(&a.lines).then_with(|| a.path.cmp(&b.path)));
    for violation in &violations {
        eprintln!("{}", violation.text);
    }
    let rendered = violations
        .iter()
        .map(|v| v.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        violations.is_empty(),
        "module-layout violations (.claude/rules/module-layout.md):\n{rendered}"
    );
}
