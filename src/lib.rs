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

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

/// A wrapper type that can contain either inline data or a reference to a component.
///
/// This is a core type in OpenAPI specifications that allows for reusable components.
/// You can either define data inline or reference a component defined elsewhere.
///
/// # Examples
///
/// ```rust
/// use oas::{Referenceable, Schema};
///
/// // Inline data
/// let inline = Referenceable::data(Schema::string());
///
/// // Reference to a component
/// let reference: Referenceable<Schema> = Referenceable::reference("#/components/schemas/User");
///
/// // Convenience method for component references
/// let component_ref = Referenceable::schema_ref("User");
/// ```
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Referenceable<T> {
    /// Inline data
    Data(T),
    /// Reference to a component
    Reference(Reference),
}

impl<T> Referenceable<T> {
    /// Create a new Referenceable with inline data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use oas::{Referenceable, Schema};
    ///
    /// let inline_schema = Referenceable::data(Schema::string());
    /// ```
    pub fn data(data: T) -> Self {
        Self::Data(data)
    }

    /// Create a new Referenceable with a reference string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use oas::{Referenceable, Schema};
    ///
    /// let ref_schema: Referenceable<Schema> = Referenceable::reference("#/components/schemas/User");
    /// ```
    pub fn reference(reference: impl Into<String>) -> Self {
        Self::Reference(Reference::new(reference))
    }

    /// Create a reference to a component using OpenAPI component path format.
    ///
    /// This is a convenience method that constructs the proper `#/components/{type}/{name}` format.
    ///
    /// # Arguments
    ///
    /// * `component_type` - The type of component (e.g., "schemas", "responses", "parameters")
    /// * `name` - The name of the component
    ///
    /// # Examples
    ///
    /// ```rust
    /// use oas::{Referenceable, Schema, Response};
    ///
    /// let schema_ref: Referenceable<Schema> = Referenceable::component_ref("schemas", "User");
    /// let response_ref: Referenceable<Response> = Referenceable::component_ref("responses", "NotFound");
    /// ```
    pub fn component_ref(component_type: &str, name: impl Into<String>) -> Self {
        Self::Reference(Reference::new(format!("#/components/{}/{}", component_type, name.into())))
    }

    /// Get the inline data if this is a Data variant.
    ///
    /// Returns `Some(&T)` if this contains inline data, `None` if it's a reference.
    pub fn as_data(&self) -> Option<&T> {
        match self {
            Self::Data(data) => Some(data),
            Self::Reference(_) => None,
        }
    }

    /// Get the reference if this is a Reference variant.
    ///
    /// Returns `Some(&Reference)` if this contains a reference, `None` if it's inline data.
    pub fn as_reference(&self) -> Option<&Reference> {
        match self {
            Self::Data(_) => None,
            Self::Reference(ref_) => Some(ref_),
        }
    }

    /// Check if this contains inline data.
    ///
    /// Returns `true` if this is a `Data` variant, `false` if it's a `Reference`.
    pub fn is_data(&self) -> bool {
        matches!(self, Self::Data(_))
    }

    /// Check if this contains a reference.
    ///
    /// Returns `true` if this is a `Reference` variant, `false` if it's `Data`.
    pub fn is_reference(&self) -> bool {
        matches!(self, Self::Reference(_))
    }
}

/// The root document object of an OpenAPI v3.0 specification.
///
/// This is the main entry point for an OpenAPI specification document. It contains
/// metadata about the API, server information, available paths and operations,
/// reusable components, security requirements, and additional documentation.
///
/// # Examples
///
/// ```rust
/// use oas::{OpenAPIV3, Info, Server, PathItem, builders};
///
/// let spec = OpenAPIV3::new(Info::new("My API", "1.0.0"))
///     .with_description("A sample API")
///     .add_server(Server::new("https://api.example.com"))
///     .add_path("/users", PathItem::new()
///         .with_get(builders::get("List users").build()));
/// ```
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAPIV3 {
    /// The semantic version number of the OpenAPI Specification version.
    ///
    /// This MUST be the semantic version number of the OpenAPI Specification version
    /// that the OpenAPI document uses. This is not related to the API `info.version` string.
    /// Defaults to "3.0.0" when using `OpenAPIV3::new()`.
    pub openapi: String,

    /// Provides metadata about the API.
    ///
    /// The metadata MAY be used by the clients if needed, and MAY be presented
    /// in editing or documentation generation tools for convenience.
    pub info: Info,

    /// An array of Server Objects providing connectivity information to target servers.
    ///
    /// If the `servers` property is not provided, or is an empty array, the default
    /// value would be a Server Object with a `url` value of `/`.
    pub servers: Option<Vec<Server>>,

    /// The available paths and operations for the API.
    ///
    /// This is a map where keys are path templates (like `/users/{id}`) and values
    /// are PathItem objects describing the operations available on those paths.
    pub paths: BTreeMap<String, PathItem>,

    /// An element to hold various schemas for the specification.
    ///
    /// All objects defined within the components object will have no effect on the API
    /// unless they are explicitly referenced from properties outside the components object.
    pub components: Option<Components>,

    /// A declaration of which security mechanisms can be used across the API.
    ///
    /// The list of values includes alternative security requirement objects that can be used.
    /// Only one of the security requirement objects need to be satisfied to authorize a request.
    /// Individual operations can override this definition.
    pub security: Option<Vec<SecurityRequirement>>,

    /// A list of tags used by the specification with additional metadata.
    ///
    /// The order of the tags can be used to reflect on their order by the parsing tools.
    /// Not all tags that are used by the Operation Object must be declared. Each tag name
    /// in the list MUST be unique.
    pub tags: Option<Vec<Tag>>,

    /// Additional external documentation for the API.
    pub external_docs: Option<ExternalDocumentation>,

    /// Extension fields that start with `x-`.
    ///
    /// This allows for custom extensions to the OpenAPI specification.
    #[serde(flatten)]
    pub extras: Option<BTreeMap<String, Any>>,
}

