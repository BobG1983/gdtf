// Every word the guide wrote in backticks. That is how it names a command, so a
// name buried in prose does not count and a longer name does not swallow a shorter one.
pub(crate) fn backticked_words(guide: &str) -> Vec<&str> {
    guide
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|word| !word.is_empty() && !word.contains('\n'))
        .collect()
}

// Every path the guide links to, as written, skipping links to a heading or to the web.
pub(crate) fn linked_paths(guide: &str) -> Vec<&str> {
    let mut paths: Vec<&str> = guide
        .split("](")
        .skip(1)
        .filter_map(|tail| tail.split_once(')').map(|(target, _)| target))
        .filter_map(|target| target.split_whitespace().next())
        .filter(|target| !target.starts_with('#') && !target.starts_with("http"))
        .collect();
    paths.sort_unstable();
    paths.dedup();
    paths
}
