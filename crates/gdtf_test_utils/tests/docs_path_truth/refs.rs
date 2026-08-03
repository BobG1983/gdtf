const ANCHORS: [&str; 7] = [
    "docs/", "crates/", "bins/", "assets/", "content/", ".claude/", ".cargo/",
];

const fn is_path_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'/' | b'-')
}

pub(crate) fn root_anchored(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let Some(anchor_len) = ANCHORS
            .iter()
            .find(|a| bytes[i..].starts_with(a.as_bytes()))
            .map(|a| a.len())
        else {
            i += 1;
            continue;
        };
        if i > 0 && is_path_byte(bytes[i - 1]) {
            i += 1;
            continue;
        }
        let mut end = i + anchor_len;
        while end < bytes.len() && is_path_byte(bytes[end]) {
            end += 1;
        }
        if bytes.get(end).is_some_and(|b| *b == b'*' || *b == b'<') {
            i = end;
            continue;
        }
        let mut run = &bytes[i..end];
        while run.last().is_some_and(|b| *b == b'.') {
            run = &run[..run.len() - 1];
        }
        found.push(String::from_utf8_lossy(run).into_owned());
        i = end;
    }
    found
}

pub(crate) fn link_targets(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (idx, _) in text.match_indices("](") {
        let rest = &text[idx + 2..];
        let Some(close) = rest.find(')') else {
            continue;
        };
        let mut target = rest[..close].trim();
        if let Some(stripped) = target.strip_prefix('<') {
            target = stripped.strip_suffix('>').unwrap_or(stripped);
        }
        if let Some((path, _title)) = target.split_once(char::is_whitespace) {
            target = path;
        }
        let target = target.split('#').next().unwrap_or("");
        if target.is_empty() || target.contains("://") || target.starts_with("mailto:") {
            continue;
        }
        if target.contains('*') || target.contains('<') {
            continue;
        }
        found.push(target.to_owned());
    }
    found
}
