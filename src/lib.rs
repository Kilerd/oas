//! # OpenAPI Specification (OAS) 3.0 Rust Implementation
//!
//! This crate provides a complete implementation of the OpenAPI Specification 3.0 in Rust,
//! with comprehensive support for serialization/deserialization and convenient builder patterns
//! for programmatic API specification creation.
//!
//! ## Features
//!
//! - **Complete OAS 3.0 Support**: All OpenAPI 3.0 specification features including schemas,
//!   operations, parameters, responses, security, callbacks, and links
//! - **Serde Integration**: Full JSON/YAML serialization and deserialization support
//! - **Builder Patterns**: Fluent APIs for easy specification construction
//! - **Type Safety**: Leverages Rust's type system to prevent invalid specifications
//! - **Reference System**: Support for both inline definitions and `$ref` references
//!
//! ## Quick Start
//!
//! ```rust
//! use oas::{builders, Referenceable, PathItem, Tag, Server};
//!
//! let api = builders::api("My API", "1.0.0")
//!     .with_description("A sample API")
//!     .add_server(Server::new("https://api.example.com"))
//!     .add_path("/users", PathItem::new()
//!         .with_get(builders::get("List users")
//!             .response("200", Referenceable::ok("User list"))
//!             .build()));
//!
//! println!("{}", api.to_string());
//! ```
//!
//! ## Core Types
//!
//! - [`OpenAPIV3`] - The root OpenAPI document
//! - [`Referenceable<T>`] - Wrapper for inline data or references
//! - [`builders`] - Module containing builder utilities
//! - [`OperationBuilder`] - Builder for complex operations
//!
//! ## Reference vs Inline Data
//!
//! The [`Referenceable<T>`] type allows you to specify either inline data or references
//! to reusable components:
//!
//! ```rust
//! use oas::{Referenceable, Schema};
//!
//! // Inline schema
//! let inline = Referenceable::data(Schema::string());
//!
//! // Reference to component
//! let reference = Referenceable::schema_ref("User");
//! ```

// Module declarations
pub mod builders;
pub mod extensions;
pub mod security;
pub mod types;

// Re-export main types for convenience
pub use security::*;
pub use types::*;

// Re-export builder functionality
pub use builders::*;

#[cfg(test)]
mod test {
    mod pass {
        use crate::OpenAPIV3;
        use assert_json_diff::assert_json_eq;

        macro_rules! pass {
            ($t:ty, $value:expr) => {
                serde_json::from_str::<$t>($value).unwrap();
                let new =
                    serde_json::to_value(&serde_json::from_str::<$t>($value).unwrap()).unwrap();
                let original = serde_json::from_str::<serde_json::Value>($value).unwrap();
                assert_json_eq!(dbg!(new), original);
            };
        }

        #[test]
        fn should_should_pass() {
            pass! { OpenAPIV3, include_str!("../openapi3-examples/3.0/pass/swagger2openapi/openapi.json") }
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/api-with-examples.json") }
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/callback-example.json") }
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/link-example.json") }
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/petstore-expanded.json") }
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/petstore.json") }
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/uspto.json") }
        }
    }
}
