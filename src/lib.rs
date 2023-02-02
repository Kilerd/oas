use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Referentable<T> {
    Data(T),
    Reference(Reference),
}

#[skip_serializing_none]
/// the root document object of openAPI v3.0
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAPIV3 {
    /// This string MUST be the semantic version number of the OpenAPI Specification version that the OpenAPI document uses. The `openapi` field SHOULD be used by tooling specifications and clients to interpret the OpenAPI document. This is not related to the API info.version string.
    openapi: String,
    /// Provides metadata about the API
    info: Info,
    /// An array of Server Objects, which provide connectivity information to a target server. If the `servers` property is not provided, or is an empty array, the default value would be a `Server` Object with a url value of `/`.
    servers: Option<Vec<Server>>,
    /// The available paths and operations for the API.
    paths: BTreeMap<String, PathItem>,
    /// An element to hold various schemas for the specification.
    components: Option<Components>,
    /// A declaration of which security mechanisms can be used across the API. The list of values includes alternative security requirement objects that can be used. Only one of the security requirement objects need to be satisfied to authorize a request. Individual operations can override this definition. To make security optional, an empty security requirement (`{}`) can be included in the array.
    security: Option<Vec<SecurityRequirement>>,
    /// A list of tags used by the specification with additional metadata. The order of the tags can be used to reflect on their order by the parsing tools. Not all tags that are used by the Operation Object must be declared. The tags that are not declared MAY be organized randomly or based on the tools' logic. Each tag name in the list MUST be unique.
    tags: Option<Vec<Tag>>,
    /// Additional external documentation.
    external_docs: Option<ExternalDocumentation>,
    #[serde(flatten)]
    extras: Option<BTreeMap<String, Any>>,
}

/// The object provides metadata about the API. The metadata MAY be used by the clients if needed, and MAY be presented in editing or documentation generation tools for convenience.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    /// The title of the API.
    title: String,
    /// A short description of the API. CommonMark syntax MAY be used for rich text representation.
    description: Option<String>,
    /// A URL to the Terms of Service for the API. MUST be in the format of a URL.
    terms_of_service: Option<String>,
    /// The contact information for the exposed API.
    contact: Option<Contact>,
    /// The license information for the exposed API.
    license: Option<License>,
    /// The version of the OpenAPI document (which is distinct from the OpenAPI Specification version or the API implementation version).
    version: String,
}

/// Contact information for the exposed API.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Contact {
    /// The identifying name of the contact person/organization.
    name: Option<String>,
    /// The URL pointing to the contact information. MUST be in the format of a URL.
    url: Option<String>,
    /// The email address of the contact person/organization. MUST be in the format of an email address.
    email: Option<String>,
}

/// License information for the exposed API.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct License {
    /// The license name used for the API.
    name: String,
    /// A URL to the license used for the API. MUST be in the format of a URL.
    url: Option<String>,
}

/// An object representing a Server.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Server {
    /// A URL to the target host. This URL supports Server Variables and MAY be relative, to indicate that the host location is relative to the location where the OpenAPI document is being served. Variable substitutions will be made when a variable is named in {brackets}.
    url: String,
    /// An optional string describing the host designated by the URL. CommonMark syntax MAY be used for rich text representation.
    description: Option<String>,
    /// A map between a variable name and its value. The value is used for substitution in the server's URL template.
    variables: Option<BTreeMap<String, ServerVariable>>,
}

/// An object representing a Server Variable for server URL template substitution.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerVariable {
    /// An enumeration of string values to be used if the substitution options are from a limited set. The array SHOULD NOT be empty.
    #[serde(rename = "enum")]
    _enum: Option<Vec<String>>,
    /// The default value to use for substitution, which SHALL be sent if an alternate value is not supplied. Note this behavior is different than the Schema Object's treatment of default values, because in those cases parameter values are optional. If the `enum` is defined, the value SHOULD exist in the enum's values.
    default: String,
    /// An optional description for the server variable. CommonMark syntax MAY be used for rich text representation.
    description: Option<String>,
}

