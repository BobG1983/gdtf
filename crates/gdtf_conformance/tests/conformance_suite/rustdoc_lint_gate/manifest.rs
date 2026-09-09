use toml_edit::{DocumentMut, Item, Value};

// A manifest this guard cannot parse is a broken guard, not a pass.
pub(crate) fn parse(path: &str, text: &str) -> DocumentMut {
    match text.parse::<DocumentMut>() {
        Ok(manifest) => manifest,
        Err(error) => unreachable!("{path} is not valid TOML: {error}"),
    }
}

// The value at a dotted key path such as `workspace.lints.rustdoc.all`. Section headers, inline
// tables and dotted keys all read the same way, so an equivalent spelling finds the same value.
pub(crate) fn item_at<'a>(manifest: &'a DocumentMut, path: &str) -> Option<&'a Item> {
    let mut keys = path.split('.');
    let mut item = manifest.get(keys.next()?)?;
    for key in keys {
        item = item.get(key)?;
    }
    Some(item)
}

// The level of a lint entry, written either as `name = "deny"` or as a table carrying `level`.
pub(crate) fn lint_level(entry: &Item) -> Option<&str> {
    entry
        .as_str()
        .or_else(|| entry.get("level").and_then(Item::as_str))
}

// The priority of a lint entry, which decides whether a named lint can override a group.
pub(crate) fn lint_priority(entry: &Item) -> Option<i64> {
    entry.get("priority").and_then(Item::as_integer)
}

// Every string in a TOML array; empty when the item is not an array of strings.
pub(crate) fn strings(item: &Item) -> Vec<&str> {
    item.as_array().map_or_else(Vec::new, |array| {
        array.iter().filter_map(Value::as_str).collect()
    })
}
