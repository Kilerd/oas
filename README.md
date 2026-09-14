# OpenAPI Specification (OAS) for Rust

[![Crates.io](https://img.shields.io/crates/v/oas.svg)](https://crates.io/crates/oas)
[![Documentation](https://docs.rs/oas/badge.svg)](https://docs.rs/oas)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Rust OpenAPI models with Serde serialization and builders for constructing API
descriptions. [Version 0.3.0](https://crates.io/crates/oas/0.3.0) adds schema type
unions, boolean schemas, references with sibling keywords, and OpenAPI 3.2
streaming media types, including Server-Sent Events (SSE).

Existing document constructors still default to **OpenAPI 3.0.0**. Coverage of
**OpenAPI 3.1/3.2 is partial**; the crate represents documents and schemas without
validating them or resolving references. See the
[migration guide](https://github.com/Kilerd/oas/blob/main/MIGRATION.md) for API
changes and remaining model differences.

## Installation

```toml
[dependencies]
oas = "0.3"
serde_json = "1"
```

Upgrading from 0.1 or 0.2 requires an explicit dependency update. In 0.3,
schema-bearing fields use `SchemaValue`, and `Schema._type` uses `SchemaType`.
Existing schema helper constructors remain usable with builders. See the
[0.3.0 changelog](https://github.com/Kilerd/oas/blob/main/CHANGELOG.md#030---2026-09-14)
and [Rust API migration](https://github.com/Kilerd/oas/blob/main/MIGRATION.md#rust-api-migration).

## Quick start

This complete example produces an OpenAPI 3.0.0 document with a JSON string response:

```rust
use oas::{builders, MediaType, PathItem, Referenceable, Response, Schema};
use std::collections::BTreeMap;

fn main() {
    let content = BTreeMap::from([(
        "application/json".into(),
        MediaType::new().with_schema(Schema::string()),
    )]);
    let api = builders::api("Greeting API", "1.0.0")
        .add_path("/hello", PathItem::new().with_get(
            builders::get("Get a greeting")
                .response("200", Referenceable::data(
                    Response::new("A greeting").with_content(content),
                ))
                .build(),
        ));

    println!("{}", serde_json::to_string_pretty(&api).unwrap());
}
```

## Choosing an OpenAPI version

| Constructor | Document version | Schema usage |
| --- | --- | --- |
| `builders::api(title, version)` or `OpenAPIV3::new(info)` | `3.0.0` | Existing 3.0 schemas, including `nullable` |
| `OpenAPIV3::new_v3_1(info)` | `3.1.0` | Modern schema forms, including type unions and boolean schemas |
| `OpenAPIV3::new_v3_2(info)` | `3.2.0` | Modern schema forms plus streaming `itemSchema` |

The API's `info.version` is separate from the document's `openapi` version.
Selecting a document version does not migrate keywords: for 3.1/3.2, replace
`nullable: true` with a null union and review the other
[schema migration steps](https://github.com/Kilerd/oas/blob/main/MIGRATION.md#migrating-schema-semantics).
`itemSchema` requires OpenAPI 3.2.

## Schemas and references

`Schema` represents a schema object. `SchemaValue` represents either an object
or a boolean schema and is used in components, parameters, headers, and media
types. Schema references are objects whose `$ref` can coexist with constraints
and annotations.

```rust
use oas::{MediaType, Schema, SchemaValue};

let nullable_string = Schema::string().with_types(["string", "null"]);
let reference = Schema::reference("#/components/schemas/User")
    .with_description("The user returned by this operation");

let accepts_anything = SchemaValue::from(true);
let accepts_nothing = SchemaValue::from(false);
let boolean_type = Schema::boolean(); // {"type":"boolean"}, not a boolean schema

let media = MediaType::new().with_schema(nullable_string);
```

Use `SchemaValue::as_object()` / `as_object_mut()` to inspect or modify object
fields, and handle `SchemaValue::Boolean` separately. Keywords without dedicated
fields, including `properties`, `required`, `$defs`, and `anyOf`, are retained as
JSON values in `Schema.extras`.

Other reusable OpenAPI objects use `Referenceable<T>` for inline data or references:

```rust
use oas::{MediaType, Referenceable, Response};

let inline = Referenceable::data(Response::new("Success"));
let reference: Referenceable<Response> =
    Referenceable::component_ref("responses", "NotFound");

// Existing schema helpers are still accepted by schema-bearing builders.
let media = MediaType::new().with_schema(Referenceable::string_schema());
```

## OpenAPI 3.2 and SSE

`MediaType.item_schema` serializes as `itemSchema` and describes each parsed stream
item. `MediaType.schema` describes the complete body; the two fields are independent.

An SSE event's `event`, `id`, and `data` fields are strings; `retry` is an integer.
When `data` carries JSON, describe it as a string with `contentMediaType` and
`contentSchema`. These annotations describe the decoded content without changing
the field's wire type or performing validation in this crate.

The following example builds a complete document, including its referenced schemas:

```rust
use oas::{
    builders, Components, Info, MediaType, OpenAPIV3, PathItem,
    Referenceable, Response, Schema, SchemaValue,
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
            ("properties".into(), json!({
                "event": {"type": "string"},
                "id": {"type": "string"},
                "retry": {"type": "integer", "minimum": 0},
                "data": data
            })),
        ]),
        ..Schema::object()
    };
    let payload = Schema {
        extras: BTreeMap::from([("properties".into(), json!({
            "message": {"type": ["string", "null"]}
        }))]),
        ..Schema::object()
    };
    let schemas: BTreeMap<String, SchemaValue> = BTreeMap::from([
        ("Event".into(), event.into()),
        ("Payload".into(), payload.into()),
    ]);
    let content = BTreeMap::from([(
        "text/event-stream".into(),
        MediaType::new().with_item_schema(Schema::reference("#/components/schemas/Event")),
    )]);
    let api = OpenAPIV3::new_v3_2(Info::new("Event API", "1.0.0"))
        .with_components(Components::new().with_schemas(schemas))
        .add_path("/events", PathItem::new().with_get(
            builders::get("Stream events")
                .response("200", Referenceable::data(
                    Response::new("Events").with_content(content),
                ))
                .build(),
        ));

    println!("{}", serde_json::to_string_pretty(&api).unwrap());
}
```

From a checkout, run `cargo run --example sse`. The
[JSON fixture](https://github.com/Kilerd/oas/blob/main/examples/v3.2/json/sse.json)
also demonstrates using `schema` and `itemSchema` together and retaining reference
siblings in a round trip.

## Loading JSON

```rust
use oas::OpenAPIV3;

let json_spec = r#"{
    "openapi": "3.2.0",
    "info": {"title": "Sample API", "version": "1.0.0"},
    "paths": {}
}"#;

let spec: OpenAPIV3 = serde_json::from_str(json_spec).unwrap();
println!("{}", serde_json::to_string_pretty(&spec).unwrap());
```

Serde support can also be used with a separately chosen YAML serializer. Unknown
schema keywords are preserved; unknown fields on most other objects are currently
discarded. Successful deserialization does not guarantee a valid document or a
lossless round trip for unsupported fields.

## More examples and documentation

- [API reference for 0.3.0](https://docs.rs/oas/0.3.0/oas/)
- [Usage guide](https://github.com/Kilerd/oas/blob/main/USAGE_GUIDE.md) for builders, operations, parameters, and reusable components
- [Basic builder example](https://github.com/Kilerd/oas/blob/main/examples/easy_usage.rs)
- [Complete API example](https://github.com/Kilerd/oas/blob/main/examples/comprehensive_example.rs)
- [Runnable SSE example](https://github.com/Kilerd/oas/blob/main/examples/sse.rs)
- [Compatibility policy and remaining model differences](https://github.com/Kilerd/oas/blob/main/MIGRATION.md)

## Development

Initialize the fixture submodule before running tests on a fresh checkout:

```bash
git submodule update --init
cargo test --all-targets
cargo test --doc
cargo run --example sse
```

Tests cover existing 3.0 documents, 3.2 SSE round trips, modern schema forms, and
constructor compatibility. Pull requests and bug reports are welcome on
[GitHub](https://github.com/Kilerd/oas).

## License

Licensed under the [MIT license](https://opensource.org/licenses/MIT).
