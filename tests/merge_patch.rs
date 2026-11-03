//! RFC 7386 JSON Merge Patch on values, typed schemas and metadata nodes.
use oas::merge_patch::{merge, merge_with};
use serde_json::{json, Value};

/// RFC 7386 Appendix A: (original, patch, result).
fn appendix_a() -> Vec<(Value, Value, Value)> {
    vec![
        (json!({"a": "b"}), json!({"a": "c"}), json!({"a": "c"})),
        (
            json!({"a": "b"}),
            json!({"b": "c"}),
            json!({"a": "b", "b": "c"}),
        ),
        (json!({"a": "b"}), json!({"a": null}), json!({})),
        (
            json!({"a": "b", "b": "c"}),
            json!({"a": null}),
            json!({"b": "c"}),
        ),
        (json!({"a": ["b"]}), json!({"a": "c"}), json!({"a": "c"})),
        (json!({"a": "c"}), json!({"a": ["b"]}), json!({"a": ["b"]})),
        (
            json!({"a": {"b": "c"}}),
            json!({"a": {"b": "d", "c": null}}),
            json!({"a": {"b": "d"}}),
        ),
        (
            json!({"a": [{"b": "c"}]}),
            json!({"a": [1]}),
            json!({"a": [1]}),
        ),
        (json!(["a", "b"]), json!(["c", "d"]), json!(["c", "d"])),
        (json!({"a": "b"}), json!(["c"]), json!(["c"])),
        (json!({"a": "foo"}), json!(null), json!(null)),
        (json!({"a": "foo"}), json!("bar"), json!("bar")),
        (
            json!({"e": null}),
            json!({"a": 1}),
            json!({"e": null, "a": 1}),
        ),
        (
            json!([1, 2]),
            json!({"a": "b", "c": null}),
            json!({"a": "b"}),
        ),
        (
            json!({}),
            json!({"a": {"bb": {"ccc": null}}}),
            json!({"a": {"bb": {}}}),
        ),
    ]
}

#[test]
fn values_merge_as_in_rfc_7386_appendix_a() {
    for (index, (original, patch, result)) in appendix_a().into_iter().enumerate() {
        let mut target = original;
        merge(&mut target, &patch);
        assert_eq!(target, result, "appendix A case {}", index + 1);
    }
}

#[test]
fn values_merge_as_in_the_rfc_7386_section_3_example() {
    let mut target = json!({
        "title": "Goodbye!",
        "author": {"givenName": "John", "familyName": "Doe"},
        "tags": ["example", "sample"],
        "content": "This will be unchanged"
    });
    merge(
        &mut target,
        &json!({
            "title": "Hello!",
            "phoneNumber": "+01-123-456-7890",
            "author": {"familyName": null},
            "tags": ["example"]
        }),
    );
    assert_eq!(
        target,
        json!({
            "title": "Hello!",
            "author": {"givenName": "John"},
            "tags": ["example"],
            "content": "This will be unchanged",
            "phoneNumber": "+01-123-456-7890"
        })
    );
}

#[test]
fn charged_merges_count_patched_values_and_stop_at_the_first_error() {
    let mut target = json!({"a": {"b": 1}, "z": 0});
    let patch = json!({"a": {"b": 2, "c": null}, "d": [1, 2], "z": null});
    let mut units = 0;
    merge_with(&mut target, &patch, &mut || {
        units += 1;
        Ok::<_, ()>(())
    })
    .unwrap();
    // The patch, `a`, `a.b` and `d`; removals are free.
    assert_eq!(units, 4);
    assert_eq!(target, json!({"a": {"b": 2}, "d": [1, 2]}));

    let mut target = json!({"a": 1, "b": 1});
    let mut left = 2;
    let result = merge_with(&mut target, &json!({"a": 2, "b": 2}), &mut || {
        if left == 0 {
            return Err("exhausted");
        }
        left -= 1;
        Ok(())
    });
    assert_eq!(result, Err("exhausted"));
    assert_eq!(
        target,
        json!({"a": 2, "b": 1}),
        "earlier members stay merged"
    );
}

#[cfg(feature = "lossless-schema-keywords")]
mod typed {
    use super::appendix_a;
    use oas::lossless_asyncapi::{Message, Operation};
    use oas::lossless_component::Decode;
    use oas::merge_patch::{merge, MergePatch};
    use oas::SchemaValue;
    use serde_json::{json, Value};
    use std::path::Path;

    fn schema(value: Value) -> SchemaValue {
        SchemaValue::from_lossless_value(value).unwrap()
    }

    /// The typed merge serializes as the JSON merge and places every keyword
    /// where a fresh lossless decode of that JSON would.
    fn assert_schema_merge(target: &Value, patch: &Value, label: &str) {
        let mut expected = target.clone();
        merge(&mut expected, patch);
        let mut merged = schema(target.clone());
        merged.merge_patch(&schema(patch.clone()));
        assert_eq!(serde_json::to_value(&merged).unwrap(), expected, "{label}");
        assert_eq!(
            format!("{merged:?}"),
            format!("{:?}", schema(expected)),
            "{label}: typed placement"
        );
    }

