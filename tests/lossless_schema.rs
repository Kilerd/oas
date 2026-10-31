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
fn untyped_schema_entry_preserves_root_and_nested_legacy_fields() {
    for input in compatibility_inputs().into_iter().chain([
        json!(true),
        json!(false),
        json!({"type":null,"description":null,"required":[]}),
        json!({"type":7,"contentSchema":{"type":null}}),
    ]) {
        let parsed = SchemaValue::from_lossless_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), input);
    }
    let mut parsed = SchemaValue::from_lossless_value(json!({
        "type":null,"required":["name"],"properties":{"name":{"type":"string"}}
    }))
    .unwrap();
    let schema = parsed.as_object_mut().unwrap();
    assert_eq!(schema.extras["type"], Value::Null);
    assert_eq!(schema.required.as_deref(), Some(["name".into()].as_slice()));
    schema
        .properties
        .as_mut()
        .unwrap()
        .insert("name".into(), false.into());
    assert_eq!(
        serde_json::to_value(parsed).unwrap(),
        json!({
            "type":null,"required":["name"],"properties":{"name":false}
        })
    );
    for input in [Value::Null, json!([]), json!("string"), json!(1)] {
        assert!(SchemaValue::from_lossless_value(input.clone()).is_err());
        assert!(serde_json::from_value::<SchemaValue>(input).is_err());
    }
}

#[test]
fn owned_legacy_admission_matches_the_existing_decoder() {
    for input in compatibility_inputs().into_iter().chain([
        json!({"type":7}),
        json!({"description":null,"contentSchema":{"type":null}}),
        json!({"contentSchema":{"description":3}}),
        json!({"properties":{"item":{"description":null}}}),
    ]) {
        let expected = serde_json::from_value::<SchemaValue>(input.clone());
        let actual = SchemaValue::from_lossless_value(input.clone())
            .unwrap()
            .admit_legacy_root();
        match (expected, actual) {
            (Ok(expected), Ok(actual)) => assert_eq!(
                serde_json::to_value(actual).unwrap(),
                serde_json::to_value(expected).unwrap(),
                "legacy admission for {input}"
            ),
            (Err(_), Err(_)) => {}
            _ => panic!("legacy admission differs for {input}"),
        }
    }
}

#[test]
fn native_wrapper_children_are_typed_without_interpreting_unmarked_literals() {
    let input = json!({"properties":{
        "wrapped":{"schemaFormat":"application/vnd.aai.asyncapi+json;version=3.0.0",
                   "schema":{"description":null,"properties":{"event":{"const":"ready"}}}},
        "literal":{"schema":{"type":"string"}}
    }});
    let mut schema = SchemaValue::from_lossless_value(input.clone()).unwrap();
    assert_eq!(serde_json::to_value(&schema).unwrap(), input);
    assert!(schema
        .schema_at_tokens(&["properties", "wrapped", "schema", "properties", "event"])
        .is_some());
    assert!(schema
        .schema_at_tokens(&["properties", "literal", "schema"])
        .is_none());
    let wrapped = schema
        .as_object_mut()
        .unwrap()
        .properties
        .as_mut()
        .unwrap()
        .get_mut("wrapped")
        .unwrap()
        .as_object_mut()
        .unwrap();
    wrapped
        .schema
        .as_mut()
        .unwrap()
        .as_object_mut()
        .unwrap()
        .required = Some(vec!["event".into()]);
    assert_eq!(
        serde_json::to_value(schema).unwrap()["properties"]["wrapped"]["schema"]["required"],
        json!(["event"])
    );
}

