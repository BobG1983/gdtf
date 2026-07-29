//! Reads the commands a workflow step RUNS out of the YAML text — both the
//! single-line `run: <command>` form and the block form (`run: |` / `run: >`
//! with the command on the following, more-indented lines).
//!
//! Both forms are read because this repo already uses both: `test.yml`'s apt
//! step is `run: |` with a backslash-continued command, and a future workflow
//! could put a `cargo … --workspace` command in that form. Reading only the
//! single-line form would let such a command past the guard unchecked.
//!
//! Only `run:` lines count — a YAML comment mentioning a cargo command is prose
//! about the build, not a step CI executes.

/// Leading-space count, the block-scalar body's containment test.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Whether what follows `run:` opens a block scalar (`|`, `>`, with any
/// chomping/indent indicator) rather than holding the command inline. An empty
/// remainder is treated the same way — the command is on the next lines.
fn opens_block(rest: &str) -> bool {
    rest.is_empty() || rest.starts_with('|') || rest.starts_with('>')
}

/// Collapses runs of whitespace to single spaces so a line-wrap or indentation
/// change does not read as a different command.
fn normalise(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The logical commands inside a block scalar's body, plus the index of the
/// first line past it.
///
/// A body line ending in `\` continues onto the next line (the shell's own
/// continuation), so the two are joined into one logical command.
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

/// Every command any `run:` step in the file executes, whitespace-normalised.
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

/// Both `run:` forms are read, and a backslash-continued block line is one
/// command — the block form is the one a single-line-only reader missed.
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
