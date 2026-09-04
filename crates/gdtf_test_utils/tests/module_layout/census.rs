#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Band {
    Mod,
    Integration,
    SrcTest,
    Logic,
}

impl Band {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Integration => "integration",
            Self::SrcTest => "src-test",
            Self::Logic => "logic",
        }
    }
}

pub(crate) fn band(path: &str) -> Band {
    let base = path.rsplit('/').next().unwrap_or(path);
    if base == "mod.rs" {
        return Band::Mod;
    }
    let integration = path
        .strip_prefix("crates/")
        .or_else(|| path.strip_prefix("libs/"))
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

pub(crate) fn raw_line_count(bytes: &[u8]) -> usize {
    let newlines = bytes.split(|&b| b == b'\n').count().saturating_sub(1);
    if bytes.last().is_some_and(|b| *b != b'\n') {
        newlines + 1
    } else {
        newlines
    }
}

const fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

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
            continue;
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
                continue;
            }
            if bytes.get(i..i + 3) == Some(b"for") && bytes.get(i + 3).is_none_or(|b| !is_word(*b))
            {
                return true;
            }
        }
    }
    false
}

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

pub(crate) fn pure_wiring(src: &str) -> bool {
    let stripped = strip_comments(src);
    !has_word(&stripped, "fn") && !has_word(&stripped, "impl")
}
