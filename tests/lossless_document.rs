#![cfg(feature = "lossless-schema-keywords")]
use oas::lossless_component::{
    Components, Decode, Document, Field, Node, NodeMut, SchemaLookup, StructuralNode,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn json_fixtures(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            json_fixtures(&path, found);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            found.push(path);
        }
    }
}

// Reference Objects and schema roots, in traversal order. Schema contents are
// left to the schema visitor.
fn references(node: Node<'_>, found: &mut Vec<String>) {
    if let Some(Field::Value(reference)) = node.reference() {
        found.push(reference.clone());
    }
    if let Node::Schema(schema) = &node {
        if let Some(Value::String(reference)) = serde_json::to_value(schema).unwrap().get("$ref") {
            found.push(reference.clone());
        }
    }
    node.try_for_each_child(|_, child| {
        references(child, found);
        Ok::<_, ()>(())
    })
    .unwrap();
}

fn rewrite(node: NodeMut<'_>) {
    let mut node = node;
    if let Some(Field::Value(reference)) = node.reference_mut() {
        *reference = reference.replace("#/components/", "#/components/x-moved/");
    }
    node.try_for_each_child(|_, child| {
        rewrite(child);
        Ok::<_, ()>(())
    })
    .unwrap();
}

#[test]
fn every_json_fixture_round_trips_through_the_document_root() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut fixtures = Vec::new();
    json_fixtures(&root.join("examples"), &mut fixtures);
    json_fixtures(&root.join("openapi3-examples"), &mut fixtures);
    // Three swagger2openapi fixtures are empty files.
    fixtures.retain(|path| std::fs::metadata(path).unwrap().len() > 0);
    assert!(fixtures.len() >= 23, "{fixtures:?}");
    for path in fixtures {
        let source: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let document = Document::decode(source.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&document).unwrap(),
            source,
            "{}",
            path.display()
        );
        let mut visited = Vec::new();
        references(document.node(), &mut visited);
    }
}

fn sample() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {"title": "kept literally", "version": "1"},
        "jsonSchemaDialect": "https://spec.openapis.org/oas/3.1/dialect/base",
        "servers": [{"url": "https://api.test"}, null],
        "security": [{"key": []}],
        "tags": [{"name": "pets"}],
        "externalDocs": {"url": "https://docs.test"},
        "paths": {
            "/pets": {
                "get": {
                    "parameters": [{"$ref": "#/components/parameters/Limit"}],
                    "responses": {"200": {"$ref": "#/components/responses/Pets"}},
                    "callbacks": {"onEvent": {"$ref": "#/components/callbacks/Event"}}
                }
            },
            "/broken": 7,
            "x-path-extension": {"get": {"$ref": "literal"}}
        },
        "webhooks": {
            "newPet": {"$ref": "#/components/pathItems/NewPet"},
            "badHook": null
        },
        "components": {
            "schemas": {
                "Pet": {"type": "object", "properties": {"id": {"type": "integer"}}},
                "Alias": {"$ref": "#/components/schemas/Pet"},
                "Nope": 5
            },
            "responses": {
                "Pets": {"description": "ok", "content": {"application/json": {
                    "schema": {"$ref": "#/components/schemas/Pet"}
                }}}
            },
            "parameters": {"Limit": {"name": "limit", "in": "query", "schema": {"type": "integer"}}},
            "examples": {"One": {"value": {"$ref": "literal example"}}},
            "requestBodies": {"Pet": {"$ref": "#/components/requestBodies/Other"}},
            "headers": {"Rate": {"schema": {"type": "integer"}}},
            "securitySchemes": {"key": {"type": "apiKey", "name": "k", "in": "header"}},
            "links": {"Self": {"operationId": "listPets"}},
            "callbacks": {
                "Event": {"https://hooks.test/event": {"post": {
                    "requestBody": {"$ref": "#/components/requestBodies/Pet"}
                }}}
            },
            "pathItems": {"NewPet": {"post": {"responses": {"204": {"description": "done"}}}}},
            "x-group": {"Anything": {"$ref": "not a component"}},
            "unknownGroup": {"A": {}}
        },
        "x-root": true
    })
}

