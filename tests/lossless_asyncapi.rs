#![cfg(feature = "lossless-schema-keywords")]
use oas::lossless_asyncapi::{
    Components, Metadata, Node, NodeLookup, NodeMut, Parameter, StructuralNode,
};
use oas::lossless_component::{Decode, Field, MetadataPatch};
use serde_json::json;

#[test]
fn structural_owners_preserve_deferred_shapes_and_borrow_nested_targets() {
    let source = json!({
        "tags":[{"name":"events","externalDocs":{"url":"/events"}},null],
        "externalDocs":{"$ref":"#/components/externalDocs/Guide"},
        "bindings":{"ws":{"headers":{"properties":{"model":{"type":"string"}}},
            "query":{"type":"object","externalDocs":{"url":"/query"}},"bindingVersion":"0.1.0"}},
        "correlationId":{"location":"$message.header#/requestId"},
        "x-literal":{"payload":{"$ref":"literal"}}
    });
    let metadata = Metadata::decode(source.clone()).unwrap();
    assert_eq!(serde_json::to_value(&metadata).unwrap(), source);
    assert!(matches!(
        metadata.tags.as_value().unwrap()[1],
        Field::Invalid(_)
    ));
    let Node::Schema(model) = metadata
        .node_at_tokens(&["bindings", "ws", "headers", "properties", "model"])
        .unwrap()
    else {
        panic!("schema");
    };
    assert_eq!(
        serde_json::to_value(model).unwrap(),
        json!({"type":"string"})
    );
    let Node::Documentation(docs) = metadata
        .node_at_tokens(&["tags", "0", "externalDocs"])
        .unwrap()
    else {
        panic!("documentation");
    };
    assert_eq!(docs.url.as_value().unwrap(), "/events");
    let Node::Documentation(docs) = metadata
        .node_at_tokens(&["bindings", "ws", "query", "externalDocs"])
        .unwrap()
    else {
        panic!("schema documentation");
    };
    assert_eq!(docs.url.as_value().unwrap(), "/query");
    assert!(matches!(
        metadata.node_at_tokens(&["tags", "1"]),
        Some(Node::Raw(serde_json::Value::Null))
    ));
    let parameter = Parameter::decode(json!({
        "schema":{"properties":{"externalDocs":{"externalDocs":{"url":"/nested"}}}},
        "description":null
    }))
    .unwrap();
    let Node::Documentation(docs) = parameter
        .node_at_tokens(&["schema", "properties", "externalDocs", "externalDocs"])
        .unwrap()
    else {
        panic!("property name is not a structural keyword");
    };
    assert_eq!(docs.url.as_value().unwrap(), "/nested");
    assert!(matches!(
        parameter.node_at_tokens(&["schema"]),
        Some(Node::Schema(_))
    ));
    assert_eq!(
        metadata
            .correlation_id
            .as_value()
            .unwrap()
            .location
            .as_value()
            .unwrap(),
        "$message.header#/requestId"
    );
}

#[test]
fn selected_metadata_preserves_partition_copy_and_first_value_patch_semantics() {
    let source = json!({
        "bindings":{"ws":{"bindingVersion":null}}, "externalDocs":null,
        "tags":[], "correlationId":{"location":"$message.header#/id"},
        "title":{"literal":null}, "x-owner":true
    });
    let mut remaining = source.as_object().unwrap().clone();
    let owner = Metadata::take_from(&mut remaining, |key| key != "correlationId");
    assert_eq!(
        remaining,
        json!({
            "correlationId":{"location":"$message.header#/id"},
            "title":{"literal":null}, "x-owner":true
        })
        .as_object()
        .unwrap()
        .clone()
    );
    assert!(owner.extras.is_empty());
    for key in ["bindings", "tags", "externalDocs", "correlationId"] {
        assert!(Metadata::owns_keyword(key));
    }
    assert!(!Metadata::owns_keyword("title"));
    let mut target = Metadata::default();
    let mut policy = Patch::default();
    target
        .copy_keyword_from("externalDocs", &owner, &mut policy)
        .unwrap();
    assert!(matches!(
        target.external_docs,
        Field::Invalid(serde_json::Value::Null)
    ));
    assert!(policy.keys.is_empty());
    target
        .patch_or_insert_keyword_from("bindings", &owner, &mut policy)
        .unwrap();
    assert_eq!(
        serde_json::to_value(&target.bindings).unwrap(),
        source["bindings"]
    );
    assert_eq!(policy.keys, ["bindings"]);
    let patch =
        Metadata::decode(json!({"bindings":{"ws":{"bindingVersion":null,"x-literal":null}}}))
            .unwrap();
    target
        .patch_or_insert_keyword_from("bindings", &patch, &mut policy)
        .unwrap();
    assert_eq!(
        serde_json::to_value(&target.bindings).unwrap(),
        json!({"ws":{"x-literal":null}})
    );
    assert_eq!(
        policy.keys,
        ["bindings", "bindings", "ws", "bindingVersion", "x-literal"]
    );
    let delete = Metadata::decode(json!({"externalDocs":null})).unwrap();
    target
        .patch_or_insert_keyword_from("externalDocs", &delete, &mut policy)
        .unwrap();
    assert!(target.external_docs.is_missing());
    let before = serde_json::to_value(&target).unwrap();
    let keys = policy.keys.clone();
    target
        .copy_keyword_from("correlationId", &owner, &mut policy)
        .unwrap();
    target
        .patch_or_insert_keyword_from("correlationId", &owner, &mut policy)
        .unwrap();
    target
        .patch_or_insert_keyword_from("title", &owner, &mut policy)
        .unwrap();
    assert_eq!(serde_json::to_value(&target).unwrap(), before);
    assert_eq!(policy.keys, keys);
    assert_eq!(
        serde_json::to_value(&owner).unwrap(),
        json!({
            "bindings":{"ws":{"bindingVersion":null}}, "externalDocs":null, "tags":[]
        })
    );

    let mut fields = json!({"parameters":{"Room":{"description":null}},"tags":{},"schemas":{"Text":true},"x-owner":true}).as_object().unwrap().clone();
    let components = Components::take_from(&mut fields, |_| true);
    assert_eq!(
        serde_json::to_value(components).unwrap(),
        json!({"parameters":{"Room":{"description":null}},"tags":{}})
    );
    assert_eq!(
        fields,
        json!({"schemas":{"Text":true},"x-owner":true})
            .as_object()
            .unwrap()
            .clone()
    );
}