/// The object provides metadata about the API. The metadata MAY be used by the clients if needed, and MAY be presented in editing or documentation generation tools for convenience.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    /// The title of the API.
    pub title: String,
    /// A short description of the API. CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// A URL to the Terms of Service for the API. MUST be in the format of a URL.
    pub terms_of_service: Option<String>,
    /// The contact information for the exposed API.
    pub contact: Option<Contact>,
    /// The license information for the exposed API.
    pub license: Option<License>,
    /// The version of the OpenAPI document (which is distinct from the OpenAPI Specification version or the API implementation version).
    pub version: String,
}

/// Contact information for the exposed API.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// The identifying name of the contact person/organization.
    pub name: Option<String>,
    /// The URL pointing to the contact information. MUST be in the format of a URL.
    pub url: Option<String>,
    /// The email address of the contact person/organization. MUST be in the format of an email address.
    pub email: Option<String>,
}

/// License information for the exposed API.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// The license name used for the API.
    pub name: String,
    /// A URL to the license used for the API. MUST be in the format of a URL.
    pub url: Option<String>,
}

/// An object representing a Server.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// A URL to the target host. This URL supports Server Variables and MAY be relative, to indicate that the host location is relative to the location where the OpenAPI document is being served. Variable substitutions will be made when a variable is named in {brackets}.
    pub url: String,
    /// An optional string describing the host designated by the URL. CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// A map between a variable name and its value. The value is used for substitution in the server's URL template.
    pub variables: Option<BTreeMap<String, ServerVariable>>,
}

/// An object representing a Server Variable for server URL template substitution.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerVariable {
    /// An enumeration of string values to be used if the substitution options are from a limited set. The array SHOULD NOT be empty.
    #[serde(rename = "enum")]
    pub _enum: Option<Vec<String>>,
    /// The default value to use for substitution, which SHALL be sent if an alternate value is not supplied. Note this behavior is different than the Schema Object's treatment of default values, because in those cases parameter values are optional. If the `enum` is defined, the value SHOULD exist in the enum's values.
    pub default: String,
    /// An optional description for the server variable. CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
}

/// Holds a set of reusable objects for different aspects of the OAS. All objects defined within the components object will have no effect on the API unless they are explicitly referenced from properties outside the components object.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Components {
    /// An object to hold reusable Schema Objects.
    pub schemas: Option<BTreeMap<String, Referenceable<Schema>>>,
    /// An object to hold reusable Response Objects.
    pub responses: Option<BTreeMap<String, Referenceable<Response>>>,
    /// An object to hold reusable Parameter Objects.
    pub parameters: Option<BTreeMap<String, Referenceable<Parameter>>>,
    /// An object to hold reusable Example Objects.
    pub examples: Option<BTreeMap<String, Referenceable<Example>>>,
    /// An object to hold reusable Request Body Objects.
    pub request_bodies: Option<BTreeMap<String, Referenceable<RequestBody>>>,
    /// An object to hold reusable Header Objects.
    pub headers: Option<BTreeMap<String, Referenceable<Header>>>,
    /// An object to hold reusable Security Scheme Objects.
    pub security_schemes: Option<BTreeMap<String, Referenceable<SecurityScheme>>>,
    /// An object to hold reusable Link Objects.
    pub links: Option<BTreeMap<String, Referenceable<Link>>>,
    /// An object to hold reusable Callback Objects.
    pub callbacks: Option<BTreeMap<String, Referenceable<Callback>>>,
}