/// Holds a set of reusable objects for different aspects of the OAS. All objects defined within the components object will have no effect on the API unless they are explicitly referenced from properties outside the components object.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Components {
    /// An object to hold reusable Schema Objects.
    schemas: Option<BTreeMap<String, Referentable<Schema>>>,
    /// An object to hold reusable Response Objects.
    responses: Option<BTreeMap<String, Referentable<Response>>>,
    /// An object to hold reusable Parameter Objects.
    parameters: Option<BTreeMap<String, Referentable<Parameter>>>,
    /// An object to hold reusable Example Objects.
    examples: Option<BTreeMap<String, Referentable<Example>>>,
    /// An object to hold reusable Request Body Objects.
    request_bodies: Option<BTreeMap<String, Referentable<RequestBody>>>,
    /// An object to hold reusable Header Objects.
    headers: Option<BTreeMap<String, Referentable<Header>>>,
    /// An object to hold reusable Security Scheme Objects.
    security_schemes: Option<BTreeMap<String, Referentable<SecurityScheme>>>,
    /// An object to hold reusable Link Objects.
    links: Option<BTreeMap<String, Referentable<Link>>>,
    /// An object to hold reusable Callback Objects.
    callbacks: Option<BTreeMap<String, Referentable<Callback>>>,
}

/// Describes the operations available on a single path. A Path Item MAY be empty, due to ACL constraints. The path itself is still exposed to the documentation viewer but they will not know which operations and parameters are available.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct PathItem {
    /// Allows for an external definition of this path item. The referenced structure MUST be in the format of a Path Item Object. In case a Path Item Object field appears both in the defined object and the referenced object, the behavior is undefined.
    #[serde(rename = "$ref")]
    _ref: Option<String>,
    /// An optional, string summary, intended to apply to all operations in this path.
    summary: Option<String>,
    /// An optional, string description, intended to apply to all operations in this path. CommonMark syntax MAY be used for rich text representation.
    description: Option<String>,
    /// A definition of a GET operation on this path.
    get: Option<Operation>,
    /// A definition of a PUT operation on this path.
    put: Option<Operation>,
    /// A definition of a POST operation on this path.
    post: Option<Operation>,
    /// A definition of a DELETE operation on this path.
    delete: Option<Operation>,
    /// A definition of a OPTIONS operation on this path.
    options: Option<Operation>,
    /// A definition of a HEAD operation on this path.
    head: Option<Operation>,
    /// A definition of a PATCH operation on this path.
    patch: Option<Operation>,
    /// A definition of a TRACE operation on this path.
    trace: Option<Operation>,
    /// An alternative `server` array to service all operations in this path.
    servers: Option<Vec<Server>>,
    /// A list of parameters that are applicable for all the operations described under this path. These parameters can be overridden at the operation level, but cannot be removed there. The list MUST NOT include duplicated parameters. A unique parameter is defined by a combination of a name and location. The list can use the Reference Object to link to parameters that are defined at the OpenAPI Object's components/parameters.
    parameters: Option<Vec<Referentable<Parameter>>>,
}

/// Describes a single API operation on a path.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    /// A list of tags for API documentation control. Tags can be used for logical grouping of operations by resources or any other qualifier.
    tags: Option<Vec<String>>,
    ///A short summary of what the operation does.
    summary: Option<String>,
    /// A verbose explanation of the operation behavior. CommonMark syntax MAY be used for rich text representation.
    description: Option<String>,
    /// Additional external documentation for this operation.
    external_docs: Option<ExternalDocumentation>,
    /// Unique string used to identify the operation. The id MUST be unique among all operations described in the API. The operationId value is case-sensitive. Tools and libraries MAY use the operationId to uniquely identify an operation, therefore, it is RECOMMENDED to follow common programming naming conventions.
    operation_id: Option<String>,
    /// A list of parameters that are applicable for this operation. If a parameter is already defined at the Path Item, the new definition will override it but can never remove it. The list MUST NOT include duplicated parameters. A unique parameter is defined by a combination of a name and location. The list can use the Reference Object to link to parameters that are defined at the OpenAPI Object's components/parameters.
    parameters: Option<Vec<Referentable<Parameter>>>,
    /// The request body applicable for this operation. The requestBody is only supported in HTTP methods where the HTTP 1.1 specification RFC7231 has explicitly defined semantics for request bodies. In other cases where the HTTP spec is vague, requestBody SHALL be ignored by consumers.
    request_body: Option<Referentable<RequestBody>>,
    /// The list of possible responses as they are returned from executing this operation.
    responses: Responses,
    /// A map of possible out-of band callbacks related to the parent operation. The key is a unique identifier for the Callback Object. Each value in the map is a Callback Object that describes a request that may be initiated by the API provider and the expected responses.
    callbacks: Option<BTreeMap<String, Referentable<Callback>>>,
    /// Declares this operation to be deprecated. Consumers SHOULD refrain from usage of the declared operation. Default value is `false`.
    deprecated: Option<bool>,
    /// A declaration of which security mechanisms can be used for this operation. The list of values includes alternative security requirement objects that can be used. Only one of the security requirement objects need to be satisfied to authorize a request. To make security optional, an empty security requirement (`{}`) can be included in the array. This definition overrides any declared top-level security. To remove a top-level security declaration, an empty array can be used.
    security: Option<Vec<SecurityRequirement>>,
    /// An alternative server array to service this operation. If an alternative server object is specified at the Path Item Object or Root level, it will be overridden by this value.
    servers: Option<Vec<Server>>,
}

