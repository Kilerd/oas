// Comprehensive example showing various OpenAPI features

use oas::{
    builders, Referenceable, PathItem, Tag, Server, Components, Schema, Parameter,
    ParameterIn, Response, MediaType, SecurityScheme, SecurityType,
    SecurityRequirement, Info, Contact, License, ExternalDocumentation
};
use std::collections::BTreeMap;

fn main() {
    // Create comprehensive API specification
    let api = create_comprehensive_api();
    
    // Print as formatted JSON
    println!("{}", serde_json::to_string_pretty(&api).unwrap());
    
    // Demonstrate other features
    demonstrate_referenceable_types();
    demonstrate_schema_creation();
}

fn create_comprehensive_api() -> oas::OpenAPIV3 {
    // Create reusable schemas
    let mut schemas = BTreeMap::new();
    
    // User schema
    let mut user_properties = BTreeMap::new();
    user_properties.insert("properties".to_string(), serde_json::json!({
        "id": {
            "type": "integer",
            "format": "int64",
            "description": "Unique identifier for the user"
        },
        "username": {
            "type": "string",
            "description": "Username for login"
        },
        "email": {
            "type": "string",
            "format": "email",
            "description": "User's email address"
        },
        "created_at": {
            "type": "string",
            "format": "date-time",
            "description": "User creation timestamp"
        }
    }));
    user_properties.insert("required".to_string(), serde_json::json!(["id", "username", "email"]));
    
    let user_schema = Schema {
        _type: Some("object".to_string()),
        description: Some("A user in the system".to_string()),
        extras: user_properties,
        format: None,
        nullable: None,
    };
    
    schemas.insert("User".to_string(), Referenceable::data(user_schema));
    
    // Error schema
    let mut error_properties = BTreeMap::new();
    error_properties.insert("properties".to_string(), serde_json::json!({
        "code": {
            "type": "integer",
            "description": "Error code"
        },
        "message": {
            "type": "string",
            "description": "Error message"
        }
    }));
    error_properties.insert("required".to_string(), serde_json::json!(["code", "message"]));
    
    let error_schema = Schema {
        _type: Some("object".to_string()),
        description: Some("Error response".to_string()),
        extras: error_properties,
        format: None,
        nullable: None,
    };
    
    schemas.insert("Error".to_string(), Referenceable::data(error_schema));
    
    // Create reusable responses
    let mut responses = BTreeMap::new();
    
    responses.insert("NotFound".to_string(), Referenceable::data(
        Response::new("Resource not found")
            .with_content({
                let mut content = BTreeMap::new();
                content.insert("application/json".to_string(),
                    MediaType::new().with_schema(Referenceable::schema_ref("Error")));
                content
            })
    ));
    
    responses.insert("ValidationError".to_string(), Referenceable::data(
        Response::new("Validation error")
            .with_content({
                let mut content = BTreeMap::new();
                content.insert("application/json".to_string(),
                    MediaType::new().with_schema(Referenceable::schema_ref("Error")));
                content
            })
    ));
    
    // Create reusable parameters
    let mut parameters = BTreeMap::new();
    
    parameters.insert("limitParam".to_string(), Referenceable::data(
        Parameter::new("limit", ParameterIn::Query)
            .with_description("Number of items to return")
            .with_schema(Referenceable::data(
                Schema::integer()
                    .with_format("int32")
            ))
    ));
    
    parameters.insert("offsetParam".to_string(), Referenceable::data(
        Parameter::new("offset", ParameterIn::Query)
            .with_description("Number of items to skip")
            .with_schema(Referenceable::integer_schema())
    ));
    
    // Create security schemes
    let mut security_schemes = BTreeMap::new();
    
    security_schemes.insert("bearerAuth".to_string(), Referenceable::data(
        SecurityScheme {
            _type: SecurityType::Http {
                scheme: "bearer".to_string(),
                bearer_format: Some("JWT".to_string()),
            },
            description: Some("JWT Bearer token authentication".to_string()),
        }
    ));
    
    security_schemes.insert("apiKey".to_string(), Referenceable::data(
        SecurityScheme {
            _type: SecurityType::ApiKey {
                name: "X-API-Key".to_string(),
                _in: ParameterIn::Header,
            },
            description: Some("API Key authentication".to_string()),
        }
    ));
    
    // Create components
    let components = Components::new()
        .with_schemas(schemas)
        .with_responses(responses)
        .with_parameters(parameters);
    
    // Create security requirements
    let mut security_requirement_data = BTreeMap::new();
    security_requirement_data.insert("bearerAuth".to_string(), vec![]);
    let security_requirements = vec![SecurityRequirement { data: security_requirement_data }];
    
    // Create contact information
    let contact = Contact::new()
        .with_name("API Team")
        .with_email("api-team@example.com")
        .with_url("https://example.com/contact");
    
    // Create license information
    let license = License::new("MIT")
        .with_url("https://opensource.org/licenses/MIT");
    
    // Create info with all metadata
    let info = Info::new("User Management API", "2.1.0")
        .with_description("A comprehensive API for managing users with authentication, validation, and full CRUD operations")
        .with_contact(contact)
        .with_license(license);
    
    // Create servers
    let servers = vec![
        Server::new("https://api.example.com/v2")
            .with_description("Production server"),
        Server::new("https://staging-api.example.com/v2")
            .with_description("Staging server"),
        Server::new("http://localhost:3000/v2")
            .with_description("Development server"),
    ];
    
    // Create tags
    let tags = vec![
        Tag::with_description("users", "User management operations"),
        Tag::with_description("auth", "Authentication operations"),
    ];
    
    // Create external documentation
    let external_docs = ExternalDocumentation {
        description: Some("Find more info here".to_string()),
        url: "https://example.com/docs".to_string(),
    };
    
    // Build the complete API specification
    let mut api = builders::api("User Management API", "2.1.0")
        .with_description("A comprehensive API for managing users")
        .with_servers(servers)
        .with_components(components);

    // Set the enhanced info with all metadata
    api.info = info;
    
    // Set security, tags, and external docs
    api.security = Some(security_requirements);
    api.tags = Some(tags);
    api.external_docs = Some(external_docs);
    
    // Add paths
    api = api
        .add_path("/auth/login", PathItem::new()
            .with_post(builders::post("User login")
                .tag("auth")
                .operation_id("loginUser")
                .request_body(Referenceable::json_body(
                    Referenceable::data(Schema::object()
                        .with_description("Login credentials"))
                ))
                .response("200", Referenceable::data(
                    Response::new("Login successful")
                        .with_content({
                            let mut content = BTreeMap::new();
                            content.insert("application/json".to_string(),
                                MediaType::new().with_schema(
                                    Referenceable::data(Schema::object()
                                        .with_description("Login response with token"))
                                ));
                            content
                        })
                ))
                .response("401", Referenceable::error("Invalid credentials"))
                .build()))
        .add_path("/users", PathItem::new()
            .with_get(builders::get("List users")
                .tag("users")
                .operation_id("listUsers")
                .parameter(Referenceable::parameter_ref("limitParam"))
                .parameter(Referenceable::parameter_ref("offsetParam"))
                .response("200", Referenceable::data(
                    Response::new("List of users")
                        .with_content({
                            let mut content = BTreeMap::new();
                            content.insert("application/json".to_string(),
                                MediaType::new().with_schema(
                                    Referenceable::data(Schema::array())
                                ));
                            content
                        })
                ))
                .build())
            .with_post(builders::post("Create user")
                .tag("users")
                .operation_id("createUser")
                .request_body(Referenceable::json_body(
                    Referenceable::schema_ref("User")
                ))
                .response("201", Referenceable::data(
                    Response::new("User created")
                        .with_content({
                            let mut content = BTreeMap::new();
                            content.insert("application/json".to_string(),
                                MediaType::new().with_schema(Referenceable::schema_ref("User")));
                            content
                        })
                ))
                .response("400", Referenceable::response_ref("ValidationError"))
                .build()))
        .add_path("/users/{id}", PathItem::new()
            .with_get(builders::get("Get user by ID")
                .tag("users")
                .operation_id("getUserById")
                .parameter(Referenceable::path_param("id")
                    .with_schema(Referenceable::integer_schema())
                    .with_description("User ID"))
                .response("200", Referenceable::data(
                    Response::new("User details")
                        .with_content({
                            let mut content = BTreeMap::new();
                            content.insert("application/json".to_string(),
                                MediaType::new().with_schema(Referenceable::schema_ref("User")));
                            content
                        })
                ))
                .response("404", Referenceable::response_ref("NotFound"))
                .build())
            .with_put(builders::put("Update user")
                .tag("users")
                .operation_id("updateUser")
                .parameter(Referenceable::path_param("id")
                    .with_schema(Referenceable::integer_schema())
                    .with_description("User ID"))
                .request_body(Referenceable::json_body(
                    Referenceable::schema_ref("User")
                ))
                .response("200", Referenceable::data(
                    Response::new("User updated")
                        .with_content({
                            let mut content = BTreeMap::new();
                            content.insert("application/json".to_string(),
                                MediaType::new().with_schema(Referenceable::schema_ref("User")));
                            content
                        })
                ))
                .response("404", Referenceable::response_ref("NotFound"))
                .response("400", Referenceable::response_ref("ValidationError"))
                .build())
            .with_delete(builders::delete("Delete user")
                .tag("users")
                .operation_id("deleteUser")
                .parameter(Referenceable::path_param("id")
                    .with_schema(Referenceable::integer_schema())
                    .with_description("User ID"))
                .response("204", Referenceable::ok("User deleted"))
                .response("404", Referenceable::response_ref("NotFound"))
                .build()));
    
    api
}

