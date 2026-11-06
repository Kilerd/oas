//! OpenAPI types with JSON Schema 2020-12 and OpenAPI 3.2 streaming support.
//!
//! The core document and operation models retain their 3.0 defaults. Schema positions
//! additionally support modern schemas and streaming items; see MIGRATION.md for
//! the remaining 3.1/3.2 model differences.

use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

pub use crate::schema::SchemaCount;

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
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Referenceable<T> {
    /// Reference to a component
    Reference(Reference),
    /// Inline data
    Data(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Referenceable<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if keeps_schema_reference_siblings::<T>() {
            // JSON Schema 2020-12 makes `$ref` an ordinary keyword with
            // sibling keywords, so a schema position keeps its schema-first
            // rule. A strict Reference Object still selects `Reference`, and
            // malformed inline siblings fall back to reference extras.
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum Candidate<T> {
                Reference(#[serde(deserialize_with = "deserialize_reference_object")] Reference),
                Data(T),
                ReferenceWithAdditionalProperties(Reference),
            }

            return Ok(match Candidate::deserialize(deserializer)? {
                Candidate::Reference(reference)
                | Candidate::ReferenceWithAdditionalProperties(reference) => {
                    Self::Reference(reference)
                }
                Candidate::Data(data) => Self::Data(data),
            });
        }

        // OpenAPI 3.1 §4.8.23: an object whose `$ref` is a string is a
        // Reference Object. Other properties are ignored semantically; the
        // strict model retains them in `Reference::extras` for editing.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Candidate<T> {
            Reference(ReferenceFirst),
            Data(T),
        }

        Ok(match Candidate::deserialize(deserializer)? {
            Candidate::Reference(ReferenceFirst(reference)) => Self::Reference(reference),
            Candidate::Data(data) => Self::Data(data),
        })
    }
}

/// JSON Schema 2020-12 gives `$ref` sibling keywords meaning, so the legacy
/// `Referenceable<Schema>` target keeps the schema-first rule. Every other
/// target follows OpenAPI's Reference Object rule.
fn keeps_schema_reference_siblings<T>() -> bool {
    std::any::type_name::<T>() == std::any::type_name::<Schema>()
}

/// First untagged candidate: a map with a string `$ref` is a Reference
/// Object, even when it also carries properties an inline object would accept.
struct ReferenceFirst(Reference);

impl<'de> Deserialize<'de> for ReferenceFirst {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ReferenceVisitor;

        impl<'de> serde::de::Visitor<'de> for ReferenceVisitor {
            type Value = ReferenceFirst;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a Reference Object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut reference = None;
                let mut summary = None;
                let mut description = None;
                let mut extras = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "$ref" => reference = Some(map.next_value::<String>()?),
                        "summary" => summary = map.next_value::<Option<String>>()?,
                        "description" => description = map.next_value::<Option<String>>()?,
                        _ => {
                            extras.insert(key, map.next_value::<Any>()?);
                        }
                    }
                }
                match reference {
                    Some(reference) => Ok(ReferenceFirst(Reference {
                        _ref: reference,
                        summary,
                        description,
                        extras,
                    })),
                    None => Err(serde::de::Error::custom("expected a Reference Object")),
                }
            }
        }

        deserializer.deserialize_any(ReferenceVisitor)
    }
}

fn deserialize_reference_object<'de, D>(deserializer: D) -> Result<Reference, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Only the Reference Object shape takes precedence over inline data. In
    // particular, schema references with sibling keywords must remain schemas.
    #[derive(Deserialize)]
    #[serde(remote = "Reference", deny_unknown_fields)]
    struct ReferenceObject {
        #[serde(rename = "$ref")]
        _ref: String,
        summary: Option<String>,
        description: Option<String>,
        #[serde(skip)]
        extras: BTreeMap<String, Any>,
    }

    ReferenceObject::deserialize(deserializer)
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
        Self::Reference(Reference::new(format!(
            "#/components/{}/{}",
            component_type,
            name.into()
        )))
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

