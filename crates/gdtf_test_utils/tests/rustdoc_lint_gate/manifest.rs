fn normalize(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn section_lines(manifest: &str, section: &str) -> Vec<String> {
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

pub(crate) fn declares(manifest: &str, section: &str, declaration: &str) -> bool {
    let wanted = normalize(declaration);
    section_lines(manifest, section).contains(&wanted)
}