/// Describes the operations available on a single path. A Path Item MAY be empty, due to ACL constraints. The path itself is still exposed to the documentation viewer but they will not know which operations and parameters are available.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    /// Allows for an external definition of this path item. The referenced structure MUST be in the format of a Path Item Object. In case a Path Item Object field appears both in the defined object and the referenced object, the behavior is undefined.
    #[serde(rename = "$ref")]
    pub _ref: Option<String>,
    /// An optional, string summary, intended to apply to all operations in this path.
    pub summary: Option<String>,
    /// An optional, string description, intended to apply to all operations in this path. CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// A definition of a GET operation on this path.
    pub get: Option<Operation>,
    /// A definition of a PUT operation on this path.
    pub put: Option<Operation>,
    /// A definition of a POST operation on this path.
    pub post: Option<Operation>,
    /// A definition of a DELETE operation on this path.
    pub delete: Option<Operation>,
    /// A definition of a OPTIONS operation on this path.
    pub options: Option<Operation>,
    /// A definition of a HEAD operation on this path.
    pub head: Option<Operation>,
    /// A definition of a PATCH operation on this path.
    pub patch: Option<Operation>,
    /// A definition of a TRACE operation on this path.
    pub trace: Option<Operation>,
    /// An alternative `server` array to service all operations in this path.
    pub servers: Option<Vec<Server>>,
    /// A list of parameters that are applicable for all the operations described under this path. These parameters can be overridden at the operation level, but cannot be removed there. The list MUST NOT include duplicated parameters. A unique parameter is defined by a combination of a name and location. The list can use the Reference Object to link to parameters that are defined at the OpenAPI Object's components/parameters.
    pub parameters: Option<Vec<Referenceable<Parameter>>>,
}

/// Describes a single API operation on a path.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    /// A list of tags for API documentation control. Tags can be used for logical grouping of operations by resources or any other qualifier.
    pub tags: Option<Vec<String>>,
    /// A short summary of what the operation does.
    pub summary: Option<String>,
    /// A verbose explanation of the operation behavior. CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// Additional external documentation for this operation.
    pub external_docs: Option<ExternalDocumentation>,
    /// Unique string used to identify the operation. The id MUST be unique among all operations described in the API. The operationId value is case-sensitive. Tools and libraries MAY use the operationId to uniquely identify an operation, therefore, it is RECOMMENDED to follow common programming naming conventions.
    pub operation_id: Option<String>,
    /// A list of parameters that are applicable for this operation. If a parameter is already defined at the Path Item, the new definition will override it but can never remove it. The list MUST NOT include duplicated parameters. A unique parameter is defined by a combination of a name and location. The list can use the Reference Object to link to parameters that are defined at the OpenAPI Object's components/parameters.
    pub parameters: Option<Vec<Referenceable<Parameter>>>,
    /// The request body applicable for this operation. The requestBody is only supported in HTTP methods where the HTTP 1.1 specification RFC7231 has explicitly defined semantics for request bodies. In other cases where the HTTP spec is vague, requestBody SHALL be ignored by consumers.
    pub request_body: Option<Referenceable<RequestBody>>,
    /// The list of possible responses as they are returned from executing this operation.
    pub responses: Responses,
    /// A map of possible out-of band callbacks related to the parent operation. The key is a unique identifier for the Callback Object. Each value in the map is a Callback Object that describes a request that may be initiated by the API provider and the expected responses.
    pub callbacks: Option<BTreeMap<String, Referenceable<Callback>>>,
    /// Declares this operation to be deprecated. Consumers SHOULD refrain from usage of the declared operation. Default value is `false`.
    pub deprecated: Option<bool>,
    /// A declaration of which security mechanisms can be used for this operation. The list of values includes alternative security requirement objects that can be used. Only one of the security requirement objects need to be satisfied to authorize a request. To make security optional, an empty security requirement (`{}`) can be included in the array. This definition overrides any declared top-level security. To remove a top-level security declaration, an empty array can be used.
    pub security: Option<Vec<SecurityRequirement>>,
    /// An alternative server array to service this operation. If an alternative server object is specified at the Path Item Object or Root level, it will be overridden by this value.
    pub servers: Option<Vec<Server>>,
}

/// Allows referencing an external resource for extended documentation.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocumentation {
    /// A short description of the target documentation. CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// The URL for the target documentation. Value MUST be in the format of a URL.
    pub url: String,
}

/// The location of the parameter
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterIn {
    Query,
    Header,
    Path,
    Cookie,
}

