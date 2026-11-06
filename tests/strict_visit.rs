use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use oas::visit::{
    ComponentKind, ComponentRef, ComponentValue, DirectRef, RefError, RefSite, Visit, VisitMut,
};
use oas::OpenAPIV3;
use serde_json::{json, Value};

#[derive(Default)]
struct Refs(Vec<(ComponentKind, String, bool)>);

impl Visit for Refs {
    fn reference(&mut self, site: RefSite<'_>) {
        self.0
            .push((site.kind, site.target.to_owned(), site.dynamic));
    }
}

fn references(doc: &OpenAPIV3) -> Vec<(ComponentKind, String, bool)> {
    let mut refs = Refs::default();
    doc.walk(&mut refs);
    refs.0
}

#[derive(Default)]
struct Extensions(BTreeSet<String>);

impl Visit for Extensions {
    fn extensions(&mut self, extras: &BTreeMap<String, serde_json::Value>) {
        self.0.extend(extras.keys().cloned());
    }
}

#[derive(Default)]
struct Schemes(Vec<String>);

impl Visit for Schemes {
    fn security_requirement(&mut self, requirement: &oas::SecurityRequirement) {
        self.0.extend(requirement.data.keys().cloned());
    }
}

fn positions_document() -> OpenAPIV3 {
    serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1", "contact": {"name": "n", "x-contact": true},
                 "license": {"name": "MIT", "x-license": true}, "x-info": true},
        "servers": [{"url": "https://e.test", "x-server": true}],
        "security": [{"S": []}],
        "tags": [{"name": "t", "externalDocs": {"url": "https://e.test", "x-tag": true}}],
        "paths": {"/x": {
            "parameters": [{"$ref": "#/components/parameters/P"}],
            "get": {
                "tags": ["t"],
                "externalDocs": {"url": "https://e.test", "x-doc": true},
                "parameters": [{"$ref": "#/components/parameters/P"}],
                "requestBody": {"$ref": "#/components/requestBodies/B"},
                "responses": {
                    "200": {"$ref": "#/components/responses/Ok"},
                    "default": {"$ref": "#/components/responses/Ref"}
                },
                "callbacks": {"cb": {"$ref": "#/components/callbacks/CB"}},
                "security": [{"S": []}],
                "servers": [{"url": "https://e.test",
                             "variables": {"v": {"default": "a", "x-var": true}}}],
                "x-op": {"$ref": "#/components/schemas/DataOnly"}
            },
            "put": {"responses": {"204": {"description": "done"}}}
        }},
        "components": {
            "schemas": {
                "A": {"$ref": "#/components/schemas/B"},
                "B": {"type": "object", "discriminator": {
                    "propertyName": "t",
                    "mapping": {"a": "#/components/schemas/A", "bare": "NotAPointer"}
                }},
                "Dyn": {"$dynamicRef": "#meta"},
                "Data": {
                    "default": {"$ref": "#/components/schemas/DataOnly"},
                    "enum": [{"$ref": "#/components/schemas/DataOnly"}],
                    "const": {"$ref": "#/components/schemas/DataOnly"},
                    "examples": [{"$ref": "#/components/schemas/DataOnly"}],
                    "x-note": {"$ref": "#/components/schemas/DataOnly"}
                }
            },
            "responses": {
                "Ref": {"$ref": "#/components/responses/Ok", "x-ref": 1},
                "Ok": {
                    "description": "ok",
                    "headers": {"H": {"$ref": "#/components/headers/Header"}},
                    "content": {"application/json": {
                        "schema": {"$ref": "#/components/schemas/A"},
                        "examples": {"e": {"$ref": "#/components/examples/E"}}
                    }},
                    "links": {"L": {"$ref": "#/components/links/Link"}}
                }
            },
            "parameters": {
                "Pref": {"$ref": "#/components/parameters/P"},
                "P": {"name": "q", "in": "query",
                      "schema": {"$ref": "#/components/schemas/B"},
                      "examples": {"E": {"$ref": "#/components/examples/E"}},
                      "content": {"text/plain": {"schema": {"$ref": "#/components/schemas/A"}}}}
            },
            "examples": {
                "ERef": {"$ref": "#/components/examples/E"},
                "E": {"value": {"$ref": "#/components/schemas/DataOnly"}}
            },
            "requestBodies": {
                "BRef": {"$ref": "#/components/requestBodies/B"},
                "B": {"content": {"application/json": {"schema": {"$ref": "#/components/schemas/A"}}}}
            },
            "headers": {
                "HRef": {"$ref": "#/components/headers/Header"},
                "Header": {"schema": {"$ref": "#/components/schemas/B"}}
            },
            "securitySchemes": {
                "SRef": {"$ref": "#/components/securitySchemes/S"},
                "S": {"type": "http", "scheme": "bearer", "x-scheme": true}
            },
            "links": {
                "LRef": {"$ref": "#/components/links/Link"},
                "Link": {"operationId": "x"}
            },
            "callbacks": {
                "CRef": {"$ref": "#/components/callbacks/CB"},
                "CB": {"{$request.body#/url}": {"$ref": "#/components/pathItems/PI"}}
            },
            "pathItems": {
                "PIRef": {"$ref": "#/components/pathItems/PI"},
                "PI": {"get": {"responses": {"200": {"$ref": "#/components/responses/Ok"}}}}
            }
        }
    }))
    .expect("the positions document decodes")
}

