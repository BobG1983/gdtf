//! Compiling a workflow script the way its own harness loads it.

use std::{path::Path, process::Command};

/// The binary this guard parses JavaScript with. The workspace carries no JavaScript
/// parser, so there is nothing to fall back to when it is missing.
pub(crate) const NODE: &str = "node";

// The checker handed to `node -e`, with the script paths as its arguments.
const CHECKER: &str = r"
const fs = require('node:fs');
const vm = require('node:vm');
const answer = (fields) => process.stdout.write(fields.join('\t') + '\n');
for (const path of process.argv.slice(1)) {
  let source;
  try {
    source = fs.readFileSync(path, 'utf8');
  } catch (err) {
    answer(['fail', path, '0', 'unreadable: ' + err.message]);
    continue;
  }
  const bare = source.replace(/^export /gm, '');
  try {
    new vm.Script('(async function(){' + bare + '\n})', { filename: path });
    answer(['ok', path]);
  } catch (err) {
    const head = String(err.stack).split('\n')[0];
    const tail = head.slice(head.lastIndexOf(':') + 1);
    const line = /^[0-9]+$/.test(tail) ? tail : '0';
    answer(['fail', path, line, String(err.message).replace(/\s+/g, ' ')]);
  }
}
";

/// What the checker said about a set of scripts.
pub(crate) enum Outcome {
    /// Every script compiled.
    AllParse,
    /// One line per script the parser rejected, naming the path and the line.
    Rejected(Vec<String>),
    /// The checker could not be run, so nothing was parsed.
    Unavailable(String),
}

impl Outcome {
    /// True only when every script compiled.
    pub(crate) const fn is_clean(&self) -> bool {
        matches!(self, Self::AllParse)
    }

    /// What to print when this outcome fails a guard.
    pub(crate) fn report(&self) -> String {
        match self {
            Self::AllParse => "every script compiled".to_owned(),
            Self::Rejected(found) => format!(
                "these workflow scripts do not parse:\n{}\n\nA script that does not parse \
                 is refused at launch, so every run of it is blocked. A bare backtick \
                 inside a template literal is the usual cause: escape it as \\` the way \
                 the surrounding prompt text already does.",
                found.join("\n"),
            ),
            Self::Unavailable(why) => format!(
                "this guard parses every workflow script with `{NODE}` and could not: \
                 {why}\n\nInstall `{NODE}` and run the suite again. Skipping the parse \
                 would let a script that no run can load reach develop.",
            ),
        }
    }
}

/// Compile each path with `node`, from `root` so a repo-relative path resolves.
pub(crate) fn compile(root: &Path, paths: &[String]) -> Outcome {
    let Ok(output) = Command::new(NODE)
        .current_dir(root)
        .arg("-e")
        .arg(CHECKER)
        .args(paths)
        .output()
    else {
        return Outcome::Unavailable(format!("`{NODE}` could not be executed"));
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let answers: Vec<&str> = stdout.lines().collect();
    if answers.len() != paths.len() {
        return Outcome::Unavailable(format!(
            "it answered for {} of {} scripts. Its stderr was:\n{}",
            answers.len(),
            paths.len(),
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    let rejected: Vec<String> = answers
        .iter()
        .filter_map(|answer| rejection(answer))
        .collect();
    if rejected.is_empty() {
        Outcome::AllParse
    } else {
        Outcome::Rejected(rejected)
    }
}

// One failure line, or None when the checker accepted that script.
fn rejection(answer: &str) -> Option<String> {
    let mut fields = answer.split('\t');
    match (fields.next(), fields.next(), fields.next(), fields.next()) {
        (Some("ok"), Some(_), None, None) => None,
        (Some("fail"), Some(path), Some(line), Some(message)) => {
            Some(format!("{path}:{line}  {message}"))
        }
        _ => Some(format!("`{answer}` is an answer this guard cannot read")),
    }
}