/// Describes a single operation parameter.
/// A unique parameter is defined by a combination of a name and location.
/// Parameter Locations
/// There are four possible parameter locations specified by the in field:
/// - path - Used together with Path Templating, where the parameter value is actually part of the operation's URL. This does not include the host or base path of the API. For example, in /items/{itemId}, the path parameter is itemId.
/// - query - Parameters that are appended to the URL. For example, in /items?id=###, the query parameter is id.
/// - header - Custom headers that are expected as part of the request. Note that RFC7230 states header names are case insensitive.
/// - cookie - Used to pass a specific cookie value to the API.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    /// The name of the parameter
    pub name: String,
    /// The location of the parameter
    #[serde(alias = "in")]
    pub _in: ParameterIn,
    /// A brief description of the parameter. This could contain examples of use. CommonMark syntax MAY be used for rich text representation.
    pub description: Option<String>,
    /// Determines whether this parameter is mandatory
    pub required: Option<bool>,
    /// Specifies that a parameter is deprecated and SHOULD be transitioned out of usage. Default value is `false`.
    pub deprecated: Option<bool>,
    /// Sets the ability to pass empty-valued parameters
    pub allow_empty_value: Option<bool>,
    /// Describes how the parameter value will be serialized depending on the type of the parameter value
    pub style: Option<String>,
    pub explode: Option<bool>,
    pub allow_reserved: Option<bool>,
    /// The schema defining the type used for the parameter.
    pub schema: Option<Referenceable<Schema>>,
    /// Example of the parameter's potential value.
    pub example: Option<Any>,
    /// Examples of the parameter's potential value.
    pub examples: Option<BTreeMap<String, Referenceable<Example>>>,
    /// A map containing the representations for the parameter. The key is the media type and the value describes it.
    pub content: Option<BTreeMap<String, MediaType>>,
}

/// Describes a single request body.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    /// A brief description of the request body.
    pub description: Option<String>,
    /// Determines if the request body is required in the request. Defaults to `false`.
    pub required: Option<bool>,
    /// The content of the request body.
    pub content: BTreeMap<String, MediaType>,
}

/// Each Media Type Object provides schema and examples for the media type identified by its key.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    /// The schema defining the content of the request, response, or parameter.
    pub schema: Option<Referenceable<Schema>>,
    /// Example of the media type.
    pub example: Option<Any>,
    /// Examples of the media type.
    pub examples: Option<BTreeMap<String, Referenceable<Example>>>,
    /// A map between a property name and its encoding information.
    pub encoding: Option<BTreeMap<String, Encoding>>,
}

/// A single encoding definition applied to a single schema property.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Encoding {
    /// The Content-Type for encoding a specific property.
    pub content_type: Option<String>,
    /// map allowing additional information to be provided as headers, for example `Content-Disposition`. `Content-Type` is described separately and SHALL be ignored in this section. This property SHALL be ignored if the request body media type is not a `multipart`.
    pub headers: Option<BTreeMap<String, Referenceable<Header>>>,
    /// Describes how a specific property value will be serialized depending on its type.
    pub style: Option<String>,
    pub explode: Option<bool>,
    pub allow_reserved: Option<bool>,
}

/// A container for the expected responses of an operation. The container maps a HTTP response code to the expected response.
/// The documentation is not necessarily expected to cover all possible HTTP response codes because they may not be known in advance. However, documentation is expected to cover a successful operation response and any known errors.
/// The default MAY be used as a default response object for all HTTP codes that are not covered individually by the specification.
/// The Responses Object MUST contain at least one response code, and it SHOULD be the response for a successful operation call.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Responses {
    /// The documentation of responses other than the ones declared for specific HTTP response codes. Use this field to cover undeclared responses. A Reference Object can link to a response that the OpenAPI Object's components/responses section defines.
    pub default: Option<Referenceable<Response>>,
    #[serde(flatten)]
    pub data: BTreeMap<String, Referenceable<Response>>,
}

/// Describes a single response from an API Operation, including design-time, static `links` to operations based on the response.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// A short description of the response.
    pub description: String,
    /// Maps a header name to its definition.
    pub headers: Option<BTreeMap<String, Referenceable<Header>>>,
    /// A map containing descriptions of potential response payloads.
    pub content: Option<BTreeMap<String, MediaType>>,
    /// A map of operations links that can be followed from the response.
    pub links: Option<BTreeMap<String, Referenceable<Link>>>,
}

/// A map of possible out-of band callbacks related to the parent operation. Each value in the map is a Path Item Object that describes a set of requests that may be initiated by the API provider and the expected responses. The key value used to identify the path item object is an expression, evaluated at runtime, that identifies a URL to use for the callback operation.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Callback {
    #[serde(flatten)]
    pub data: BTreeMap<String, PathItem>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    /// Short description for the example.
    pub summary: Option<String>,
    /// Long description for the example.
    pub description: Option<String>,
    /// Embedded literal example.
    pub value: Option<Any>,
    pub external_value: Option<String>,
}

pub type Any = serde_json::Value;

