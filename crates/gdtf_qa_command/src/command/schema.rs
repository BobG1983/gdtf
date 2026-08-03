use schemars::JsonSchema;

pub(crate) fn schema_text<T: JsonSchema>() -> String {
    let schema = schemars::schema_for!(T);
    serde_json::to_string(&schema).unwrap_or_else(|_| "{\"type\":\"object\"}".to_owned())
}
