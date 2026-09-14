# OpenAPI Specification (OAS) for Rust

[![Crates.io](https://img.shields.io/crates/v/oas.svg)](https://crates.io/crates/oas)
[![Documentation](https://docs.rs/oas/badge.svg)](https://docs.rs/oas)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Rust OpenAPI models with Serde serialization and convenient builders. The 0.3 API
adds JSON Schema 2020-12 representations and OpenAPI 3.2 streaming media types while
keeping existing constructors on OpenAPI 3.0.0 by default.

**OpenAPI 3.1/3.2 coverage is partial.** See [MIGRATION.md](MIGRATION.md) for source
compatibility changes, schema migration, and the remaining model differences.

## Features

- **OpenAPI Models** - Operations, parameters, responses, security, callbacks, and links
- **Modern Schemas** - Type unions, boolean schemas, reference siblings, and string-encoded content
- **Streaming Media Types** - OpenAPI 3.2 `itemSchema` for parsed stream items, including SSE
- 🔄 **Serde Integration** - Full JSON/YAML serialization and deserialization
- 🛠️ **Builder Patterns** - Fluent APIs for easy specification construction
- **Typed Construction** - Rust models and builders; document/schema validation is left to callers
- 📚 **Reference System** - Support for both inline definitions and `$ref` references
- 🚀 **Convenience Methods** - Extensive helper functions to reduce boilerplate

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
oas = "0.3"
serde_json = "1.0"  # For JSON serialization
```

### Basic Example

```rust
use oas::{builders, Referenceable, PathItem, Tag, Server};

fn main() {
    let api = builders::api("Pet Store API", "1.0.0")
        .with_description("A sample Pet Store API")
        .add_server(Server::new("https://api.petstore.com"))
        .add_tag(Tag::with_description("pets", "Pet operations"))
        .add_path("/pets", PathItem::new()
            .with_get(builders::get("List all pets")
                .tag("pets")
                .parameter(Referenceable::query_param("limit")
                    .with_schema(Referenceable::integer_schema()))
                .build())
            .with_post(builders::post("Create a pet")
                .tag("pets")
                .request_body(Referenceable::json_body(
                    Referenceable::schema_ref("Pet")))
                .build()));

    println!("{}", api.to_string());
}
```

## Core Concepts

### OpenAPI 3.2 and SSE

Choose `OpenAPIV3::new_v3_2(info)` explicitly and use `itemSchema` for individual
parsed events. JSON in an SSE `data` field remains a string on the wire:

```rust
use oas::{Info, MediaType, OpenAPIV3, Schema};

let api = OpenAPIV3::new_v3_2(Info::new("Event API", "1.0.0"));
let events = MediaType::new()
    .with_item_schema(Schema::reference("#/components/schemas/Event"));
let data = Schema::string()
    .with_content_media_type("application/json")
    .with_content_schema(Schema::reference("#/components/schemas/Payload"));
let nullable_string = Schema::string().with_types(["string", "null"]);
```

See [examples/sse.rs](examples/sse.rs) for a complete document (`cargo run --example sse`).
`SchemaValue` represents either a `Schema` object or a boolean schema; schema
positions accept both. Unknown JSON Schema keywords are retained in `Schema.extras`.

### Referenceable Types

The `Referenceable<T>` type is central to OpenAPI specifications, allowing you to use either inline data or references to reusable components:

```rust
use oas::{Referenceable, Schema};

// Inline schema
let inline = Referenceable::data(Schema::string());

// Reference to a component
let reference = Referenceable::schema_ref("User");

// Component reference with custom path
let custom_ref = Referenceable::component_ref("schemas", "CustomType");
```

### Builder Pattern

Use the builder pattern for complex operations:

```rust
use oas::{builders, Referenceable};

let operation = builders::get("Get user by ID")
    .tag("users")
    .operation_id("getUserById")
    .parameter(Referenceable::path_param("userId")
        .with_schema(Referenceable::string_schema())
        .with_description("The ID of the user to retrieve"))
    .response("200", Referenceable::ok("User retrieved successfully"))
    .response("404", Referenceable::error("User not found"))
    .build();
```

### Schema Creation

Create schemas with type-specific constructors:

```rust
use oas::{Schema, Referenceable};

// Basic types
let string_schema = Schema::string();
let integer_schema = Schema::integer();
let boolean_schema = Schema::boolean();

// Or as Referenceable for use in parameters/responses
let ref_schema = Referenceable::string_schema();
```

## Examples

### Complete API Specification

```rust
use oas::{builders, Referenceable, PathItem, Tag, Server, Components, Schema};
use std::collections::BTreeMap;

let mut schemas = BTreeMap::new();
schemas.insert("User".to_string(), Referenceable::data(
    Schema::object()
        .with_description("A user object")
));

let api = builders::api("User Management API", "2.0.0")
    .with_description("API for managing users")
    .add_server(Server::new("https://api.example.com/v2")
        .with_description("Production server"))
    .add_server(Server::new("https://staging.api.example.com/v2")
        .with_description("Staging server"))
    .with_components(Components::new()
        .with_schemas(schemas))
    .add_tag(Tag::with_description("users", "User management operations"))
    .add_path("/users", PathItem::new()
        .with_get(builders::get("List users")
            .tag("users")
            .parameter(Referenceable::query_param("page")
                .with_schema(Referenceable::integer_schema())
                .with_description("Page number"))
            .parameter(Referenceable::query_param("limit")
                .with_schema(Referenceable::integer_schema())
                .with_description("Items per page"))
            .build())
        .with_post(builders::post("Create user")
            .tag("users")
            .request_body(Referenceable::json_body(
                Referenceable::schema_ref("User")))
            .build()))
    .add_path("/users/{id}", PathItem::new()
        .with_get(builders::get("Get user")
            .tag("users")
            .parameter(Referenceable::path_param("id")
                .with_schema(Referenceable::string_schema()))
            .build())
        .with_put(builders::put("Update user")
            .tag("users")
            .parameter(Referenceable::path_param("id")
                .with_schema(Referenceable::string_schema()))
            .request_body(Referenceable::json_body(
                Referenceable::schema_ref("User")))
            .build())
        .with_delete(builders::delete("Delete user")
            .tag("users")
            .parameter(Referenceable::path_param("id")
                .with_schema(Referenceable::string_schema()))
            .build()));

println!("{}", serde_json::to_string_pretty(&api).unwrap());
```

### Loading from JSON

```rust
use oas::OpenAPIV3;

let json_spec = r#"{
    "openapi": "3.0.0",
    "info": {
        "title": "Sample API",
        "version": "1.0.0"
    },
    "paths": {}
}"#;

let spec: OpenAPIV3 = serde_json::from_str(json_spec).unwrap();
println!("Loaded API: {}", spec.info.title);
```

## API Reference

### Quick Builders

- `builders::api(title, version)` - Create a new API specification
- `builders::get(summary)` - GET operation with 200 response
- `builders::post(summary)` - POST operation with 201/400 responses
- `builders::put(summary)` - PUT operation with 200/404 responses
- `builders::delete(summary)` - DELETE operation with 204/404 responses

### Referenceable Helpers

- `Referenceable::data(item)` - Wrap inline data
- `Referenceable::reference(ref_str)` - Create reference
- `Referenceable::schema_ref(name)` - Schema component reference
- `Referenceable::query_param(name)` - Query parameter
- `Referenceable::path_param(name)` - Path parameter (automatically required)
- `Referenceable::json_body(schema)` - JSON request body

### Schema Shortcuts

- `Schema::string()`, `Schema::integer()`, `Schema::boolean()`
- `Schema::array()`, `Schema::object()`
- `Referenceable::string_schema()`, `Referenceable::integer_schema()`, etc.

## Testing

Tests cover existing 3.0 documents, 3.2 SSE round trips, modern schema forms, and
constructor compatibility. Initialize the fixture submodule on a fresh checkout:

```bash
git submodule update --init
cargo test --all-targets
cargo test --doc
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
