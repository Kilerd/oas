use oas::{
    Example, Header, MediaType, OpenAPIV3, Parameter, Reference, Referenceable, Response, Schema,
    SchemaType, SchemaValue,
};
use serde_json::json;

/// Optional inline fields previously allowed pure references to deserialize as empty objects.
#[test]
fn document_preserves_header_and_example_references() {
    let document = json!({
        "openapi": "3.1.0",
        "info": {"title": "References", "version": "1"},
        "paths": {
            "/items": {"get": {"responses": {"200": {
                "description": "Items",
                "headers": {"X-Request-Id": {"$ref": "#/components/headers/RequestId"}},
                "content": {"application/json": {
                    "examples": {"item": {"$ref": "#/components/examples/Item"}}
                }}
            }}}}
        },
        "components": {
            "headers": {"RequestId": {"schema": {"type": "string"}}},
            "examples": {"Item": {"value": {"id": 1}}}
        }
    });
    let api: OpenAPIV3 = serde_json::from_value(document.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&api).unwrap(), document,
        "document references must round-trip intact, otherwise typed editing changes their targets or annotations"
    );
    let response = api.paths["/items"].get.as_ref().unwrap().responses.data["200"]
        .as_data()
        .unwrap();
    assert!(response.headers.as_ref().unwrap()["X-Request-Id"].is_reference());
    assert!(response.content.as_ref().unwrap()["application/json"]
        .examples
        .as_ref()
        .unwrap()["item"]
        .is_reference());
}

/// A reference description also matches an inline Response field and must not hide its target.
#[test]
fn document_preserves_response_reference_annotations() {
    let document = json!({
        "openapi": "3.1.0",
        "info": {"title": "References", "version": "1"},
        "paths": {"/items": {"get": {"responses": {
            "400": {"$ref": "#/components/responses/Error", "summary": "Bad input"},
            "500": {
                "$ref": "#/components/responses/Error",
                "summary": "Failed",
                "description": "This endpoint's error"
            }
        }}}},
        "components": {"responses": {"Error": {"description": "An error"}}}
    });
    let api: OpenAPIV3 = serde_json::from_value(document.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&api).unwrap(), document,
        "document references must round-trip intact, otherwise typed editing changes their targets or annotations"
    );
    let responses = &api.paths["/items"].get.as_ref().unwrap().responses;
    assert!(responses.data["400"].is_reference());
    assert!(responses.data["500"].is_reference());
}

/// Extra reference properties previously either rejected the document or disappeared on output.
#[test]
fn document_preserves_parameter_reference_unknown_properties() {
    let input = json!({
        "openapi": "3.1.0",
        "info": {"title": "References", "version": "1"},
        "paths": {"/items": {"get": {
            "parameters": [{
                "$ref": "#/components/parameters/Id",
                "summary": "Item ID",
                "description": "The selected item",
                "x-note": true,
                "unrecognized": {"nested": [1, false]}
            }],
            "responses": {"200": {"description": "Success"}}
        }}},
        "components": {"parameters": {"Id": {
            "name": "id", "in": "query", "schema": {"type": "string"}
        }}}
    });
    let api: OpenAPIV3 = serde_json::from_value(input).unwrap();
    let parameter = &api.paths["/items"]
        .get
        .as_ref()
        .unwrap()
        .parameters
        .as_ref()
        .unwrap()[0];
    assert!(parameter.is_reference());
    assert_eq!(
        serde_json::to_value(parameter).unwrap(),
        json!({
            "$ref": "#/components/parameters/Id",
            "summary": "Item ID",
            "description": "The selected item",
            "x-note": true,
            "unrecognized": {"nested": [1, false]}
        }),
        "reference siblings must survive nested document parsing, otherwise editing loses unmodeled content"
    );
}

/// The consumer lost x-public while editing a response reference through the typed model.
#[test]
fn response_reference_retains_consumer_siblings() {
    let input = json!({
        "$ref": "#/components/responses/Ok",
        "summary": "override",
        "x-public": {"order": [2, 1], "$ref": "literal"},
        "unrecognized": [false, null, {"$ref": "also literal"}],
        "extras": {"literal": true}
    });
    let reference: Referenceable<Response> = serde_json::from_value(input.clone()).unwrap();
    assert!(reference.is_reference());
    assert_eq!(
        serde_json::to_value(reference).unwrap(),
        input,
        "reference editing must retain every sibling, otherwise consumer metadata is silently lost"
    );
}

/// Schema-bearing builders previously dropped reference annotations and unknown siblings.
#[test]
fn reference_metadata_survives_conversion_to_schema_fields() {
    let reference: Reference = serde_json::from_value(json!({
        "$ref": "#/components/schemas/Item",
        "summary": "An item",
        "description": "An item returned here",
        "x-public": {"$ref": "literal", "order": [2, 1]},
        "unrecognized": false
    }))
    .unwrap();
    let media = MediaType::new().with_schema(Referenceable::<Schema>::Reference(reference));
    assert_eq!(
        serde_json::to_value(media).unwrap(),
        json!({
            "schema": {
                "$ref": "#/components/schemas/Item",
                "summary": "An item",
                "description": "An item returned here",
                "x-public": {"$ref": "literal", "order": [2, 1]},
                "unrecognized": false
            }
        }),
        "schema-bearing builders must retain reference metadata, otherwise conversion loses source fields"
    );
}

