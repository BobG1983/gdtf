fn normalize(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

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

pub(crate) fn feature_declaration(manifest: &str, feature: &str) -> Option<String> {
    section_lines(manifest, "features")
        .into_iter()
        .find(|line| {
            line.split_once('=')
                .is_some_and(|(lhs, _)| lhs.trim() == feature)
        })
}