/// The Paths Object: every path template with its Path Item, plus `x-*`
/// extensions.
///
/// A key starting with `x-` is a Paths extension and never a path; the rest
/// are path templates. Indexing and map methods such as `insert`, `get`,
/// `values` and `retain` dereference to [`Paths::paths`].
#[derive(Debug, Clone, Default)]
pub struct Paths {
    /// Path templates such as `/users/{id}` and their Path Items.
    pub paths: BTreeMap<String, PathItem>,
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    pub extras: BTreeMap<String, Any>,
}

impl Paths {
    /// An empty Paths Object.
    pub fn new() -> Self {
        Self::default()
    }
}

impl std::ops::Deref for Paths {
    type Target = BTreeMap<String, PathItem>;

    fn deref(&self) -> &Self::Target {
        &self.paths
    }
}

impl std::ops::DerefMut for Paths {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.paths
    }
}

impl Serialize for Paths {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.paths.len() + self.extras.len()))?;
        for (key, value) in &self.paths {
            map.serialize_entry(key, value)?;
        }
        for (key, value) in &self.extras {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Paths {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let fields = BTreeMap::<String, Any>::deserialize(deserializer)?;
        let mut paths = BTreeMap::new();
        let mut extras = BTreeMap::new();
        for (key, value) in fields {
            if key.starts_with("x-") {
                extras.insert(key, value);
            } else {
                paths.insert(
                    key,
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?,
                );
            }
        }
        Ok(Self { paths, extras })
    }
}

impl<'a> IntoIterator for &'a Paths {
    type Item = (&'a String, &'a PathItem);
    type IntoIter = std::collections::btree_map::Iter<'a, String, PathItem>;

    fn into_iter(self) -> Self::IntoIter {
        self.paths.iter()
    }
}

impl<'a> IntoIterator for &'a mut Paths {
    type Item = (&'a String, &'a mut PathItem);
    type IntoIter = std::collections::btree_map::IterMut<'a, String, PathItem>;

    fn into_iter(self) -> Self::IntoIter {
        self.paths.iter_mut()
    }
}

impl IntoIterator for Paths {
    type Item = (String, PathItem);
    type IntoIter = std::collections::btree_map::IntoIter<String, PathItem>;

    fn into_iter(self) -> Self::IntoIter {
        self.paths.into_iter()
    }
}

/// The root OpenAPI v3 document, defaulting to 3.0.0 for existing constructors.
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
    /// Keys starting with `x-` are Paths extensions.
    pub paths: Paths,

    /// Incoming webhooks described by Path Item objects, keyed by name.
    ///
    /// OpenAPI 3.1 and later place webhooks at the document root; they are not
    /// part of `paths` and are not served by a route table.
    pub webhooks: Option<BTreeMap<String, PathItem>>,

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
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
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
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

/// License information for the exposed API.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// The license name used for the API.
    pub name: String,
    /// A URL to the license used for the API. MUST be in the format of a URL.
    pub url: Option<String>,
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
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
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
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
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

/// Holds a set of reusable objects for different aspects of the OAS.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Components {
    /// An object to hold reusable Schema Objects.
    pub schemas: Option<BTreeMap<String, SchemaValue>>,
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
    /// Reusable Path Item Objects (OpenAPI 3.1 and later).
    pub path_items: Option<BTreeMap<String, PathItem>>,
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

/// Describes the operations available on a single path.
///
/// Operations are boxed so an empty path item costs eight pointers instead of
/// eight inline operation objects; a document of empty path items otherwise
/// amplifies its JSON size by hundreds of times.
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
    pub get: Option<Box<Operation>>,
    /// A definition of a PUT operation on this path.
    pub put: Option<Box<Operation>>,
    /// A definition of a POST operation on this path.
    pub post: Option<Box<Operation>>,
    /// A definition of a DELETE operation on this path.
    pub delete: Option<Box<Operation>>,
    /// A definition of a OPTIONS operation on this path.
    pub options: Option<Box<Operation>>,
    /// A definition of a HEAD operation on this path.
    pub head: Option<Box<Operation>>,
    /// A definition of a PATCH operation on this path.
    pub patch: Option<Box<Operation>>,
    /// A definition of a TRACE operation on this path.
    pub trace: Option<Box<Operation>>,
    /// An alternative `server` array to service all operations in this path.
    pub servers: Option<Vec<Server>>,
    /// A list of parameters that are applicable for all the operations described under this path.
    pub parameters: Option<Vec<Referenceable<Parameter>>>,
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
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
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

