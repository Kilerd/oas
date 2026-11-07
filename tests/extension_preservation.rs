use oas::{builders, OpenAPIV3, Operation, Referenceable, Response, Responses};
use serde_json::json;

/// Regression: editing a typed summary used to drop operation and response extensions.
#[test]
fn typed_summary_edit_and_operation_deletion_preserve_extensions() {
    let input = json!({
        "openapi": "3.1.0",
        "info": {"title": "Example", "version": "1"},
        "x-document": {"source": "catalog"},
        "paths": {"/items": {
            "get": {
                "summary": "before",
                "x-audience": "internal",
                "x-routing": {"enabled": true, "weight": 1.5, "fallback": null},
                "responses": {"200": {
                    "description": "ok",
                    "x-owner": "catalog",
                    "x-labels": ["stable", 2, false, null],
                    "x-empty": {},
                    "content": {"application/json": {"schema": {
                        "type": ["string", "null"],
                        "x-schema": {"sensitive": false}
                    }}}
                }}
            },
            "post": {
                "summary": "create",
                "x-audience": "private",
                "responses": {"201": {"description": "created", "x-owner": "writer"}}
            }
        }},
        "components": {"schemas": {"Item": {
            "$ref": "other.json#/$defs/Item",
            "description": "Referenced item",
            "x-schema": [true, null]
        }}}
    });
    let mut doc: OpenAPIV3 = serde_json::from_value(input.clone()).unwrap();
    let mut expected = input;
    expected["paths"]["/items"]["get"]["summary"] = json!("after");
    expected["paths"]["/items"]
        .as_object_mut()
        .unwrap()
        .remove("post");

    let path = doc.paths.get_mut("/items").unwrap();
    path.get.as_mut().unwrap().summary = Some("after".into());
    path.post = None;

    let output = serde_json::to_string(&doc).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&output).unwrap(),
        expected,
        "must preserve all other JSON fields and remove POST, otherwise typed edits lose metadata or restore deleted operations"
    );
}

/// Guard: object-local extras must support replacement and deletion after typed readback.
#[test]
fn unmodeled_operation_and_response_fields_can_be_edited_and_removed() {
    let mut operation: Operation = serde_json::from_value(json!({
        "responses": {"default": {
            "description": "fallback",
            "x-owner": "legacy",
            "vendorMetadata": {"revision": 1}
        }},
        "x-audience": "internal",
        "vendorMetadata": false
    }))
    .unwrap();
    operation.extras.remove("x-audience");
    operation
        .extras
        .insert("vendorMetadata".into(), json!({"revision": 2}));
    let Referenceable::Data(response) = operation.responses.default.as_mut().unwrap() else {
        panic!("expected inline default response");
    };
    response.extras.insert("x-owner".into(), json!(null));

    assert_eq!(
        serde_json::to_value(operation).unwrap(),
        json!({
            "responses": {"default": {
                "description": "fallback",
                "x-owner": null,
                "vendorMetadata": {"revision": 1}
            }},
            "vendorMetadata": {"revision": 2}
        }),
        "must serialize current extras including removals, otherwise edits retain stale metadata"
    );
}

/// Compatibility: adding extras must not alter the existing compact constructor wire format.
#[test]
fn constructors_and_builders_do_not_emit_empty_extras() {
    let operation = Operation::new(
        Responses::new().with_status("200", Referenceable::data(Response::new("ok"))),
    );
    let built = builders::get("List items")
        .response("200", Referenceable::ok("ok"))
        .build();
    assert_eq!(
        serde_json::to_string(&operation).unwrap(),
        r#"{"responses":{"200":{"description":"ok"}}}"#,
        "must keep compact constructor bytes stable, otherwise empty extras change the wire format"
    );
    assert_eq!(
        serde_json::to_string(&built).unwrap(),
        r#"{"summary":"List items","responses":{"200":{"description":"ok"}}}"#,
        "must keep compact builder bytes stable, otherwise empty extras change the wire format"
    );
}

/// Regression: the sample-app document lost Components metadata during typed readback.
#[test]
fn components_metadata_survives_typed_readback_and_schema_deletion() {
    let input = json!({
        "openapi": "3.1.0",
        "info": {"title": "Sample", "version": "1"},
        "paths": {},
        "components": {
            "schemas": {"Retired": {"type": "string"}},
            "x-business-metadata": {
                "source": "repository-sample-app",
                "literal": {"$ref": "ordinary metadata"},
                "sequence": [2, 1]
            },
            "vendorMetadata": {"enabled": false, "value": null}
        }
    });
    assert_eq!(
        serde_json::to_value(oas::Components::new()).unwrap(),
        json!({})
    );
    assert_eq!(
        serde_json::to_value(oas::Components::default()).unwrap(),
        json!({})
    );
    let mut doc: OpenAPIV3 = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&doc).unwrap(),
        input,
        "must retain the complete input on readback, otherwise typed parsing silently discards component metadata"
    );
    doc.components
        .as_mut()
        .unwrap()
        .schemas
        .as_mut()
        .unwrap()
        .remove("Retired");
    let mut expected = input;
    expected["components"]["schemas"]
        .as_object_mut()
        .unwrap()
        .remove("Retired");
    assert_eq!(
        serde_json::to_value(doc).unwrap(),
        expected,
        "must retain container metadata while removing Retired, otherwise component edits lose metadata or restore deleted schemas"
    );
}
