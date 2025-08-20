//! Core OpenAPI 3.0 type definitions.
//!
//! This module contains all the main data structures that represent the OpenAPI 3.0 specification,
//! including the root document, schemas, operations, parameters, responses, and related types.

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

/// Type alias for arbitrary JSON values.
pub type Any = serde_json::Value;

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

/// The object provides metadata about the API.
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
    /// The version of the OpenAPI document.
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
    /// The email address of the contact person/organization.
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
    /// A URL to the target host.
    pub url: String,
    /// An optional string describing the host designated by the URL.
    pub description: Option<String>,
    /// A map between a variable name and its value.
    pub variables: Option<BTreeMap<String, ServerVariable>>,
}

/// An object representing a Server Variable for server URL template substitution.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerVariable {
    /// An enumeration of string values to be used if the substitution options are from a limited set.
    #[serde(rename = "enum")]
    pub _enum: Option<Vec<String>>,
    /// The default value to use for substitution.
    pub default: String,
    /// An optional description for the server variable.
    pub description: Option<String>,
}

/// Holds a set of reusable objects for different aspects of the OAS.
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

/// Describes the operations available on a single path.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    /// Allows for an external definition of this path item.
    #[serde(rename = "$ref")]
    pub _ref: Option<String>,
    /// An optional, string summary, intended to apply to all operations in this path.
    pub summary: Option<String>,
    /// An optional, string description, intended to apply to all operations in this path.
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
    /// A list of parameters that are applicable for all the operations described under this path.
    pub parameters: Option<Vec<Referenceable<Parameter>>>,
}

/// Describes a single API operation on a path.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    /// A list of tags for API documentation control.
    pub tags: Option<Vec<String>>,
    /// A short summary of what the operation does.
    pub summary: Option<String>,
    /// A verbose explanation of the operation behavior.
    pub description: Option<String>,
    /// Additional external documentation for this operation.
    pub external_docs: Option<ExternalDocumentation>,
    /// Unique string used to identify the operation.
    pub operation_id: Option<String>,
    /// A list of parameters that are applicable for this operation.
    pub parameters: Option<Vec<Referenceable<Parameter>>>,
    /// The request body applicable for this operation.
    pub request_body: Option<Referenceable<RequestBody>>,
    /// The list of possible responses as they are returned from executing this operation.
    pub responses: Responses,
    /// A map of possible out-of band callbacks related to the parent operation.
    pub callbacks: Option<BTreeMap<String, Referenceable<Callback>>>,
    /// Declares this operation to be deprecated.
    pub deprecated: Option<bool>,
    /// A declaration of which security mechanisms can be used for this operation.
    pub security: Option<Vec<SecurityRequirement>>,
    /// An alternative server array to service this operation.
    pub servers: Option<Vec<Server>>,
}

/// Allows referencing an external resource for extended documentation.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocumentation {
    /// A short description of the target documentation.
    pub description: Option<String>,
    /// The URL for the target documentation.
    pub url: String,
}

/// The location of the parameter.
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
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    /// The name of the parameter
    pub name: String,
    /// The location of the parameter
    #[serde(alias = "in")]
    pub _in: ParameterIn,
    /// A brief description of the parameter.
    pub description: Option<String>,
    /// Determines whether this parameter is mandatory
    pub required: Option<bool>,
    /// Specifies that a parameter is deprecated.
    pub deprecated: Option<bool>,
    /// Sets the ability to pass empty-valued parameters
    pub allow_empty_value: Option<bool>,
    /// Describes how the parameter value will be serialized.
    pub style: Option<String>,
    pub explode: Option<bool>,
    pub allow_reserved: Option<bool>,
    /// The schema defining the type used for the parameter.
    pub schema: Option<Referenceable<Schema>>,
    /// Example of the parameter's potential value.
    pub example: Option<Any>,
    /// Examples of the parameter's potential value.
    pub examples: Option<BTreeMap<String, Referenceable<Example>>>,
    /// A map containing the representations for the parameter.
    pub content: Option<BTreeMap<String, MediaType>>,
}

/// Describes a single request body.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    /// A brief description of the request body.
    pub description: Option<String>,
    /// Determines if the request body is required in the request.
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
    /// Map allowing additional information to be provided as headers.
    pub headers: Option<BTreeMap<String, Referenceable<Header>>>,
    /// Describes how a specific property value will be serialized.
    pub style: Option<String>,
    pub explode: Option<bool>,
    pub allow_reserved: Option<bool>,
}

/// A container for the expected responses of an operation.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Responses {
    /// The documentation of responses other than the ones declared for specific HTTP response codes.
    pub default: Option<Referenceable<Response>>,
    #[serde(flatten)]
    pub data: BTreeMap<String, Referenceable<Response>>,
}

/// Describes a single response from an API Operation.
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

/// A map of possible out-of band callbacks related to the parent operation.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Callback {
    #[serde(flatten)]
    pub data: BTreeMap<String, PathItem>,
}

/// Example object.
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

/// Represents a possible design-time link for a response.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// A relative or absolute URI reference to an OAS operation.
    pub operation_ref: Option<String>,
    /// The name of an existing, resolvable OAS operation
    pub operation_id: String,
    /// A map representing parameters to pass to an operation.
    pub parameters: Option<BTreeMap<String, Any>>,
    /// A literal value or `{expression}` to use as a request body.
    pub request_body: Option<Any>,
    /// A description of the link.
    pub description: Option<String>,
    /// A server object to be used by the target operation.
    pub server: Option<Server>,
}

/// Header object.
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

/// Adds metadata to a single tag that is used by the `Operation` Object.
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

/// A simple object to allow referencing other components in the specification.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// The reference string.
    #[serde(rename = "$ref")]
    pub _ref: String,
}

impl Reference {
    pub fn new(reference: impl Into<String>) -> Self {
        Self {
            _ref: reference.into(),
        }
    }
}

/// The Schema Object allows the definition of input and output data types.
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

/// When request bodies or response payloads may be one of a number of different schemas.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discriminator {
    /// The name of the property in the payload that will hold the discriminator value.
    pub property_name: String,
    /// An object to hold mappings between payload values and schema names or references.
    pub maapping: Option<BTreeMap<String, String>>,
}

// Import security types
use crate::security::{SecurityRequirement, SecurityScheme};