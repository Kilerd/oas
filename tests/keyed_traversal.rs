#![cfg(feature = "lossless-schema-keywords")]
//! Keyed child visitors: every step builds the RFC 6901 pointer of its child.
use oas::json_pointer::Step;
use oas::lossless_asyncapi as asyncapi;
use oas::lossless_component::{Decode, Document, Node, NodeMut, StructuralNode};
use oas::SchemaValue;
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
            && std::fs::metadata(&path).unwrap().len() > 0
        {
            found.push(path);
        }
    }
}

fn openapi_fixtures() -> Vec<(PathBuf, Value)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    json_fixtures(&root.join("examples"), &mut paths);
    json_fixtures(&root.join("openapi3-examples"), &mut paths);
    assert!(paths.len() >= 23, "{paths:?}");
    paths
        .into_iter()
        .map(|path| {
            let value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            (path, value)
        })
        .collect()
}

fn asyncapi_fixtures() -> [Value; 2] {
    [
        serde_json::from_str(include_str!("fixtures/asyncapi/chat.json")).unwrap(),
        serde_json::from_str(include_str!("fixtures/asyncapi/deferred-shapes.json")).unwrap(),
    ]
}

fn to_value(value: &impl serde::Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}

fn node_value(node: &Node<'_>) -> Value {
    match node {
        Node::Schema(value) => to_value(value),
        Node::Response(value) => to_value(value),
        Node::Parameter(value) => to_value(value),
        Node::Example(value) => to_value(value),
        Node::RequestBody(value) => to_value(value),
        Node::Header(value) => to_value(value),
        Node::SecurityScheme(value) => to_value(value),
        Node::Link(value) => to_value(value),
        Node::Callback(value) => to_value(value),
        Node::PathItem(value) => to_value(value),
        Node::Operation(value) => to_value(value),
        Node::Responses(value) => to_value(value),
        Node::MediaType(value) => to_value(value),
        Node::Encoding(value) => to_value(value),
        Node::Document(value) => to_value(value),
        Node::Components(value) => to_value(value),
        Node::Paths(value) => to_value(value),
        Node::Invalid(value) => to_value(value),
    }
}

