use oas::{
    builders, Components, Header, Info, MediaType, OpenAPIV3, Parameter, ParameterIn,
    Referenceable, RequestBody, Schema, SchemaType, SchemaValue,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn round_trip<T: DeserializeOwned + Serialize>(value: Value) -> T {
    let model: T = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(&model).unwrap(), value);
    model
}

#[test]
fn openapi_3_2_sse_document_retains_event_contract() {
    let api = round_trip::<OpenAPIV3>(
        serde_json::from_str(include_str!("../examples/v3.2/json/sse.json")).unwrap(),
    );
    let response = api.paths["/events"].get.as_ref().unwrap().responses.data["200"]
        .as_data()
        .unwrap();
    let media = &response.content.as_ref().unwrap()["text/event-stream"];
    let item = media.item_schema.as_ref().unwrap().as_object().unwrap();
    assert_eq!(item._ref.as_deref(), Some("#/components/schemas/Event"));
    assert_eq!(item.extras["properties"]["event"]["const"], "job.updated");
    assert_eq!(
        media.schema.as_ref().unwrap().as_object().unwrap()._type,
        Some(SchemaType::Single("array".into()))
    );

    let schemas = api.components.unwrap().schemas.unwrap();
    let event = schemas["Event"].as_object().unwrap();
    let fields = &event.extras["properties"];
    for name in ["event", "id", "data"] {
        assert_eq!(fields[name]["type"], "string");
    }
    assert_eq!(fields["retry"]["type"], "integer");
    let data = round_trip::<SchemaValue>(fields["data"].clone());
    let data = data.as_object().unwrap();
    assert_eq!(data.content_media_type.as_deref(), Some("application/json"));
    assert_eq!(
        data.content_schema
            .as_ref()
            .unwrap()
            .as_object()
            .unwrap()
            ._ref
            .as_deref(),
        Some("#/components/schemas/Job")
    );
    assert!(matches!(schemas["Anything"], SchemaValue::Boolean(true)));
    assert!(matches!(schemas["Nothing"], SchemaValue::Boolean(false)));
}

#[test]
fn json_schema_keywords_and_reference_siblings_round_trip() {
    for value in [
        json!(true),
        json!(false),
        json!({}),
        json!({"type": "null"}),
        json!({"type": ["string", "null"]}),
        json!({"type": ["string"]}),
        json!({"$ref": "other.json#/$defs/Value", "type": ["number", "null"],
            "description": "Reference with constraints", "minimum": 1}),
        json!({"$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/node", "$anchor": "node", "$dynamicAnchor": "tree",
            "$defs": {"Value": false}, "$dynamicRef": "#tree",
            "allOf": [true, {"not": false}], "anyOf": [false, {"type": "null"}],
            "oneOf": [{"const": null}, {"type": "string"}],
            "if": {"required": ["kind"]}, "then": true, "else": false,
            "properties": {"value": true}, "patternProperties": {"^x-": false},
            "additionalProperties": false, "unevaluatedProperties": false,
            "dependentSchemas": {"kind": {"required": ["value"]}},
            "dependentRequired": {"kind": ["value"]}, "propertyNames": true,
            "prefixItems": [true, {"type": "integer"}], "items": false,
            "contains": {"type": "number"}, "minContains": 1, "maxContains": 2,
            "unevaluatedItems": false, "exclusiveMinimum": 0.5, "exclusiveMaximum": 10,
            "examples": [null, 1], "default": null, "const": null,
            "customVocabularyKeyword": {"preserved": true}}),
        json!({"type": "string", "contentEncoding": "base64",
            "contentMediaType": "application/json", "contentSchema": false}),
        json!({"type": "string", "contentSchema": true}),
    ] {
        round_trip::<SchemaValue>(value);
    }
}

#[test]
fn every_schema_position_accepts_booleans_unions_and_references() {
    for schema in [
        json!(true),
        json!(false),
        json!({"type": ["integer", "null"]}),
        json!({"$ref": "#/components/schemas/Count", "minimum": 0}),
    ] {
        round_trip::<Components>(json!({"schemas": {"Count": schema}}));
        round_trip::<Header>(json!({"schema": schema}));
        round_trip::<Parameter>(json!({"name": "count", "in": "query", "schema": schema}));
        round_trip::<MediaType>(json!({"schema": schema, "itemSchema": schema}));
        round_trip::<RequestBody>(json!({"content": {"application/json": {"schema": schema}}}));
    }
}