/// Allows referencing an external resource for extended documentation.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalDocumentation {
    /// A short description of the target documentation. CommonMark syntax MAY be used for rich text representation.
    description: Option<String>,
    /// The URL for the target documentation. Value MUST be in the format of a URL.
    url: String,
}

/// The location of the parameter
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    /// The name of the parameter
    name: String,
    /// The location of the parameter
    #[serde(alias = "in")]
    _in: ParameterIn,
    /// A brief description of the parameter. This could contain examples of use. CommonMark syntax MAY be used for rich text representation.
    description: Option<String>,
    /// Determines whether this parameter is mandatory
    required: Option<bool>,
    /// Specifies that a parameter is deprecated and SHOULD be transitioned out of usage. Default value is `false`.
    deprecated: Option<bool>,
    /// Sets the ability to pass empty-valued parameters
    allow_empty_value: Option<bool>,
    /// Describes how the parameter value will be serialized depending on the type of the parameter value
    style: Option<String>,
    explode: Option<bool>,
    allow_reserved: Option<bool>,
    /// The schema defining the type used for the parameter.
    schema: Option<Referentable<Schema>>,
    /// Example of the parameter's potential value.
    example: Option<Any>,
    /// Examples of the parameter's potential value.
    examples: Option<BTreeMap<String, Referentable<Example>>>,
    /// A map containing the representations for the parameter. The key is the media type and the value describes it.
    content: Option<BTreeMap<String, MediaType>>,
}

/// Describes a single request body.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestBody {
    /// A brief description of the request body.
    description: Option<String>,
    /// Determines if the request body is required in the request. Defaults to `false`.
    required: Option<bool>,
    /// The content of the request body.
    content: BTreeMap<String, MediaType>,
}

/// Each Media Type Object provides schema and examples for the media type identified by its key.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct MediaType {
    /// The schema defining the content of the request, response, or parameter.
    schema: Option<Referentable<Schema>>,
    /// Example of the media type.
    example: Option<Any>,
    /// Examples of the media type.
    examples: Option<BTreeMap<String, Referentable<Example>>>,
    /// A map between a property name and its encoding information.
    encoding: Option<BTreeMap<String, Encoding>>,
}

/// A single encoding definition applied to a single schema property.

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Encoding {
    /// The Content-Type for encoding a specific property.
    content_type: Option<String>,
    /// map allowing additional information to be provided as headers, for example `Content-Disposition`. `Content-Type` is described separately and SHALL be ignored in this section. This property SHALL be ignored if the request body media type is not a `multipart`.
    headers: Option<BTreeMap<String, Referentable<Header>>>,
    /// Describes how a specific property value will be serialized depending on its type.
    style: Option<String>,
    explode: Option<bool>,
    allow_reserved: Option<bool>,
}

/// A container for the expected responses of an operation. The container maps a HTTP response code to the expected response.
/// The documentation is not necessarily expected to cover all possible HTTP response codes because they may not be known in advance. However, documentation is expected to cover a successful operation response and any known errors.
/// The default MAY be used as a default response object for all HTTP codes that are not covered individually by the specification.
/// The Responses Object MUST contain at least one response code, and it SHOULD be the response for a successful operation call.

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Responses {
    /// The documentation of responses other than the ones declared for specific HTTP response codes. Use this field to cover undeclared responses. A Reference Object can link to a response that the OpenAPI Object's components/responses section defines.
    default: Option<Referentable<Response>>,
    #[serde(flatten)]
    data: BTreeMap<String, Referentable<Response>>,
}