/// Allows referencing an external resource for extended documentation.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocumentation {
    /// A short description of the target documentation.
    pub description: Option<String>,
    /// The URL for the target documentation.
    pub url: String,
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

// Preserve the published path while both model modules depend on the shared leaf.
pub use crate::parameter_location::ParameterIn;

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
    pub schema: Option<SchemaValue>,
    /// Example of the parameter's potential value.
    #[serde(default, deserialize_with = "crate::schema::present")]
    pub example: Option<Any>,
    /// Examples of the parameter's potential value.
    pub examples: Option<BTreeMap<String, Referenceable<Example>>>,
    /// A map containing the representations for the parameter.
    pub content: Option<BTreeMap<String, MediaType>>,
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
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
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

/// Each Media Type Object provides schema and examples for the media type identified by its key.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    /// The schema defining the content of the request, response, or parameter.
    pub schema: Option<SchemaValue>,
    /// OpenAPI 3.2 schema for each parsed item in a sequential media type.
    /// Unlike `schema`, this does not describe the complete body.
    #[serde(rename = "itemSchema")]
    pub item_schema: Option<SchemaValue>,
    /// Example of the media type.
    #[serde(default, deserialize_with = "crate::schema::present")]
    pub example: Option<Any>,
    /// Examples of the media type.
    pub examples: Option<BTreeMap<String, Referenceable<Example>>>,
    /// A map between a property name and its encoding information.
    pub encoding: Option<BTreeMap<String, Encoding>>,
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
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
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

/// A container for the expected responses of an operation.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct Responses {
    /// The documentation of responses other than the ones declared for specific HTTP response codes.
    pub default: Option<Referenceable<Response>>,
    /// HTTP status codes (100–599) and uppercase ranges (1XX–5XX).
    #[serde(flatten)]
    pub data: BTreeMap<String, Referenceable<Response>>,
    /// Other container fields, including extensions, retained as ordinary JSON.
    /// Do not duplicate `default` or status keys in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

pub(crate) fn is_response_status(key: &str) -> bool {
    let bytes = key.as_bytes();
    bytes.len() == 3
        && (b'1'..=b'5').contains(&bytes[0])
        && (bytes[1..] == *b"XX" || bytes[1..].iter().all(u8::is_ascii_digit))
}

impl<'de> Deserialize<'de> for Responses {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let fields = BTreeMap::<String, Any>::deserialize(deserializer)?;
        let mut responses = Self::new();
        for (key, value) in fields {
            if key == "default" {
                responses.default =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            } else if is_response_status(&key) {
                responses.data.insert(
                    key,
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?,
                );
            } else {
                responses.extras.insert(key, value);
            }
        }
        Ok(responses)
    }
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
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

/// A map of possible out-of band callbacks related to the parent operation.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct Callback {
    /// Root reference, retaining explicit null and malformed values for selective consumers.
    #[cfg(feature = "lossless-schema-keywords")]
    #[serde(
        rename = "$ref",
        skip_serializing_if = "crate::lossless_component::Field::is_missing"
    )]
    pub reference: crate::lossless_component::Field<String>,
    /// Callback URL expressions, including URLs with an embedded `{expression}`.
    #[serde(flatten)]
    pub data: BTreeMap<String, PathItem>,
    /// Other container fields, including extensions, retained as ordinary JSON.
    /// Do not duplicate expression keys in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