#[test]
fn every_reference_position_reports_its_kind() {
    let doc = positions_document();
    let found: BTreeSet<(ComponentKind, String, bool)> = references(&doc).into_iter().collect();
    let expected: BTreeSet<(ComponentKind, String, bool)> = [
        (ComponentKind::Schemas, "#/components/schemas/A", false),
        (ComponentKind::Schemas, "#/components/schemas/B", false),
        (ComponentKind::Schemas, "#meta", true),
        (ComponentKind::Schemas, "NotAPointer", false),
        (ComponentKind::Responses, "#/components/responses/Ok", false),
        (
            ComponentKind::Responses,
            "#/components/responses/Ref",
            false,
        ),
        (ComponentKind::Headers, "#/components/headers/Header", false),
        (ComponentKind::Examples, "#/components/examples/E", false),
        (ComponentKind::Links, "#/components/links/Link", false),
        (
            ComponentKind::Parameters,
            "#/components/parameters/P",
            false,
        ),
        (
            ComponentKind::RequestBodies,
            "#/components/requestBodies/B",
            false,
        ),
        (
            ComponentKind::SecuritySchemes,
            "#/components/securitySchemes/S",
            false,
        ),
        (ComponentKind::Callbacks, "#/components/callbacks/CB", false),
        (ComponentKind::PathItems, "#/components/pathItems/PI", false),
    ]
    .into_iter()
    .map(|(kind, target, dynamic)| (kind, target.to_string(), dynamic))
    .collect();
    assert_eq!(
        found, expected,
        "every reference position must report the kind of the position"
    );
    assert!(
        !found
            .iter()
            .any(|(_, target, _)| target == "#/components/schemas/DataOnly"),
        "$ref-shaped data in values, defaults, enums, consts or extensions must not be a reference"
    );
}

#[test]
fn extensions_and_security_requirement_keys_are_visited() {
    let doc = positions_document();
    let mut extensions = Extensions::default();
    let mut schemes = Schemes::default();
    doc.walk(&mut extensions);
    doc.walk(&mut schemes);
    for key in [
        "x-info",
        "x-contact",
        "x-license",
        "x-server",
        "x-tag",
        "x-var",
        "x-doc",
        "x-op",
        "x-note",
        "x-ref",
        "x-scheme",
    ] {
        assert!(
            extensions.0.contains(key),
            "extension {key} was not visited"
        );
    }
    assert_eq!(schemes.0, ["S".to_string(), "S".to_string()]);
}

