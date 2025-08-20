# OAS Usage Guide

This guide provides detailed examples and patterns for using the OAS crate effectively.

## Table of Contents

1. [Basic Concepts](#basic-concepts)
2. [Creating Your First API](#creating-your-first-api)
3. [Working with Referenceable Types](#working-with-referenceable-types)
4. [Building Complex Operations](#building-complex-operations)
5. [Managing Components](#managing-components)
6. [Security and Authentication](#security-and-authentication)
7. [Best Practices](#best-practices)
8. [Common Patterns](#common-patterns)

## Basic Concepts

### OpenAPI Structure

An OpenAPI specification consists of several key components:

- **Info**: Metadata about the API (title, version, description)
- **Servers**: List of server URLs where the API is available
- **Paths**: Available endpoints and their operations
- **Components**: Reusable schemas, responses, parameters, etc.
- **Security**: Authentication and authorization schemes

### Referenceable Pattern

The `Referenceable<T>` type allows you to either:
- Use inline definitions (`Referenceable::data(...)`)
- Reference reusable components (`Referenceable::reference(...)`)

## Creating Your First API

### Minimal Example

```rust
use oas::{builders, PathItem};

let api = builders::api("My API", "1.0.0")
    .add_path("/ping", PathItem::new()
        .with_get(builders::get("Health check").build()));
```

### Adding Metadata

```rust
use oas::{builders, Info, Contact, License, Server};

let contact = Contact::new()
    .with_name("API Team")
    .with_email("team@example.com");

let license = License::new("MIT")
    .with_url("https://opensource.org/licenses/MIT");

let info = Info::new("My API", "1.0.0")
    .with_description("A sample API")
    .with_contact(contact)
    .with_license(license);

let api = builders::api("My API", "1.0.0")
    .with_description("A sample API")
    .add_server(Server::new("https://api.example.com"))
    .add_server(Server::new("http://localhost:3000"));
```

## Working with Referenceable Types

### Inline vs Reference

```rust
use oas::{Referenceable, Schema};

// Inline schema - defined directly where used
let inline_schema = Referenceable::data(
    Schema::string().with_description("User name")
);

// Reference to component - defined once, used multiple times
let ref_schema = Referenceable::schema_ref("User");
```

### Common Referenceable Helpers

```rust
use oas::Referenceable;

// Schema references
let user_ref = Referenceable::schema_ref("User");
let error_ref = Referenceable::schema_ref("Error");

// Response references
let not_found = Referenceable::response_ref("NotFound");
let success = Referenceable::ok("Operation successful");

// Parameter shortcuts
let query_param = Referenceable::query_param("filter")
    .with_schema(Referenceable::string_schema());

let path_param = Referenceable::path_param("id")
    .with_schema(Referenceable::integer_schema());

// Request body shortcuts
let json_body = Referenceable::json_body(
    Referenceable::schema_ref("CreateUserRequest")
);
```

## Building Complex Operations

### Using OperationBuilder

```rust
use oas::{builders, Referenceable};

let operation = builders::operation()
    .summary("Get user profile")
    .description("Retrieves detailed information about a user")
    .operation_id("getUserProfile")
    .tag("users")
    .parameter(Referenceable::path_param("userId")
        .with_schema(Referenceable::string_schema())
        .with_description("Unique identifier for the user"))
    .parameter(Referenceable::query_param("include")
        .with_schema(Referenceable::string_schema())
        .with_description("Comma-separated list of related resources to include"))
    .response("200", Referenceable::data(
        oas::Response::new("User profile retrieved successfully")
            .with_content({
                let mut content = std::collections::BTreeMap::new();
                content.insert("application/json".to_string(),
                    oas::MediaType::new()
                        .with_schema(Referenceable::schema_ref("UserProfile")));
                content
            })
    ))
    .response("404", Referenceable::error("User not found"))
    .response("403", Referenceable::error("Access denied"))
    .build();
```

### HTTP Method Shortcuts

```rust
use oas::builders;

// GET with default 200 response
let get_op = builders::get("List items")
    .tag("items")
    .build();

// POST with 201 and 400 responses
let post_op = builders::post("Create item")
    .tag("items")
    .request_body(Referenceable::json_body(
        Referenceable::schema_ref("CreateItemRequest")
    ))
    .build();

// PUT with 200 and 404 responses  
let put_op = builders::put("Update item")
    .tag("items")
    .build();

// DELETE with 204 and 404 responses
let delete_op = builders::delete("Delete item")
    .tag("items")
    .build();
```

## Managing Components

### Creating Reusable Schemas

```rust
use oas::{Schema, Referenceable, Components};
use std::collections::BTreeMap;

// Create schemas
let mut schemas = BTreeMap::new();

// User schema
let mut user_properties = BTreeMap::new();
user_properties.insert("properties".to_string(), serde_json::json!({
    "id": {"type": "integer", "format": "int64"},
    "name": {"type": "string"},
    "email": {"type": "string", "format": "email"}
}));
user_properties.insert("required".to_string(), serde_json::json!(["id", "name", "email"]));

schemas.insert("User".to_string(), Referenceable::data(Schema {
    _type: Some("object".to_string()),
    description: Some("A user object".to_string()),
    extras: user_properties,
    format: None,
    nullable: None,
}));

// Error schema
schemas.insert("Error".to_string(), Referenceable::data(
    Schema::object().with_description("Standard error response")
));

// Create components
let components = Components::new()
    .with_schemas(schemas);
```

### Creating Reusable Responses

```rust
use oas::{Response, MediaType, Referenceable};
use std::collections::BTreeMap;

let mut responses = BTreeMap::new();

responses.insert("NotFound".to_string(), Referenceable::data(
    Response::new("The specified resource was not found")
        .with_content({
            let mut content = BTreeMap::new();
            content.insert("application/json".to_string(),
                MediaType::new().with_schema(Referenceable::schema_ref("Error")));
            content
        })
));

responses.insert("Unauthorized".to_string(), Referenceable::data(
    Response::new("Authentication is required")
        .with_content({
            let mut content = BTreeMap::new();
            content.insert("application/json".to_string(),
                MediaType::new().with_schema(Referenceable::schema_ref("Error")));
            content
        })
));
```

## Security and Authentication

### API Key Authentication

```rust
use oas::{SecurityScheme, SecurityType, ParameterIn, SecurityRequirement};
use std::collections::BTreeMap;

let mut security_schemes = BTreeMap::new();

security_schemes.insert("apiKey".to_string(), Referenceable::data(
    SecurityScheme {
        _type: SecurityType::ApiKey {
            name: "X-API-Key".to_string(),
            _in: ParameterIn::Header,
        },
        description: Some("API key authentication".to_string()),
    }
));

// Apply to entire API
let mut security_data = BTreeMap::new();
security_data.insert("apiKey".to_string(), vec![]);
let security_requirements = vec![SecurityRequirement { data: security_data }];
```

### JWT Bearer Authentication

```rust
use oas::{SecurityScheme, SecurityType};

let bearer_auth = SecurityScheme {
    _type: SecurityType::Http {
        scheme: "bearer".to_string(),
        bearer_format: Some("JWT".to_string()),
    },
    description: Some("JWT Bearer token".to_string()),
};
```

### OAuth2 Authentication

```rust
use oas::{SecurityScheme, SecurityType, OauthFlows, OauthFlow};
use std::collections::BTreeMap;

let mut scopes = BTreeMap::new();
scopes.insert("read".to_string(), "Read access".to_string());
scopes.insert("write".to_string(), "Write access".to_string());

let oauth2_auth = SecurityScheme {
    _type: SecurityType::Oauth2 {
        flows: OauthFlows {
            authorization_code: Some(OauthFlow {
                authorization_url: "https://example.com/oauth/authorize".to_string(),
                token_url: Some("https://example.com/oauth/token".to_string()),
                refresh_url: None,
                scopes: scopes,
            }),
            implicit: None,
            password: None,
            client_credentials: None,
        },
    },
    description: Some("OAuth2 authentication".to_string()),
};
```

## Best Practices

### 1. Use Components for Reusability

Always define reusable elements in components rather than inlining them everywhere:

```rust
// Good
let user_ref = Referenceable::schema_ref("User");

// Avoid (unless schema is only used once)
let inline_user = Referenceable::data(Schema::object());
```

### 2. Consistent Naming

Use consistent naming conventions:
- **Schemas**: PascalCase (`User`, `CreateUserRequest`)
- **Operations**: camelCase (`getUserById`, `createUser`)
- **Parameters**: camelCase (`userId`, `includeDeleted`)

### 3. Comprehensive Error Handling

Always include common error responses:

```rust
let operation = builders::get("Get resource")
    .response("200", Referenceable::ok("Success"))
    .response("400", Referenceable::response_ref("BadRequest"))
    .response("401", Referenceable::response_ref("Unauthorized"))
    .response("404", Referenceable::response_ref("NotFound"))
    .response("500", Referenceable::response_ref("InternalError"))
    .build();
```

### 4. Use Tags for Organization

Group related operations with tags:

```rust
let api = builders::api("My API", "1.0.0")
    .add_tag(Tag::with_description("users", "User management"))
    .add_tag(Tag::with_description("auth", "Authentication"))
    .add_path("/users", PathItem::new()
        .with_get(builders::get("List users").tag("users").build()))
    .add_path("/auth/login", PathItem::new()
        .with_post(builders::post("Login").tag("auth").build()));
```

## Common Patterns

### CRUD Operations

```rust
use oas::{builders, PathItem, Referenceable};

// List resources
let list_path = PathItem::new()
    .with_get(builders::get("List users")
        .tag("users")
        .parameter(Referenceable::parameter_ref("limitParam"))
        .parameter(Referenceable::parameter_ref("offsetParam"))
        .build())
    .with_post(builders::post("Create user")
        .tag("users")
        .request_body(Referenceable::json_body(
            Referenceable::schema_ref("CreateUserRequest")))
        .build());

// Single resource
let resource_path = PathItem::new()
    .with_get(builders::get("Get user")
        .tag("users")
        .parameter(Referenceable::path_param("id")
            .with_schema(Referenceable::integer_schema()))
        .build())
    .with_put(builders::put("Update user")
        .tag("users")
        .parameter(Referenceable::path_param("id")
            .with_schema(Referenceable::integer_schema()))
        .request_body(Referenceable::json_body(
            Referenceable::schema_ref("UpdateUserRequest")))
        .build())
    .with_delete(builders::delete("Delete user")
        .tag("users")
        .parameter(Referenceable::path_param("id")
            .with_schema(Referenceable::integer_schema()))
        .build());
```

### Pagination Parameters

```rust
use oas::{Parameter, ParameterIn, Referenceable};

// Create reusable pagination parameters
let limit_param = Parameter::new("limit", ParameterIn::Query)
    .with_description("Number of items to return (default: 20, max: 100)")
    .with_schema(Referenceable::data(
        Schema::integer()
            .with_format("int32")
    ));

let offset_param = Parameter::new("offset", ParameterIn::Query)
    .with_description("Number of items to skip (default: 0)")
    .with_schema(Referenceable::integer_schema());
```

### Content Negotiation

```rust
use oas::{MediaType, Referenceable};
use std::collections::BTreeMap;

// Support multiple content types
let mut content = BTreeMap::new();

content.insert("application/json".to_string(),
    MediaType::new().with_schema(Referenceable::schema_ref("User")));

content.insert("application/xml".to_string(),
    MediaType::new().with_schema(Referenceable::schema_ref("User")));

let response = oas::Response::new("User data")
    .with_content(content);
```

This guide covers the most common use cases and patterns. For more advanced features, refer to the API documentation and examples in the repository.