# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This is a Rust crate called `oas` that provides OpenAPI Specification v3.0 types and utilities. The crate defines comprehensive Rust structs and enums that correspond to the OpenAPI v3.0 specification, with full serialization/deserialization support via Serde.

## Architecture

The entire OpenAPI v3.0 specification is modeled in a single file `src/lib.rs` containing:

- **Core Types**: `OpenAPIV3` (root document), `Info`, `Server`, `Components`, `PathItem`, `Operation`
- **Schema System**: `Schema`, `Parameter`, `RequestBody`, `Response`, `MediaType` 
- **Security**: `SecurityScheme`, `SecurityRequirement`, `OauthFlows`
- **Reference System**: `Referenceable<T>` enum allowing either direct data or `$ref` references
- **Utilities**: Each main type has `to_string()` and `to_value()` methods for JSON serialization

Key design patterns:
- Uses `#[skip_serializing_none]` from serde_with for optional fields
- `BTreeMap<String, T>` for extensible mappings
- `Referenceable<T>` wrapper allows JSON references (`{"$ref": "..."}`) or inline data
- Flattened serde attributes for flexible JSON structure handling

## Development Commands

### Building and Testing
```bash
cargo build              # Build the crate
cargo check              # Fast compilation check
cargo test               # Run all tests
```

### Code Quality
```bash
cargo fmt                # Format code
cargo clippy             # Linting
```

## Testing Strategy

The project uses a comprehensive test suite with real OpenAPI specification files:

- **Pass tests**: Located in `openapi3-examples/3.0/pass/` - valid OpenAPI specs that should deserialize and re-serialize identically
- **Fail tests**: Located in `openapi3-examples/3.0/fail/` - invalid specs used for negative testing
- **Examples**: `examples/v3.0/json/` contains official OpenAPI example files

The main test pattern validates round-trip serialization: JSON → Rust struct → JSON should produce identical results.

## Convenience Features

The crate now includes extensive convenience methods and builder patterns to make it much easier to construct OpenAPI specifications programmatically:

### Constructor Functions
Most main types now have `new()` constructors and fluent `with_*()` methods:
- `OpenAPIV3::new(info)` - Creates a basic API spec
- `Info::new(title, version).with_description(desc)` - Info with chaining
- `Server::new(url).with_description(desc)` - Server configuration
- `Operation::new(responses).with_summary(summary)` - Operation builder
- `Parameter::new(name, location).with_schema(schema)` - Parameter setup

### Referenceable Helpers
The `Referenceable<T>` type includes many convenience methods:
- `Referenceable::data(item)` - Wrap inline data
- `Referenceable::reference(ref_string)` - Create reference
- `Referenceable::component_ref(type, name)` - Component references
- Type-specific helpers like `schema_ref()`, `query_param()`, `path_param()`
- Chainable methods like `with_schema()`, `with_description()`

### Builder Pattern
An `OperationBuilder` provides a fluent interface for complex operations:
```rust
use oas::builders;

let operation = builders::get("List items")
    .tag("items")
    .parameter(Referenceable::query_param("limit"))
    .response("200", Referenceable::ok("Success"))
    .build();
```

### Quick Builders Module
The `builders` module provides shortcuts for common patterns:
- `builders::api(title, version)` - Quick API setup
- `builders::get(summary)`, `builders::post(summary)` - HTTP method shortcuts
- Pre-configured common responses (200, 201, 400, 404, etc.)

### Schema Shortcuts
Schema creation is simplified with type-specific constructors:
- `Schema::string()`, `Schema::integer()`, `Schema::boolean()`
- `Referenceable::string_schema()`, `Referenceable::array_schema()`

## Key Considerations

- The crate focuses on OpenAPI v3.0 specification compliance
- All major OpenAPI v3.0 features are supported including callbacks, links, discriminators, and security schemes
- The `Any` type alias maps to `serde_json::Value` for handling arbitrary JSON values
- Extensions and additional properties are supported via flattened `extras` fields
- All convenience methods maintain full compatibility with existing code
- The builder pattern and convenience methods significantly reduce boilerplate when creating specs programmatically