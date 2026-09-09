use std::{collections::BTreeSet, fs};

use crate::module_layout::{
    census::{Band, band, modrs_reasons, pure_wiring, raw_line_count},
    tree::{tracked_rs, workspace_root},
};

const BLOCK_LINES: usize = 400;

// Files the module-layout rule excuses, by repo-relative path. Adding one is a code
// change: propose it, get the user's approval, then land the entry with the reason.
const EXEMPT_PATHS: &[&str] = &[];

struct Violation {
    lines: usize,
    path:  String,
    text:  String,
}

#[test]
fn module_layout_conformance() {
    let root = workspace_root();
    let files = tracked_rs(&root);
    let mut violations: Vec<Violation> = Vec::new();
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
            continue;
        };
        let lines = raw_line_count(&bytes);
        let file_band = band(path);
        let exempt = EXEMPT_PATHS.contains(&path.as_str());
        if file_band == Band::Mod {
            let reasons = modrs_reasons(&String::from_utf8_lossy(&bytes));
            if reasons.is_empty() {
                continue;
            }
            if exempt {
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
            if exempt {
                live_exemptions.insert(path.clone());
                let base = path.rsplit('/').next().unwrap_or(path);
                let crate_root = base == "lib.rs" || base == "main.rs";
                if crate_root && !pure_wiring(&String::from_utf8_lossy(&bytes)) {
                    let text = format!(
                        "BLOCK {} {lines:5} {path}  [exempt crate root grew logic — must stay pure wiring]",
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
        }
    }
    for &entry in EXEMPT_PATHS {
        if !live_exemptions.contains(entry) {
            let text = format!(
                "{entry} is excused by EXEMPT_PATHS in this guard, but it no longer breaks the \
                 rule (or no longer exists) — delete that entry"
            );
            violations.push(Violation {
                lines: 0,
                path: entry.to_owned(),
                text,
            });
        }
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