impl<'de> Deserialize<'de> for Callback {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let fields = BTreeMap::<String, Any>::deserialize(deserializer)?;
        let mut callback = Self {
            #[cfg(feature = "lossless-schema-keywords")]
            reference: crate::lossless_component::Field::Missing,
            data: BTreeMap::new(),
            extras: BTreeMap::new(),
        };
        for (key, value) in fields {
            if key == "$ref" {
                #[cfg(feature = "lossless-schema-keywords")]
                {
                    callback.reference = crate::lossless_component::Field::from_value(value);
                }
                #[cfg(not(feature = "lossless-schema-keywords"))]
                {
                    callback.extras.insert(key, value);
                }
                continue;
            }
            // Reference Object annotations stay with the reference; every
            // other non-extension object is a callback expression, including a
            // constant URL without braces. Extensions and non-object values
            // remain opaque in extras, so one malformed entry does not reject
            // the whole document.
            if matches!(key.as_str(), "summary" | "description")
                || key.starts_with("x-")
                || !value.is_object()
            {
                callback.extras.insert(key, value);
            } else {
                callback.data.insert(
                    key,
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?,
                );
            }
        }
        Ok(callback)
    }
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
    #[serde(default, deserialize_with = "crate::schema::present")]
    pub value: Option<Any>,
    pub external_value: Option<String>,
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

/// Represents a possible design-time link for a response.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// A relative or absolute URI reference to an OAS operation.
    pub operation_ref: Option<String>,
    /// The name of an existing, resolvable OAS operation
    pub operation_id: Option<String>,
    /// A map representing parameters to pass to an operation.
    pub parameters: Option<BTreeMap<String, Any>>,
    /// A literal value or `{expression}` to use as a request body.
    #[serde(default, deserialize_with = "crate::schema::present")]
    pub request_body: Option<Any>,
    /// A description of the link.
    pub description: Option<String>,
    /// A server object to be used by the target operation.
    pub server: Option<Server>,
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
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
    pub schema: Option<SchemaValue>,
    #[serde(default, deserialize_with = "crate::schema::present")]
    pub example: Option<Any>,
    pub examples: Option<BTreeMap<String, Referenceable<Example>>>,
    pub content: Option<BTreeMap<String, MediaType>>,
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
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
    /// Unmodeled fields, including `x-*` extensions, retained as JSON values.
    /// Do not duplicate fields with dedicated members in this map.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

/// A simple object to allow referencing other components in the specification.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// The reference string.
    #[serde(rename = "$ref")]
    pub _ref: String,
    /// OpenAPI 3.1 summary overriding the referenced object's summary.
    pub summary: Option<String>,
    /// OpenAPI 3.1 description overriding the referenced object's description.
    pub description: Option<String>,
    /// Unmodeled properties retained for document editing, without reference semantics.
    #[serde(flatten)]
    pub extras: BTreeMap<String, Any>,
}

impl Reference {
    pub fn new(reference: impl Into<String>) -> Self {
        Self {
            _ref: reference.into(),
            summary: None,
            description: None,
            extras: BTreeMap::new(),
        }
    }
}

/// A JSON Schema `type` keyword: a single type or a union of types.
///
/// Use `SchemaType::Multiple(vec!["string".into(), "null".into()])` for
/// a nullable string in OpenAPI 3.1/3.2. Values are represented, not validated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaType {
    Single(String),
    Multiple(Vec<String>),
}

impl From<String> for SchemaType {
    fn from(value: String) -> Self {
        Self::Single(value)
    }
}

impl From<&str> for SchemaType {
    fn from(value: &str) -> Self {
        Self::Single(value.into())
    }
}

impl From<Vec<String>> for SchemaType {
    fn from(value: Vec<String>) -> Self {
        Self::Multiple(value)
    }
}

/// A schema in any OpenAPI schema position, including boolean schemas.
///
/// JSON Schema references are ordinary schema objects: `$ref` and its sibling
/// keywords are preserved together. They do not use OpenAPI Reference Object
/// semantics. Boolean schemas are valid in OpenAPI 3.1/3.2, but not 3.0.
///
/// ```
/// use oas::{MediaType, Schema, SchemaValue};
/// let unrestricted = MediaType::new().with_schema(true);
/// let forbidden = SchemaValue::from(false);
/// let referenced = Schema::reference("#/components/schemas/Event")
///     .with_description("A parsed event");
/// ```
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SchemaValue {
    Boolean(bool),
    Object(Box<Schema>),
    /// A malformed structural child retained by lossless decoding.
    #[cfg(feature = "lossless-schema-keywords")]
    Invalid(Any),
}

