//! The pinned census detectors — band classification, raw line counting, and
//! the comment-stripped `mod.rs` logic scanners, mirroring the census command
//! in `.claude/rules/module-layout.md` byte-for-byte so the guard and the
//! census can never disagree.

/// The census band a tracked `.rs` file falls into (pinned precedence).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Band {
    /// `mod.rs` wiring files — checked for logic, never for line count.
    Mod,
    /// Integration-test files directly under `crates/<crate>/tests/`.
    Integration,
    /// In-src test modules.
    SrcTest,
    /// Everything else — production logic.
    Logic,
}

impl Band {
    /// The census label used in violation lines.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Integration => "integration",
            Self::SrcTest => "src-test",
            Self::Logic => "logic",
        }
    }
}

/// Classify `path` (repo-relative, forward slashes) with the census precedence.
pub(crate) fn band(path: &str) -> Band {
    let base = path.rsplit('/').next().unwrap_or(path);
    if base == "mod.rs" {
        return Band::Mod;
    }
    let integration = path
        .strip_prefix("crates/")
        .and_then(|rest| rest.split_once('/'))
        .is_some_and(|(krate, tail)| !krate.is_empty() && tail.starts_with("tests/"));
    if integration {
        return Band::Integration;
    }
    if path.ends_with("/test.rs")
        || path.ends_with("/tests.rs")
        || path.contains("/test/")
        || path.contains("/tests/")
        || base.starts_with("test_")
        || path.contains("test_support")
    {
        return Band::SrcTest;
    }
    Band::Logic
}

/// Raw `wc -l`-equivalent line count over the file bytes.
pub(crate) fn raw_line_count(bytes: &[u8]) -> usize {
    let newlines = bytes.split(|&b| b == b'\n').count().saturating_sub(1);
    if bytes.last().is_some_and(|b| *b != b'\n') {
        newlines + 1
    } else {
        newlines
    }
}

/// Whether `b` is a `\w` word byte (for the census regexes' `\b` boundaries).
const fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Strip `/* … */` block comments (single left-to-right pass, non-nested — the
/// pinned census regex semantics), then everything from `//` to end-of-line.
fn strip_comments(src: &str) -> String {
    let mut blocks = String::with_capacity(src.len());
    let mut rest = src;
    loop {
        let Some(start) = rest.find("/*") else {
            blocks.push_str(rest);
            break;
        };
        let Some(end) = rest[start + 2..].find("*/") else {
            blocks.push_str(rest);
            break;
        };
        blocks.push_str(&rest[..start]);
        rest = &rest[start + 2 + end + 2..];
    }
    blocks
        .lines()
        .map(|line| line.find("//").map_or(line, |i| &line[..i]))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The census `\bfn\s+(\w+)` matcher — every fn name in comment-stripped source.
fn fn_names(stripped: &str) -> Vec<String> {
    let bytes = stripped.as_bytes();
    let mut names = Vec::new();
    let mut from = 0;
    while let Some(rel) = stripped.get(from..).and_then(|s| s.find("fn")) {
        let at = from + rel;
        from = at + 2;
        if at > 0 && is_word(bytes[at - 1]) {
            continue;
        }
        let mut i = at + 2;
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if i == at + 2 {
            continue; // the census regex requires `\s+` after `fn`
        }
        let start = i;
        while bytes.get(i).is_some_and(|b| is_word(*b)) {
            i += 1;
        }
        if i > start {
            names.push(String::from_utf8_lossy(&bytes[start..i]).into_owned());
        }
    }
    names
}

/// Word-boundary substring search (`\b<word>\b`).
fn has_word(text: &str, word: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(rel) = text.get(from..).and_then(|s| s.find(word)) {
        let at = from + rel;
        from = at + word.len();
        let before_ok = at == 0 || !is_word(bytes[at - 1]);
        let after_ok = bytes.get(at + word.len()).is_none_or(|b| !is_word(*b));
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// Whether an impl-header segment matches the census
/// `\b(Plugin|PluginGroup)\s+for\b` allowance.
fn plugin_for(seg: &str) -> bool {
    let bytes = seg.as_bytes();
    for word in ["Plugin", "PluginGroup"] {
        let mut from = 0;
        while let Some(rel) = seg.get(from..).and_then(|s| s.find(word)) {
            let at = from + rel;
            from = at + word.len();
            if at > 0 && is_word(bytes[at - 1]) {
                continue;
            }
            let mut i = at + word.len();
            while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
                i += 1;
            }
            if i == at + word.len() {
                continue; // needs `\s+` — also rejects `PluginGroup` read as `Plugin`
            }
            if bytes.get(i..i + 3) == Some(b"for") && bytes.get(i + 3).is_none_or(|b| !is_word(*b))
            {
                return true;
            }
        }
    }
    false
}

/// The census `\bimpl\b[^{;]*` matcher — impl headers that are not
/// `Plugin for` / `PluginGroup for`.
fn bad_impls(stripped: &str) -> Vec<String> {
    let bytes = stripped.as_bytes();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(rel) = stripped.get(from..).and_then(|s| s.find("impl")) {
        let at = from + rel;
        from = at + 4;
        let before_ok = at == 0 || !is_word(bytes[at - 1]);
        let after_ok = bytes.get(at + 4).is_none_or(|b| !is_word(*b));
        if !(before_ok && after_ok) {
            continue;
        }
        let mut end = at + 4;
        while bytes.get(end).is_some_and(|b| *b != b'{' && *b != b';') {
            end += 1;
        }
        let seg = String::from_utf8_lossy(&bytes[at..end]);
        if !plugin_for(&seg) {
            found.push(seg.trim().to_owned());
        }
        from = end;
    }
    found
}

/// The census `add_systems\s*\([^)]*\|` matcher — an inline closure system.
fn closure_in_add_systems(stripped: &str) -> bool {
    let bytes = stripped.as_bytes();
    let mut from = 0;
    while let Some(rel) = stripped.get(from..).and_then(|s| s.find("add_systems")) {
        let at = from + rel;
        from = at + "add_systems".len();
        let mut i = from;
        while bytes.get(i).is_some_and(u8::is_ascii_whitespace) {
            i += 1;
        }
        if bytes.get(i) != Some(&b'(') {
            continue;
        }
        i += 1;
        while let Some(b) = bytes.get(i) {
            match b {
                b')' => break,
                b'|' => return true,
                _ => i += 1,
            }
        }
    }
    false
}

/// The census `modrs_logic` detector — the reasons a `mod.rs` is logic-bearing.
/// The fn allowlist is EMPTY (`build`/`name`/`register_*` were all removed by
/// the 2026-07-04 user ruling rejecting the fn-form aggregation carve-out).
pub(crate) fn modrs_reasons(src: &str) -> Vec<String> {
    let stripped = strip_comments(src);
    let mut reasons = Vec::new();
    let names: std::collections::BTreeSet<String> = fn_names(&stripped).into_iter().collect();
    if !names.is_empty() {
        let joined = names.into_iter().collect::<Vec<_>>().join(",");
        reasons.push(format!("fn {joined}"));
    }
    let impls = bad_impls(&stripped);
    if !impls.is_empty() {
        reasons.push(format!("impl: {}", impls.join("; ")));
    }
    if closure_in_add_systems(&stripped) {
        reasons.push("closure-system in add_systems".to_owned());
    }
    reasons
}

/// Whether a crate root is PURE WIRING — zero `fn`/`impl` after comment stripping.
pub(crate) fn pure_wiring(src: &str) -> bool {
    let stripped = strip_comments(src);
    !has_word(&stripped, "fn") && !has_word(&stripped, "impl")
}
