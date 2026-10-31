#![cfg(feature = "lossless-schema-keywords")]

use oas::lossless_component::{Decode, Operation};
use serde_json::{json, Value};

fn equivalent(input: Value) {
    let strict = serde_json::from_value::<oas::Operation>(input.clone());
    let admitted = Operation::decode(input.clone()).unwrap().admit();
    assert_eq!(
        admitted.is_ok(),
        strict.is_ok(),
        "acceptance for {input}: admitted={admitted:?}, strict={strict:?}"
    );
    if let (Ok(admitted), Ok(strict)) = (admitted, strict) {
        assert_eq!(
            serde_json::to_vec(&admitted).unwrap(),
            serde_json::to_vec(&strict).unwrap(),
            "output for {input}"
        );
    }
}

#[test]
fn existing_example_operations_match_strict_decoder() {
    let fixtures = [
        include_str!("../examples/v3.0/json/petstore.json"),
        include_str!("../examples/v3.0/json/petstore-expanded.json"),
        include_str!("../examples/v3.0/json/api-with-examples.json"),
        include_str!("../examples/v3.0/json/callback-example.json"),
        include_str!("../examples/v3.0/json/link-example.json"),
        include_str!("../examples/v3.0/json/uspto.json"),
        include_str!("../examples/v3.2/json/sse.json"),
    ];
    let mut count = 0;
    for fixture in fixtures {
        let document: Value = serde_json::from_str(fixture).unwrap();
        for path in document["paths"].as_object().unwrap().values() {
            for method in [
                "get", "put", "post", "delete", "options", "head", "patch", "trace",
            ] {
                if let Some(operation) = path.get(method) {
                    equivalent(operation.clone());
                    count += 1;
                }
            }
        }
    }
    assert!(count > 10);
}

#[test]
fn nested_models_and_literal_nulls_match_strict_decoder() {
    equivalent(json!({
        "tags": ["events"], "summary": "Subscribe", "description": null,
        "externalDocs": {"url":"https://example.test/docs", "$ref":null, "summary":7},
        "operationId":"subscribe", "deprecated": false,
        "security":[{"key":["read"]}, {}],
        "servers":[{"url":"https://{zone}.example.test", "variables":{"zone":{"default":"a", "enum":["a","b"], "description":null, "x-note":true}}}],
        "parameters":[{"name":"query", "in":"query", "required":false, "schema":{"type":"string", "description":null}, "itemSchema":{"type":7}, "example":null,
            "examples":{"named":{"value":null, "externalValue":null, "x-tag":1}},
            "content":{"text/plain":{"schema":true}}}],
        "requestBody":{"content":{"multipart/form-data":{"schema":{"type":"object", "properties":{"name":{"type":"string"}}},
            "example":null, "itemSchema":false,
            "encoding":{"name":{"contentType":"text/plain", "headers":{"X-Mode":{"schema":{"type":"string"}, "itemSchema":null, "example":null}}, "style":"form", "explode":true, "allowReserved":null}}}}},
        "responses":{"default":null, "200":{"description":"ok", "headers":{"X-Id":{"schema":{"type":"string"}, "examples":{"zero":{"value":0}}}},
            "links":{"next":{"operationRef":"#/paths/~1next/get", "operationId":null, "parameters":{"id":null}, "requestBody":null, "server":{"url":"/next"}, "summary":7}},
            "content":{"text/event-stream":{"schema":true, "itemSchema":{"type":"string"}, "example":null}}},
            "2XX":{"description":"success"}, "600":{"description":null, "headers":null}, "x-note":{"description":null}},
        "callbacks":{"event":{"https://example.test/{$request.body#/url}":{"$ref":"#/components/pathItems/Event", "summary":null,
            "post":{"responses":{"204":{"description":"done"}}}, "parameters":[], "servers":null}, "x-{literal}":{"post":7}}},
        "x-note":{"schema":{"type":7}}
    }));
}

#[test]
fn missing_invalid_and_optional_fields_match_strict_decoder() {
    for input in [
        json!({}),
        json!({"responses":null}),
        json!({"responses":{}}),
        json!({"responses":{"default":null}}),
        json!({"responses":{"200":null}}),
    ] {
        equivalent(input);
    }
    let ordinary = json!({"responses":{"200":{"description":"ok"}}});
    for key in [
        "tags",
        "summary",
        "description",
        "externalDocs",
        "operationId",
        "parameters",
        "requestBody",
        "callbacks",
        "deprecated",
        "security",
        "servers",
    ] {
        for value in [Value::Null, json!(17), json!({}), json!([])] {
            let mut input = ordinary.clone();
            input[key] = value;
            equivalent(input);
        }
    }
    for parameter in [
        json!({"name":"q", "in":"query"}),
        json!({"name":"q", "_in":"query"}),
        json!({"name":"q", "In":"query"}),
        json!({"name":"q", "in":"query", "In":"query"}),
        json!({"name":"q", "in":"query", "_in":"query"}),
        json!({"name":"q", "in":"unknown"}),
        json!({"name":null, "in":"query"}),
        json!({"name":"q", "in":null}),
        json!({"name":"q", "in":{"query":null}}),
        json!({"name":"q", "in":"query", "schema":{"type":17}}),
        json!({"name":"q", "in":"query", "schema":{"type":null}}),
    ] {
        equivalent(json!({"responses":{}, "parameters":[parameter]}));
    }
    for schema in [
        json!({"type":"string"}),
        json!({"format":null}),
        json!({"nullable":"yes"}),
        json!({"contentSchema":{"type":17}}),
        json!({"properties":{"one":{"type":17}}}),
    ] {
        equivalent(
            json!({"responses":{"200":{"description":"ok", "content":{"application/json":{"schema":schema}}}}}),
        );
    }
}