#[test]
fn schema_metadata_keeps_nulls_and_supports_typed_reference_edits() {
    use oas::lossless_component::Field;
    let input = json!({
        "externalDocs":{"$ref":"#/components/externalDocs/Guide","description":null},
        "discriminator":{"propertyName":"kind","mapping":{"ready":"#/components/schemas/Ready","opaque":null}},
        "examples":[{"externalDocs":{"$ref":"literal"}}]
    });
    let mut schema = SchemaValue::from_lossless_value(input.clone()).unwrap();
    assert_eq!(serde_json::to_value(&schema).unwrap(), input);
    let object = schema.as_object_mut().unwrap();
    object.external_docs.as_mut().unwrap().reference =
        Field::Value("#/components/externalDocs/PublicGuide".into());
    object
        .discriminator
        .as_mut()
        .unwrap()
        .mapping
        .as_value_mut()
        .unwrap()
        .insert(
            "ready".into(),
            Field::Value("#/components/schemas/PublicReady".into()),
        );
    let mut expected = input;
    expected["externalDocs"]["$ref"] = json!("#/components/externalDocs/PublicGuide");
    expected["discriminator"]["mapping"]["ready"] = json!("#/components/schemas/PublicReady");
    assert_eq!(serde_json::to_value(schema).unwrap(), expected);
}