/// represents a possible design-time link for a response.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// A relative or absolute URI reference to an OAS operation.
    pub operation_ref: Option<String>,
    /// The name of an existing, resolvable OAS operation
    pub operation_id: String,
    /// A map representing parameters to pass to an operation as specified with `operation_id` or identified via `operation_ef`.
    pub parameters: Option<BTreeMap<String, Any>>,
    /// A literal value or `{expression}` to use as a request body when calling the target operation.
    pub request_body: Option<Any>,
    /// A description of the link.
    pub description: Option<String>,
    /// A server object to be used by the target operation.
    pub server: Option<Server>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    pub description: Option<String>,
    pub required: Option<bool>,
    pub deprecated: Option<bool>,
    pub allow_empty_value: Option<bool>,
    pub style: Option<String>,
    pub explode: Option<bool>,
    pub allow_reserved: Option<bool>,
    pub schema: Option<Referenceable<Schema>>,
    pub example: Option<Any>,
    pub examples: Option<BTreeMap<String, Referenceable<Example>>>,
    pub content: Option<BTreeMap<String, MediaType>>,
}

/// Adds metadata to a single tag that is used by the `Operation` Object. It is not mandatory to have a Tag Object per tag defined in the Operation Object instances.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    /// The name of the tag.
    pub name: String,
    /// A short description for the tag.
    pub description: Option<String>,
    /// Additional external documentation for this tag.
    pub external_docs: Option<ExternalDocumentation>,
}

impl Tag {
    pub fn new(name: impl Into<String>, description: Option<impl Into<String>>) -> Tag {
        Self {
            name: name.into(),
            description: description.map(|d| d.into()),
            external_docs: None,
        }
    }

    pub fn simple(name: impl Into<String>) -> Tag {
        Self {
            name: name.into(),
            description: None,
            external_docs: None,
        }
    }

    pub fn with_description(name: impl Into<String>, description: impl Into<String>) -> Tag {
        Self {
            name: name.into(),
            description: Some(description.into()),
            external_docs: None,
        }
    }
}

// Convenience constructors for main types
impl OpenAPIV3 {
    pub fn new(info: Info) -> Self {
        Self {
            openapi: "3.0.0".to_string(),
            info,
            servers: None,
            paths: BTreeMap::new(),
            components: None,
            security: None,
            tags: None,
            external_docs: None,
            extras: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.info.description = Some(description.into());
        self
    }

    pub fn with_paths(mut self, paths: BTreeMap<String, PathItem>) -> Self {
        self.paths = paths;
        self
    }

    pub fn with_components(mut self, components: Components) -> Self {
        self.components = Some(components);
        self
    }

    pub fn with_servers(mut self, servers: Vec<Server>) -> Self {
        self.servers = Some(servers);
        self
    }
}

impl Info {
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            terms_of_service: None,
            contact: None,
            license: None,
            version: version.into(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_contact(mut self, contact: Contact) -> Self {
        self.contact = Some(contact);
        self
    }

    pub fn with_license(mut self, license: License) -> Self {
        self.license = Some(license);
        self
    }
}

impl Contact {
    pub fn new() -> Self {
        Self {
            name: None,
            url: None,
            email: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }
}

impl Default for Contact {
    fn default() -> Self {
        Self::new()
    }
}

impl License {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: None,
        }
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

impl Server {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            description: None,
            variables: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_variables(mut self, variables: BTreeMap<String, ServerVariable>) -> Self {
        self.variables = Some(variables);
        self
    }
}

impl ServerVariable {
    pub fn new(default: impl Into<String>) -> Self {
        Self {
            _enum: None,
            default: default.into(),
            description: None,
        }
    }

