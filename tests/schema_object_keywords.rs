#![cfg(feature = "lossless-schema-keywords")]
//! Typed `properties` and `required` access that keeps malformed values.
use oas::{Schema, SchemaValue};
use serde_json::{json, Value};
use std::path::Path;

fn object(value: Value) -> Box<Schema> {
    match SchemaValue::from_lossless_value(value).unwrap() {
        SchemaValue::Object(schema) => schema,
        other => panic!("expected an object schema, got {other:?}"),
    }
}

fn to_value(schema: &Schema) -> Value {
    serde_json::to_value(schema).unwrap()
}

#[test]
fn present_absent_and_malformed_keywords_are_distinct() {
    let schema = object(json!({"properties": {"a": true}, "required": ["a"]}));
    assert_eq!(
        schema
            .properties_map()
            .unwrap()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["a"]
    );
    assert_eq!(schema.required_names(), Ok(Some(&["a".to_owned()][..])));

    let schema = object(json!({"type": "object"}));
    assert!(matches!(schema.properties_map(), Ok(None)));
    assert_eq!(schema.required_names(), Ok(None));

    let schema = object(json!({"properties": 5, "required": ["a", 1]}));
    assert_eq!(schema.properties_map().unwrap_err(), &json!(5));
    assert_eq!(schema.required_names().unwrap_err(), &json!(["a", 1]));
    assert_eq!(
        object(json!({"required": null}))
            .required_names()
            .unwrap_err(),
        &Value::Null
    );
}

#[test]
fn typed_fields_shadow_retained_fallbacks() {
    let mut schema = object(json!({"properties": 5, "required": "a"}));
    schema.properties = Some([("b".to_owned(), SchemaValue::from(false))].into());
    schema.required = Some(vec!["b".to_owned()]);
    assert!(schema.properties_map().unwrap().unwrap().contains_key("b"));
    assert_eq!(schema.required_names(), Ok(Some(&["b".to_owned()][..])));
    assert!(schema.properties_map_mut().unwrap().is_some());
    assert!(schema.required_names_mut().unwrap().is_some());
    assert_eq!(
        to_value(&schema),
        json!({"properties": {"b": false}, "required": ["b"]})
    );
}

#[test]
fn malformed_keywords_reject_edits_and_stay_unchanged() {
    let source = json!({"properties": "x", "required": {"a": 1}, "x-keep": [1]});
    let mut schema = object(source.clone());
    assert_eq!(schema.properties_map_mut().unwrap_err(), &mut json!("x"));
    assert_eq!(
        schema.properties_map_or_insert().unwrap_err(),
        &mut json!("x")
    );
    assert_eq!(
        schema.required_names_mut().unwrap_err(),
        &mut json!({"a": 1})
    );
    assert_eq!(
        schema.required_names_or_insert().unwrap_err(),
        &mut json!({"a": 1})
    );
    assert_eq!(
        schema.insert_required_name("a").unwrap_err(),
        &mut json!({"a": 1})
    );
    assert_eq!(to_value(&schema), source);
}

#[test]
fn absent_keywords_are_created_on_insert() {
    let mut schema = object(json!({"description": "d"}));
    assert!(schema.properties_map_mut().unwrap().is_none());
    assert!(schema.required_names_mut().unwrap().is_none());
    assert_eq!(to_value(&schema), json!({"description": "d"}));
    schema.properties_map_or_insert().unwrap();
    schema.required_names_or_insert().unwrap();
    assert_eq!(
        to_value(&schema),
        json!({"description": "d", "properties": {}, "required": []})
    );
    assert_eq!(schema.insert_required_name("a"), Ok(true));
    assert_eq!(schema.insert_required_name("a"), Ok(false));
    assert_eq!(schema.insert_required_name("b"), Ok(true));
    assert_eq!(
        schema.required_names(),
        Ok(Some(&["a".to_owned(), "b".to_owned()][..]))
    );
}

/// The handshake-header rewrite of a WebSocket binding: drop a header in any
/// case and gateway-owned headers, then publish the gateway's own header.
#[test]
fn handshake_headers_are_rewritten_without_leaving_the_typed_schema() {
    let mut headers = object(json!({
        "type": "object",
        "description": "handshake",
        "additionalProperties": false,
        "x-vendor": {"keep": true},
        "properties": {
            "MODEL": {"type": "string"},
            "x-fish-internal-id": {},
            "trace": {"type": "string", "x-note": 1}
        },
        "required": ["MODEL", "trace", "x-fish-internal-id"]
    }));
    let keep = |name: &str| !name.eq_ignore_ascii_case("model") && !name.starts_with("x-fish-");
    headers
        .properties_map_mut()
        .unwrap()
        .unwrap()
        .retain(|name, _| keep(name));
    headers.retain_required_names(keep);
    headers.properties_map_or_insert().unwrap().insert(
        "model".into(),
        SchemaValue::from_lossless_value(json!({"type": "string", "enum": ["m1"]})).unwrap(),
    );
    assert_eq!(headers.insert_required_name("model"), Ok(true));
    assert_eq!(
        to_value(&headers),
        json!({
            "type": "object",
            "description": "handshake",
            "additionalProperties": false,
            "x-vendor": {"keep": true},
            "properties": {
                "model": {"type": "string", "enum": ["m1"]},
                "trace": {"type": "string", "x-note": 1}
            },
            "required": ["trace", "model"]
        })
    );
}

fn check_schema(schema: &SchemaValue, json: &Value, checked: &mut usize) {
    let Some(object) = schema.as_object() else {
        return;
    };
    *checked += 1;
    match object.properties_map() {
        Ok(Some(properties)) => assert_eq!(
            &serde_json::to_value(properties).unwrap(),
            &json["properties"]
        ),
        Ok(None) => assert!(json.get("properties").is_none()),
        Err(raw) => {
            assert!(!raw.is_object());
            assert_eq!(raw, &json["properties"]);
        }
    }
    match object.required_names() {
        Ok(Some(required)) => assert_eq!(&json!(required), &json["required"]),
        Ok(None) => assert!(json.get("required").is_none()),
        Err(raw) => assert_eq!(raw, &json["required"]),
    }
    object
        .try_for_each_child_at(|step, child| {
            let pointer = step.to_string();
            check_schema(child, json.pointer(&pointer).unwrap(), checked);
            Ok::<_, ()>(())
        })
        .unwrap();
}

#[test]
fn fixture_schemas_read_back_their_own_keywords() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    for dir in ["examples/v3.0/json", "examples/v3.2/json"] {
        for entry in std::fs::read_dir(root.join(dir)).unwrap() {
            let source: Value =
                serde_json::from_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap())
                    .unwrap();
            let Some(Value::Object(schemas)) = source.pointer("/components/schemas") else {
                continue;
            };
            for json in schemas.values() {
                let schema = SchemaValue::from_lossless_value(json.clone()).unwrap();
                assert_eq!(&serde_json::to_value(&schema).unwrap(), json, "fidelity");
                check_schema(&schema, json, &mut checked);
            }
        }
    }
    assert!(checked >= 20, "{checked}");
}