impl<'de> Deserialize<'de> for SchemaValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SchemaVisitor;

        impl<'de> serde::de::Visitor<'de> for SchemaVisitor {
            type Value = SchemaValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a schema object or boolean")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(SchemaValue::Boolean(value))
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                // A schema object is decoded once from its map; the derived
                // untagged enum used to buffer the whole subtree at every
                // nesting level, which amplified deep schemas without bound.
                let schema =
                    Schema::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
                Ok(SchemaValue::Object(Box::new(schema)))
            }
        }

        deserializer.deserialize_any(SchemaVisitor)
    }
}

impl SchemaValue {
    pub fn as_object(&self) -> Option<&Schema> {
        match self {
            Self::Object(schema) => Some(schema),
            Self::Boolean(_) => None,
            #[cfg(feature = "lossless-schema-keywords")]
            Self::Invalid(_) => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Schema> {
        match self {
            Self::Object(schema) => Some(schema),
            Self::Boolean(_) => None,
            #[cfg(feature = "lossless-schema-keywords")]
            Self::Invalid(_) => None,
        }
    }
}

impl Default for SchemaValue {
    fn default() -> Self {
        Schema::new().into()
    }
}

impl From<Schema> for SchemaValue {
    fn from(schema: Schema) -> Self {
        Self::Object(Box::new(schema))
    }
}

impl From<bool> for SchemaValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<Reference> for SchemaValue {
    fn from(reference: Reference) -> Self {
        let siblings = Any::Object(reference.extras.into_iter().collect());
        #[cfg(feature = "lossless-schema-keywords")]
        let mut schema = match SchemaValue::from_lossless_value(siblings)
            .expect("reference siblings are an object")
        {
            SchemaValue::Object(schema) => *schema,
            _ => unreachable!("reference siblings are an object"),
        };
        // Strict schemas cannot retain malformed known fields alongside typed ones.
        // Only promote when serialization preserves the arbitrary reference siblings.
        #[cfg(not(feature = "lossless-schema-keywords"))]
        let mut schema = Schema::deserialize(&siblings)
            .ok()
            .filter(|schema| serde_json::to_value(schema).ok().as_ref() == Some(&siblings))
            .unwrap_or_else(|| {
                let Any::Object(extras) = siblings else {
                    unreachable!("reference siblings are an object")
                };
                Schema {
                    extras: extras.into_iter().collect(),
                    ..Schema::default()
                }
            });
        schema._ref = Some(reference._ref);
        schema.description = reference.description;
        if let Some(summary) = reference.summary {
            schema.extras.insert("summary".into(), summary.into());
        }
        schema.into()
    }
}

/// Keeps existing schema helper constructors usable with schema-bearing builders.
impl From<Referenceable<Schema>> for SchemaValue {
    fn from(schema: Referenceable<Schema>) -> Self {
        match schema {
            Referenceable::Data(schema) => schema.into(),
            Referenceable::Reference(reference) => reference.into(),
        }
    }
}

/// A schema object defining input and output data types.
///
/// Use [`SchemaValue`] where a schema may also be a boolean. Structural keywords
/// have typed fields; unknown vocabulary and literal payloads remain in `extras`.
/// This is a representation, not a JSON Schema validator or reference resolver.
///
/// The opt-in `lossless-schema-keywords` feature retains malformed newly modeled
/// keywords in `extras` and promotes valid fields without changing formerly opaque
/// child values. Previously typed scalar positions retain their old admission.
/// Typed assignments override matching extras during serialization and counting;
/// clearing an assignment exposes any retained fallback again. Cargo feature
/// unification applies this mode to all consumers of the crate.
#[cfg_attr(not(feature = "lossless-schema-keywords"), skip_serializing_none)]
#[derive(Debug, Clone, Default)]
#[cfg_attr(not(feature = "lossless-schema-keywords"), derive(Serialize))]
#[cfg_attr(
    not(feature = "lossless-schema-keywords"),
    serde(rename_all = "camelCase")
)]
pub struct Schema {
    #[cfg_attr(not(feature = "lossless-schema-keywords"), serde(rename = "type"))]
    pub _type: Option<SchemaType>,
    #[cfg_attr(not(feature = "lossless-schema-keywords"), serde(rename = "$ref"))]
    pub _ref: Option<String>,
    pub format: Option<String>,
    /// Legacy OpenAPI 3.0 keyword, retained without automatic conversion.
    /// For 3.1/3.2, use a type union containing `null` instead.
    pub nullable: Option<bool>,
    pub description: Option<String>,
    /// Media type of content encoded inside a string, e.g. `application/json`.
    pub content_media_type: Option<String>,
    /// Encoding of the string content, e.g. `base64`.
    pub content_encoding: Option<String>,
    /// Schema describing decoded string content. Does not change the wire type.
    pub content_schema: Option<Box<SchemaValue>>,
    /// Native schema wrapper marker, retained without interpreting its media type.
    #[cfg(feature = "lossless-schema-keywords")]
    pub schema_format: Option<String>,
    /// Wrapped schema when a `schemaFormat` marker is present.
    #[cfg(feature = "lossless-schema-keywords")]
    pub schema: Option<Box<SchemaValue>>,
    /// Documentation metadata, retaining missing and malformed fields individually.
    #[cfg(feature = "lossless-schema-keywords")]
    pub external_docs: Option<crate::lossless_component::ExternalDocumentation>,
    /// Discriminator metadata; mapping entries retain their original field states.
    #[cfg(feature = "lossless-schema-keywords")]
    pub discriminator: Option<crate::lossless_component::Discriminator>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub properties: Option<BTreeMap<String, SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub pattern_properties: Option<BTreeMap<String, SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub required: Option<Vec<String>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub min_properties: Option<SchemaCount>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub max_properties: Option<SchemaCount>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub additional_properties: Option<Box<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub unevaluated_properties: Option<Box<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub property_names: Option<Box<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub all_of: Option<Vec<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub any_of: Option<Vec<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub one_of: Option<Vec<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub not: Option<Box<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(rename = "if", default, deserialize_with = "crate::schema::present")
    )]
    pub if_schema: Option<Box<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(rename = "then", default, deserialize_with = "crate::schema::present")
    )]
    pub then_schema: Option<Box<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(rename = "else", default, deserialize_with = "crate::schema::present")
    )]
    pub else_schema: Option<Box<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub dependent_required: Option<BTreeMap<String, Vec<String>>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub dependent_schemas: Option<BTreeMap<String, SchemaValue>>,
    /// Legacy JSON Schema dependencies: property names or a schema.
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub dependencies: Option<BTreeMap<String, SchemaDependency>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(rename = "$defs", default, deserialize_with = "crate::schema::present")
    )]
    pub defs: Option<BTreeMap<String, SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub definitions: Option<BTreeMap<String, SchemaValue>>,
    /// A single schema, or a tuple of schemas in older JSON Schema dialects.
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub items: Option<SchemaItems>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub prefix_items: Option<Vec<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub additional_items: Option<Box<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub unevaluated_items: Option<Box<SchemaValue>>,
    #[cfg_attr(
        not(feature = "lossless-schema-keywords"),
        serde(default, deserialize_with = "crate::schema::present")
    )]
    pub contains: Option<Box<SchemaValue>>,
    #[cfg_attr(not(feature = "lossless-schema-keywords"), serde(flatten))]
    pub extras: BTreeMap<String, Any>,
}

/// The `items` keyword differs between tuple-capable and modern dialects.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaItems {
    Schema(Box<SchemaValue>),
    Tuple(Vec<SchemaValue>),
}

/// A legacy `dependencies` entry contains property names or a schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaDependency {
    Required(Vec<String>),
    Schema(Box<SchemaValue>),
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
