use serde::Serialize;
use serde_json::{Value, json};

pub(super) fn text_content<T: Serialize>(value: &T) -> Value {
    let text =
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "<unserializable>".to_owned());
    json!({ "content": [ { "type": "text", "text": text } ], "isError": false })
}

pub(super) fn tool_error(message: &str) -> Value {
    json!({ "content": [ { "type": "text", "text": message } ], "isError": true })
}
