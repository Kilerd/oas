//! Builder patterns and utilities for constructing OpenAPI specifications.
//!
//! This module provides convenient builder patterns for complex operations
//! and quick utility functions for common OpenAPI constructs.
//!
//! ## Usage
//!
//! The module provides two main types of builders:
//!
//! 1. **OperationBuilder** - For constructing complex operations with a fluent interface
//! 2. **Quick builder functions** - For creating common operations with sensible defaults
//!
//! ## Examples
//!
//! ### Using OperationBuilder
//!
//! ```rust
//! use oas::{builders::OperationBuilder, Referenceable};
//!
//! let operation = OperationBuilder::new()
//!     .summary("Get user profile")
//!     .tag("users")
//!     .parameter(Referenceable::path_param("id"))
//!     .response("200", Referenceable::ok("User profile"))
//!     .build();
//! ```
//!
//! ### Using quick builder functions
//!
//! ```rust
//! use oas::{builders, PathItem};
//!
//! let api = builders::api("My API", "1.0.0")
//!     .add_path("/users", PathItem::new()
//!         .with_get(builders::get("List users").build())
//!         .with_post(builders::post("Create user").build()));
//! ```

use crate::types::*;

/// Builder for constructing complex operations with a fluent interface.
pub struct OperationBuilder {
    operation: Operation,
}

impl OperationBuilder {
    pub fn new() -> Self {
        Self {
            operation: Operation::new(Responses::new()),
        }
    }

    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.operation.summary = Some(summary.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.operation.description = Some(description.into());
        self
    }

    pub fn operation_id(mut self, operation_id: impl Into<String>) -> Self {
        self.operation.operation_id = Some(operation_id.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.operation
            .tags
            .get_or_insert_with(Vec::new)
            .push(tag.into());
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.operation.tags = Some(tags);
        self
    }

    pub fn parameter(mut self, parameter: Referenceable<Parameter>) -> Self {
        self.operation
            .parameters
            .get_or_insert_with(Vec::new)
            .push(parameter);
        self
    }

    pub fn parameters(mut self, parameters: Vec<Referenceable<Parameter>>) -> Self {
        self.operation.parameters = Some(parameters);
        self
    }

    pub fn request_body(mut self, request_body: Referenceable<RequestBody>) -> Self {
        self.operation.request_body = Some(request_body);
        self
    }

    pub fn response(
        mut self,
        status: impl Into<String>,
        response: Referenceable<Response>,
    ) -> Self {
        self.operation
            .responses
            .data
            .insert(status.into(), response);
        self
    }

    pub fn default_response(mut self, response: Referenceable<Response>) -> Self {
        self.operation.responses.default = Some(response);
        self
    }

    pub fn deprecated(mut self) -> Self {
        self.operation.deprecated = Some(true);
        self
    }

    pub fn build(self) -> Operation {
        self.operation
    }
}

impl Default for OperationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder utilities for quickly constructing OpenAPI specifications.
///
/// This module provides convenient functions for creating common OpenAPI constructs
/// with sensible defaults. It's designed to reduce boilerplate when building
/// API specifications programmatically.
///
/// # Examples
///
/// ```rust
/// use oas::{builders, PathItem};
///
/// let api = builders::api("My API", "1.0.0")
///     .add_path("/users", PathItem::new()
///         .with_get(builders::get("List users").build())
///         .with_post(builders::post("Create user").build()));
/// ```

/// Create a new OpenAPI specification with basic info.
///
/// This creates an OpenAPIV3 document with the specified title and version,
/// using OpenAPI version "3.0.0" and empty paths.
///
/// # Examples
///
/// ```rust
/// use oas::builders;
///
/// let api = builders::api("My API", "1.0.0");
/// ```
pub fn api(title: impl Into<String>, version: impl Into<String>) -> OpenAPIV3 {
    OpenAPIV3::new(Info::new(title, version))
}

/// Create a new operation builder.
///
/// This returns an `OperationBuilder` for constructing operations with
/// a fluent interface.
///
/// # Examples
///
/// ```rust
/// use oas::{builders, Referenceable};
///
/// let operation = builders::operation()
///     .summary("Get user")
///     .response("200", Referenceable::ok("User retrieved"))
///     .build();
/// ```
pub fn operation() -> OperationBuilder {
    OperationBuilder::new()
}

/// Create a GET operation builder with common defaults.
///
/// Creates an operation builder pre-configured with:
/// - The provided summary
/// - A 200 "Success" response
///
/// # Examples
///
/// ```rust
/// use oas::builders;
///
/// let get_users = builders::get("List users")
///     .tag("users")
///     .build();
/// ```
pub fn get(summary: impl Into<String>) -> OperationBuilder {
    OperationBuilder::new()
        .summary(summary)
        .response("200", Referenceable::ok("Success"))
}

/// Create a POST operation builder with common defaults.
///
/// Creates an operation builder pre-configured with:
/// - The provided summary
/// - A 201 "Created" response
/// - A 400 "Bad Request" response
///
/// # Examples
///
/// ```rust
/// use oas::builders;
///
/// let create_user = builders::post("Create user")
///     .tag("users")
///     .build();
/// ```
pub fn post(summary: impl Into<String>) -> OperationBuilder {
    OperationBuilder::new()
        .summary(summary)
        .response("201", Referenceable::ok("Created"))
        .response("400", Referenceable::error("Bad Request"))
}

/// Create a PUT operation builder with common defaults.
///
/// Creates an operation builder pre-configured with:
/// - The provided summary
/// - A 200 "Updated" response
/// - A 404 "Not Found" response
///
/// # Examples
///
/// ```rust
/// use oas::builders;
///
/// let update_user = builders::put("Update user")
///     .tag("users")
///     .build();
/// ```
pub fn put(summary: impl Into<String>) -> OperationBuilder {
    OperationBuilder::new()
        .summary(summary)
        .response("200", Referenceable::ok("Updated"))
        .response("404", Referenceable::error("Not Found"))
}

/// Create a DELETE operation builder with common defaults.
///
/// Creates an operation builder pre-configured with:
/// - The provided summary
/// - A 204 "Deleted" response
/// - A 404 "Not Found" response
///
/// # Examples
///
/// ```rust
/// use oas::builders;
///
/// let delete_user = builders::delete("Delete user")
///     .tag("users")
///     .build();
/// ```
pub fn delete(summary: impl Into<String>) -> OperationBuilder {
    OperationBuilder::new()
        .summary(summary)
        .response("204", Referenceable::ok("Deleted"))
        .response("404", Referenceable::error("Not Found"))
}