#[test]
fn document_and_components_keep_declared_invalid_and_unknown_fields() {
    let source = sample();
    let document = Document::decode(source.clone()).unwrap();
    assert_eq!(serde_json::to_value(&document).unwrap(), source);
    assert_eq!(document.openapi.as_value().unwrap(), "3.1.0");
    assert_eq!(
        document.extras.keys().collect::<Vec<_>>(),
        ["info", "tags", "x-root"]
    );
    let paths = document.paths.as_value().unwrap();
    assert!(matches!(paths.data["/broken"], Field::Invalid(_)));
    assert_eq!(
        paths.extras.keys().collect::<Vec<_>>(),
        ["x-path-extension"]
    );
    let components = document.components.as_value().unwrap();
    assert!(matches!(
        components.schemas.as_value().unwrap()["Nope"],
        Field::Invalid(_)
    ));
    assert_eq!(
        components.extras.keys().collect::<Vec<_>>(),
        ["unknownGroup", "x-group"]
    );

    for shape in [
        json!({"openapi": null, "paths": [], "webhooks": 1, "components": "x", "servers": {}}),
        json!({"openapi": "3.0.3", "components": {"schemas": null, "callbacks": []}}),
        json!({}),
    ] {
        let document = Document::decode(shape.clone()).unwrap();
        assert_eq!(serde_json::to_value(&document).unwrap(), shape);
        assert_eq!(
            serde_json::to_value(Components::decode(shape.clone()).unwrap()).unwrap(),
            shape
        );
    }
    assert_eq!(Document::decode(json!([1])).unwrap_err(), json!([1]));
    assert!(serde_json::from_value::<Field<Document>>(json!(null))
        .is_ok_and(|field| matches!(field, Field::Invalid(Value::Null))));
}

#[test]
fn root_traversal_reaches_every_structural_reference_and_skips_literals() {
    let document = Document::decode(sample()).unwrap();
    let mut children = Vec::new();
    document
        .node()
        .try_for_each_child(|key, child| {
            children.push((key.to_owned(), matches!(child, Node::Invalid(_))));
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(
        children,
        [
            ("components".to_owned(), false),
            ("paths".to_owned(), false),
            ("webhooks".to_owned(), true),
            ("webhooks".to_owned(), false),
        ]
    );

    let mut found = Vec::new();
    references(document.node(), &mut found);
    assert_eq!(
        found,
        [
            "#/components/requestBodies/Pet",
            "#/components/requestBodies/Other",
            "#/components/schemas/Pet",
            "#/components/schemas/Pet",
            "#/components/callbacks/Event",
            "#/components/parameters/Limit",
            "#/components/responses/Pets",
            "#/components/pathItems/NewPet",
        ]
    );

    let mut keys = Vec::new();
    let components = document.components.as_value().unwrap();
    components
        .node()
        .try_for_each_child(|key, _| {
            keys.push(key.to_owned());
            Ok::<_, ()>(())
        })
        .unwrap();
    keys.dedup();
    assert_eq!(
        keys,
        [
            "callbacks",
            "examples",
            "headers",
            "links",
            "parameters",
            "pathItems",
            "requestBodies",
            "responses",
            "schemas",
            "securitySchemes"
        ]
    );
}

#[test]
fn editable_root_rewrites_references_in_place() {
    let mut document = Document::decode(sample()).unwrap();
    rewrite(document.node_mut());
    let output = serde_json::to_value(&document).unwrap();
    assert_eq!(
        output["paths"]["/pets"]["get"]["parameters"][0]["$ref"],
        "#/components/x-moved/parameters/Limit"
    );
    assert_eq!(
        output["webhooks"]["newPet"]["$ref"],
        "#/components/x-moved/pathItems/NewPet"
    );
    assert_eq!(
        output["components"]["callbacks"]["Event"]["https://hooks.test/event"]["post"]
            ["requestBody"]["$ref"],
        "#/components/x-moved/requestBodies/Pet"
    );
    assert_eq!(
        output["paths"]["x-path-extension"]["get"]["$ref"],
        "literal"
    );
    assert_eq!(
        output["components"]["x-group"]["Anything"]["$ref"],
        "not a component"
    );
}

#[test]
fn schema_lookup_starts_at_the_document_root() {
    let document = Document::decode(sample()).unwrap();
    let id = document
        .schema_at_tokens(&["components", "schemas", "Pet", "properties", "id"])
        .unwrap();
    assert_eq!(
        serde_json::to_value(id).unwrap(),
        json!({"type": "integer"})
    );
    assert!(document
        .schema_at_tokens(&[
            "components",
            "responses",
            "Pets",
            "content",
            "application/json",
            "schema"
        ])
        .is_some());
    assert!(document
        .schema_at_tokens(&["components", "schemas", "Nope"])
        .is_none());
    assert!(document.schema_at_tokens(&["paths", "/pets"]).is_none());
}