#[test]
fn schema_constructors_distinguish_boolean_types_from_boolean_schemas() {
    assert_eq!(
        serde_json::to_value(Schema::boolean()).unwrap(),
        json!({"type": "boolean"})
    );
    assert_eq!(
        serde_json::to_value(SchemaValue::from(false)).unwrap(),
        json!(false)
    );
    assert_eq!(
        serde_json::to_value(Schema::null()).unwrap(),
        json!({"type": "null"})
    );
    let media = MediaType::new()
        .with_schema(Schema::array())
        .with_item_schema(
            Schema::string()
                .with_types(["string", "null"])
                .with_content_encoding("base64")
                .with_content_media_type("application/json")
                .with_content_schema(
                    Schema::reference("#/components/schemas/Payload")
                        .with_description("Decoded payload"),
                ),
        );
    assert_eq!(
        serde_json::to_value(media).unwrap(),
        json!({
            "schema": {"type": "array"},
            "itemSchema": {"type": ["string", "null"], "contentEncoding": "base64",
                "contentMediaType": "application/json", "contentSchema": {
                    "$ref": "#/components/schemas/Payload", "description": "Decoded payload"}}
        })
    );
    assert_eq!(
        serde_json::to_value(MediaType::new().with_item_schema(false)).unwrap(),
        json!({"itemSchema": false})
    );
}

#[test]
fn existing_3_0_constructors_keep_their_wire_behavior() {
    assert_eq!(builders::api("Legacy", "1").openapi, "3.0.0");
    assert_eq!(OpenAPIV3::new(Info::new("Legacy", "1")).openapi, "3.0.0");
    assert_eq!(
        OpenAPIV3::new_v3_1(Info::new("Modern", "1")).openapi,
        "3.1.0"
    );
    assert_eq!(
        OpenAPIV3::new_v3_2(Info::new("Stream", "1")).openapi,
        "3.2.0"
    );
    let mut schema = Schema::string()
        .with_format("uuid")
        .with_description("Legacy ID");
    schema.nullable = Some(true);
    let media = MediaType::new().with_schema(Referenceable::data(schema.clone()));
    assert_eq!(
        serde_json::to_value(media).unwrap(),
        json!({"schema": {
            "type": "string", "format": "uuid", "description": "Legacy ID", "nullable": true
        }})
    );
    let components = Components::new().with_schemas(BTreeMap::from([
        ("ID".into(), Referenceable::data(schema)),
        ("Alias".into(), Referenceable::schema_ref("ID")),
    ]));
    let mut modern =
        OpenAPIV3::new_v3_2(Info::new("No implicit migration", "1")).with_components(components);
    // Selecting a document version never rewrites schema keywords.
    assert_eq!(
        serde_json::to_value(&modern).unwrap()["components"]["schemas"]["ID"]["nullable"],
        true
    );
    modern.openapi = "3.0.0".into();
    round_trip::<OpenAPIV3>(serde_json::to_value(modern).unwrap());

    let legacy_ref = Referenceable::schema_ref("ID");
    assert_eq!(
        serde_json::to_value(MediaType::new().with_schema(legacy_ref)).unwrap(),
        json!({"schema": {"$ref": "#/components/schemas/ID"}})
    );
    let body = Referenceable::json_body(Referenceable::string_schema());
    assert_eq!(
        serde_json::to_value(body).unwrap(),
        json!({"content": {"application/json": {"schema": {"type": "string"}}}})
    );
    let parameter =
        Parameter::new("name", ParameterIn::Query).with_schema(Referenceable::string_schema());
    assert_eq!(
        serde_json::to_value(parameter.schema).unwrap(),
        json!({"type": "string"})
    );
    for (schema, expected) in [
        (Schema::string(), "string"),
        (Schema::integer(), "integer"),
        (Schema::number(), "number"),
        (Schema::boolean(), "boolean"),
        (Schema::array(), "array"),
        (Schema::object(), "object"),
    ] {
        assert_eq!(
            serde_json::to_value(schema).unwrap(),
            json!({"type": expected})
        );
    }
}

#[test]
fn existing_3_0_nullable_and_boolean_exclusive_bounds_are_not_rewritten() {
    round_trip::<SchemaValue>(json!({"type": "number", "nullable": true,
        "minimum": 0, "maximum": 10, "exclusiveMinimum": true, "exclusiveMaximum": false,
        "example": null, "additionalProperties": false}));
}

#[test]
fn non_schema_values_are_rejected_without_enabling_boolean_responses() {
    for invalid in [
        json!(null),
        json!(42),
        json!("string"),
        json!([]),
        json!([true]),
    ] {
        assert!(serde_json::from_value::<SchemaValue>(invalid).is_err());
    }
    for invalid in [
        json!({"type": false}),
        json!({"type": ["string", 42]}),
        json!({"contentSchema": 42}),
    ] {
        assert!(serde_json::from_value::<SchemaValue>(invalid).is_err());
    }
    assert!(serde_json::from_value::<Referenceable<oas::Response>>(json!(false)).is_err());
}