/// Reference siblings that are schema keywords must support ordinary typed edits.
#[test]
fn converted_reference_schema_fields_behave_like_direct_schema_fields() {
    let input = json!({
        "$ref": "#/components/schemas/Message",
        "type": "string",
        "format": "byte",
        "contentSchema": {"type": "object", "x-note": true},
        "x-public": {"order": [2, 1]}
    });
    let reference: Reference = serde_json::from_value(input.clone()).unwrap();
    let mut converted: SchemaValue = reference.into();
    let mut direct: SchemaValue = serde_json::from_value(input).unwrap();
    let schema = converted.as_object().unwrap();
    assert!(
        schema._type.is_some() && schema.format.is_some() && schema.content_schema.is_some(),
        "schema keywords must populate typed fields, otherwise ordinary edits compete with extras"
    );
    for value in [&mut converted, &mut direct] {
        let schema = value.as_object_mut().unwrap();
        schema._type = Some("object".into());
        schema.format = Some("custom".into());
        schema.content_schema = Some(Box::new(false.into()));
    }
    let expected = json!({
        "$ref": "#/components/schemas/Message",
        "type": "object",
        "format": "custom",
        "contentSchema": false,
        "x-public": {"order": [2, 1]}
    });
    for value in [converted, direct] {
        assert_eq!(
            serde_json::to_value(value).unwrap(),
            expected,
            "edited converted and direct schemas must stay identical, otherwise retained extras compete with typed edits"
        );
    }
}

/// Reference Object siblings are arbitrary values, even when named like schema fields.
#[test]
fn schema_conversion_preserves_reference_siblings_that_cannot_be_promoted_losslessly() {
    for sibling in [json!(null), json!({"label": "application metadata"})] {
        let input = json!({
            "$ref": "#/components/schemas/Message",
            "format": sibling,
            "x-public": {"order": [2, 1]}
        });
        let reference: Reference = serde_json::from_value(input.clone()).unwrap();
        let converted: SchemaValue = reference.into();
        assert_eq!(
            serde_json::to_value(converted).unwrap(),
            input,
            "infallible conversion must retain arbitrary reference siblings, otherwise editing loses source content"
        );
    }
}

#[cfg(feature = "lossless-schema-keywords")]
#[test]
fn reference_conversion_promotes_valid_fields_beside_opaque_siblings() {
    let input = json!({
        "$ref": "#/components/schemas/Message",
        "format": null,
        "properties": {"event": {"type": "string"}},
        "contentSchema": {"type": null}
    });
    let reference: Reference = serde_json::from_value(input.clone()).unwrap();
    let mut converted: SchemaValue = reference.into();
    assert_eq!(serde_json::to_value(&converted).unwrap(), input);
    let schema = converted.as_object_mut().unwrap();
    schema
        .properties
        .as_mut()
        .unwrap()
        .insert("event".into(), false.into());
    let mut expected = input;
    expected["properties"]["event"] = json!(false);
    assert_eq!(serde_json::to_value(converted).unwrap(), expected);
}

/// Adding reference metadata must not change existing constructors' serialized bytes.
#[test]
fn constructors_keep_pure_reference_wire_format() {
    let reference: Referenceable<Header> =
        Referenceable::reference("#/components/headers/RequestId");
    let component: Referenceable<Response> = Referenceable::component_ref("responses", "Error");
    for (serialized, expected) in [
        (
            serde_json::to_string(&Reference::new("#/components/headers/RequestId")).unwrap(),
            r##"{"$ref":"#/components/headers/RequestId"}"##,
        ),
        (
            serde_json::to_string(&reference).unwrap(),
            r##"{"$ref":"#/components/headers/RequestId"}"##,
        ),
        (
            serde_json::to_string(&component).unwrap(),
            r##"{"$ref":"#/components/responses/Error"}"##,
        ),
    ] {
        assert_eq!(
            serialized, expected,
            "pure reference constructors must retain their bytes, otherwise metadata additions change existing output"
        );
    }
}

