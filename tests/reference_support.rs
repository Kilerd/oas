use oas::{
    Example, Header, MediaType, OpenAPIV3, Reference, Referenceable, Response, Schema, SchemaValue,
};
use serde_json::json;

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
    assert_eq!(serde_json::to_value(&api).unwrap(), document);
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
    assert_eq!(serde_json::to_value(&api).unwrap(), document);
    let responses = &api.paths["/items"].get.as_ref().unwrap().responses;
    assert!(responses.data["400"].is_reference());
    assert!(responses.data["500"].is_reference());
}

#[test]
fn document_loads_parameter_references_with_ignored_properties() {
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
            "description": "The selected item"
        })
    );
}

#[test]
fn reference_metadata_survives_conversion_to_schema_fields() {
    let reference: Reference = serde_json::from_value(json!({
        "$ref": "#/components/schemas/Item",
        "summary": "An item",
        "description": "An item returned here"
    }))
    .unwrap();
    let media = MediaType::new().with_schema(Referenceable::<Schema>::Reference(reference));
    assert_eq!(
        serde_json::to_value(media).unwrap(),
        json!({
            "schema": {
                "$ref": "#/components/schemas/Item",
                "summary": "An item",
                "description": "An item returned here"
            }
        })
    );
}

#[test]
fn constructors_keep_pure_reference_wire_format() {
    assert_eq!(
        serde_json::to_value(Reference::new("#/components/headers/RequestId")).unwrap(),
        json!({"$ref": "#/components/headers/RequestId"})
    );
    let reference: Referenceable<Header> =
        Referenceable::reference("#/components/headers/RequestId");
    assert_eq!(
        serde_json::to_value(reference).unwrap(),
        json!({
            "$ref": "#/components/headers/RequestId"
        })
    );
    let reference: Referenceable<Response> = Referenceable::component_ref("responses", "Error");
    assert_eq!(
        serde_json::to_value(reference).unwrap(),
        json!({
            "$ref": "#/components/responses/Error"
        })
    );
}

#[test]
fn schema_and_inline_siblings_are_not_discarded_by_reference_matching() {
    let schema = json!({
        "$ref": "#/components/schemas/Item",
        "description": "An item",
        "type": "object",
        "properties": {"id": {"type": "integer"}},
        "required": ["id"]
    });
    let legacy: Referenceable<Schema> = serde_json::from_value(schema.clone()).unwrap();
    assert!(legacy.is_data());
    assert_eq!(serde_json::to_value(legacy).unwrap(), schema);
    let modern: SchemaValue = serde_json::from_value(schema.clone()).unwrap();
    assert_eq!(serde_json::to_value(modern).unwrap(), schema);

    // A generic inline payload may use $ref without being a Reference Object.
    let inline = json!({"$ref": "application-specific", "value": 42});
    let example: Referenceable<Example> = serde_json::from_value(inline).unwrap();
    assert_eq!(example.as_data().unwrap().value, Some(json!(42)));
    let inline: Referenceable<serde_json::Value> =
        Referenceable::data(json!({"$ref": "application-specific", "value": 42}));
    assert_eq!(
        serde_json::to_value(inline).unwrap(),
        json!({
            "$ref": "application-specific", "value": 42
        })
    );
}

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
    assert_eq!(reference._ref, "#/components/responses/Error");
    assert_eq!(reference.summary.as_deref(), Some("Failure"));
    assert_eq!(reference.description.as_deref(), Some("Endpoint failure"));
}

#[test]
fn generic_inline_data_still_borrows_from_input() {
    #[derive(serde::Deserialize)]
    struct Borrowed<'a> {
        value: &'a str,
    }
    let value: Referenceable<Borrowed<'_>> =
        serde_json::from_str(r#"{"value":"borrowed"}"#).unwrap();
    assert_eq!(value.as_data().unwrap().value, "borrowed");
}
