use oas::{Schema, SchemaDependency, SchemaValue};
use serde_json::json;

#[test]
fn typed_edits_preserve_unknown_keywords_and_literal_payloads() {
    let mut schema: Schema = serde_json::from_value(json!({
        "type": "object",
        "properties": {"name": {"type": "string"}, "obsolete": true},
        "required": ["name"], "minProperties": 1, "maxProperties": 3,
        "allOf": [{"required": ["name"]}], "anyOf": [false], "oneOf": [true],
        "dependentRequired": {"name": ["obsolete"]},
        "dependentSchemas": {"name": {"minProperties": 2}},
        "dependencies": {"name": ["obsolete"], "obsolete": {"maxProperties": 2}},
        "default": null, "examples": [null, {"properties": null}],
        "futureKeyword": {"required": null}, "x-display": {"type": "badge"}
    }))
    .unwrap();

    let properties = schema.properties.as_mut().unwrap();
    properties.remove("obsolete");
    properties
        .get_mut("name")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .description = Some("Display name".into());
    properties.insert("active".into(), Schema::boolean().into());
    schema.required.as_mut().unwrap().push("active".into());
    schema.min_properties = Some((schema.min_properties.as_ref().unwrap().value() + 1).into());
    schema.max_properties = Some((schema.max_properties.as_ref().unwrap().value() - 1).into());
    schema.all_of.as_mut().unwrap()[0]
        .as_object_mut()
        .unwrap()
        .required
        .as_mut()
        .unwrap()
        .push("active".into());
    schema.any_of.as_mut().unwrap()[0] = true.into();
    schema.one_of.as_mut().unwrap().push(false.into());
    schema
        .dependent_required
        .as_mut()
        .unwrap()
        .get_mut("name")
        .unwrap()[0] = "active".into();
    schema
        .dependent_schemas
        .as_mut()
        .unwrap()
        .get_mut("name")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .min_properties = Some(1.into());
    let dependencies = schema.dependencies.as_mut().unwrap();
    let SchemaDependency::Required(required) = dependencies.get_mut("name").unwrap() else {
        panic!("property dependency should remain a string array");
    };
    required[0] = "active".into();
    let SchemaDependency::Schema(child) = dependencies.get_mut("obsolete").unwrap() else {
        panic!("schema dependency should remain a schema");
    };
    child.as_object_mut().unwrap().max_properties = Some(1.into());

    assert_eq!(
        serde_json::to_value(schema).unwrap(),
        json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Display name"},
                "active": {"type": "boolean"}
            },
            "required": ["name", "active"], "minProperties": 2, "maxProperties": 2,
            "allOf": [{"required": ["name", "active"]}], "anyOf": [true], "oneOf": [true, false],
            "dependentRequired": {"name": ["active"]},
            "dependentSchemas": {"name": {"minProperties": 1}},
            "dependencies": {"name": ["active"], "obsolete": {"maxProperties": 1}},
            "default": null, "examples": [null, {"properties": null}],
            "futureKeyword": {"required": null}, "x-display": {"type": "badge"}
        })
    );
}