#[test]
fn structural_fields_interleave_typed_and_invalid_values_in_keyword_order() {
    use oas::SchemaFieldMut;
    let input = json!({"allOf":null,"contentSchema":true,"items":[false],
        "properties":{"a":{}},"schema":{"literal":true},"examples":[{"allOf":false}]});
    let mut schema = SchemaValue::from_lossless_value(input.clone()).unwrap();
    let mut visited = Vec::new();
    schema
        .as_object_mut()
        .unwrap()
        .try_for_each_structural_field_mut::<()>(|key, field| {
            visited.push((key, matches!(field, SchemaFieldMut::Invalid(_))));
            if let SchemaFieldMut::Map(children) = field {
                children.insert("b".into(), true.into());
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(
        visited,
        vec![
            ("allOf", true),
            ("contentSchema", false),
            ("items", false),
            ("properties", false)
        ]
    );
    let mut expected = input;
    expected["properties"]["b"] = json!(true);
    assert_eq!(serde_json::to_value(schema).unwrap(), expected);
}

#[test]
fn taking_structural_fields_preserves_literals_without_revealing_shadowed_values() {
    use oas::OwnedSchemaField;
    let mut schema = SchemaValue::from_lossless_value(json!({
        "properties":null,"allOf":null,"contentSchema":true,
        "schema":{"literal":true},"examples":[{"properties":7}]
    }))
    .unwrap();
    let object = schema.as_object_mut().unwrap();
    object.properties = Some([("event".into(), Schema::string().into())].into());
    let fields = object.take_structural_fields();
    assert_eq!(
        fields.keys().copied().collect::<Vec<_>>(),
        vec!["allOf", "contentSchema", "properties"]
    );
    assert!(matches!(
        fields["allOf"],
        OwnedSchemaField::Invalid(Value::Null)
    ));
    let OwnedSchemaField::Map(properties) = &fields["properties"] else {
        panic!("typed properties")
    };
    assert_eq!(
        serde_json::to_value(&properties["event"]).unwrap(),
        json!({"type":"string"})
    );
    assert_eq!(
        serde_json::to_value(schema).unwrap(),
        json!({
            "schema":{"literal":true},"examples":[{"properties":7}]
        })
    );
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
fn mixed_children_preserve_typed_siblings_and_wire_values() {
    let input = json!({
        "properties":{"event":{"const":"x-fish-usage"},"other":7},
        "required":["event"],
        "allOf":[{"type":"string"},null,true],
        "items":[{"type":"number"},[],false],
        "dependencies":{"event":{"type":"string"},"other":[1],"names":["event"]}
    });
    let parsed = SchemaValue::from_lossless_value(input.clone()).unwrap();
    let object = parsed.as_object().unwrap();
    let properties = object.properties.as_ref().expect("typed properties");
    let Some(oas::SchemaField::Map(borrowed)) = object.structural_field("properties", false) else {
        panic!("borrowed properties")
    };
    assert!(std::ptr::eq(properties, borrowed));
    assert!(object.structural_field("required", false).is_none());
    assert!(object.structural_field("missing", false).is_none());
    assert_eq!(
        properties.keys().map(String::as_str).collect::<Vec<_>>(),
        ["event", "other"]
    );
    assert!(matches!(&properties["other"], SchemaValue::Invalid(value) if value == &json!(7)));
    assert!(properties["other"].as_object().is_none());
    let event = &properties["event"];
    assert!(std::ptr::eq(
        event,
        parsed.schema_at_tokens(&["properties", "event"]).unwrap()
    ));
    assert_eq!(
        serde_json::to_value(event).unwrap(),
        json!({"const":"x-fish-usage"})
    );
    assert!(object.all_of.as_ref().unwrap()[0].as_object().is_some());
    assert!(matches!(
        object.all_of.as_ref().unwrap()[1],
        SchemaValue::Invalid(Value::Null)
    ));
    let Some(SchemaItems::Tuple(items)) = &object.items else {
        panic!("typed items")
    };
    assert!(items[0].as_object().is_some());
    assert!(matches!(&items[1], SchemaValue::Invalid(value) if value == &json!([])));
    let SchemaDependency::Schema(dependency) = &object.dependencies.as_ref().unwrap()["event"]
    else {
        panic!("typed dependency")
    };
    assert!(dependency.as_object().is_some());
    assert_eq!(serde_json::to_value(&parsed).unwrap(), input);
}

#[test]
fn keyword_moves_preserve_literal_nulls_and_remove_both_field_representations() {
    let mut target = SchemaValue::from_lossless_value(json!({
        "description":"old","required":["old"],"default":1,"x-meta":{"old":true}
    }))
    .unwrap();
    let mut patch = SchemaValue::from_lossless_value(json!({
        "description":null,"required":null,"default":null,"x-meta":null
    }))
    .unwrap();
    let target = target.as_object_mut().unwrap();
    let patch = patch.as_object_mut().unwrap();
    assert_eq!(
        patch.keyword_names(),
        ["default", "description", "required", "x-meta"]
    );
    for key in ["default", "description", "required", "x-meta"] {
        assert!(patch.keyword_is_null(key));
        let mut wire = Vec::new();
        assert_eq!(
            patch
                .serialize_keyword(key, &mut serde_json::Serializer::new(&mut wire))
                .unwrap(),
            Some(())
        );
        assert_eq!(wire, b"null");
    }
    // The caller owns deletion policy; moving null itself always preserves it.
    assert!(target.remove_keyword("description"));
    assert!(target.remove_keyword("required"));
    assert!(target.move_keyword_from("default", patch));
    assert!(target.clone_keyword_from("x-meta", patch));
    assert!(patch.keyword_is_null("x-meta"));
    assert_eq!(
        serde_json::to_value(&*target).unwrap(),
        json!({"default":null,"x-meta":null})
    );
    patch.description = Some("new".into());
    assert!(!patch.keyword_is_null("description"));
    assert!(target.clone_keyword_from("description", patch));
    assert_eq!(target.description.as_deref(), Some("new"));
    assert_eq!(patch.description.as_deref(), Some("new"));
    assert!(target.move_keyword_from("description", patch));
    assert!(!patch.remove_keyword("description"));
    assert!(!target.move_keyword_from("description", patch));
    assert_eq!(target.description.as_deref(), Some("new"));
    target
        .extras
        .insert("description".into(), json!("shadowed"));
    assert_eq!(target.keyword_names(), ["default", "description", "x-meta"]);
    assert!(target.remove_keyword("description"));
    assert!(!target.remove_keyword("description"));
    let mut wire = Vec::new();
    assert_eq!(
        target
            .serialize_keyword("description", &mut serde_json::Serializer::new(&mut wire))
            .unwrap(),
        None
    );
    assert!(wire.is_empty());
}

#[test]
fn structural_patch_fields_move_back_without_reparsing_typed_children() {
    use oas::OwnedSchemaField;
    let mut target = SchemaValue::from_lossless_value(json!({
        "properties":{"drop":{"type":"string"},"keep":{"default":1}}
    }))
    .unwrap();
    let mut patch = SchemaValue::from_lossless_value(json!({
        "properties":{"drop":null,"keep":{"default":null}}
    }))
    .unwrap();
    let object = target.as_object_mut().unwrap();
    let mut fields = object.take_structural_fields();
    let mut patch_fields = patch.as_object_mut().unwrap().take_structural_fields();
    let OwnedSchemaField::Map(properties) = fields.get_mut("properties").unwrap() else {
        panic!("map")
    };
    let OwnedSchemaField::Map(mut updates) = patch_fields.remove("properties").unwrap() else {
        panic!("map")
    };
    assert!(matches!(
        updates.remove("drop"),
        Some(SchemaValue::Invalid(Value::Null))
    ));
    properties.remove("drop");
    properties
        .get_mut("keep")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .move_keyword_from(
            "default",
            updates.get_mut("keep").unwrap().as_object_mut().unwrap(),
        );
    let child = properties["keep"].as_object().unwrap() as *const Schema;
    object.extras.insert("properties".into(), Value::Null);
    object
        .put_structural_field("properties", fields.remove("properties").unwrap())
        .unwrap();
    assert!(std::ptr::eq(
        object.properties.as_ref().unwrap()["keep"]
            .as_object()
            .unwrap(),
        child
    ));
    assert!(!object.extras.contains_key("properties"));
    assert!(object
        .put_structural_field("properties", OwnedSchemaField::Array(vec![]))
        .is_err());
    assert!(object
        .put_structural_field("unknown", OwnedSchemaField::Map(Default::default()))
        .is_err());
    assert_eq!(
        serde_json::to_value(&target).unwrap(),
        json!({"properties":{"keep":{"default":null}}})
    );
    target
        .as_object_mut()
        .unwrap()
        .put_structural_field("properties", OwnedSchemaField::Invalid(Value::Null))
        .unwrap();
    assert_eq!(
        serde_json::to_value(target).unwrap(),
        json!({"properties":null})
    );
}

#[test]
fn structural_schema_context_survives_marker_removal_and_promotes_literal_once() {
    use oas::OwnedSchemaField;
    for (target, patch) in [
        (
            json!({"schemaFormat":"native","schema":{"description":"old"}}),
            json!({"schema":{"description":"new"}}),
        ),
        (
            json!({"schema":{"description":"old"}}),
            json!({"schemaFormat":"native","schema":{"description":"new"}}),
        ),
        (
            json!({"schemaFormat":"native","schema":{"description":"old"}}),
            json!({"schemaFormat":null,"schema":{"description":"new"}}),
        ),
    ] {
        let mut target = SchemaValue::from_lossless_value(target).unwrap();
        let mut patch = SchemaValue::from_lossless_value(patch).unwrap();
        let target = target.as_object_mut().unwrap();
        let patch = patch.as_object_mut().unwrap();
        let structural = target.keyword_names().contains(&"schemaFormat")
            || patch.keyword_names().contains(&"schemaFormat");
        if patch.keyword_is_null("schemaFormat") {
            target.remove_keyword("schemaFormat");
        }
        let target_fields = target.take_structural_fields_with_schema(structural);
        let mut patch_fields = patch.take_structural_fields_with_schema(structural);
        assert!(matches!(
            target_fields["schema"],
            OwnedSchemaField::Single(_)
        ));
        let field = patch_fields.remove("schema").unwrap();
        assert!(matches!(&field, OwnedSchemaField::Single(_)));
        target.put_structural_field("schema", field).unwrap();
        assert_eq!(
            serde_json::to_value(target).unwrap()["schema"],
            json!({"description":"new"})
        );
    }
    let mut literal =
        SchemaValue::from_lossless_value(json!({"schema":{"description":null}})).unwrap();
    let object = literal.as_object().unwrap();
    assert!(object.structural_field("schema", false).is_none());
    let Some(oas::SchemaField::Invalid(raw)) = object.structural_field("schema", true) else {
        panic!("context borrows the retained literal without decoding")
    };
    assert!(std::ptr::eq(raw, &object.extras["schema"]));
    assert!(literal
        .as_object_mut()
        .unwrap()
        .take_structural_fields_with_schema(false)
        .is_empty());
    assert_eq!(
        serde_json::to_value(literal).unwrap(),
        json!({"schema":{"description":null}})
    );
}

#[test]
fn malformed_fields_and_empty_containers_preserve_wire_values() {
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