/// Describes a single response from an API Operation, including design-time, static `links` to operations based on the response.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    /// A short description of the response.
    description: String,
    /// Maps a header name to its definition.
    headers: Option<BTreeMap<String, Referentable<Header>>>,
    /// A map containing descriptions of potential response payloads.
    content: Option<BTreeMap<String, MediaType>>,
    /// A map of operations links that can be followed from the response.
    links: Option<BTreeMap<String, Referentable<Link>>>,
}

/// A map of possible out-of band callbacks related to the parent operation. Each value in the map is a Path Item Object that describes a set of requests that may be initiated by the API provider and the expected responses. The key value used to identify the path item object is an expression, evaluated at runtime, that identifies a URL to use for the callback operation.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Callback {
    #[serde(flatten)]
    data: BTreeMap<String, PathItem>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    /// Short description for the example.
    summary: Option<String>,
    /// Long description for the example.
    description: Option<String>,
    /// Embedded literal example.
    value: Option<Any>,
    external_value: Option<String>,
}

pub type Any = serde_json::Value;

/// represents a possible design-time link for a response.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// A relative or absolute URI reference to an OAS operation.
    operation_ref: Option<String>,
    /// The name of an existing, resolvable OAS operation
    operation_id: String,
    /// A map representing parameters to pass to an operation as specified with `operation_id` or identified via `operation_ef`.
    parameters: Option<BTreeMap<String, Any>>,
    /// A literal value or `{expression}` to use as a request body when calling the target operation.
    request_body: Option<Any>,
    /// A description of the link.
    description: Option<String>,
    /// A server object to be used by the target operation.
    server: Option<Server>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Header {
    description: Option<String>,
    required: Option<bool>,
    deprecated: Option<bool>,
    allow_empty_value: Option<bool>,
    style: Option<String>,
    explode: Option<bool>,
    allow_reserved: Option<bool>,
    schema: Option<Referentable<Schema>>,
    example: Option<Any>,
    examples: Option<BTreeMap<String, Referentable<Example>>>,
    content: Option<BTreeMap<String, MediaType>>,
}

/// Adds metadata to a single tag that is used by the `Operation` Object. It is not mandatory to have a Tag Object per tag defined in the Operation Object instances.

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    /// The name of the tag.
    name: String,
    /// A short description for the tag.
    description: Option<String>,
    /// Additional external documentation for this tag.
    external_docs: Option<ExternalDocumentation>,
}

/// A simple object to allow referencing other components in the specification, internally and externally.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Reference {
    /// The reference string.
    #[serde(rename = "$ref")]
    _ref: String,
}

/// The Schema Object allows the definition of input and output data types. These types can be objects, but also primitives and arrays.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Schema {
    #[serde(rename = "type")]
    _type: Option<String>,
    format: Option<String>,
    nullable: Option<bool>,
    #[serde(flatten)]
    extras: BTreeMap<String, Any>,
}

/// When request bodies or response payloads may be one of a number of different schemas, a `discriminator` object can be used to aid in serialization, deserialization, and validation. The discriminator is a specific object in a schema which is used to inform the consumer of the specification of an alternative schema based on the value associated with it.

/// When using the discriminator, inline schemas will not be considered.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discriminator {
    /// The name of the property in the payload that will hold the discriminator value.
    property_name: String,
    /// An object to hold mappings between payload values and schema names or references.
    maapping: Option<BTreeMap<String, String>>,
}

/// The type of the security scheme.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityScheme {
    #[serde(flatten)]
    _type: SecurityType,
    /// A short description for security scheme.
    description: Option<String>,
}

// todo should be enum
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthFlows {
    /// Configuration for the OAuth Implicit flow
    implicit: Option<OauthFlow>,
    /// Configuration for the OAuth Resource Owner Password flow
    password: Option<OauthFlow>,
    /// Configuration for the OAuth Client Credentials flow.
    client_credentials: Option<OauthFlow>,
    /// Configuration for the OAuth Authorization Code flow.
    authorization_code: Option<OauthFlow>,
}

/// Configuration details for a supported OAuth Flow
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthFlow {
    /// The authorization URL to be used for this flow. This MUST be in the form of a URL.
    authorization_url: String,
    /// he token URL to be used for this flow. This MUST be in the form of a URL.
    token_url: Option<String>,
    /// The URL to be used for obtaining refresh tokens. This MUST be in the form of a URL.
    refresh_url: Option<String>,
    /// The available scopes for the OAuth2 security scheme. A map between the scope name and a short description for it. The map MAY be empty.
    scopes: BTreeMap<String, String>,
}

