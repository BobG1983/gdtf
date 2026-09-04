use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::command::shape::shape_text;

#[derive(Debug, Serialize, Deserialize)]
struct ProbeUnit;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct ProbeTag(String);

#[derive(Debug, Serialize, Deserialize)]
struct ProbeCount(u32);

#[derive(Debug, Serialize, Deserialize)]
struct ProbePair(u32, String);

#[derive(Debug, Serialize, Deserialize)]
struct ProbeRecord {
    flag:  bool,
    maybe: Option<u32>,
    many:  Vec<String>,
    pair:  (u32, f32),
}

#[derive(Debug, Serialize, Deserialize)]
struct ProbeTable {
    rows: BTreeMap<String, u32>,
}

#[derive(Debug, Serialize, Deserialize)]
enum ProbeChoice {
    Idle,
    Named(String),
    Spanned(u32, u32),
    Placed { at: u32, tall: bool },
}

#[derive(Debug, Serialize, Deserialize)]
struct ProbeNested {
    tag:    ProbeTag,
    choice: ProbeChoice,
}

#[test]
fn a_unit_struct_wraps_nothing() {
    assert_eq!(
        shape_text::<ProbeUnit>(),
        r#"(root:Named("ProbeUnit"),defs:[("ProbeUnit",Wraps(Unit))])"#,
    );
}

#[test]
fn a_transparent_newtype_traces_as_its_bare_inner() {
    assert_eq!(
        shape_text::<ProbeTag>(),
        "(root:Text,defs:[])",
        "a transparent newtype forwards deserialize, so the wrapper has no name on the wire",
    );
}

#[test]
fn a_newtype_struct_that_is_not_transparent_wraps_a_one_element_tuple() {
    assert_eq!(
        shape_text::<ProbeCount>(),
        r#"(root:Named("ProbeCount"),defs:[("ProbeCount",Wraps(Tuple([Int])))])"#,
    );
}

#[test]
fn a_multi_field_tuple_struct_wraps_a_tuple() {
    assert_eq!(
        shape_text::<ProbePair>(),
        r#"(root:Named("ProbePair"),defs:[("ProbePair",Wraps(Tuple([Int,Text])))])"#,
    );
}

#[test]
fn a_named_field_struct_records_optionality_lists_and_tuples() {
    assert_eq!(
        shape_text::<ProbeRecord>(),
        concat!(
            r#"(root:Named("ProbeRecord"),defs:[("ProbeRecord",Record(["#,
            r#"("flag",Bool),("maybe",Optional(Int)),("many",List(Text)),"#,
            r#"("pair",Tuple([Int,Float]))]))])"#,
        ),
    );
}

#[test]
fn a_map_field_records_its_key_and_value() {
    assert_eq!(
        shape_text::<ProbeTable>(),
        r#"(root:Named("ProbeTable"),defs:[("ProbeTable",Record([("rows",Map(Text,Int))]))])"#,
    );
}

#[test]
fn an_enum_records_every_variant_body() {
    assert_eq!(
        shape_text::<ProbeChoice>(),
        concat!(
            r#"(root:Named("ProbeChoice"),defs:[("ProbeChoice",Choice([("Idle",Unit),"#,
            r#"("Named",Newtype(Text)),("Spanned",Tuple([Int,Int])),"#,
            r#"("Placed",Record([("at",Int),("tall",Bool)]))]))])"#,
        ),
    );
}

#[test]
fn a_named_type_is_defined_once_and_referenced_by_name() {
    assert_eq!(
        shape_text::<ProbeNested>(),
        concat!(
            r#"(root:Named("ProbeNested"),defs:[("ProbeChoice",Choice([("Idle",Unit),"#,
            r#"("Named",Newtype(Text)),("Spanned",Tuple([Int,Int])),"#,
            r#"("Placed",Record([("at",Int),("tall",Bool)]))])),"#,
            r#"("ProbeNested",Record([("tag",Text),("choice",Named("ProbeChoice"))]))])"#,
        ),
    );
}