    pub fn with_enum(mut self, enum_values: Vec<String>) -> Self {
        self._enum = Some(enum_values);
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl PathItem {
    pub fn new() -> Self {
        Self {
            _ref: None,
            summary: None,
            description: None,
            get: None,
            put: None,
            post: None,
            delete: None,
            options: None,
            head: None,
            patch: None,
            trace: None,
            servers: None,
            parameters: None,
        }
    }

    pub fn with_get(mut self, operation: Operation) -> Self {
        self.get = Some(operation);
        self
    }

    pub fn with_post(mut self, operation: Operation) -> Self {
        self.post = Some(operation);
        self
    }

    pub fn with_put(mut self, operation: Operation) -> Self {
        self.put = Some(operation);
        self
    }

    pub fn with_delete(mut self, operation: Operation) -> Self {
        self.delete = Some(operation);
        self
    }

    pub fn with_patch(mut self, operation: Operation) -> Self {
        self.patch = Some(operation);
        self
    }
}

impl Default for PathItem {
    fn default() -> Self {
        Self::new()
    }
}

impl Operation {
    pub fn new(responses: Responses) -> Self {
        Self {
            tags: None,
            summary: None,
            description: None,
            external_docs: None,
            operation_id: None,
            parameters: None,
            request_body: None,
            responses,
            callbacks: None,
            deprecated: None,
            security: None,
            servers: None,
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_operation_id(mut self, operation_id: impl Into<String>) -> Self {
        self.operation_id = Some(operation_id.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_parameters(mut self, parameters: Vec<Referenceable<Parameter>>) -> Self {
        self.parameters = Some(parameters);
        self
    }

    pub fn with_request_body(mut self, request_body: Referenceable<RequestBody>) -> Self {
        self.request_body = Some(request_body);
        self
    }
}

impl Parameter {
    pub fn new(name: impl Into<String>, location: ParameterIn) -> Self {
        Self {
            name: name.into(),
            _in: location,
            description: None,
            required: None,
            deprecated: None,
            allow_empty_value: None,
            style: None,
            explode: None,
            allow_reserved: None,
            schema: None,
            example: None,
            examples: None,
            content: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    pub fn with_schema(mut self, schema: Referenceable<Schema>) -> Self {
        self.schema = Some(schema);
        self
    }
}

impl RequestBody {
    pub fn new(content: BTreeMap<String, MediaType>) -> Self {
        Self {
            description: None,
            required: None,
            content,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }
}

impl Response {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            headers: None,
            content: None,
            links: None,
        }
    }

    pub fn with_content(mut self, content: BTreeMap<String, MediaType>) -> Self {
        self.content = Some(content);
        self
    }

    pub fn with_headers(mut self, headers: BTreeMap<String, Referenceable<Header>>) -> Self {
        self.headers = Some(headers);
        self
    }
}

impl Responses {
    pub fn new() -> Self {
        Self {
            default: None,
            data: BTreeMap::new(),
        }
    }

    pub fn with_status(mut self, status: impl Into<String>, response: Referenceable<Response>) -> Self {
        self.data.insert(status.into(), response);
        self
    }

    pub fn with_default(mut self, response: Referenceable<Response>) -> Self {
        self.default = Some(response);
        self
    }
}

impl Default for Responses {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaType {
    pub fn new() -> Self {
        Self {
            schema: None,
            example: None,
            examples: None,
            encoding: None,
        }
    }

    pub fn with_schema(mut self, schema: Referenceable<Schema>) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_example(mut self, example: Any) -> Self {
        self.example = Some(example);
        self
    }
}

impl Default for MediaType {
    fn default() -> Self {
        Self::new()
    }
}

impl Schema {
    pub fn new() -> Self {
        Self {
            _type: None,
            format: None,
            nullable: None,
            description: None,
            extras: BTreeMap::new(),
        }
    }

    pub fn with_type(mut self, schema_type: impl Into<String>) -> Self {
        self._type = Some(schema_type.into());
        self
    }

    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn string() -> Self {
        Self::new().with_type("string")
    }

    pub fn integer() -> Self {
        Self::new().with_type("integer")
    }

    pub fn number() -> Self {
        Self::new().with_type("number")
    }

    pub fn boolean() -> Self {
        Self::new().with_type("boolean")
    }

    pub fn array() -> Self {
        Self::new().with_type("array")
    }

    pub fn object() -> Self {
        Self::new().with_type("object")
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

impl Reference {
    pub fn new(reference: impl Into<String>) -> Self {
        Self {
            _ref: reference.into(),
        }
    }
}

impl Components {
    pub fn new() -> Self {
        Self {
            schemas: None,
            responses: None,
            parameters: None,
            examples: None,
            request_bodies: None,
            headers: None,
            security_schemes: None,
            links: None,
            callbacks: None,
        }
    }

    pub fn with_schemas(mut self, schemas: BTreeMap<String, Referenceable<Schema>>) -> Self {
        self.schemas = Some(schemas);
        self
    }

    pub fn with_responses(mut self, responses: BTreeMap<String, Referenceable<Response>>) -> Self {
        self.responses = Some(responses);
        self
    }

    pub fn with_parameters(mut self, parameters: BTreeMap<String, Referenceable<Parameter>>) -> Self {
        self.parameters = Some(parameters);
        self
    }
}

impl Default for Components {
    fn default() -> Self {
        Self::new()
    }
}

// Common helper functions and convenience methods
impl Referenceable<Schema> {
    /// Create a reference to a schema component
    pub fn schema_ref(name: impl Into<String>) -> Self {
        Self::component_ref("schemas", name)
    }

    /// Create inline string schema
    pub fn string_schema() -> Self {
        Self::data(Schema::string())
    }

    /// Create inline integer schema
    pub fn integer_schema() -> Self {
        Self::data(Schema::integer())
    }

    /// Create inline number schema
    pub fn number_schema() -> Self {
        Self::data(Schema::number())
    }

    /// Create inline boolean schema
    pub fn boolean_schema() -> Self {
        Self::data(Schema::boolean())
    }

    /// Create inline array schema
    pub fn array_schema() -> Self {
        Self::data(Schema::array())
    }

    /// Create inline object schema
    pub fn object_schema() -> Self {
        Self::data(Schema::object())
    }
}

impl Referenceable<Response> {
    /// Create a reference to a response component
    pub fn response_ref(name: impl Into<String>) -> Self {
        Self::component_ref("responses", name)
    }

    /// Create a simple success response
    pub fn ok(description: impl Into<String>) -> Self {
        Self::data(Response::new(description))
    }

    /// Create a simple error response
    pub fn error(description: impl Into<String>) -> Self {
        Self::data(Response::new(description))
    }

    /// Add content to response if it contains data
    pub fn with_content(self, content: BTreeMap<String, MediaType>) -> Self {
        match self {
            Self::Data(response) => Self::data(response.with_content(content)),
            Self::Reference(r) => Self::Reference(r),
        }
    }

    /// Add headers to response if it contains data
    pub fn with_headers(self, headers: BTreeMap<String, Referenceable<Header>>) -> Self {
        match self {
            Self::Data(response) => Self::data(response.with_headers(headers)),
            Self::Reference(r) => Self::Reference(r),
        }
    }
}

impl Referenceable<Parameter> {
    /// Create a reference to a parameter component
    pub fn parameter_ref(name: impl Into<String>) -> Self {
        Self::component_ref("parameters", name)
    }

    /// Create a query parameter
    pub fn query_param(name: impl Into<String>) -> Self {
        Self::data(Parameter::new(name, ParameterIn::Query))
    }

    /// Create a path parameter
    pub fn path_param(name: impl Into<String>) -> Self {
        Self::data(Parameter::new(name, ParameterIn::Path).with_required(true))
    }

    /// Create a header parameter
    pub fn header_param(name: impl Into<String>) -> Self {
        Self::data(Parameter::new(name, ParameterIn::Header))
    }

    /// Add schema to parameter if it contains data
    pub fn with_schema(self, schema: Referenceable<Schema>) -> Self {
        match self {
            Self::Data(param) => Self::data(param.with_schema(schema)),
            Self::Reference(r) => Self::Reference(r),
        }
    }

    /// Add description to parameter if it contains data
    pub fn with_description(self, description: impl Into<String>) -> Self {
        match self {
            Self::Data(param) => Self::data(param.with_description(description)),
            Self::Reference(r) => Self::Reference(r),
        }
    }

    /// Mark parameter as required if it contains data
    pub fn with_required(self, required: bool) -> Self {
        match self {
            Self::Data(param) => Self::data(param.with_required(required)),
            Self::Reference(r) => Self::Reference(r),
        }
    }
}

impl Referenceable<RequestBody> {
    /// Create a reference to a request body component
    pub fn request_body_ref(name: impl Into<String>) -> Self {
        Self::component_ref("requestBodies", name)
    }

    /// Create a JSON request body
    pub fn json_body(schema: Referenceable<Schema>) -> Self {
        let mut content = BTreeMap::new();
        content.insert(
            "application/json".to_string(),
            MediaType::new().with_schema(schema),
        );
        Self::data(RequestBody::new(content))
    }
}

// Additional convenience methods for PathItem
impl PathItem {
    /// Add multiple HTTP methods at once
    pub fn with_operations(
        mut self,
        operations: Vec<(&str, Operation)>,
    ) -> Self {
        for (method, operation) in operations {
            match method.to_lowercase().as_str() {
                "get" => self.get = Some(operation),
                "post" => self.post = Some(operation),
                "put" => self.put = Some(operation),
                "delete" => self.delete = Some(operation),
                "patch" => self.patch = Some(operation),
                "options" => self.options = Some(operation),
                "head" => self.head = Some(operation),
                "trace" => self.trace = Some(operation),
                _ => {} // Ignore unknown methods
            }
        }
        self
    }
}

// Additional convenience methods for OpenAPIV3
impl OpenAPIV3 {
    /// Add a single path
    pub fn add_path(mut self, path: impl Into<String>, path_item: PathItem) -> Self {
        self.paths.insert(path.into(), path_item);
        self
    }

    /// Add multiple paths at once
    pub fn add_paths(mut self, paths: Vec<(impl Into<String>, PathItem)>) -> Self {
        for (path, path_item) in paths {
            self.paths.insert(path.into(), path_item);
        }
        self
    }

    /// Add a single server
    pub fn add_server(mut self, server: Server) -> Self {
        self.servers.get_or_insert_with(Vec::new).push(server);
        self
    }

    /// Add a tag
    pub fn add_tag(mut self, tag: Tag) -> Self {
        self.tags.get_or_insert_with(Vec::new).push(tag);
        self
    }
}

// Builder pattern for complex operations
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
        self.operation.tags.get_or_insert_with(Vec::new).push(tag.into());
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.operation.tags = Some(tags);
        self
    }

    pub fn parameter(mut self, parameter: Referenceable<Parameter>) -> Self {
        self.operation.parameters.get_or_insert_with(Vec::new).push(parameter);
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

    pub fn response(mut self, status: impl Into<String>, response: Referenceable<Response>) -> Self {
        self.operation.responses.data.insert(status.into(), response);
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
pub mod builders {
    use super::*;

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
}

/// A simple object to allow referencing other components in the specification, internally and externally.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// The reference string.
    #[serde(rename = "$ref")]
    pub _ref: String,
}

/// The Schema Object allows the definition of input and output data types. These types can be objects, but also primitives and arrays.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    #[serde(rename = "type")]
    pub _type: Option<String>,
    pub format: Option<String>,
    pub nullable: Option<bool>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

/// When request bodies or response payloads may be one of a number of different schemas, a `discriminator` object can be used to aid in serialization, deserialization, and validation. The discriminator is a specific object in a schema which is used to inform the consumer of the specification of an alternative schema based on the value associated with it.

/// When using the discriminator, inline schemas will not be considered.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discriminator {
    /// The name of the property in the payload that will hold the discriminator value.
    pub property_name: String,
    /// An object to hold mappings between payload values and schema names or references.
    pub maapping: Option<BTreeMap<String, String>>,
}

/// The type of the security scheme.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum SecurityType {
    ApiKey {
        /// The name of the header
        name: String,
        /// The location of the API key. Valid values are `query`, `header or `cookie`.
        #[serde(rename = "in")]
        _in: ParameterIn,
    },
    Http {
        /// The name of the HTTP Authorization scheme to be used in the Authorization header as defined in RFC7235. The values used SHOULD be registered in the IANA Authentication Scheme registry.
        scheme: String,
        /// A hint to the client to identify how the bearer token is formatted. Bearer tokens are usually generated by an authorization server, so this information is primarily for documentation purposes.
        #[serde(rename = "bearerFormat")]
        bearer_format: Option<String>,
    },
    Oauth2 {
        /// An object containing configuration information for the flow types supported.
        flows: OauthFlows,
    },
    OpenIdConnect {
        /// OpenId Connect URL to discover OAuth2 configuration values. This MUST be in the form of a URL.
        open_id_connect_url: String,
    },
}

/// Defines a security scheme that can be used by the operations.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScheme {
    #[serde(flatten)]
    pub _type: SecurityType,
    /// A short description for security scheme.
    pub description: Option<String>,
}

// todo should be enum
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthFlows {
    /// Configuration for the OAuth Implicit flow
    pub implicit: Option<OauthFlow>,
    /// Configuration for the OAuth Resource Owner Password flow
    pub password: Option<OauthFlow>,
    /// Configuration for the OAuth Client Credentials flow.
    pub client_credentials: Option<OauthFlow>,
    /// Configuration for the OAuth Authorization Code flow.
    pub authorization_code: Option<OauthFlow>,
}

/// Configuration details for a supported OAuth Flow
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthFlow {
    /// The authorization URL to be used for this flow. This MUST be in the form of a URL.
    pub authorization_url: String,
    /// he token URL to be used for this flow. This MUST be in the form of a URL.
    pub token_url: Option<String>,
    /// The URL to be used for obtaining refresh tokens. This MUST be in the form of a URL.
    pub refresh_url: Option<String>,
    /// The available scopes for the OAuth2 security scheme. A map between the scope name and a short description for it. The map MAY be empty.
    pub scopes: BTreeMap<String, String>,
}

/// Lists the required security schemes to execute this operation.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecurityRequirement {
    #[serde(flatten)]
    pub data: BTreeMap<String, Vec<String>>,
}

macro_rules! impl_serde_json {
    ($($st:ty,)+) => {
        $(
        impl $st {

            pub fn to_string(&self) -> String {
                serde_json::to_string(&self).unwrap()
            }
            pub fn to_value(&self) -> serde_json::Value {
                serde_json::to_value(&self).unwrap()
            }
        }
        )+
    };
}
impl_serde_json! {
    OpenAPIV3, Info, Contact, License, Server, ServerVariable, Components, PathItem,
    Operation, ExternalDocumentation, ParameterIn, Parameter, RequestBody, MediaType,
    Encoding, Responses, Response, Callback, Example, Link, Header, Tag, Reference,
    Schema, Discriminator, SecurityType, SecurityScheme, OauthFlows, OauthFlow, SecurityRequirement,
}

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
