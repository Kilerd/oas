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
        json!({"parameters":{"Room":{"description":null}},"tags":{},"schemas":{"Text":true}})
    );
    assert_eq!(fields, json!({"x-owner":true}).as_object().unwrap().clone());
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

struct RejectPatch {
    callback: &'static str,
    remaining: usize,
}

impl RejectPatch {
    fn visit(&mut self, callback: &'static str) -> Result<(), &'static str> {
        if self.callback == callback {
            if self.remaining == 0 {
                return Err(callback);
            }
            self.remaining -= 1;
        }
        Ok(())
    }
}

impl MetadataPatch for RejectPatch {
    type Error = &'static str;

    fn key(&mut self, _: &str) -> Result<bool, Self::Error> {
        self.visit("key")?;
        Ok(false)
    }

    fn replacement(&mut self, _: &impl serde::Serialize) -> Result<(), Self::Error> {
        self.visit("replacement")
    }

    fn raw(
        &mut self,
        target: &mut serde_json::Value,
        patch: &serde_json::Value,
    ) -> Result<(), Self::Error> {
        self.visit("raw")?;
        *target = patch.clone();
        Ok(())
    }
}

#[test]
fn rejected_raw_metadata_patch_preserves_the_existing_field() {
    for bindings in [json!({"ws":{"query":true}}), json!([7, null])] {
        let before = json!({"bindings":bindings,"tags":[{"name":"keep"}]});
        let mut target = Metadata::decode(before.clone()).unwrap();
        let patch = Metadata::decode(json!({"bindings":7})).unwrap();
        let result = target.patch_keyword_from(
            "bindings",
            &patch,
            &mut RejectPatch {
                callback: "raw",
                remaining: 0,
            },
        );
        let after = serde_json::to_value(&target).unwrap();
        println!("raw metadata: before={before} result={result:?} after={after}");
        assert_eq!(result, Err("raw"));
        assert_eq!(after, before);
    }
}

#[test]
fn rejected_metadata_callbacks_preserve_siblings_and_earlier_edits() {
    let before = json!({
        "bindings":{"ws":{"headers":true,"query":true},"x-keep":[7,null]},
        "tags":[{"name":"keep"}]
    });
    for (callback, remaining, patch, expected) in [
        ("key", 0, json!({"bindings":7}), before.clone()),
        (
            "replacement",
            0,
            json!({"bindings":{"ws":{"headers":false}}}),
            before.clone(),
        ),
        (
            "key",
            3,
            json!({"bindings":{"ws":{"headers":false,"query":false}}}),
            json!({"bindings":{"ws":{"headers":false,"query":true},"x-keep":[7,null]},"tags":[{"name":"keep"}]}),
        ),
        (
            "replacement",
            1,
            json!({"bindings":{"ws":{"headers":false,"query":false}}}),
            json!({"bindings":{"ws":{"headers":false,"query":true},"x-keep":[7,null]},"tags":[{"name":"keep"}]}),
        ),
        (
            "raw",
            0,
            json!({"bindings":{"ws":{"headers":false,"query":7}}}),
            json!({"bindings":{"ws":{"headers":false,"query":true},"x-keep":[7,null]},"tags":[{"name":"keep"}]}),
        ),
    ] {
        let mut target = Metadata::decode(before.clone()).unwrap();
        let patch = Metadata::decode(patch).unwrap();
        assert_eq!(
            target.patch_keyword_from(
                "bindings",
                &patch,
                &mut RejectPatch {
                    callback,
                    remaining,
                },
            ),
            Err(callback),
        );
        assert_eq!(serde_json::to_value(target).unwrap(), expected);
    }
}

#[test]
fn rejected_schema_metadata_patch_preserves_the_fallback_map() {
    let before = json!({"dependentRequired":{"keep":[7],"other":["x"]}});
    let patch = oas::SchemaValue::from_lossless_value(json!({
        "dependentRequired":{"a":["first"],"b":["second"]}
    }))
    .unwrap();
    for callback in ["key", "replacement"] {
        for remaining in [0, 1] {
            let mut target = oas::SchemaValue::from_lossless_value(before.clone()).unwrap();
            let result = target.as_object_mut().unwrap().patch_metadata_keyword(
                "dependentRequired",
                patch.as_object().unwrap(),
                &mut RejectPatch {
                    callback,
                    remaining,
                },
            );
            let after = serde_json::to_value(&target).unwrap();
            println!("schema metadata ({callback}, {remaining}): before={before} result={result:?} after={after}");
            assert_eq!(result, Err(callback));
            let mut expected = before.clone();
            if remaining == 1 {
                expected["dependentRequired"]["a"] = json!(["first"]);
            }
            assert_eq!(after, expected);
            assert!(target.as_object().unwrap().dependent_required.is_none());
        }
    }
}

#[test]
fn rejected_schema_metadata_patch_promotes_a_repaired_map() {
    for callback in ["key", "replacement"] {
        let mut target = oas::SchemaValue::from_lossless_value(json!({
            "dependentRequired":{"a":[7],"other":["x"]}
        }))
        .unwrap();
        let patch = oas::SchemaValue::from_lossless_value(json!({
            "dependentRequired":{"a":["fixed"],"b":["rejected"]}
        }))
        .unwrap();
        assert_eq!(
            target.as_object_mut().unwrap().patch_metadata_keyword(
                "dependentRequired",
                patch.as_object().unwrap(),
                &mut RejectPatch {
                    callback,
                    remaining: 1,
                },
            ),
            Err(callback),
        );
        assert_eq!(
            serde_json::to_value(&target).unwrap(),
            json!({"dependentRequired":{"a":["fixed"],"other":["x"]}}),
        );
        let target = target.as_object().unwrap();
        assert!(target.dependent_required.is_some());
        assert!(!target.extras.contains_key("dependentRequired"));
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

mod structure {
    use oas::lossless_asyncapi::{
        Channel, Components, Document, Message, MessageTrait, Node, NodeLookup, NodeMut, OauthFlow,
        OauthFlows, Operation, OperationReply, OperationReplyAddress, OperationTrait, Reference,
        SecurityScheme, Server, ServerVariable, StructuralNode,
    };
    use oas::lossless_component::{Decode, Field};
    use serde::Serialize;
    use serde_json::{json, Value};

    fn fixtures() -> [Value; 2] {
        [
            serde_json::from_str(include_str!("fixtures/asyncapi/chat.json")).unwrap(),
            serde_json::from_str(include_str!("fixtures/asyncapi/deferred-shapes.json")).unwrap(),
        ]
    }

    /// Values at a JSON pointer; a trailing `/*` selects every map entry or list element.
    fn select(document: &Value, pattern: &str) -> Vec<Value> {
        let (pointer, all) = match pattern.strip_suffix("/*") {
            Some(pointer) => (pointer, true),
            None => (pattern, false),
        };
        match (document.pointer(pointer), all) {
            (None, _) => Vec::new(),
            (Some(value), false) => vec![value.clone()],
            (Some(Value::Object(map)), true) => map.values().cloned().collect(),
            (Some(Value::Array(values)), true) => values.clone(),
            (Some(_), true) => Vec::new(),
        }
    }

    fn assert_lossless<T: Decode + Serialize>(node: &str, patterns: &[&str]) {
        let mut checked = 0;
        for document in fixtures() {
            for value in patterns
                .iter()
                .flat_map(|pattern| select(&document, pattern))
            {
                checked += 1;
                match T::decode(value.clone()) {
                    Ok(decoded) => assert_eq!(
                        serde_json::to_value(&decoded).unwrap(),
                        value,
                        "{node} must round-trip"
                    ),
                    Err(rejected) => {
                        assert!(!value.is_object(), "{node} must accept every object");
                        assert_eq!(
                            rejected, value,
                            "{node} must return a rejected value unchanged"
                        );
                    }
                }
            }
        }
        assert!(
            checked >= 2,
            "{node} patterns must select fixtures, got {checked}"
        );
    }

    #[test]
    fn documents_round_trip_losslessly() {
        for fixture in fixtures() {
            let document = Document::decode(fixture.clone()).unwrap();
            assert_eq!(serde_json::to_value(&document).unwrap(), fixture);
            assert!(oas::HeapSize::heap_size(&document) > 0);
            let deserialized: Document = serde_json::from_value(fixture.clone()).unwrap();
            assert_eq!(serde_json::to_value(deserialized.node()).unwrap(), fixture);
        }
        assert_eq!(Document::decode(json!([])).unwrap_err(), json!([]));
    }

    #[test]
    fn every_structural_node_round_trips_its_fixtures() {
        assert_lossless::<Components>("Components", &["/components", "/x-root"]);
        assert_lossless::<Server>("Server", &["/servers/*", "/components/servers/*"]);
        assert_lossless::<ServerVariable>(
            "ServerVariable",
            &[
                "/servers/production/variables/*",
                "/servers/entries/variables/*",
                "/components/serverVariables/*",
            ],
        );
        assert_lossless::<Channel>("Channel", &["/channels/*", "/components/channels/*"]);
        assert_lossless::<Operation>("Operation", &["/operations/*", "/components/operations/*"]);
        assert_lossless::<OperationTrait>(
            "OperationTrait",
            &[
                "/operations/sendChat/traits/*",
                "/operations/badTrait/traits/*",
                "/components/operationTraits/*",
            ],
        );
        assert_lossless::<OperationReply>(
            "OperationReply",
            &[
                "/operations/sendChat/reply",
                "/operations/badReply/reply",
                "/operations/badChannel/reply",
                "/components/replies/*",
            ],
        );
        assert_lossless::<OperationReplyAddress>(
            "OperationReplyAddress",
            &[
                "/operations/sendChat/reply/address",
                "/operations/badReply/reply/address",
                "/components/replyAddresses/*",
            ],
        );
        assert_lossless::<Message>(
            "Message",
            &[
                "/channels/room/messages/*",
                "/channels/badEntries/messages/*",
                "/components/messages/*",
            ],
        );
        assert_lossless::<MessageTrait>(
            "MessageTrait",
            &[
                "/components/messages/Chat/traits/*",
                "/components/messageTraits/*",
            ],
        );
        assert_lossless::<SecurityScheme>(
            "SecurityScheme",
            &[
                "/servers/production/security/*",
                "/servers/entries/security/*",
                "/components/securitySchemes/*",
            ],
        );
        assert_lossless::<OauthFlows>(
            "OauthFlows",
            &[
                "/components/securitySchemes/OAuth/flows",
                "/components/securitySchemes/s/flows",
                "/components/securitySchemes/t/flows",
            ],
        );
        assert_lossless::<OauthFlow>(
            "OauthFlow",
            &[
                "/components/securitySchemes/OAuth/flows/*",
                "/components/securitySchemes/t/flows/*",
            ],
        );
        assert_lossless::<Reference>(
            "Reference",
            &[
                "/operations/sendChat/channel",
                "/operations/sendChat/messages/*",
                "/channels/room/servers/*",
                "/channels/badEntries/servers/*",
                "/operations/badReply/reply/messages/*",
            ],
        );
    }

    #[test]
    fn presence_null_and_invalid_shapes_remain_distinct() {
        let address = |value: Value| Channel::decode(value).unwrap().address;
        assert!(
            matches!(address(json!({"address":"/rooms"})), Field::Value(value) if value == "/rooms")
        );
        assert!(matches!(
            address(json!({"address":null})),
            Field::Invalid(Value::Null)
        ));
        assert!(
            matches!(address(json!({"address":7})), Field::Invalid(value) if value == json!(7))
        );
        assert!(address(json!({})).is_missing());

        let channel = Channel::decode(json!({"messages":{}})).unwrap();
        assert!(matches!(&channel.messages, Field::Value(messages) if messages.is_empty()));
        assert!(Channel::decode(json!({})).unwrap().messages.is_missing());
        let operation = Operation::decode(json!({"messages":[]})).unwrap();
        assert!(matches!(&operation.messages, Field::Value(messages) if messages.is_empty()));
        assert!(Operation::decode(json!({})).unwrap().messages.is_missing());

        let [chat, deferred] = fixtures();
        let chat = Document::decode(chat).unwrap();
        assert!(
            chat.extras.contains_key("info"),
            "undeclared vocabulary stays in extras"
        );
        let room = chat.channels.as_value().unwrap()["room"]
            .as_value()
            .unwrap();
        assert!(
            !room.extras.contains_key("address"),
            "declared fields never live in extras"
        );
        assert!(room.extras.contains_key("x-channel-note"));
        let deferred = Document::decode(deferred).unwrap();
        assert!(matches!(deferred.asyncapi, Field::Invalid(_)));
        let channels = deferred.channels.as_value().unwrap();
        assert!(matches!(channels["notObject"], Field::Invalid(_)));
        let bad = channels["badAddress"].as_value().unwrap();
        assert!(matches!(bad.messages, Field::Invalid(_)));
        assert!(matches!(bad.parameters, Field::Invalid(Value::Null)));
    }

    fn collect_references(node: Node<'_>, references: &mut Vec<String>) {
        if let Some(Field::Value(reference)) = node.reference() {
            references.push(reference.clone());
        }
        node.try_for_each_child(|_, child| {
            collect_references(child, references);
            Ok::<_, ()>(())
        })
        .unwrap();
    }

    const STRUCTURAL_REFERENCES: [&str; 27] = [
        "#/channels/dynamic",
        "#/channels/room",
        "#/channels/room",
        "#/channels/room",
        "#/channels/room",
        "#/channels/room/messages/chat",
        "#/channels/room/messages/typing",
        "#/components/channels/Shared",
        "#/components/channels/Shared",
        "#/components/correlationIds/Request",
        "#/components/externalDocs/Guide",
        "#/components/messageTraits/Envelope",
        "#/components/messages/Chat",
        "#/components/messages/Chat",
        "#/components/operationBindings/Ws",
        "#/components/operationTraits/Audited",
        "#/components/operations/Shared",
        "#/components/parameters/RoomId",
        "#/components/replies/Ack",
        "#/components/replyAddresses/Inbox",
        "#/components/securitySchemes/OAuth",
        "#/components/securitySchemes/Token",
        "#/components/securitySchemes/Token",
        "#/components/serverVariables/Shared",
        "#/components/servers/Staging",
        "#/components/tags/Public",
        "#/servers/production",
    ];

    #[test]
    fn traversal_exposes_structural_references_but_not_literals() {
        let [chat, _] = fixtures();
        let document = Document::decode(chat).unwrap();
        let mut references = Vec::new();
        collect_references(document.node(), &mut references);
        references.sort();
        assert_eq!(references, STRUCTURAL_REFERENCES);

        let mut visited = Vec::new();
        document
            .node()
            .try_for_each_child(|key, _| {
                visited.push(key.to_owned());
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(
            visited,
            [
                "channels",
                "channels",
                "channels",
                "channels",
                "components",
                "operations",
                "operations",
                "operations",
                "operations",
                "servers",
                "servers"
            ]
        );
    }

    fn rename_references(node: NodeMut<'_>) {
        let mut node = node;
        if let Some(Field::Value(reference)) = node.reference_mut() {
            if let Some(rest) = reference.strip_prefix("#/channels/room") {
                *reference = format!("#/channels/lobby{rest}");
            }
        }
        node.try_for_each_child(|_, child| {
            rename_references(child);
            Ok::<_, ()>(())
        })
        .unwrap();
    }

    #[test]
    fn editable_traversal_rewrites_references_in_place() {
        let [chat, deferred] = fixtures();
        let mut document = Document::decode(chat.clone()).unwrap();
        rename_references(document.node_mut());
        let expected: Value = serde_json::from_str(
            &chat
                .to_string()
                .replace("\"#/channels/room", "\"#/channels/lobby"),
        )
        .unwrap();
        assert_ne!(expected, chat);
        assert_eq!(serde_json::to_value(&document).unwrap(), expected);

        let mut document = Document::decode(deferred.clone()).unwrap();
        rename_references(document.node_mut());
        assert_eq!(serde_json::to_value(&document).unwrap(), deferred);
    }

    #[test]
    fn lookup_borrows_nested_structure_without_following_references() {
        let [chat, deferred] = fixtures();
        let chat = Document::decode(chat).unwrap();
        let schema = |tokens: &[&str]| match chat.node_at_tokens(tokens) {
            Some(Node::Schema(schema)) => serde_json::to_value(schema).unwrap(),
            _ => panic!("schema at {tokens:?}"),
        };
        assert_eq!(
            schema(&[
                "channels",
                "room",
                "messages",
                "typing",
                "payload",
                "properties",
                "user"
            ]),
            json!({"$ref":"#/components/schemas/User"})
        );
        assert_eq!(
            schema(&[
                "components",
                "messages",
                "Chat",
                "payload",
                "properties",
                "text"
            ]),
            json!({"type":"string"})
        );
        assert_eq!(
            schema(&[
                "channels",
                "room",
                "bindings",
                "ws",
                "headers",
                "properties",
                "x-model"
            ]),
            json!({"type":"string","enum":["a","b"]})
        );
        assert_eq!(
            schema(&["components", "messages", "Avro", "payload", "schema"]),
            json!({"type":"string"})
        );
        assert_eq!(
            schema(&["components", "messageTraits", "Envelope", "headers"])["properties"]
                ["x-trace"],
            json!({"type":"string"})
        );
        let Some(Node::OperationReplyAddress(address)) =
            chat.node_at_tokens(&["operations", "sendChat", "reply", "address"])
        else {
            panic!("reply address");
        };
        assert_eq!(
            address.reference.as_value().unwrap(),
            "#/components/replyAddresses/Inbox"
        );
        let Some(Node::OauthFlow(flow)) = chat.node_at_tokens(&[
            "components",
            "securitySchemes",
            "OAuth",
            "flows",
            "clientCredentials",
        ]) else {
            panic!("oauth flow");
        };
        assert_eq!(
            flow.token_url.as_value().unwrap(),
            "https://example.com/token"
        );
        assert!(matches!(
            chat.node_at_tokens(&["operations", "sendChat", "messages", "0"]),
            Some(Node::Reference(_))
        ));
        assert!(matches!(
            chat.node_at_tokens(&["components", "operationTraits", "Audited", "security", "0"]),
            Some(Node::SecurityScheme(_))
        ));
        assert!(matches!(
            chat.node_at_tokens::<&str>(&[]),
            Some(Node::Document(_))
        ));
        assert!(matches!(
            chat.node_at_tokens(&["components"]),
            Some(Node::Components(_))
        ));
        assert!(chat
            .node_at_tokens(&["channels", "shared", "address"])
            .is_none());
        assert!(matches!(
            chat.node_at_tokens(&["info", "title"]),
            Some(Node::Raw(Value::String(title))) if title == "Chat"
        ));

        let deferred = Document::decode(deferred).unwrap();
        assert!(matches!(
            deferred.node_at_tokens(&["channels", "badAddress", "messages"]),
            Some(Node::Raw(Value::Array(_)))
        ));
        assert!(matches!(
            deferred.node_at_tokens(&["servers", "partial", "security", "Token"]),
            Some(Node::Raw(Value::Array(_)))
        ));
        assert!(matches!(
            deferred.node_at_tokens(&["components", "schemas", "Null"]),
            Some(Node::Raw(Value::Null))
        ));
    }

    #[test]
    fn structural_owners_patch_nested_entries_through_the_policy() {
        let mut channel = Channel::decode(json!({
            "address":"/rooms", "messages":{"chat":{"name":"chat","payload":{"type":"string"}},"old":{}},
            "x-keep":true
        }))
        .unwrap();
        let patch = Channel::decode(json!({
            "address":null, "messages":{"chat":{"name":"renamed","payload":{"const":"x"}},"old":null}
        }))
        .unwrap();
        let mut policy = super::Patch::default();
        channel.patch_metadata(&patch, &mut policy).unwrap();
        assert_eq!(
            policy.keys,
            ["address", "messages", "chat", "name", "payload", "old"]
        );
        assert!(channel.address.is_missing());
        assert_eq!(
            serde_json::to_value(&channel).unwrap(),
            json!({"messages":{"chat":{"name":"renamed","payload":{"const":"x"}}},"x-keep":true})
        );
    }

    #[test]
    fn components_own_and_rekey_every_declared_group() {
        let [chat, _] = fixtures();
        let mut fields = chat["components"].as_object().unwrap().clone();
        let mut components = Components::take_from(&mut fields, |key| key != "messages");
        assert_eq!(
            fields.keys().collect::<Vec<_>>(),
            ["messages", "x-components-owner"]
        );
        assert!(components.extras.is_empty());

        let source = Components::decode(json!({
            "messages":{"Chat":{"payload":{"type":"string"}}},
            "schemas":{"User":true}, "messageTraits":{"Envelope":{}},
            "replies":{"Ack":{}}, "serverBindings":{"Ws":{"ws":{}}}
        }))
        .unwrap();
        let mut renamed = Vec::new();
        components
            .append_rekeyed(source, |group, name| {
                renamed.push(format!("{group}/{name}"));
                format!("Other{name}")
            })
            .unwrap();
        assert_eq!(
            renamed,
            [
                "schemas/User",
                "messages/Chat",
                "replies/Ack",
                "messageTraits/Envelope",
                "serverBindings/Ws"
            ]
        );
        let value = serde_json::to_value(&components).unwrap();
        assert_eq!(
            value["messages"],
            json!({"OtherChat":{"payload":{"type":"string"}}})
        );
        assert_eq!(value["schemas"]["OtherUser"], json!(true));
        assert_eq!(
            value["schemas"]["User"],
            chat["components"]["schemas"]["User"]
        );
        assert_eq!(value["messageTraits"]["OtherEnvelope"], json!({}));
    }
}