#[test]
fn ref_shaped_example_values_are_not_references() {
    let doc: OpenAPIV3 = serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1"},
        "paths": {},
        "components": {
            "examples": {"E": {"value": {"$ref": "#/components/schemas/A"}}},
            "schemas": {"A": {"default": {"$ref": "#/components/schemas/B"},
                              "x-ext": {"$ref": "#/components/schemas/B"}}}
        }
    }))
    .unwrap();
    assert!(
        references(&doc).is_empty(),
        "only typed reference positions and discriminator mappings hold references"
    );
}

#[test]
fn graph_closes_cycles_propagates_broken_and_reverses() {
    let mut schemas = serde_json::Map::new();
    for (name, target) in [
        ("A", "#/components/schemas/B"),
        ("B", "#/components/schemas/C"),
        ("C", "#/components/schemas/A"),
        ("D", "#/components/schemas/Missing"),
        ("E", "#/components/schemas/D"),
        ("F", "#/components/schemas/E"),
        ("I", "#meta"),
    ] {
        let value = if name == "I" {
            json!({"$dynamicRef": target})
        } else {
            json!({"$ref": target})
        };
        schemas.insert(name.into(), value);
    }
    schemas.insert("G".into(), json!({"$ref": "#/components/schemas/A"}));
    let doc: OpenAPIV3 = serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1"},
        "paths": {},
        "components": {
            "schemas": schemas,
            "responses": {
                "H": {"$ref": "#/components/schemas/A"},
                "J": {"$ref": "other.json#/components/responses/X"},
                "K": {"$ref": "#/paths/~1x"}
            }
        }
    }))
    .unwrap();
    let graph = doc.ref_graph();
    let key = |kind, name: &str| ComponentRef::new(kind, name);

    let missing = RefError::Missing(key(ComponentKind::Schemas, "Missing"));
    assert_eq!(graph.broken()[&key(ComponentKind::Schemas, "D")], missing);
    assert_eq!(graph.broken()[&key(ComponentKind::Schemas, "E")], missing);
    assert_eq!(graph.broken()[&key(ComponentKind::Schemas, "F")], missing);
    assert_eq!(
        graph.broken()[&key(ComponentKind::Responses, "H")],
        RefError::WrongKind {
            expected: ComponentKind::Responses,
            found: ComponentKind::Schemas
        }
    );
    assert_eq!(
        graph.broken()[&key(ComponentKind::Schemas, "I")],
        RefError::Dynamic
    );
    assert_eq!(
        graph.broken()[&key(ComponentKind::Responses, "J")],
        RefError::External
    );
    assert_eq!(
        graph.broken()[&key(ComponentKind::Responses, "K")],
        RefError::OutsideComponents
    );

    let cycle: BTreeSet<_> = ["A", "B", "C"]
        .into_iter()
        .map(|name| key(ComponentKind::Schemas, name))
        .collect();
    assert_eq!(
        graph.closure([key(ComponentKind::Schemas, "A")]).unwrap(),
        cycle,
        "cycles terminate and every reachable component is returned once"
    );
    assert_eq!(
        graph.closure([key(ComponentKind::Schemas, "F")]),
        Err(missing.clone()),
        "a reference to a broken component fails with that component's error"
    );

    let within: BTreeSet<_> = ["A", "B", "C", "D", "E", "F"]
        .into_iter()
        .map(|name| key(ComponentKind::Schemas, name))
        .collect();
    let dependents = graph.dependents(&BTreeSet::from([key(ComponentKind::Schemas, "D")]), &within);
    assert_eq!(
        dependents,
        ["D", "E", "F"]
            .into_iter()
            .map(|name| key(ComponentKind::Schemas, name))
            .collect()
    );

    assert_eq!(
        graph
            .roots([DirectRef {
                kind: ComponentKind::Schemas,
                target: "#/components/schemas/A".into(),
                dynamic: false,
            }])
            .unwrap(),
        BTreeSet::from([key(ComponentKind::Schemas, "A")])
    );
    assert_eq!(
        graph.roots([DirectRef {
            kind: ComponentKind::Responses,
            target: "#/components/schemas/A".into(),
            dynamic: false,
        }]),
        Err(RefError::WrongKind {
            expected: ComponentKind::Responses,
            found: ComponentKind::Schemas
        })
    );
    assert_eq!(
        graph.roots([DirectRef {
            kind: ComponentKind::Schemas,
            target: "#/components/schemas/Nope".into(),
            dynamic: false,
        }]),
        Err(RefError::Missing(key(ComponentKind::Schemas, "Nope")))
    );
    assert_eq!(
        graph.roots([DirectRef {
            kind: ComponentKind::Schemas,
            target: "#meta".into(),
            dynamic: true,
        }]),
        Err(RefError::Dynamic)
    );
}

