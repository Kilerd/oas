#![cfg(feature = "lossless-schema-keywords")]
use oas::lossless_component::{Component, ComponentKind, Decode, Field, PathItem, Response};
use serde_json::json;

#[test]
fn selected_component_keeps_missing_null_and_invalid_fields_distinct() {
    let source = json!({"type":"http","scheme":"bearer","description":null,"flows":{"implicit":{"scopes":{"read":null}}},"x-doc":{"$ref":"literal"}});
    let component = Component::from_value(ComponentKind::SecurityScheme, source.clone()).unwrap();
    let Component::SecurityScheme(scheme) = &component else {
        panic!("security scheme");
    };
    assert!(matches!(
        scheme.description,
        Field::Invalid(serde_json::Value::Null)
    ));
    assert!(scheme.name.is_missing());
    assert_eq!(scheme.scheme.as_value().unwrap(), "bearer");
    assert_eq!(serde_json::to_value(&component).unwrap(), source);
}

#[test]
fn sibling_fields_do_not_force_response_admission() {
    let source = json!({"headers":{"Good":{"schema":{"type":"string"}},"Bad":null},"content":{"application/json":{"schema":{"properties":{"value":{"type":"string"}},"description":null}}}});
    let mut response = Response::decode(source.clone()).unwrap();
    assert!(response.description.is_missing());
    let headers = response.headers.as_value_mut().unwrap();
    assert!(matches!(
        headers["Bad"],
        Field::Invalid(serde_json::Value::Null)
    ));
    let schema = headers
        .get_mut("Good")
        .unwrap()
        .as_value_mut()
        .unwrap()
        .schema
        .as_value_mut()
        .unwrap();
    schema.as_object_mut().unwrap().description = Some("edited".into());
    let mut expected = source;
    expected["headers"]["Good"]["schema"]["description"] = "edited".into();
    assert_eq!(serde_json::to_value(response).unwrap(), expected);
}

#[test]
fn path_selection_keeps_unselected_operation_and_callback_literals() {
    let source = json!({"get":{"responses":{"200":{"description":"ok"}},"callbacks":{"Event":{"https://example.test/{id}":{"post":{"responses":{"204":{}}}},"x-{literal}":{"post":7}}}},"post":null,"servers":[{"url":null,"variables":{"zone":{"default":"a","enum":["a",null]}}}]});
    let path = PathItem::decode(source.clone()).unwrap();
    assert!(matches!(path.post, Field::Invalid(serde_json::Value::Null)));
    let callback = path.get.as_value().unwrap().callbacks.as_value().unwrap()["Event"]
        .as_value()
        .unwrap();
    assert_eq!(callback.paths.len(), 1);
    assert!(callback.extras.contains_key("x-{literal}"));
    assert_eq!(serde_json::to_value(path).unwrap(), source);
}

#[test]
fn borrowed_children_preserve_order_and_report_invalid_structure() {
    use oas::lossless_component::{Node, StructuralNode};
    let response = Response::decode(json!({
        "content":{"text/plain":{"schema":{"type":"string"}}},
        "headers":{"A":{"$ref":"#/components/headers/A"},"B":null},
        "links":{"Next":{"operationId":"next"}},
        "x-literal":{"$ref":"not a structural reference"}
    }))
    .unwrap();
    let mut observed = Vec::new();
    response
        .node()
        .try_for_each_child(|key, child| {
            let detail = match &child {
                Node::MediaType(_) => "media",
                Node::Header(_) => child.reference().and_then(Field::as_value).unwrap(),
                Node::Invalid(_) => "invalid",
                Node::Link(link) => link.operation_id.as_value().unwrap(),
                _ => panic!("unexpected response child"),
            };
            observed.push((key.to_owned(), detail.to_owned()));
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(
        observed,
        [
            ("content".into(), "media".into()),
            ("headers".into(), "#/components/headers/A".into()),
            ("headers".into(), "invalid".into()),
            ("links".into(), "next".into()),
        ]
    );
    let mut visited = Vec::new();
    let result = response.node().try_for_each_child(|key, _| {
        visited.push(key.to_owned());
        if key == "headers" {
            Err("stop")
        } else {
            Ok(())
        }
    });
    assert_eq!(result, Err("stop"));
    assert_eq!(visited, ["content", "headers"]);
}

#[test]
fn shallow_overlay_preserves_null_and_replaces_entire_present_field() {
    let mut target = Response::decode(json!({"description":"target","content":{"application/json":{"schema":{"type":"string"}}},"headers":{"Target":{"description":"kept?"}},"x-owner":{"a":1}})).unwrap();
    let siblings =
        Response::decode(json!({"description":null,"headers":{"Sibling":null},"x-owner":{"b":2}}))
            .unwrap();
    target.overlay(siblings);
    assert_eq!(
        serde_json::to_value(target).unwrap(),
        json!({
            "description":null,"content":{"application/json":{"schema":{"type":"string"}}},
            "headers":{"Sibling":null},"x-owner":{"b":2}
        })
    );
}