/// Lists the required security schemes to execute this operation.
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecurityRequirement {
    #[serde(flatten)]
    data: BTreeMap<String, Vec<String>>,
}

#[cfg(test)]
mod test {
    mod pass {
        use crate::{
            Header, OpenAPIV3, Operation, Parameter, RequestBody, Response, Responses, Schema,
            SecurityScheme,
        };
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
            pass! {
              OpenAPIV3,
              include_str!("../openapi3-examples/3.0/pass/swagger2openapi/openapi.json")
            }
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/api-with-examples.json")}
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/callback-example.json")}
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/link-example.json")}
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/petstore-expanded.json")}
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/petstore.json")}
            pass! { OpenAPIV3, include_str!("../examples/v3.0/json/uspto.json")}
        }

        #[test]
        fn should_pass_parameter() {
            pass! { Parameter,r#"
              {
                "name": "status",
                "in": "query",
                "description": "Status values that need to be considered for filter",
                "required": true,
                "explode": true,
                "schema": {
                  "type": "array",
                  "items": {
                    "type": "string",
                    "enum": [
                      "available",
                      "pending",
                      "sold"
                    ],
                    "default": "available"
                  }
                }
            }
            "#
            }
        }

        #[test]
        fn should_pass_operation() {
            pass! { Operation ,
                  r#"{
                "tags": [
                  "user"
                ],
                "summary": "Logs out current logged in user session",
                "description": "",
                "operationId": "logoutUser",
                "parameters": [],
                "responses": {
                  "default": {
                    "description": "successful operation"
                  }
                }
              }
            "#
            }
        }

        #[test]
        fn should_pass_operation2() {
            pass! { Operation, r####"{
                "tags": [
                  "pet"
                ],
                "summary": "Add a new pet to the store",
                "description": "",
                "operationId": "addPet",
                "parameters": [],
                "responses": {
                  "405": {
                    "description": "Invalid input"
                  }
                },
                "security": [
                  {
                    "petstore_auth": [
                      "write:pets",
                      "read:pets"
                    ]
                  }
                ],
                "requestBody": {
                  "$ref": "#/components/requestBodies/Pet"
                }
              }
            "####
            }
        }
        #[test]
        fn should_pass_schema() {
            pass! { Schema, r####"{
                "type": "array",
                "items": {
                  "type": "string",
                  "enum": [
                    "available",
                    "pending",
                    "sold"
                  ],
                  "default": "available"
                }
              }
            "####}
        }

        #[test]
        fn should_pass_request_body() {
            pass! {RequestBody, r####"{
                "content": {
                  "application/x-www-form-urlencoded": {
                    "schema": {
                      "type": "object",
                      "properties": {
                        "name": {
                          "type": "string"
                        },
                        "status": {
                          "type": "string"
                        }
                      }
                    }
                  }
                },
                "description": "Updated name of the pet"
              }
            "#### }
        }

        #[test]
        fn should_pass_responses() {
            pass! { Responses,  r####"{
                "200": {
                  "description": "successful operation",
                  "headers": {
                    "X-Rate-Limit": {
                      "description": "calls per hour allowed by the user",
                      "schema": {
                        "type": "integer",
                        "format": "int32"
                      }
                    },
                    "X-Expires-After": {
                      "description": "date in UTC when token expires",
                      "schema": {
                        "type": "string",
                        "format": "date-time"
                      }
                    }
                  },
                  "content": {
                    "application/xml": {
                      "schema": {
                        "type": "string"
                      }
                    },
                    "application/json": {
                      "schema": {
                        "type": "string"
                      }
                    }
                  }
                },
                "400": {
                  "description": "Invalid username/password supplied"
                }
              }
            "#### }

            pass! {Responses, r##"
            {
              "200": {
                "description": "200 response",
                "content": {
                  "application/json": {
                    "examples": {
                      "foo": {
                        "value": {
                          "versions": [
                            {
                              "status": "CURRENT",
                              "updated": "2011-01-21T11:33:21Z",
                              "id": "v2.0",
                              "links": [
                                {
                                  "href": "http://127.0.0.1:8774/v2/",
                                  "rel": "self"
                                }
                              ]
                            },
                            {
                              "status": "EXPERIMENTAL",
                              "updated": "2013-07-23T11:33:21Z",
                              "id": "v3.0",
                              "links": [
                                {
                                  "href": "http://127.0.0.1:8774/v3/",
                                  "rel": "self"
                                }
                              ]
                            }
                          ]
                        }
                      }
                    }
                  }
                }
              },
              "300": {
                "description": "300 response",
                "content": {
                  "application/json": {
                    "examples": {
                      "foo": {
                        "value": "{\n \"versions\": [\n       {\n         \"status\": \"CURRENT\",\n         \"updated\": \"2011-01-21T11:33:21Z\",\n         \"id\": \"v2.0\",\n         \"links\": [\n             {\n                 \"href\": \"http://127.0.0.1:8774/v2/\",\n                 \"rel\": \"self\"\n             }\n         ]\n     },\n     {\n         \"status\": \"EXPERIMENTAL\",\n         \"updated\": \"2013-07-23T11:33:21Z\",\n         \"id\": \"v3.0\",\n         \"links\": [\n             {\n                 \"href\": \"http://127.0.0.1:8774/v3/\",\n                 \"rel\": \"self\"\n             }\n         ]\n     }\n ]\n}\n"
                      }
                    }
                  }
                }
              }
            }
            "##}
        }

        #[test]
        fn should_pass_response_1() {
            pass! { Response,  r####"{
                "description": "successful operation",
                "headers": {
                  "X-Rate-Limit": {
                    "description": "calls per hour allowed by the user",
                    "schema": {
                      "type": "integer",
                      "format": "int32"
                    }
                  },
                  "X-Expires-After": {
                    "description": "date in UTC when token expires",
                    "schema": {
                      "type": "string",
                      "format": "date-time"
                    }
                  }
                },
                "content": {
                  "application/xml": {
                    "schema": {
                      "type": "string"
                    }
                  },
                  "application/json": {
                    "schema": {
                      "type": "string"
                    }
                  }
                }
              }
            "####}
        }
        #[test]
        fn should_pass_response_2() {
            pass! { Response, r####"{
                "description": "Invalid username/password supplied"
              }
            "####}

            pass! {Response, r###"
            {
              "description": "200 response",
              "content": {
                "application/json": {
                  "examples": {
                    "foo": {
                      "value": {
                        "versions": [
                          {
                            "status": "CURRENT",
                            "updated": "2011-01-21T11:33:21Z",
                            "id": "v2.0",
                            "links": [
                              {
                                "href": "http://127.0.0.1:8774/v2/",
                                "rel": "self"
                              }
                            ]
                          },
                          {
                            "status": "EXPERIMENTAL",
                            "updated": "2013-07-23T11:33:21Z",
                            "id": "v3.0",
                            "links": [
                              {
                                "href": "http://127.0.0.1:8774/v3/",
                                "rel": "self"
                              }
                            ]
                          }
                        ]
                      }
                    }
                  }
                }
              }
            }
            "###}

            pass! {Response, r###"
            {
              "description": "subscription successfully created",
              "content": {
                "application/json": {
                  "schema": {
                    "description": "subscription information",
                    "required": [
                      "subscriptionId"
                    ],
                    "properties": {
                      "subscriptionId": {
                        "description": "this unique identifier allows management of the subscription",
                        "type": "string",
                        "example": "2531329f-fb09-4ef7-887e-84e648214436"
                      }
                    }
                  }
                }
              }
            }
            "###}
        }

        #[test]
        fn should_pass_header() {
            pass! {Header, r##"
            {
                "description": "calls per hour allowed by the user",
                "schema": {
                  "type": "integer",
                  "format": "int32"
                }
            }
            "##}
        }

        #[test]
        fn should_pass_security() {
            pass! {SecurityScheme, r##"
              {
                "type": "oauth2",
                "flows": {
                  "implicit": {
                    "authorizationUrl": "http://petstore.swagger.io/oauth/dialog",
                    "scopes": {
                      "write:pets": "modify pets in your account",
                      "read:pets": "read your pets"
                    }
                  }
                }
              }
            "##}
            pass! {SecurityScheme, r##"
            {
                "type": "http",
                "scheme": "basic"
              }
            "##}

            pass! {SecurityScheme, r##"
            {
                "type": "apiKey",
                "name": "api_key",
                "in": "header"
              }
            "##}

            pass! {SecurityScheme, r##"
            {
                "type": "http",
                "scheme": "bearer",
                "bearerFormat": "JWT"
              }
            "##}
        }
    }
}