#[test]
fn operation_refs_are_the_roots_of_its_own_references() {
    let doc: OpenAPIV3 = serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1"},
        "paths": {"/x": {"get": {
            "parameters": [{"$ref": "#/components/parameters/P"}],
            "responses": {"200": {"$ref": "#/components/responses/Ok"}}
        }}},
        "components": {
            "parameters": {"P": {"name": "q", "in": "query"}},
            "responses": {"Ok": {"description": "ok"}}
        }
    }))
    .unwrap();
    let operation = doc.paths["/x"].get.as_ref().unwrap();
    let refs = operation.refs();
    assert_eq!(
        refs,
        [
            DirectRef {
                kind: ComponentKind::Parameters,
                target: "#/components/parameters/P".into(),
                dynamic: false,
            },
            DirectRef {
                kind: ComponentKind::Responses,
                target: "#/components/responses/Ok".into(),
                dynamic: false,
            }
        ]
    );
    let graph = doc.ref_graph();
    assert_eq!(graph.roots(refs).unwrap().len(), 2);
}

#[test]
fn rename_refs_covers_pointer_positions_and_security_keys() {
    let mut doc: OpenAPIV3 = serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1"},
        "security": [{"Bearer": []}],
        "paths": {"/x": {"get": {
            "security": [{"Bearer": ["read"]}],
            "parameters": [{"name": "q", "in": "query",
                            "schema": {"$ref": "#/components/schemas/A"}}],
            "responses": {"200": {"$ref": "#/components/responses/Ok"}}
        }}},
        "components": {
            "schemas": {
                "A": {"type": "object", "discriminator": {
                    "propertyName": "t",
                    "mapping": {"a": "#/components/schemas/A"}
                }},
                "Dyn": {"$dynamicRef": "#/components/schemas/A"}
            },
            "responses": {"Ok": {"description": "ok",
                "content": {"application/json": {"schema": {"$ref": "#/components/schemas/A"}}}}},
            "securitySchemes": {"Bearer": {"type": "http", "scheme": "bearer"}}
        }
    }))
    .unwrap();
    let renames = BTreeMap::from([
        (
            ComponentRef::new(ComponentKind::Schemas, "A"),
            "Renamed".to_string(),
        ),
        (
            ComponentRef::new(ComponentKind::Responses, "Ok"),
            "Success".to_string(),
        ),
        (
            ComponentRef::new(ComponentKind::SecuritySchemes, "Bearer"),
            "Auth".to_string(),
        ),
    ]);
    doc.rename_refs(&renames);
    let value = serde_json::to_value(&doc).unwrap();
    let text = value.to_string();
    assert_eq!(
        text.matches("#/components/schemas/A\"").count(),
        1,
        "only the dynamic reference keeps the old target: {text}"
    );
    assert!(text.contains("#/components/schemas/Renamed"), "{text}");
    assert!(text.contains("#/components/responses/Success"), "{text}");
    assert_eq!(value["security"][0]["Auth"], json!([]));
    assert_eq!(
        value["paths"]["/x"]["get"]["security"][0]["Auth"],
        json!(["read"])
    );
    assert_eq!(
        value["components"]["schemas"]["A"]["discriminator"]["mapping"]["a"],
        "#/components/schemas/Renamed"
    );
    assert_eq!(
        value["components"]["schemas"]["Dyn"]["$dynamicRef"], "#/components/schemas/A",
        "dynamic references are never rewritten"
    );
}