/// Schema descendants with their pointers, the schema itself included.
fn schema_pointers(schema: &SchemaValue, at: &str, found: &mut Vec<(String, Value)>) {
    found.push((at.to_owned(), to_value(schema)));
    if let Some(object) = schema.as_object() {
        let mut names = Vec::new();
        object
            .try_for_each_child_with_keyword(|keyword, _| {
                names.push(keyword);
                Ok::<_, ()>(())
            })
            .unwrap();
        let mut steps = Vec::new();
        object
            .try_for_each_child_at(|step, child| {
                steps.push(step.name().to_owned());
                schema_pointers(child, &format!("{at}{step}"), found);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(steps, names, "keyed schema children at {at}");
    }
}

/// Every structural node and schema below `node`, with its pointer.
fn openapi_pointers(node: Node<'_>, at: &str, found: &mut Vec<(String, Value)>) {
    if let Node::Schema(schema) = &node {
        return schema_pointers(schema, at, found);
    }
    found.push((at.to_owned(), node_value(&node)));
    let mut names = Vec::new();
    node.try_for_each_child(|name, _| {
        names.push(name.to_owned());
        Ok::<_, ()>(())
    })
    .unwrap();
    let mut steps = Vec::new();
    node.try_for_each_child_at(|step, child| {
        steps.push(step.name().to_owned());
        openapi_pointers(child, &format!("{at}{step}"), found);
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(steps, names, "keyed children at {at}");
}

fn asyncapi_pointers(node: asyncapi::Node<'_>, at: &str, found: &mut Vec<(String, Value)>) {
    if let asyncapi::Node::Schema(schema) = &node {
        return schema_pointers(schema, at, found);
    }
    found.push((at.to_owned(), to_value(&node)));
    let mut names = Vec::new();
    node.try_for_each_child(|name, _| {
        names.push(name.to_owned());
        Ok::<_, ()>(())
    })
    .unwrap();
    let mut steps = Vec::new();
    node.try_for_each_child_at(|step, child| {
        steps.push(step.name().to_owned());
        asyncapi_pointers(child, &format!("{at}{step}"), found);
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(steps, names, "keyed children at {at}");
}

fn assert_pointers_resolve(source: &Value, found: &[(String, Value)], label: &str) {
    for (pointer, value) in found {
        assert_eq!(
            source.pointer(pointer),
            Some(value),
            "{label}: {pointer} must resolve to the visited node"
        );
    }
}

#[test]
fn every_openapi_fixture_node_resolves_at_its_keyed_pointer() {
    let mut total = 0;
    for (path, source) in openapi_fixtures() {
        let document = Document::decode(source.clone()).unwrap();
        assert_eq!(to_value(&document), source, "{}", path.display());
        let mut found = Vec::new();
        openapi_pointers(document.node(), "", &mut found);
        assert_pointers_resolve(&source, &found, &path.display().to_string());
        total += found.len();
    }
    assert!(
        total > 1000,
        "fixtures must reach nested nodes, got {total}"
    );
}

#[test]
fn every_asyncapi_fixture_node_resolves_at_its_keyed_pointer() {
    for source in asyncapi_fixtures() {
        let document = asyncapi::Document::decode(source.clone()).unwrap();
        assert_eq!(to_value(&document), source);
        let mut found = Vec::new();
        asyncapi::StructuralNode::node(&document)
            .try_for_each_child_at(|step, child| {
                asyncapi_pointers(child, &step.to_string(), &mut found);
                Ok::<_, ()>(())
            })
            .unwrap();
        assert!(found.len() > 20, "{}", found.len());
        assert_pointers_resolve(&source, &found, "asyncapi");
    }
}

fn escaped_sample() -> Value {
    json!({
        "openapi": "3.1.0",
        "paths": {
            "/a~b/{c}": {
                "get": {
                    "parameters": [
                        {"name": "c", "in": "path", "schema": {"type": "string"}},
                        {"$ref": "#/components/parameters/q~1x"}
                    ],
                    "responses": {"200": {"description": "ok", "content": {
                        "application/json": {"schema": {
                            "properties": {"a/b": {"items": [true, {"type": "integer"}]}, "~": false},
                            "allOf": [{"$ref": "#/components/schemas/x~1y~0z"}]
                        }}
                    }}}
                }
            }
        },
        "components": {
            "schemas": {"x/y~z": {"type": "object"}},
            "parameters": {"q/x": {"name": "x", "in": "query"}},
            "headers": 5,
            "responses": {"bad": 7}
        }
    })
}

#[test]
fn keyed_steps_escape_keys_and_index_elements() {
    let source = escaped_sample();
    let document = Document::decode(source.clone()).unwrap();
    let mut found = Vec::new();
    openapi_pointers(document.node(), "", &mut found);
    assert_pointers_resolve(&source, &found, "sample");
    let pointers: Vec<&str> = found.iter().map(|(pointer, _)| pointer.as_str()).collect();
    for expected in [
        "/components/headers",
        "/components/parameters/q~1x",
        "/components/responses/bad",
        "/components/schemas/x~1y~0z",
        "/paths/~1a~0b~1{c}/get/parameters/0",
        "/paths/~1a~0b~1{c}/get/parameters/0/schema",
        "/paths/~1a~0b~1{c}/get/parameters/1",
        "/paths/~1a~0b~1{c}/get/responses/200/content/application~1json/schema/allOf/0",
        "/paths/~1a~0b~1{c}/get/responses/200/content/application~1json/schema/properties/a~1b/items/1",
        "/paths/~1a~0b~1{c}/get/responses/200/content/application~1json/schema/properties/~0",
    ] {
        assert!(pointers.contains(&expected), "{expected} in {pointers:#?}");
    }
    // A malformed collection is one invalid child at the field; a malformed entry has its key.
    let components = &found
        .iter()
        .find(|(pointer, _)| pointer == "/components")
        .unwrap()
        .1;
    assert_eq!(components["headers"], json!(5));
    let mut invalid = Vec::new();
    document
        .components
        .as_value()
        .unwrap()
        .node()
        .try_for_each_child_at(|step, child| {
            if matches!(child, Node::Invalid(_)) {
                invalid.push(step.to_string());
            }
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(invalid, ["/headers", "/responses/bad"]);
}

#[test]
fn steps_decode_back_to_their_tokens() {
    let steps = [
        Step::Token("~/"),
        Step::Entry {
            field: "properties",
            key: "a/b~c",
        },
        Step::Element {
            field: "allOf",
            index: 12,
        },
    ];
    let mut tokens = Vec::new();
    let mut pointer = String::new();
    for step in steps {
        let before = tokens.len();
        assert_eq!(step.push_tokens(&mut tokens), step.token_count());
        assert_eq!(tokens.len() - before, step.token_count());
        pointer.push_str(&step.to_string());
    }
    assert_eq!(pointer, "/~0~1/properties/a~1b~0c/allOf/12");
    assert_eq!(oas::json_pointer::tokens(&pointer).unwrap(), tokens);
    assert_eq!(steps.map(|step| step.name()), ["~/", "properties", "allOf"]);
}

/// Rewrite every reference through the editable visitor, recording where.
fn rewrite_openapi(node: NodeMut<'_>, at: &str, found: &mut Vec<String>) {
    let mut node = node;
    if let Some(oas::lossless_component::Field::Value(reference)) = node.reference_mut() {
        found.push(format!("{at}/$ref"));
        reference.push_str("-moved");
    }
    if let NodeMut::Schema(schema) = &mut node {
        return rewrite_schema(schema, at, found);
    }
    node.try_for_each_child_at(|step, child| {
        rewrite_openapi(child, &format!("{at}{step}"), found);
        Ok::<_, ()>(())
    })
    .unwrap();
}

fn rewrite_schema(schema: &mut SchemaValue, at: &str, found: &mut Vec<String>) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    if let Some(reference) = &mut object._ref {
        found.push(format!("{at}/$ref"));
        reference.push_str("-moved");
    }
    object
        .try_for_each_child_mut_at(|step, child| {
            rewrite_schema(child, &format!("{at}{step}"), found);
            Ok::<_, ()>(())
        })
        .unwrap();
}

#[test]
fn editable_keyed_visitors_locate_the_edits_they_make() {
    let source = escaped_sample();
    let mut document = Document::decode(source.clone()).unwrap();
    let mut found = Vec::new();
    rewrite_openapi(document.node_mut(), "", &mut found);
    assert_eq!(
        found,
        [
            "/paths/~1a~0b~1{c}/get/parameters/1/$ref",
            "/paths/~1a~0b~1{c}/get/responses/200/content/application~1json/schema/allOf/0/$ref",
        ]
    );
    let edited = to_value(&document);
    for pointer in &found {
        let before = source.pointer(pointer).unwrap().as_str().unwrap();
        assert_eq!(
            edited.pointer(pointer).unwrap(),
            &json!(format!("{before}-moved"))
        );
    }
}

#[test]
fn editable_asyncapi_visitor_reports_the_same_steps() {
    for source in asyncapi_fixtures() {
        let mut document = asyncapi::Document::decode(source.clone()).unwrap();
        let mut borrowed = Vec::new();
        asyncapi::StructuralNode::node(&document)
            .try_for_each_child_at(|step, _| {
                borrowed.push(step.to_string());
                Ok::<_, ()>(())
            })
            .unwrap();
        let mut editable = Vec::new();
        asyncapi::StructuralNode::node_mut(&mut document)
            .try_for_each_child_at(|step, _| {
                editable.push(step.to_string());
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(borrowed, editable);
        assert!(borrowed.iter().any(|step| step.starts_with("/channels/")));
    }
}
