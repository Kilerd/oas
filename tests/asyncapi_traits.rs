#![cfg(feature = "lossless-schema-keywords")]
//! AsyncAPI 3.0 traits merge on typed nodes.
use oas::lossless_asyncapi::{Document, Message, MessageTrait, Operation, OperationTrait};
use oas::lossless_component::{Decode, Field};
use oas::merge_patch::{merge, MergePatch};
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt::Debug;

fn to_value(value: &impl Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}

/// The traits merge mechanism on JSON: traits in order, the object last.
fn json_merge(object: &Value, traits: &[Value]) -> Value {
    let mut merged = json!({});
    for item in traits {
        merge(&mut merged, item);
    }
    merge(&mut merged, object);
    merged
}

fn assert_traits<T, S>(object: Value, traits: Vec<Value>, apply: impl Fn(&mut T, Vec<S>))
where
    T: Decode + Serialize + Debug,
    S: Decode,
{
    let expected = json_merge(&object, &traits);
    let mut merged = T::decode(object).unwrap();
    apply(
        &mut merged,
        traits
            .into_iter()
            .map(|item| S::decode(item).unwrap())
            .collect(),
    );
    assert_eq!(to_value(&merged), expected);
    assert_eq!(
        format!("{merged:?}"),
        format!("{:?}", T::decode(expected).unwrap()),
        "typed placement"
    );
}

#[test]
fn traits_convert_to_their_objects_without_losing_keys() {
    let source = json!({
        "name": "n", "headers": {"type": "object"}, "examples": [{"payload": 1}],
        "payload": {"type": "string"}, "traits": [], "x-extension": 1, "unknown": {"a": 1}
    });
    let message = Message::from(MessageTrait::decode(source.clone()).unwrap());
    assert_eq!(to_value(&message), source);
    assert_eq!(
        format!("{message:?}"),
        format!("{:?}", Message::decode(source).unwrap())
    );
    assert!(matches!(message.payload, Field::Value(_)));
    assert!(!message.extras.contains_key("payload"));

    let source = json!({"summary": "s", "action": "send", "channel": {"$ref": "#/c"}, "security": [{"type": "http"}]});
    let operation = Operation::from(OperationTrait::decode(source.clone()).unwrap());
    assert_eq!(to_value(&operation), source);
    assert!(matches!(operation.channel, Field::Value(_)));
}

#[test]
fn message_traits_merge_in_order_and_the_message_wins() {
    assert_traits::<Message, MessageTrait>(
        json!({
            "name": "own", "payload": {"type": "object"},
            "headers": {"properties": {"b": {"type": "integer"}}, "required": ["b"]},
            "traits": [{"$ref": "#/components/messageTraits/a"}]
        }),
        vec![
            json!({
                "name": "first", "title": "first", "summary": "first", "contentType": "application/json",
                "headers": {"type": "object", "properties": {"a": {"type": "string"}}, "required": ["a"]},
                "bindings": {"ws": {"method": "GET"}, "mqtt": {"qos": 1}},
                "tags": [{"name": "first"}], "x-extension": {"keep": 1, "drop": 1}
            }),
            json!({
                "title": "second", "summary": null, "contentType": "text/plain",
                "bindings": {"ws": {"bindingVersion": "0.1.0"}, "mqtt": null},
                "tags": [{"name": "second"}], "x-extension": {"drop": null},
                "correlationId": {"location": "$message.header#/id"}
            }),
        ],
        |message, traits| {
            message
                .merge_traits(traits, &mut MergePatch::new())
                .unwrap()
        },
    );
}

#[test]
fn operation_traits_merge_in_order_and_the_operation_wins() {
    assert_traits::<Operation, OperationTrait>(
        json!({"action": "send", "channel": {"$ref": "#/channels/c"}, "summary": "own", "security": null}),
        vec![
            json!({"summary": "trait", "description": "d", "security": [{"type": "http"}], "bindings": {"ws": {}}}),
            json!({"description": null, "externalDocs": {"url": "https://e"}, "tags": [{"name": "t"}]}),
        ],
        |operation, traits| {
            operation
                .merge_traits(traits, &mut MergePatch::new())
                .unwrap()
        },
    );
}

#[test]
fn no_traits_leave_the_object_as_it_was() {
    let source = json!({"name": "n", "headers": {"type": "object"}, "x-a": null});
    let mut message = Message::decode(source.clone()).unwrap();
    message
        .merge_traits(Vec::new(), &mut MergePatch::new())
        .unwrap();
    assert_eq!(
        to_value(&message),
        source,
        "without traits nothing is merged"
    );
}

#[test]
fn a_policy_error_leaves_the_object_unchanged() {
    let source = json!({"name": "n", "bindings": {"ws": {"method": "GET"}}});
    let mut message = Message::decode(source.clone()).unwrap();
    let traits = vec![MessageTrait::decode(json!({"title": "t", "summary": "s"})).unwrap()];
    let mut left = 3;
    let result = message.merge_traits(
        traits,
        &mut MergePatch::charging(|| {
            if left == 0 {
                return Err("budget");
            }
            left -= 1;
            Ok(())
        }),
    );
    assert_eq!(result, Err("budget"));
    assert_eq!(to_value(&message), source);
}

fn resolve<'a>(components: &'a Value, group: &str, item: &'a Value) -> &'a Value {
    match item.get("$ref").and_then(Value::as_str) {
        Some(reference) => {
            let name = reference
                .strip_prefix(&format!("#/components/{group}/"))
                .unwrap();
            &components[group][name]
        }
        None => item,
    }
}

#[test]
fn fixture_traits_merge_as_their_json() {
    let source: Value = serde_json::from_str(include_str!("fixtures/asyncapi/chat.json")).unwrap();
    // Fidelity of the whole document is unchanged.
    assert_eq!(to_value(&Document::decode(source.clone()).unwrap()), source);
    let components = &source["components"];
    let mut checked = 0;
    for (pointer, group) in [
        ("/operations", "operationTraits"),
        ("/components/messages", "messageTraits"),
        ("/channels", "messageTraits"),
    ] {
        let mut objects: Vec<Value> = Vec::new();
        match source.pointer(pointer) {
            Some(Value::Object(entries)) if pointer == "/channels" => {
                for channel in entries.values() {
                    if let Some(Value::Object(messages)) = channel.get("messages") {
                        objects.extend(messages.values().cloned());
                    }
                }
            }
            Some(Value::Object(entries)) => objects.extend(entries.values().cloned()),
            _ => {}
        }
        for object in objects {
            let Some(Value::Array(traits)) = object.get("traits") else {
                continue;
            };
            let resolved: Vec<Value> = traits
                .iter()
                .map(|item| resolve(components, group, item).clone())
                .collect();
            let mut own = object.clone();
            own.as_object_mut().unwrap().remove("traits");
            if group == "operationTraits" {
                assert_traits::<Operation, OperationTrait>(own, resolved, |target, traits| {
                    target.merge_traits(traits, &mut MergePatch::new()).unwrap()
                });
            } else {
                assert_traits::<Message, MessageTrait>(own, resolved, |target, traits| {
                    target.merge_traits(traits, &mut MergePatch::new()).unwrap()
                });
            }
            checked += 1;
        }
    }
    assert!(checked >= 2, "{checked}");
}