#[test]
fn components_rename_and_copy_rewrite_inside_the_copied_components() {
    let source: oas::Components = serde_json::from_value(json!({
        "schemas": {"A": {"type": "string"}},
        "responses": {"R": {"description": "ok",
            "headers": {"H": {"$ref": "#/components/headers/X"}},
            "content": {"application/json": {"schema": {"$ref": "#/components/schemas/A"}}}}},
        "headers": {"X": {"schema": {"$ref": "#/components/schemas/A"}}}
    }))
    .unwrap();
    let refs = [
        ComponentRef::new(ComponentKind::Schemas, "A"),
        ComponentRef::new(ComponentKind::Responses, "R"),
        ComponentRef::new(ComponentKind::Headers, "X"),
    ];
    let renames = BTreeMap::from([
        (
            ComponentRef::new(ComponentKind::Schemas, "A"),
            "A2".to_string(),
        ),
        (
            ComponentRef::new(ComponentKind::Responses, "R"),
            "R2".to_string(),
        ),
        (
            ComponentRef::new(ComponentKind::Headers, "X"),
            "X2".to_string(),
        ),
    ]);
    let mut target = oas::Components::default();
    let copied = target.copy_from(&source, refs.clone(), &renames).unwrap();
    assert_eq!(
        copied,
        BTreeSet::from([
            ComponentRef::new(ComponentKind::Schemas, "A2"),
            ComponentRef::new(ComponentKind::Responses, "R2"),
            ComponentRef::new(ComponentKind::Headers, "X2"),
        ])
    );
    assert_eq!(
        target.get(&ComponentRef::new(ComponentKind::Schemas, "A2")),
        source.get(&ComponentRef::new(ComponentKind::Schemas, "A")),
        "Components::get values compare by JSON"
    );
    let value = serde_json::to_value(&target).unwrap();
    assert_eq!(
        value["responses"]["R2"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/A2"
    );
    assert_eq!(
        value["responses"]["R2"]["headers"]["H"]["$ref"],
        "#/components/headers/X2"
    );
    assert_eq!(
        value["headers"]["X2"]["schema"]["$ref"],
        "#/components/schemas/A2"
    );
    assert!(
        target
            .copy_from(&source, refs, &renames)
            .unwrap()
            .is_empty(),
        "a component whose target name exists is not copied again"
    );
    assert_eq!(
        target.copy_from(
            &source,
            [ComponentRef::new(ComponentKind::Schemas, "Missing")],
            &renames
        ),
        Err(RefError::Missing(ComponentRef::new(
            ComponentKind::Schemas,
            "Missing"
        )))
    );
}

#[test]
fn a_no_op_mutable_walk_keeps_every_document_byte_identical() {
    struct Noop;
    impl VisitMut for Noop {}

    let fixtures: [&[u8]; 8] = [
        include_bytes!("../examples/v3.0/json/api-with-examples.json"),
        include_bytes!("../examples/v3.0/json/callback-example.json"),
        include_bytes!("../examples/v3.0/json/link-example.json"),
        include_bytes!("../examples/v3.0/json/petstore-expanded.json"),
        include_bytes!("../examples/v3.0/json/petstore.json"),
        include_bytes!("../examples/v3.0/json/uspto.json"),
        include_bytes!("../examples/v3.2/json/sse.json"),
        include_bytes!("../openapi3-examples/3.0/pass/swagger2openapi/openapi.json"),
    ];
    for bytes in fixtures {
        let mut doc: OpenAPIV3 = serde_json::from_slice(bytes).unwrap();
        let before = serde_json::to_vec(&doc).unwrap();
        doc.walk_mut(&mut Noop);
        let after = serde_json::to_vec(&doc).unwrap();
        assert_eq!(
            before, after,
            "a no-op mutable walk must not change a document"
        );
        let mut refs = Refs::default();
        doc.walk(&mut refs);
    }
}

#[test]
fn a_long_reference_chain_builds_and_closes_in_linear_time() {
    const CHAIN: usize = 20_000;
    let mut schemas = serde_json::Map::new();
    for index in 0..CHAIN {
        let schema = if index + 1 < CHAIN {
            json!({"$ref": format!("#/components/schemas/S{}", index + 1)})
        } else {
            json!({"type": "string"})
        };
        schemas.insert(format!("S{index}"), schema);
    }
    let doc: OpenAPIV3 = serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1"},
        "paths": {},
        "components": {"schemas": schemas}
    }))
    .unwrap();
    let started = Instant::now();
    let graph = doc.ref_graph();
    let closure = graph
        .closure([ComponentRef::new(ComponentKind::Schemas, "S0")])
        .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(closure.len(), CHAIN);
    assert!(
        elapsed < Duration::from_secs(10),
        "a {} component chain took {:?}; the graph is not linear",
        CHAIN,
        elapsed
    );
}