    #[test]
    fn appendix_a_merges_inside_schema_keywords() {
        for (index, (original, patch, _)) in appendix_a().into_iter().enumerate() {
            let wrap = |value: &Value| {
                json!({
                    "default": value,
                    "x-extension": value,
                    "properties": {"p": {"const": value}},
                    "items": {"examples": [value], "default": value}
                })
            };
            assert_schema_merge(
                &wrap(&original),
                &wrap(&patch),
                &format!("appendix A case {}", index + 1),
            );
            if original.is_object() && patch.is_object() {
                assert_schema_merge(
                    &original,
                    &patch,
                    &format!("appendix A case {} as a schema", index + 1),
                );
            }
        }
    }

    #[test]
    fn schema_keywords_merge_by_their_json_shape() {
        let cases = [
            (
                json!({"type": "object", "properties": {"a": {"type": "string"}, "b": {}, "~/": true}, "required": ["a"]}),
                json!({"properties": {"a": {"maxLength": 3, "type": null}, "b": null, "c": false, "~/": {"type": "null"}}, "required": ["c"]}),
            ),
            (json!(true), json!({"type": "string", "description": null})),
            (json!({"type": "string"}), json!(false)),
            (
                json!({"items": [{"type": "string"}]}),
                json!({"items": {"type": "integer", "x": null}}),
            ),
            (
                json!({"items": {"type": "integer"}}),
                json!({"items": [true, {"minimum": 1}]}),
            ),
            (
                json!({"description": "d", "nullable": true, "format": "uuid", "$ref": "#/a"}),
                json!({"description": null, "nullable": null, "format": "email", "$ref": "#/b"}),
            ),
            (
                json!({"externalDocs": {"url": "https://a", "description": "d"}, "discriminator": {"propertyName": "k", "mapping": {"a": "#/a", "b": "#/b"}}}),
                json!({"externalDocs": {"url": null, "x-e": 1}, "discriminator": {"mapping": {"a": null, "c": "#/c"}}}),
            ),
            (
                json!({"dependentRequired": {"a": 5}}),
                json!({"dependentRequired": {"a": ["b"]}}),
            ),
            (
                json!({"dependentRequired": {"a": ["b"], "c": ["d"]}}),
                json!({"dependentRequired": {"a": null}}),
            ),
            (
                json!({"dependentRequired": {"a": ["b"]}}),
                json!({"dependentRequired": {"c": ["d"]}}),
            ),
            (
                json!({"dependencies": {"a": ["b"], "s": {"type": "string"}, "t": {}}}),
                json!({"dependencies": {"a": {"required": ["c"]}, "s": {"minLength": 1}, "t": null, "u": ["v"]}}),
            ),
            (
                json!({"schemaFormat": "x", "schema": {"type": "string"}}),
                json!({"schemaFormat": null, "schema": {"minLength": 1}}),
            ),
            (
                json!({"schema": {"a": 1}}),
                json!({"schemaFormat": "y", "schema": {"type": "integer", "a": null}}),
            ),
            (
                json!({"schemaFormat": "x", "schema": {"type": "string"}}),
                json!({"schema": {"type": null, "minLength": 1}}),
            ),
            (
                json!({"properties": {"a": {"type": "string"}, "b": 4}}),
                json!({"properties": {"a": 5, "b": {"type": "string"}}}),
            ),
            (
                json!({"properties": 3, "required": ["a", 1]}),
                json!({"properties": {"a": {"type": "string"}, "b": null}, "required": null}),
            ),
            (
                json!({"additionalProperties": false, "minProperties": 1}),
                json!({"additionalProperties": {"type": "string", "a": null}, "minProperties": 2}),
            ),
            (
                json!({"allOf": [{"type": "string"}], "type": "string"}),
                json!({"allOf": [{"minLength": 1}], "type": {"weird": 1}}),
            ),
            (
                json!({"type": {"weird": 1, "keep": 2}}),
                json!({"type": {"weird": null}}),
            ),
            (
                json!({"contentSchema": {"type": "string", "minLength": 1}}),
                json!({"contentSchema": {"type": null}}),
            ),
            (
                json!({"contentSchema": 5, "$defs": {"a": {"$ref": "#/b"}}}),
                json!({"contentSchema": {"type": "integer"}, "$defs": {"a": {"$ref": null}}}),
            ),
        ];
        for (index, (target, patch)) in cases.iter().enumerate() {
            assert_schema_merge(target, patch, &format!("case {index}"));
        }
    }

