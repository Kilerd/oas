use oas::{OpenAPIV3, Operation};
use serde_json::json;

/// Regression: gateway summary edits dropped metadata on nested retained objects.
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
    assert_eq!(
        serde_json::to_value(doc).unwrap(), expected,
        "must retain unrelated object metadata, otherwise a summary edit silently loses document fields"
    );
}

/// Regression: callback metadata failed parsing and Responses metadata was treated as responses.
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
    assert_eq!(
        serde_json::to_value(operation).unwrap(), expected,
        "must preserve metadata and remove typed entries, otherwise container edits lose data or restore deleted entries"
    );
}

/// Guard: retaining container metadata must not hide malformed values at typed keys.
#[test]
fn malformed_patterned_entries_are_not_hidden_as_metadata() {
    for input in [
        json!({"200": false}),
        json!({"2XX": {}}),
        json!({"default": 4}),
    ] {
        assert!(serde_json::from_value::<oas::Responses>(input).is_err(),
            "must reject malformed status responses, otherwise metadata retention hides invalid typed values");
    }
    for key in [
        "{$request.query.callback}",
        "https://example.com/{$request.body#/id}",
    ] {
        assert!(serde_json::from_value::<oas::Callback>(json!({key: false})).is_err(),
            "must reject malformed callback path items, otherwise expression entries become opaque metadata");
    }
}

/// Regression: reusable path items were opaque JSON and unavailable for typed edits.
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
        }),
        "must apply typed path-item edits without restoring removed entries, otherwise component assembly keeps stale paths"
    );
}

/// Regression: operationRef-only and documentation-only links failed on a required operationId.
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
    assert_eq!(
        serde_json::to_value(operation).unwrap(), expected,
        "must retain all Link forms, otherwise editing an operation discards links without operationId"
    );
}

/// Regression: nullable response examples and example values disappeared during typed readback.
#[test]
fn arbitrary_json_payloads_distinguish_null_from_absence_and_can_be_deleted() {
    let consumer_input = json!({"responses":{"200":{"description":"nullable body","content":{
        "application/json":{"schema":{"type":["string","null"]},"example":null,
            "examples":{"empty":{"value":null}}}
    }}}});
    let operation: Operation = serde_json::from_value(consumer_input.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(operation).unwrap(),
        consumer_input,
        "must preserve explicit null examples, otherwise nullable payload examples disappear"
    );

    for value in [
        None,
        Some(json!(null)),
        Some(json!(false)),
        Some(json!({"$ref": "literal", "items": [2, 1]})),
    ] {
        let mut input = json!({
            "parameters": [{"name": "q", "in": "query"}],
            "responses": {"200": {"description": "ok", "headers": {"Public": {}},
                "content": {"application/json": {"examples": {"empty": {}}}},
                "links": {"next": {"operationId": "next"}}}}
        });
        let absent = input.clone();
        if let Some(value) = value {
            input["parameters"][0]["example"] = value.clone();
            input["responses"]["200"]["headers"]["Public"]["example"] = value.clone();
            input["responses"]["200"]["content"]["application/json"]["example"] = value.clone();
            input["responses"]["200"]["content"]["application/json"]["examples"]["empty"]
                ["value"] = value.clone();
            input["responses"]["200"]["links"]["next"]["requestBody"] = value;
        }
        let mut operation: Operation = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(&operation).unwrap(), input,
            "must distinguish absent payloads from JSON values, otherwise typed readback changes example semantics");
        let oas::Referenceable::Data(parameter) = &mut operation.parameters.as_mut().unwrap()[0]
        else {
            panic!("expected inline parameter")
        };
        parameter.example = None;
        let oas::Referenceable::Data(response) = operation.responses.data.get_mut("200").unwrap()
        else {
            panic!("expected inline response")
        };
        let oas::Referenceable::Data(header) = response
            .headers
            .as_mut()
            .unwrap()
            .get_mut("Public")
            .unwrap()
        else {
            panic!("expected inline header")
        };
        header.example = None;
        let media = response
            .content
            .as_mut()
            .unwrap()
            .get_mut("application/json")
            .unwrap();
        media.example = None;
        let oas::Referenceable::Data(example) =
            media.examples.as_mut().unwrap().get_mut("empty").unwrap()
        else {
            panic!("expected inline example")
        };
        example.value = None;
        let oas::Referenceable::Data(link) =
            response.links.as_mut().unwrap().get_mut("next").unwrap()
        else {
            panic!("expected inline link")
        };
        link.request_body = None;
        assert_eq!(serde_json::to_value(operation).unwrap(), absent,
            "must omit payloads set to None, otherwise typed deletion leaves stale examples or link bodies");
    }
}