#[test]
fn component_value_kinds_and_pointers_are_stable() {
    let schema = ComponentRef::parse("#/components/schemas/a~1b").unwrap();
    assert_eq!(schema.kind, ComponentKind::Schemas);
    assert_eq!(schema.name, "a/b");
    assert_eq!(schema.to_pointer(), "#/components/schemas/a~1b");
    assert_eq!(
        ComponentRef::parse("https://example.test/schema").unwrap_err(),
        RefError::External
    );
    assert_eq!(
        ComponentRef::parse("#/components/schemas").unwrap_err(),
        RefError::OutsideComponents
    );
    assert_eq!(
        ComponentRef::parse("#/components/schemas/A/properties/x").unwrap_err(),
        RefError::OutsideComponents
    );
    assert_eq!(
        ComponentRef::parse("#/paths/~1x").unwrap_err(),
        RefError::OutsideComponents
    );
    assert_eq!(
        ComponentRef::parse("$dynamicRef").unwrap_err(),
        RefError::Dynamic
    );
    let value: Value = json!({"type": "string"});
    let components: oas::Components =
        serde_json::from_value(json!({"schemas": {"A": value}})).unwrap();
    let found = components
        .get(&ComponentRef::new(ComponentKind::Schemas, "A"))
        .unwrap();
    assert_eq!(found.kind(), ComponentKind::Schemas);
    assert_eq!(found.to_value(), json!({"type": "string"}));
    let _: Option<ComponentValue<'_>> =
        components.get(&ComponentRef::new(ComponentKind::Schemas, "Missing"));
}

#[test]
fn path_item_and_component_entry_points_walk_their_subtrees() {
    let doc = positions_document();
    let component = ComponentRef::new(ComponentKind::Responses, "Ok");
    let mut found = Refs::default();
    doc.components
        .as_ref()
        .unwrap()
        .walk_component(&component, &mut found);
    assert!(found
        .0
        .iter()
        .any(|(kind, target, _)| *kind == ComponentKind::Headers
            && target == "#/components/headers/Header"));

    let mut found = Refs::default();
    doc.paths["/x"].walk(&mut found);
    assert!(found
        .0
        .iter()
        .any(|(kind, target, _)| *kind == ComponentKind::Parameters
            && target == "#/components/parameters/P"));
    assert!(found
        .0
        .iter()
        .any(|(kind, target, _)| *kind == ComponentKind::Responses
            && target == "#/components/responses/Ok"));
}

