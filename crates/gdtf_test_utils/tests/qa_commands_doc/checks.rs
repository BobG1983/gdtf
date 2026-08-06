use super::{
    commands::{published_command_names, published_families, reads_as_command_name},
    guide::{backticked_words, linked_paths},
    tree::{GUIDE, WORKED_EXAMPLE, read, repo_root},
};

#[test]
fn the_guide_cites_its_worked_example_by_path() {
    let guide = read(GUIDE);
    assert!(
        guide.contains(WORKED_EXAMPLE),
        "the guide must cite {WORKED_EXAMPLE} by path, so a reader can open the code it \
         describes",
    );
}

#[test]
fn every_path_the_guide_links_to_is_there() {
    let guide = read(GUIDE);
    let guide_path = repo_root().join(GUIDE);
    let Some(directory) = guide_path.parent() else {
        unreachable!("{GUIDE} must sit in a directory");
    };
    let linked = linked_paths(&guide);
    assert!(
        !linked.is_empty(),
        "the guide must link to the code it describes, and this guard found no links at all \
         — check how it reads links before changing the guide",
    );
    let missing: Vec<&str> = linked
        .into_iter()
        .filter(|target| !directory.join(target).exists())
        .collect();
    assert!(
        missing.is_empty(),
        "the guide links to files that are not there, so a reader following one lands \
         nowhere — point each link at where the file moved to, or drop it: {missing:?}",
    );
}

#[test]
fn the_guide_names_every_command_the_game_publishes() {
    let guide = read(GUIDE);
    let published = published_command_names();
    assert!(
        !published.is_empty(),
        "the command sources must declare at least one name, or this guard reads nothing",
    );
    let words = backticked_words(&guide);
    let undocumented: Vec<&String> = published
        .iter()
        .filter(|name| !words.contains(&name.as_str()))
        .collect();
    assert!(
        undocumented.is_empty(),
        "a client learns what it can call from this guide, so every published command has to \
         appear in it, written in backticks: {undocumented:?}",
    );
}

#[test]
fn every_command_the_guide_names_is_one_a_host_publishes() {
    let guide = read(GUIDE);
    let published = published_command_names();
    let families = published_families(&published);
    assert!(
        !families.is_empty(),
        "the command sources must declare at least one dotted name, or this guard reads \
         nothing",
    );
    let invented: Vec<&str> = backticked_words(&guide)
        .into_iter()
        .filter(|word| reads_as_command_name(word, &families))
        .filter(|word| !published.iter().any(|name| name.as_str() == *word))
        .collect();
    assert!(
        invented.is_empty(),
        "the guide writes these the way it writes a command name, and no host publishes them, \
         so a caller that tries one is told the name is unknown — build them or take them out \
         of the guide: {invented:?}",
    );
}