/// JSON Schema keeps `$ref` siblings as keywords; every other reference
/// position follows the OpenAPI Reference Object rule.
#[test]
fn schema_siblings_stay_data_and_non_schema_refs_are_references() {
    let schema = json!({
        "$ref": "#/components/schemas/Item",
        "description": "An item",
        "type": "object",
        "properties": {"id": {"type": "integer"}},
        "required": ["id"]
    });
    let legacy: Referenceable<Schema> = serde_json::from_value(schema.clone()).unwrap();
    assert!(
        legacy.is_data(),
        "schema siblings must remain inline data, otherwise typed schema access changes meaning"
    );
    assert_eq!(serde_json::to_value(legacy).unwrap(), schema);
    let modern: SchemaValue = serde_json::from_value(schema.clone()).unwrap();
    assert_eq!(serde_json::to_value(modern).unwrap(), schema);

    // Outside schema positions a string `$ref` makes the object a Reference
    // Object; extra properties are retained for editing but never parsed inline.
    let inline = json!({"$ref": "application-specific", "value": 42});
    let example: Referenceable<Example> = serde_json::from_value(inline.clone()).unwrap();
    assert!(
        example.is_reference(),
        "an object with a string $ref must not be reclassified as inline data"
    );
    assert_eq!(example.as_reference().unwrap()._ref, "application-specific");
    assert_eq!(serde_json::to_value(example).unwrap(), inline);
    let literal = json!({"$ref": "literal", "x-public": {"$ref": "nested", "order": [2, 1]}});
    let inline: Referenceable<serde_json::Value> = serde_json::from_value(literal.clone()).unwrap();
    assert!(inline.is_reference());
    assert_eq!(serde_json::to_value(inline).unwrap(), literal);
    let inline: Referenceable<serde_json::Value> =
        Referenceable::data(json!({"$ref": "application-specific", "value": 42}));
    assert_eq!(
        serde_json::to_value(inline).unwrap(),
        json!({
            "$ref": "application-specific", "value": 42
        })
    );
}

/// OpenAPI 3.1 §4.8.23: a Reference Object cannot be extended, so properties
/// that would fit the inline object must not reclassify it as data.
#[test]
fn reference_objects_with_extra_properties_stay_references() {
    let input = json!({
        "$ref": "#/components/responses/NotFound",
        "description": "nf",
        "x-note": 1
    });
    let response: Referenceable<Response> = serde_json::from_value(input.clone()).unwrap();
    assert!(
        response.is_reference(),
        "a response with `$ref` and a description is a Reference Object, not an inline response"
    );
    let reference = response.as_reference().unwrap();
    assert_eq!(reference._ref, "#/components/responses/NotFound");
    assert_eq!(reference.description.as_deref(), Some("nf"));
    assert_eq!(reference.extras["x-note"], json!(1));
    assert_eq!(serde_json::to_value(&response).unwrap(), input);

    let header: Referenceable<Header> = serde_json::from_value(json!({
        "$ref": "#/components/headers/Rate", "x-note": 1
    }))
    .unwrap();
    assert!(header.is_reference());

    let parameter: Referenceable<Parameter> = serde_json::from_value(json!({
        "$ref": "#/components/parameters/Id", "name": "q", "in": "query", "x-note": 1
    }))
    .unwrap();
    assert!(parameter.is_reference());

    // A non-string `$ref` is not a Reference Object; the inline object wins.
    let inline: Referenceable<Response> = serde_json::from_value(json!({
        "$ref": null, "description": "ok"
    }))
    .unwrap();
    assert!(inline.is_data());

    // JSON Schema 2020-12 gives `$ref` siblings meaning, so schema positions
    // keep the sibling keywords as schema fields.
    let schema = json!({
        "$ref": "#/components/schemas/Item",
        "description": "an item",
        "type": "object",
        "x-note": 1
    });
    let value: SchemaValue = serde_json::from_value(schema.clone()).unwrap();
    let object = value
        .as_object()
        .expect("a schema `$ref` with siblings stays an inline schema");
    assert_eq!(object._ref.as_deref(), Some("#/components/schemas/Item"));
    assert_eq!(object._type, Some(SchemaType::Single("object".into())));
    assert_eq!(serde_json::to_value(value).unwrap(), schema);

    // The legacy schema target keeps the same schema-first rule.
    let legacy: Referenceable<Schema> = serde_json::from_value(schema.clone()).unwrap();
    assert!(legacy.is_data());
    assert_eq!(serde_json::to_value(legacy).unwrap(), schema);
}

/// Reference parsing must remain usable with Serde inputs other than JSON text.
#[test]
fn reference_shape_is_supported_by_non_json_deserializers() {
    use serde::de::value::{Error, MapDeserializer};
    use serde::Deserialize;
    let fields = [
        ("$ref", "#/components/responses/Error"),
        ("summary", "Failure"),
        ("description", "Endpoint failure"),
    ];
    let deserializer = MapDeserializer::<_, Error>::new(fields.into_iter());
    let reference = Referenceable::<Response>::deserialize(deserializer).unwrap();
    let reference = reference.as_reference().unwrap();
    assert_eq!(
        reference._ref, "#/components/responses/Error",
        "Serde map inputs must retain reference targets, otherwise parsing becomes JSON-only"
    );
    assert_eq!(reference.summary.as_deref(), Some("Failure"));
    assert_eq!(reference.description.as_deref(), Some("Endpoint failure"));
}

/// Whole-Value buffering previously broke borrowed, Deserialize-only generic payloads.
#[test]
fn generic_inline_data_still_borrows_from_input() {
    #[derive(serde::Deserialize)]
    struct Borrowed<'a> {
        value: &'a str,
    }
    let value: Referenceable<Borrowed<'_>> =
        serde_json::from_str(r#"{"value":"borrowed"}"#).unwrap();
    assert_eq!(value.as_data().unwrap().value, "borrowed", "generic inline data must borrow from its input, otherwise existing Deserialize-only callers break");
}
