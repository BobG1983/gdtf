use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn repo_root() -> PathBuf {
    std::env::var_os("GDTF_NO_GTW_ROOT").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

/// Run `rg GTW-` over the repo. Falls back to `git grep` if ripgrep is missing.
fn gtw_hits(root: &Path) -> Result<Vec<String>, String> {
    let rg = Command::new("rg")
        .arg("-n")
        .arg("--no-heading")
        .arg("-S")
        .arg("GTW-")
        .arg("--glob")
        .arg("!target/**")
        .arg("--glob")
        .arg("!.git/**")
        .arg(".")
        .current_dir(root)
        .output();

    match rg {
        Ok(out) if out.status.success() || out.status.code() == Some(1) => {
            // rg exits 1 when no matches; 0 when matches found.
            let text = String::from_utf8_lossy(&out.stdout);
            Ok(text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(str::to_owned)
                .collect())
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            Err(format!("rg exited {}: {}", out.status, err.trim()))
        }
        Err(_) => {
            // No rg on PATH — use git grep as fallback.
            let out = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["grep", "-n", "GTW-"])
                .output()
                .map_err(|e| format!("git grep failed to spawn: {e}"))?;
            if out.status.success() || out.status.code() == Some(1) {
                let text = String::from_utf8_lossy(&out.stdout);
                Ok(text
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(str::to_owned)
                    .collect())
            } else {
                let err = String::from_utf8_lossy(&out.stderr);
                Err(format!("git grep exited {}: {}", out.status, err.trim()))
            }
        }
    }
}

#[test]
fn no_gtw_ticket_strings_remain() {
    let root = repo_root();
    let hits = gtw_hits(&root).unwrap_or_else(|err| {
        panic!("no-GTW gate could not run search: {err}");
    });

    for line in &hits {
        eprintln!("{line}");
    }
    let rendered = hits.join("\n");
    assert!(
        hits.is_empty(),
        "GTW- ticket strings still present in the tree (no-GTW gate). \
         Strip them — comments, asserts, docs, assets, everything:\n{rendered}"
    );
}