#[test]
fn paths_extensions_are_not_path_items() {
    let doc: OpenAPIV3 = serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1"},
        "paths": {
            "x-flag": true,
            "x-meta": {"$ref": "#/components/schemas/S"},
            "/x": {"get": {"responses": {"200": {"$ref": "#/components/responses/R"}}}}
        },
        "components": {"responses": {"R": {"description": "ok"}}}
    }))
    .expect("Paths extensions must not decode as path items");
    assert_eq!(doc.paths.extras["x-flag"], json!(true));
    assert_eq!(
        doc.paths.extras["x-meta"],
        json!({"$ref": "#/components/schemas/S"})
    );
    let mut extensions = Extensions::default();
    doc.walk(&mut extensions);
    assert!(extensions.0.contains("x-flag"));
    assert!(extensions.0.contains("x-meta"));
    let found = references(&doc);
    assert!(
        !found
            .iter()
            .any(|(_, target, _)| target == "#/components/schemas/S"),
        "a Paths extension value is data, not a reference"
    );
    assert!(found
        .iter()
        .any(|(kind, target, _)| *kind == ComponentKind::Responses
            && target == "#/components/responses/R"));
}

#[test]
fn webhooks_are_typed_and_walked() {
    let mut doc: OpenAPIV3 = serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1"},
        "paths": {},
        "webhooks": {"newEvent": {"post": {
            "responses": {"200": {"$ref": "#/components/responses/Ok"}}
        }}},
        "components": {"responses": {"Ok": {"description": "ok"}}}
    }))
    .unwrap();
    assert!(
        doc.extras
            .as_ref()
            .map_or(true, |extras| !extras.contains_key("webhooks")),
        "webhooks must not stay in the root extension map"
    );
    assert_eq!(doc.webhooks.as_ref().unwrap().len(), 1);
    let found = references(&doc);
    assert!(
        found
            .iter()
            .any(|(kind, target, _)| *kind == ComponentKind::Responses
                && target == "#/components/responses/Ok"),
        "a reference inside a webhook must be walked: {found:?}"
    );

    doc.rename_refs(&BTreeMap::from([(
        ComponentRef::new(ComponentKind::Responses, "Ok"),
        "Success".to_string(),
    )]));
    let value = serde_json::to_value(&doc).unwrap();
    assert_eq!(
        value["webhooks"]["newEvent"]["post"]["responses"]["200"]["$ref"],
        "#/components/responses/Success",
        "rename_refs must rewrite references inside webhooks"
    );
}

#[test]
fn renaming_security_requirements_keeps_every_key_and_unions_scopes() {
    let key = |name: &str| ComponentRef::new(ComponentKind::SecuritySchemes, name);
    let mut doc: OpenAPIV3 = serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1"},
        "paths": {},
        "security": [{"K": ["r"], "L": ["w"]}]
    }))
    .unwrap();
    doc.rename_refs(&BTreeMap::from([
        (key("K"), "L".to_string()),
        (key("L"), "K".to_string()),
    ]));
    let value = serde_json::to_value(&doc).unwrap();
    assert_eq!(value["security"][0]["L"], json!(["r"]));
    assert_eq!(value["security"][0]["K"], json!(["w"]));

    let mut doc: OpenAPIV3 = serde_json::from_value(json!({
        "openapi": "3.1.0",
        "info": {"title": "t", "version": "1"},
        "paths": {},
        "security": [{"K": ["r"], "L": ["w"]}]
    }))
    .unwrap();
    doc.rename_refs(&BTreeMap::from([(key("K"), "L".to_string())]));
    let value = serde_json::to_value(&doc).unwrap();
    assert_eq!(
        value["security"][0]["L"],
        json!(["r", "w"]),
        "a collision unions the scopes instead of overwriting one"
    );
    assert!(value["security"][0].get("K").is_none());
}
