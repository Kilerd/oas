use oas::{OpenAPIV3, Operation};
use serde_json::json;

#[test]
fn retained_objects_keep_metadata_after_typed_summary_edit() {
    let input = json!({
        "openapi": "3.1.0",
        "info": {"title": "service", "version": "1", "x-info": [2, 1],
            "contact": {"x-contact": null}, "license": {"name": "MIT", "x-license": true}},
        "tags": [{"name": "public", "x-tag": false}],
        "paths": {"/a": {"x-path": {"$ref": "literal"}, "get": {
            "summary": "before", "externalDocs": {"url": "https://example.com", "x-doc": 1},
            "servers": [{"url": "https://{host}", "x-server": [2, 1],
                "variables": {"host": {"default": "example.com", "x-variable": null}}}],
            "parameters": [{"in": "query", "name": "q", "x-parameter": {"$ref": "literal"},
                "schema": {"type": "string"}, "examples": {"sample": {
                    "value": {"$ref": "example literal"}, "x-example": 4}}}],
            "requestBody": {"x-body": {"keep": [2, 1]}, "content": {"application/json": {
                "schema": true, "encoding": {"field": {"x-encoding": false}}
            }}},
            "responses": {"200": {"description": "ok", "headers": {"Public": {
                "schema": true, "x-header": 7}}, "content": {"application/json": {
                "schema": {"type": "string"}, "x-media": {"example": [2, 1]}}},
                "links": {"next": {"operationId": "next", "x-link": null}}
            }}
        }}}
    });
    let mut doc: OpenAPIV3 = serde_json::from_value(input.clone()).unwrap();
    doc.paths
        .get_mut("/a")
        .unwrap()
        .get
        .as_mut()
        .unwrap()
        .summary = Some("after".into());
    let mut expected = input;
    expected["paths"]["/a"]["get"]["summary"] = json!("after");
    assert_eq!(serde_json::to_value(doc).unwrap(), expected);
}

#[test]
fn patterned_containers_keep_metadata_when_typed_entries_are_removed() {
    let input = json!({
        "responses": {
            "default": {"description": "fallback"},
            "200": {"description": "ok"},
            "2XX": {"description": "success"},
            "x-public": {"$ref": "literal", "sequence": [2, 1]},
            "description": "response metadata", "vendor": false
        },
        "callbacks": {"event": {
            "summary": "callback metadata", "description": "keep",
            "x-public": {"$ref": "literal"}, "x-{$ignored}": null,
            "{$request.query.callback}": {"post": {"responses": {"200": {"description": "ok"}}}},
            "https://example.com/cb?id={$request.body#/id}": {"x-path": true}
        }}
    });
    let mut operation: Operation = serde_json::from_value(input.clone()).unwrap();
    operation.responses.data.remove("200");
    operation.responses.default = None;
    let callback = operation
        .callbacks
        .as_mut()
        .unwrap()
        .get_mut("event")
        .unwrap();
    let oas::Referenceable::Data(callback) = callback else {
        panic!("expected inline callback")
    };
    callback.data.remove("{$request.query.callback}");
    let mut expected = input;
    expected["responses"].as_object_mut().unwrap().remove("200");
    expected["responses"]
        .as_object_mut()
        .unwrap()
        .remove("default");
    expected["callbacks"]["event"]
        .as_object_mut()
        .unwrap()
        .remove("{$request.query.callback}");
    assert_eq!(serde_json::to_value(operation).unwrap(), expected);
}

#[test]
fn malformed_patterned_entries_are_not_hidden_as_metadata() {
    for input in [
        json!({"200": false}),
        json!({"2XX": {}}),
        json!({"default": 4}),
    ] {
        assert!(serde_json::from_value::<oas::Responses>(input).is_err());
    }
    for key in [
        "{$request.query.callback}",
        "https://example.com/{$request.body#/id}",
    ] {
        assert!(serde_json::from_value::<oas::Callback>(json!({key: false})).is_err());
    }
}

#[test]
fn reusable_path_items_can_be_edited_and_deleted_without_metadata_backfill() {
    let input = json!({"pathItems": {
        "Public": {"$ref": "#/paths/~1a", "x-path": {"$ref": "literal"}},
        "Unused": {"summary": "remove"}
    }, "x-components": [2, 1]});
    let mut components: oas::Components = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(serde_json::to_value(&components).unwrap(), input);
    let paths = components.path_items.as_mut().unwrap();
    paths.remove("Unused");
    paths.get_mut("Public").unwrap().summary = Some("public".into());
    assert_eq!(
        serde_json::to_value(components).unwrap(),
        json!({
            "pathItems": {"Public": {"$ref": "#/paths/~1a", "summary": "public",
                "x-path": {"$ref": "literal"}}}, "x-components": [2, 1]
        })
    );
}

#[test]
fn links_without_operation_id_survive_typed_operation_edits() {
    let input = json!({"responses": {"200": {"description": "ok", "links": {
        "byRef": {"operationRef": "#/paths/~1items/get", "x-link": [2, 1]},
        "documentation": {"description": "documentation-only link"},
        "byId": {"operationId": "listItems"}
    }}}});
    let mut operation: Operation = serde_json::from_value(input.clone()).unwrap();
    operation.summary = Some("edited".into());
    let mut expected = input;
    expected["summary"] = json!("edited");
    assert_eq!(serde_json::to_value(operation).unwrap(), expected);
}