#[test]
fn integral_float_counts_keep_their_json_number_representation() {
    let schema: Schema =
        serde_json::from_str(r#"{"minProperties":1.0,"maxProperties":2e0}"#).unwrap();
    assert_eq!(schema.min_properties.as_ref().unwrap().value(), 1);
    assert_eq!(schema.max_properties.as_ref().unwrap().value(), 2);
    assert_eq!(
        serde_json::to_value(schema).unwrap(),
        json!({"minProperties": 1.0, "maxProperties": 2.0})
    );
}

#[test]
fn explicitly_empty_keywords_remain_present() {
    let schema: Schema = serde_json::from_value(json!({
        "properties": {}, "required": [], "minProperties": 0, "maxProperties": 0,
        "allOf": [], "dependentRequired": {}, "dependentSchemas": {},
        "dependencies": {"name": []}, "items": [], "examples": []
    }))
    .unwrap();
    assert_eq!(
        serde_json::to_value(schema).unwrap(),
        json!({
            "properties": {}, "required": [], "minProperties": 0, "maxProperties": 0,
            "allOf": [], "dependentRequired": {}, "dependentSchemas": {},
            "dependencies": {"name": []}, "items": [], "examples": []
        })
    );
    assert_eq!(serde_json::to_value(Schema::default()).unwrap(), json!({}));
}

#[test]
fn malformed_known_keywords_are_rejected_even_with_reference_siblings() {
    for invalid in [
        json!({"properties": null}),
        json!({"properties": []}),
        json!({"properties": {"name": null}}),
        json!({"required": null}),
        json!({"required": [true]}),
        json!({"minProperties": null}),
        json!({"minProperties": -1}),
        json!({"maxProperties": 1.5}),
        json!({"maxProperties": "2"}),
        json!({"allOf": null}),
        json!({"allOf": {}}),
        json!({"anyOf": [null]}),
        json!({"oneOf": ["string"]}),
        json!({"additionalProperties": null}),
        json!({"not": []}),
        json!({"if": 1}),
        json!({"dependentRequired": {"name": [false]}}),
        json!({"dependentSchemas": {"name": []}}),
        json!({"dependencies": null}),
        json!({"dependencies": {"name": ["active", false]}}),
        json!({"dependencies": {"name": null}}),
        json!({"items": null}),
        json!({"items": "string"}),
        json!({"items": [true, null]}),
        json!({"$defs": {"Name": {"required": null}}}),
    ] {
        assert!(
            serde_json::from_value::<SchemaValue>(invalid.clone()).is_err(),
            "accepted malformed schema: {invalid}"
        );
        let mut reference = invalid;
        reference["$ref"] = json!("#/components/schemas/Base");
        assert!(
            serde_json::from_value::<SchemaValue>(reference.clone()).is_err(),
            "discarded malformed reference sibling: {reference}"
        );
    }
}

#[test]
fn traversal_edits_immediate_children_but_leaves_literal_data_and_grandchildren_alone() {
    let mut schema: Schema = serde_json::from_value(json!({
        "$ref": "#/components/schemas/Envelope",
        "properties": {"payload": {"properties": {"nested": {"type": "string"}}}},
        "patternProperties": {"^x-": false},
        "additionalProperties": false,
        "allOf": [{"$ref": "#/$defs/Base"}, true],
        "$defs": {"Base": {"type": "object"}},
        "items": [{"type": "string"}, false],
        "dependencies": {"payload": {"required": ["kind"]}, "kind": ["payload"]},
        "dependentRequired": {"payload": ["kind"]},
        "default": {"properties": {"nested": false}},
        "examples": [{"allOf": [false]}],
        "x-template": {"items": false},
        "futureKeyword": {"$ref": "#/$defs/Base"}
    }))
    .unwrap();

    schema
        .try_for_each_child_mut::<()>(|child| {
            match child {
                SchemaValue::Boolean(value) => *value = !*value,
                SchemaValue::Object(object) => object.description = Some("Visited".into()),
            }
            Ok(())
        })
        .unwrap();

    assert_eq!(
        serde_json::to_value(schema).unwrap(),
        json!({
            "$ref": "#/components/schemas/Envelope",
            "properties": {"payload": {
                "properties": {"nested": {"type": "string"}}, "description": "Visited"
            }},
            "patternProperties": {"^x-": true},
            "additionalProperties": true,
            "allOf": [{"$ref": "#/$defs/Base", "description": "Visited"}, false],
            "$defs": {"Base": {"type": "object", "description": "Visited"}},
            "items": [{"type": "string", "description": "Visited"}, true],
            "dependencies": {
                "payload": {"required": ["kind"], "description": "Visited"}, "kind": ["payload"]
            },
            "dependentRequired": {"payload": ["kind"]},
            "default": {"properties": {"nested": false}},
            "examples": [{"allOf": [false]}],
            "x-template": {"items": false},
            "futureKeyword": {"$ref": "#/$defs/Base"}
        })
    );
}

#[test]
fn traversal_handles_single_items_and_conditional_schema_children() {
    let mut schema: Schema = serde_json::from_value(json!({
        "items": {"type": "string"}, "prefixItems": [true],
        "contains": {"type": "number"}, "unevaluatedItems": false,
        "if": {"required": ["encoded"]}, "then": true, "else": false,
        "contentSchema": {"type": "object"},
        "dependentSchemas": {"encoded": {"required": ["payload"]}},
        "definitions": {"Payload": {"type": "string"}},
        "anyOf": [false, {"type": "object"}], "oneOf": [true], "not": false,
        "unevaluatedProperties": false, "propertyNames": {"type": "string"},
        "additionalItems": true,
        "default": null, "examples": [null]
    }))
    .unwrap();
    schema
        .try_for_each_child_mut::<()>(|child| {
            *child = Schema::integer().into();
            Ok(())
        })
        .unwrap();
    assert_eq!(
        serde_json::to_value(schema).unwrap(),
        json!({
            "items": {"type": "integer"}, "prefixItems": [{"type": "integer"}],
            "contains": {"type": "integer"}, "unevaluatedItems": {"type": "integer"},
            "if": {"type": "integer"}, "then": {"type": "integer"}, "else": {"type": "integer"},
            "contentSchema": {"type": "integer"},
            "dependentSchemas": {"encoded": {"type": "integer"}},
            "definitions": {"Payload": {"type": "integer"}},
            "anyOf": [{"type": "integer"}, {"type": "integer"}],
            "oneOf": [{"type": "integer"}], "not": {"type": "integer"},
            "unevaluatedProperties": {"type": "integer"}, "propertyNames": {"type": "integer"},
            "additionalItems": {"type": "integer"},
            "default": null, "examples": [null]
        })
    );
}

#[test]
fn callback_error_stops_traversal_and_retains_prior_edits() {
    let mut schema: Schema = serde_json::from_value(json!({
        "allOf": [{"description": "first"}, {"description": "stop"}, {"description": "last"}],
        "items": true
    }))
    .unwrap();
    let mut visited = Vec::new();
    let result = schema.try_for_each_child_mut(|child| {
        let object = child.as_object_mut().expect("must stop before items");
        let description = object.description.as_deref().unwrap();
        visited.push(description.to_owned());
        if description == "stop" {
            return Err("stop requested");
        }
        object.description = Some("edited".into());
        Ok(())
    });
    assert_eq!(result, Err("stop requested"));
    assert_eq!(visited, ["first", "stop"]);
    assert_eq!(
        serde_json::to_value(schema).unwrap(),
        json!({
            "allOf": [{"description": "edited"}, {"description": "stop"}, {"description": "last"}],
            "items": true
        })
    );
}

#[test]
fn named_traversal_orders_keywords_and_map_keys_but_preserves_array_order() {
    let mut schema: Schema = serde_json::from_value(json!({
        "properties": {"z": {"description":"z"}, "a": {"description":"a"}},
        "allOf": [{"description":"second"}, {"description":"first"}],
        "$defs": {"Base": {"description":"base"}},
        "dependencies": {"name": {"description":"legacy"}},
        "items": {"description":"item"},
        "examples": [{"description":"literal"}]
    }))
    .unwrap();
    let mut visited = Vec::new();
    schema
        .try_for_each_child_mut_with_keyword::<()>(|keyword, child| {
            let child = child.as_object_mut().unwrap();
            visited.push((keyword, child.description.clone().unwrap()));
            if keyword != "dependencies" {
                child.description = Some("edited".into());
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(
        visited,
        vec![
            ("$defs", "base".into()),
            ("allOf", "second".into()),
            ("allOf", "first".into()),
            ("dependencies", "legacy".into()),
            ("items", "item".into()),
            ("properties", "a".into()),
            ("properties", "z".into()),
        ]
    );
    let output = serde_json::to_value(schema).unwrap();
    assert_eq!(output["dependencies"]["name"]["description"], "legacy");
    assert_eq!(output["properties"]["a"]["description"], "edited");
    assert_eq!(output["examples"], json!([{"description":"literal"}]));
}

#[test]
fn count_access_keeps_representation_policy_separate_from_integer_value() {
    let integer: oas::SchemaCount = serde_json::from_str("2").unwrap();
    let floating: oas::SchemaCount = serde_json::from_str("2.0").unwrap();
    assert_eq!(integer.value(), floating.value());
    assert_eq!(integer.as_number().as_u64(), Some(2));
    assert_eq!(floating.as_number().as_u64(), None);
    assert_eq!(serde_json::to_value(&floating).unwrap(), json!(2.0));
}
