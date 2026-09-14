//! Generate a document describing parsed SSE events with string-encoded JSON data.
use oas::{
    builders, Components, Info, MediaType, OpenAPIV3, PathItem, Referenceable, Response, Schema,
    SchemaValue,
};
use serde_json::json;
use std::collections::BTreeMap;

fn main() {
    let data = Schema::string()
        .with_content_media_type("application/json")
        .with_content_schema(Schema::reference("#/components/schemas/Payload"));
    let event = Schema {
        extras: BTreeMap::from([
            ("required".into(), json!(["data"])),
            (
                "properties".into(),
                json!({
                    "event": {"type": "string"},
                    "id": {"type": "string"},
                    "retry": {"type": "integer", "minimum": 0},
                    "data": data
                }),
            ),
        ]),
        ..Schema::object()
    };
    let payload = Schema {
        extras: BTreeMap::from([(
            "properties".into(),
            json!({
                "message": {"type": ["string", "null"]}
            }),
        )]),
        ..Schema::object()
    };
    let schemas: BTreeMap<String, SchemaValue> = BTreeMap::from([
        ("Event".into(), event.into()),
        ("Payload".into(), payload.into()),
        ("Unrestricted".into(), true.into()),
    ]);
    let content = BTreeMap::from([(
        "text/event-stream".into(),
        MediaType::new().with_item_schema(Schema::reference("#/components/schemas/Event")),
    )]);
    let api = OpenAPIV3::new_v3_2(Info::new("Event API", "1.0.0"))
        .with_components(Components::new().with_schemas(schemas))
        .add_path(
            "/events",
            PathItem::new().with_get(
                builders::get("Stream events")
                    .response(
                        "200",
                        Referenceable::data(Response::new("Events").with_content(content)),
                    )
                    .build(),
            ),
        );
    println!("{}", serde_json::to_string_pretty(&api).unwrap());
}