fn with_child(kind: &str, child: Value) -> Value {
    match kind {
        "response" => json!({"responses":{"200":child}}),
        "header" => json!({"responses":{"200":{"description":"ok", "headers":{"X-Id":child}}}}),
        "parameter" => json!({"responses":{}, "parameters":[child]}),
        "requestBody" => json!({"responses":{}, "requestBody":child}),
        "example" => {
            json!({"responses":{"200":{"description":"ok", "content":{"application/json":{"examples":{"one":child}}}}}})
        }
        "link" => json!({"responses":{"200":{"description":"ok", "links":{"next":child}}}}),
        "callback" => json!({"responses":{}, "callbacks":{"next":child}}),
        _ => unreachable!(),
    }
}

#[test]
fn reference_candidates_preserve_siblings_and_inline_precedence() {
    for kind in [
        "response",
        "header",
        "parameter",
        "requestBody",
        "example",
        "link",
        "callback",
    ] {
        for child in [
            json!({"$ref":"#/components/example"}),
            json!({"$ref":"#/components/example", "summary":"short", "description":null}),
            json!({"$ref":"#/components/example", "summary":17}),
            json!({"$ref":null, "description":"ok"}),
            json!({"$ref":"#/components/example", "x-note":{"nested":true}}),
            json!({"$ref":"#/components/example", "description":"ok", "name":"q", "in":"query", "content":{}, "value":null}),
            json!({"$ref":"#/components/example", "description":"ok", "name":"q", "in":"query", "schema":{"type":17}, "content":null, "value":null}),
            json!({"$ref":"#/components/example", "description":"ok", "name":"q", "in":"query", "schema":{"description":null}, "content":{}}),
            json!({"$ref":"#/components/example", "{$request.body#/url}":{"post":{"responses":{}}}}),
            json!({"$ref":"#/components/example", "{$request.body#/url}":{"post":null}}),
            json!({"$ref":"#/components/example", "{$request.body#/url}":{"post":{}}}),
            json!({"$ref":"#/components/example", "description":"ok", "content":{"application/json":{"schema":{"type":17}}}}),
            json!({"$ref":"#/components/example", "description":"ok", "content":{"application/json":{"schema":{"contentSchema":{"format":17}}}}}),
        ] {
            equivalent(with_child(kind, child));
        }
    }
}

#[test]
fn callback_reference_fields_remain_editable_with_inline_children() {
    use oas::lossless_component::Field;
    for reference in [json!("#/components/callbacks/Event"), Value::Null, json!(7)] {
        let input = json!({"$ref":reference, "{$request.body#/url}":{
            "post":{"responses":{"204":{"description":"done"}}}
        }, "x-note":{"$ref":"literal"}});
        let mut standalone: oas::Callback = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(&standalone).unwrap(), input);
        let operation = Operation::decode(with_child("callback", input.clone()))
            .unwrap()
            .admit()
            .unwrap();
        let oas::Referenceable::Data(mut admitted) =
            operation.callbacks.unwrap().remove("next").unwrap()
        else {
            panic!("valid inline callback children take precedence");
        };
        for callback in [&mut standalone, &mut admitted] {
            match (&reference, &callback.reference) {
                (Value::String(expected), Field::Value(actual)) => assert_eq!(actual, expected),
                (_, Field::Invalid(actual)) => assert_eq!(actual, &reference),
                _ => panic!("reference presence and shape must be retained"),
            }
            callback.reference = Field::Value("#/components/callbacks/PublicEvent".into());
            let mut expected = input.clone();
            expected["$ref"] = json!("#/components/callbacks/PublicEvent");
            assert_eq!(serde_json::to_value(callback).unwrap(), expected);
        }
    }
    let missing: oas::Callback = serde_json::from_value(json!({})).unwrap();
    assert!(missing.reference.is_missing());
    assert_eq!(serde_json::to_value(missing).unwrap(), json!({}));
}

#[test]
fn ordinary_nested_field_acceptance_matches_strict_decoder() {
    for (kind, base, fields) in [
        (
            "response",
            json!({"description":"ok"}),
            &["description", "headers", "content", "links"][..],
        ),
        (
            "header",
            json!({}),
            &[
                "description",
                "required",
                "deprecated",
                "allowEmptyValue",
                "style",
                "explode",
                "allowReserved",
                "schema",
                "examples",
                "content",
            ][..],
        ),
        (
            "parameter",
            json!({"name":"q","in":"query"}),
            &[
                "description",
                "required",
                "deprecated",
                "allowEmptyValue",
                "style",
                "explode",
                "allowReserved",
                "schema",
                "examples",
                "content",
            ][..],
        ),
        (
            "requestBody",
            json!({"content":{}}),
            &["description", "required", "content"][..],
        ),
        (
            "example",
            json!({}),
            &["summary", "description", "externalValue", "value"][..],
        ),
        (
            "link",
            json!({}),
            &[
                "operationRef",
                "operationId",
                "parameters",
                "requestBody",
                "description",
                "server",
            ][..],
        ),
    ] {
        for field in fields {
            for value in [Value::Null, json!(17), json!({}), json!([])] {
                let mut child = base.clone();
                child[*field] = value;
                equivalent(with_child(kind, child));
            }
        }
    }
}