    fn fixture_schemas() -> Vec<Value> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut schemas = Vec::new();
        for dir in ["examples/v3.0/json", "examples/v3.2/json"] {
            for entry in std::fs::read_dir(root.join(dir)).unwrap() {
                let source: Value =
                    serde_json::from_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap())
                        .unwrap();
                if let Some(Value::Object(found)) = source.pointer("/components/schemas") {
                    schemas.extend(found.values().cloned());
                }
            }
        }
        assert!(schemas.len() >= 10, "{}", schemas.len());
        schemas
    }

    #[test]
    fn fixture_schemas_survive_empty_patches_and_merge_pairwise() {
        let schemas = fixture_schemas();
        for value in &schemas {
            // Fidelity: decode then serialize, and an empty patch, change nothing.
            assert_eq!(&serde_json::to_value(schema(value.clone())).unwrap(), value);
            assert_schema_merge(value, &json!({}), "empty patch");
            assert_schema_merge(&json!({}), value, "onto an empty schema");
        }
        for pair in schemas.windows(2) {
            assert_schema_merge(&pair[0], &pair[1], "fixture pair");
        }
    }

    #[test]
    fn schema_charges_stop_the_merge() {
        let mut target = schema(json!({"properties": {"a": {}, "b": {}}}));
        let patch =
            schema(json!({"properties": {"a": {"type": "string"}, "b": {"type": "string"}}}));
        let mut units = 0;
        target
            .merge_patch_with(&patch, &mut || {
                units += 1;
                Ok::<_, ()>(())
            })
            .unwrap();
        // The schema, `a`, `a.type`, `b`, `b.type`.
        assert_eq!(units, 5);
        let mut target = schema(json!({"properties": {"a": {}, "b": {}}}));
        let mut left = 3;
        let result = target.merge_patch_with(&patch, &mut || {
            if left == 0 {
                return Err(());
            }
            left -= 1;
            Ok(())
        });
        assert_eq!(result, Err(()));
        assert_eq!(
            serde_json::to_value(&target).unwrap(),
            json!({"properties": {"a": {"type": "string"}, "b": {}}})
        );
    }

    fn assert_node_merge<T: Decode + serde::Serialize + std::fmt::Debug>(
        target: Value,
        patch: Value,
        apply: impl Fn(&mut T, &T),
    ) {
        let mut expected = target.clone();
        merge(&mut expected, &patch);
        let mut merged = T::decode(target).unwrap();
        apply(&mut merged, &T::decode(patch).unwrap());
        assert_eq!(serde_json::to_value(&merged).unwrap(), expected);
        assert_eq!(
            format!("{merged:?}"),
            format!("{:?}", T::decode(expected).unwrap())
        );
    }

    #[test]
    fn metadata_nodes_merge_through_the_policy() {
        assert_node_merge::<Message>(
            json!({
                "name": "chat", "title": "Chat", "contentType": "application/json",
                "headers": {"type": "object", "properties": {"a": {"type": "string"}}, "required": ["a"]},
                "payload": true,
                "bindings": {"ws": {"method": "GET", "headers": {"properties": {"x": {}}}}, "mqtt": {"qos": 1}},
                "correlationId": {"location": "$message.header#/id"},
                "tags": [{"name": "a"}], "examples": [{"payload": 1}],
                "x-extension": {"keep": 1, "drop": 2}
            }),
            json!({
                "title": null, "contentType": "text/plain", "description": 5,
                "headers": {"properties": {"a": null, "b": {"type": "integer"}}, "required": ["b"]},
                "payload": {"type": "object"},
                "bindings": {"ws": {"headers": {"properties": {"x": {"type": "string"}}}}, "mqtt": null, "http": {"method": "POST"}},
                "correlationId": {"description": "id"},
                "tags": [{"name": "b"}], "examples": null,
                "x-extension": {"drop": null, "add": [1]}
            }),
            |target, patch| {
                target
                    .patch_metadata(patch, &mut MergePatch::new())
                    .unwrap()
            },
        );
        assert_node_merge::<Operation>(
            json!({"action": "send", "channel": {"$ref": "#/channels/a"}, "reply": {"address": {"location": "$message.header#/r"}}}),
            json!({"action": "receive", "reply": {"address": {"description": "d", "location": null}}, "summary": "s"}),
            |target, patch| {
                target
                    .patch_metadata(patch, &mut MergePatch::new())
                    .unwrap()
            },
        );
    }

    #[test]
    fn metadata_policy_charges_keys_and_values() {
        let mut message = Message::decode(json!({"name": "a"})).unwrap();
        let patch =
            Message::decode(json!({"name": "b", "bindings": {"ws": {"method": "GET"}}})).unwrap();
        let mut units = 0;
        message
            .patch_metadata(
                &patch,
                &mut MergePatch::charging(|| {
                    units += 1;
                    Ok::<_, ()>(())
                }),
            )
            .unwrap();
        assert!(units >= 4, "{units}");
        let mut message = Message::decode(json!({"name": "a"})).unwrap();
        let result = message.patch_metadata(&patch, &mut MergePatch::charging(|| Err("stop")));
        assert_eq!(result, Err("stop"));
        assert_eq!(
            serde_json::to_value(&message).unwrap(),
            json!({"name": "a"})
        );
    }
}