#[test]
fn component_rekey_moves_values_and_rejects_invalid_containers_atomically() {
    let mut target =
        Components::decode(json!({"tags":{"same":{"name":"old"}},"x-target":true})).unwrap();
    let source = Components::decode(json!({
        "parameters":{}, "tags":{"a":{"name":"first"},"b":null},
        "externalDocs":{"guide":{"url":"/guide"}}
    }))
    .unwrap();
    let mut visited = Vec::new();
    target
        .append_rekeyed(source, |group, name| {
            visited.push((group.to_owned(), name));
            "same".into()
        })
        .unwrap();
    assert_eq!(
        visited,
        [
            ("tags".into(), "a".into()),
            ("tags".into(), "b".into()),
            ("externalDocs".into(), "guide".into())
        ]
    );
    assert_eq!(
        serde_json::to_value(&target).unwrap(),
        json!({
            "tags":{"same":null}, "externalDocs":{"same":{"url":"/guide"}}, "x-target":true
        })
    );
    let before = serde_json::to_value(&target).unwrap();
    for source in [
        json!({"tags":{"a":{"name":"new"}},"externalDocs":null}),
        json!({"x-source":true}),
    ] {
        assert!(target
            .append_rekeyed(Components::decode(source).unwrap(), |_, _| panic!(
                "invalid source must not invoke rename"
            ))
            .is_err());
        assert_eq!(serde_json::to_value(&target).unwrap(), before);
    }
    let mut invalid_target = Components::decode(json!({"parameters":7})).unwrap();
    assert!(invalid_target
        .append_rekeyed(Components::default(), |_, _| panic!(
            "invalid target must not invoke rename"
        ))
        .is_err());
    assert_eq!(
        serde_json::to_value(invalid_target).unwrap(),
        json!({"parameters":7})
    );
}

#[derive(Default)]
struct Patch {
    keys: Vec<String>,
}
impl MetadataPatch for Patch {
    type Error = ();
    fn key(&mut self, key: &str) -> Result<bool, ()> {
        self.keys.push(key.into());
        Ok(key.starts_with("x-"))
    }
    fn replacement(&mut self, _: &impl serde::Serialize) -> Result<(), ()> {
        Ok(())
    }
    fn raw(&mut self, target: &mut serde_json::Value, patch: &serde_json::Value) -> Result<(), ()> {
        *target = patch.clone();
        Ok(())
    }
}

#[test]
fn component_metadata_patches_and_visitors_preserve_structural_ownership() {
    let mut target = Components::decode(json!({
        "tags":{"Public":{"name":"events","description":"old"},"Removed":{"name":"old"}},
        "parameters":{"Limit":{"schema":{"type":"integer"}}}
    }))
    .unwrap();
    let patch = Components::decode(json!({
        "tags":{"Public":{"description":null,"externalDocs":{"url":"/events"}},"Removed":null}
    }))
    .unwrap();
    let mut policy = Patch::default();
    target
        .patch_keyword_from("tags", &patch, &mut policy)
        .unwrap();
    assert_eq!(
        policy.keys,
        [
            "tags",
            "Public",
            "description",
            "externalDocs",
            "url",
            "Removed"
        ]
    );
    assert_eq!(
        serde_json::to_value(&target).unwrap(),
        json!({
            "tags":{"Public":{"name":"events","externalDocs":{"url":"/events"}}},
            "parameters":{"Limit":{"schema":{"type":"integer"}}}
        })
    );
    let Node::Schema(schema) = target
        .node_at_tokens(&["parameters", "Limit", "schema"])
        .unwrap()
    else {
        panic!("parameter schema");
    };
    assert_eq!(
        serde_json::to_value(schema).unwrap(),
        json!({"type":"integer"})
    );

    let mut metadata = Metadata::decode(json!({"bindings":{"ws":{"query":true}},"tags":[{"name":"events"}],"x-literal":{"tags":[7]}})).unwrap();
    let mut visited = Vec::new();
    metadata
        .node_mut()
        .try_for_each_child(|key, child| {
            visited.push(key.to_owned());
            if let NodeMut::Tag(tag) = child {
                tag.name = Field::Value("renamed".into());
            }
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(visited, ["bindings", "tags"]);
    assert_eq!(
        serde_json::to_value(metadata.node()).unwrap(),
        json!({
            "bindings":{"ws":{"query":true}},"tags":[{"name":"renamed"}],"x-literal":{"tags":[7]}
        })
    );
}
