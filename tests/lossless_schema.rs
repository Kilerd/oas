#![cfg(feature = "lossless-schema-keywords")]
use oas::{Operation, Schema, SchemaDependency, SchemaItems, SchemaValue};
use serde_json::{json, Value};

fn operation(schema: Value) -> Value {
    json!({"responses":{"200":{"description":"ok","content":{
        "application/json":{"schema":schema}
    }}}})
}

// These ordinary inputs were compared against the published 0.4.0 Operation
// decoder. Only root description:null was elided by that version.
fn compatibility_inputs() -> Vec<Value> {
    vec![
        json!({"required":["name"]}),
        json!({"required":null}),
        json!({"required":[1]}),
        json!({"minProperties":1.0}),
        json!({"minProperties":null}),
        json!({"maxProperties":1.5}),
        json!({"dependencies":{"name":["other"]}}),
        json!({"dependencies":{"name":[1]}}),
        json!({"dependencies":null}),
        json!({"dependencies":{"name":{"description":null}}}),
        json!({"dependentRequired":null}),
        json!({"dependentRequired":{"name":[1]}}),
        json!({"properties":{"name":{"description":null}}}),
        json!({"properties":{"name":{"type":7}}}),
        json!({"properties":null}),
        json!({"items":[{"type":"string"}]}),
        json!({"additionalItems":1}),
        json!({"description":null}),
        json!({"default":null,"example":null,"x-note":null}),
        json!({"type":[]}),
        json!({"type":["null","string","null"]}),
        json!({"nullable":false}),
        json!({"properties":{"name":{"type":[],"nullable":false}}}),
        json!({"required":"name"}),
    ]
}

#[test]
fn embedded_operation_schemas_keep_the_old_admission_and_output() {
    for schema in compatibility_inputs() {
        let input = operation(schema.clone());
        let parsed: Operation = serde_json::from_value(input.clone()).unwrap();
        let expected = if schema == json!({"description":null}) {
            operation(json!({}))
        } else {
            input
        };
        assert_eq!(serde_json::to_value(parsed).unwrap(), expected);
    }
}

#[test]
fn origin_preserves_legacy_scalar_errors_and_null_elision_only_at_old_boundaries() {
    for schema in [json!({"type":7}), json!({"contentSchema":{"type":7}})] {
        assert!(serde_json::from_value::<Operation>(operation(schema)).is_err());
    }
    let parsed: Schema = serde_json::from_value(json!({
        "description":null, "nullable":null,
        "contentSchema":{"description":null,"contentSchema":{"nullable":null}},
        "properties":{"name":{"type":7,"description":null,"nullable":null,
            "contentSchema":{"type":7,"description":null}}}
    }))
    .unwrap();
    assert_eq!(
        serde_json::to_value(parsed).unwrap(),
        json!({
            "contentSchema":{"contentSchema":{}},
            "properties":{"name":{"type":7,"description":null,"nullable":null,
                "contentSchema":{"type":7,"description":null}}}
        })
    );
}

#[test]
fn valid_fields_and_mixed_children_are_typed_without_hiding_fallbacks() {
    let schema: Schema = serde_json::from_value(json!({
        "required":["name"],"minProperties":1.0,"maxProperties":1.5,
        "properties":{"name":{"type":"string"},"other":{"type":7,"description":null}},
        "dependencies":{"name":["other"],"other":{"required":["name"]}},
        "items":[true,{"type":"number"}],"dependentRequired":{"name":["other"]}
    }))
    .unwrap();
    assert_eq!(schema.required.as_deref(), Some(["name".into()].as_slice()));
    assert_eq!(schema.min_properties.as_ref().unwrap().value(), 1);
    assert!(schema.max_properties.is_none());
    assert_eq!(schema.extras["maxProperties"], json!(1.5));
    let properties = schema.properties.as_ref().unwrap();
    assert!(properties["name"].as_object().unwrap()._type.is_some());
    let other = properties["other"].as_object().unwrap();
    assert!(other._type.is_none());
    assert_eq!(other.extras["type"], 7);
    assert_eq!(other.extras["description"], Value::Null);
    assert!(matches!(
        schema.dependencies.as_ref().unwrap()["name"],
        SchemaDependency::Required(_)
    ));
    assert!(matches!(
        schema.dependencies.as_ref().unwrap()["other"],
        SchemaDependency::Schema(_)
    ));
    assert!(matches!(schema.items, Some(SchemaItems::Tuple(_))));
    assert!(schema.dependent_required.is_some());
}

#[test]
fn typed_edits_override_fallback_once_and_keyword_count_matches_output() {
    let mut schema: Schema = serde_json::from_value(json!({
        "required":null,"properties":null,"minProperties":null,"x-note":null
    }))
    .unwrap();
    assert_eq!(schema.keyword_count(), 4);
    schema.required = Some(vec!["name".into()]);
    schema.min_properties = Some(1.into());
    schema.properties = Some([("name".into(), Schema::string().into())].into());
    let encoded = serde_json::to_string(&schema).unwrap();
    for key in ["required", "properties", "minProperties"] {
        assert_eq!(encoded.matches(&format!("\"{key}\":")).count(), 1);
    }
    assert_eq!(schema.keyword_count(), 4);
    assert_eq!(
        serde_json::from_str::<Value>(&encoded).unwrap(),
        json!({
            "required":["name"],"properties":{"name":{"type":"string"}},
            "minProperties":1,"x-note":null
        })
    );
    let mut visited = 0;
    schema
        .try_for_each_child_mut::<()>(|child| {
            visited += 1;
            *child = false.into();
            Ok(())
        })
        .unwrap();
    assert_eq!(visited, 1);
    assert_eq!(
        serde_json::to_value(&schema).unwrap()["properties"]["name"],
        false
    );
    // Clearing a typed override reveals its still-owned fallback; deleting both
    // representations removes the keyword explicitly.
    schema.required = None;
    assert_eq!(
        serde_json::to_value(&schema).unwrap()["required"],
        Value::Null
    );
    schema.extras.remove("required");
    assert_eq!(schema.keyword_count(), 3);
}

#[test]
fn invalid_containers_are_retained_whole_and_empty_containers_stay_present() {
    for value in [
        json!({"properties":{"a":{"type":"string"},"b":null}}),
        json!({"allOf":[{"type":"string"},null]}),
        json!({"dependencies":{"a":true,"b":[1]}}),
        json!({"dependentRequired":{"a":["b"],"b":[1]}}),
        json!({"properties":{},"required":[],"allOf":[],"items":[],"dependencies":{}}),
        json!({}),
    ] {
        let schema: Schema = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(schema.keyword_count(), value.as_object().unwrap().len());
        assert_eq!(serde_json::to_value(schema).unwrap(), value);
    }
    assert!(serde_json::from_value::<SchemaValue>(Value::Null).is_err());
}