fn demonstrate_referenceable_types() {
    println!("\n=== Referenceable Types Examples ===");
    
    // Inline data
    let inline_schema = Referenceable::data(Schema::string());
    println!("Inline schema: {:?}", inline_schema.is_data());
    
    // Component reference
    let schema_ref = Referenceable::schema_ref("User");
    println!("Schema reference: {:?}", schema_ref.is_reference());
    
    // Custom reference
    let custom_ref: Referenceable<oas::Response> = Referenceable::component_ref("responses", "NotFound");
    println!("Custom reference: {:?}", custom_ref.as_reference());
    
    // Parameter helpers
    let query_param = Referenceable::query_param("filter")
        .with_schema(Referenceable::string_schema())
        .with_description("Filter criteria");
    
    let path_param = Referenceable::path_param("id")
        .with_schema(Referenceable::integer_schema());
    
    println!("Query param created: {:?}", query_param.is_data());
    println!("Path param created: {:?}", path_param.is_data());
}

fn demonstrate_schema_creation() {
    println!("\n=== Schema Creation Examples ===");
    
    // Basic types
    let string_schema = Schema::string();
    let integer_schema = Schema::integer().with_format("int64");
    let _boolean_schema = Schema::boolean();
    let _array_schema = Schema::array();
    let object_schema = Schema::object().with_description("Custom object");
    
    println!("String schema type: {:?}", string_schema._type);
    println!("Integer schema format: {:?}", integer_schema.format);
    println!("Object schema description: {:?}", object_schema.description);
    
    // Referenceable schemas
    let ref_string = Referenceable::string_schema();
    let ref_integer = Referenceable::integer_schema();
    let ref_array = Referenceable::array_schema();
    
    println!("Referenceable string: {:?}", ref_string.is_data());
    println!("Referenceable integer: {:?}", ref_integer.is_data());
    println!("Referenceable array: {:?}", ref_array.is_data());
}