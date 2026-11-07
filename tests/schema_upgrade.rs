#![cfg(feature = "lossless-schema-keywords")]
use oas::SchemaValue;
use serde_json::json;

#[test]
fn conversion_keeps_literals_and_uses_typed_nullable_over_fallback() {
    let mut value = SchemaValue::from_lossless_value(json!({
        "type":"string", "nullable":true, "format":"byte",
        "example":null, "exclusiveMinimum":true, "minimum":2,
        "x-data":{"nullable":true}
    }))
    .unwrap();
    let schema = value.as_object_mut().unwrap();
    schema.nullable = Some(false);
    schema.extras.insert("nullable".into(), json!(true));
    schema.upgrade_3_0().unwrap();
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        json!({
            "type":"string", "contentEncoding":"base64", "examples":[null],
            "exclusiveMinimum":2, "x-data":{"nullable":true}
        })
    );
    let mut value =
        SchemaValue::from_lossless_value(json!({"type":"string","nullable":true})).unwrap();
    let schema = value.as_object_mut().unwrap();
    schema.extras.insert("nullable".into(), json!(false));
    schema.upgrade_3_0().unwrap();
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        json!({"type":["string","null"]})
    );
}
