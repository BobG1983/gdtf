fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn opens_block(rest: &str) -> bool {
    rest.is_empty() || rest.starts_with('|') || rest.starts_with('>')
}

fn normalise(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn block_commands(lines: &[&str], start: usize, header_indent: usize) -> (Vec<String>, usize) {
    let mut commands = Vec::new();
    let mut pending = String::new();
    let mut index = start;
    while let Some(raw) = lines.get(index) {
        if !raw.trim().is_empty() && indent_of(raw) <= header_indent {
            break;
        }
        index += 1;
        let text = raw.trim();
        if text.is_empty() {
            continue;
        }
        if let Some(head) = text.strip_suffix('\\') {
            pending.push_str(head.trim_end());
            pending.push(' ');
            continue;
        }
        pending.push_str(text);
        commands.push(normalise(&pending));
        pending.clear();
    }
    if !pending.trim().is_empty() {
        commands.push(normalise(&pending));
    }
    (commands, index)
}

pub(crate) fn run_commands(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut commands = Vec::new();
    let mut index = 0;
    while let Some(raw) = lines.get(index) {
        let header_indent = indent_of(raw);
        let trimmed = raw.trim();
        let step = trimmed.strip_prefix("- ").unwrap_or(trimmed).trim_start();
        index += 1;
        let Some(rest) = step.strip_prefix("run:") else {
            continue;
        };
        let rest = rest.trim();
        if !opens_block(rest) {
            commands.push(normalise(rest));
            continue;
        }
        let (body, next) = block_commands(&lines, index, header_indent);
        commands.extend(body);
        index = next;
    }
    commands
}

#[test]
fn reads_inline_and_block_run_steps() {
    let yaml = "\
jobs:
  green:
    steps:
      - name: Inline
        run: cargo test --workspace --features a/net_qa
      - name: Block
        run: |
          sudo apt-get update
          cargo clippy --workspace --all-targets \\
            --features a/net_qa -- -D warnings
      - name: After the block
        run: cargo fmt --check
";
    assert_eq!(
        run_commands(yaml),
        vec![
            "cargo test --workspace --features a/net_qa".to_owned(),
            "sudo apt-get update".to_owned(),
            "cargo clippy --workspace --all-targets --features a/net_qa -- -D warnings".to_owned(),
            "cargo fmt --check".to_owned(),
        ]
    );
}
