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
    openapi: String,
    info: Info,
    servers: Option<Vec<Server>>,
    paths: BTreeMap<String, PathItem>,
    components: Option<Components>,
    security: Option<Vec<SecurityRequirement>>,
    tags: Option<Vec<Tag>>,
    external_docs: Option<ExternalDocumentation>,
    #[serde(flatten)]
    extras: Option<BTreeMap<String, Any>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    title: String,
    description: Option<String>,
    terms_of_service: Option<String>,
    contact: Option<Contact>,
    license: Option<License>,
    version: String,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Contact {
    name: Option<String>,
    url: Option<String>,
    email: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct License {
    name: String,
    url: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Server {
    url: String,
    description: Option<String>,
    variables: Option<BTreeMap<String, ServerVariable>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerVariable {
    #[serde(rename = "enum")]
    _enum: Option<Vec<String>>,
    default: String,
    description: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Components {
    schemas: Option<BTreeMap<String, Referentable<Schema>>>,
    responses: Option<BTreeMap<String, Referentable<Response>>>,
    parameters: Option<BTreeMap<String, Referentable<Parameter>>>,
    examples: Option<BTreeMap<String, Referentable<Example>>>,
    request_bodies: Option<BTreeMap<String, Referentable<RequestBody>>>,
    headers: Option<BTreeMap<String, Referentable<Header>>>,
    security_schemes: Option<BTreeMap<String, Referentable<SecurityScheme>>>,
    links: Option<BTreeMap<String, Referentable<Link>>>,
    callbacks: Option<BTreeMap<String, Referentable<Callback>>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct PathItem {
    #[serde(rename = "$ref")]
    _ref: Option<String>,
    summary: Option<String>,
    description: Option<String>,
    get: Option<Operation>,
    put: Option<Operation>,
    post: Option<Operation>,
    delete: Option<Operation>,
    options: Option<Operation>,
    head: Option<Operation>,
    patch: Option<Operation>,
    trace: Option<Operation>,
    servers: Option<Vec<Server>>,
    parameters: Option<Vec<Referentable<Parameter>>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    tags: Option<Vec<String>>,
    summary: Option<String>,
    description: Option<String>,
    external_docs: Option<ExternalDocumentation>,
    operation_id: Option<String>,
    parameters: Option<Vec<Referentable<Parameter>>>,
    request_body: Option<Referentable<RequestBody>>,
    responses: Responses,
    callbacks: Option<BTreeMap<String, Referentable<Callback>>>,
    deprecated: Option<bool>,
    security: Option<Vec<SecurityRequirement>>,
    servers: Option<Vec<Server>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalDocumentation {
    description: Option<String>,
    url: String,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterIn {
    Query,
    Header,
    Path,
    Cookie,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    name: String,
    #[serde(alias = "in")]
    _in: ParameterIn,
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

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestBody {
    description: Option<String>,
    required: Option<bool>,
    content: BTreeMap<String, MediaType>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct MediaType {
    schema: Option<Referentable<Schema>>,
    example: Option<Any>,
    examples: Option<BTreeMap<String, Referentable<Example>>>,
    encoding: Option<BTreeMap<String, Encoding>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Encoding {
    content_type: Option<String>,
    headers: Option<BTreeMap<String, Referentable<Header>>>,
    style: Option<String>,
    explode: Option<bool>,
    allow_reserved: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Responses {
    default: Option<Referentable<Response>>,
    #[serde(flatten)]
    data: BTreeMap<String, Referentable<Response>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    description: String,
    headers: Option<BTreeMap<String, Referentable<Header>>>,
    content: Option<BTreeMap<String, MediaType>>,
    links: Option<BTreeMap<String, Referentable<Link>>>,
}

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
    summary: Option<String>,
    description: Option<String>,
    value: Option<Any>,
    #[serde(flatten)]
    extras: Option<String>,
}

pub type Any = serde_json::Value;

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    operation_ref: Option<String>,
    operation_id: String,
    parameters: Option<BTreeMap<String, Any>>,
    request_body: Option<Any>,
    description: Option<String>,
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

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    name: String,
    description: Option<String>,
    external_docs: Option<ExternalDocumentation>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Reference {
    #[serde(rename = "$ref")]
    _ref: String,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Schema {
    // todo
    #[serde(rename = "type")]
    _type: Option<String>,
    format: Option<String>,
    nullable: Option<bool>,
    #[serde(flatten)]
    extras: BTreeMap<String, Any>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct Discriminator {
    property_name: String,
    maapping: Option<BTreeMap<String, String>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum SecurityType {
    ApiKey {
        name: String,
        #[serde(rename = "in")]
        _in: ParameterIn,
    },
    Http {
        scheme: String,
        #[serde(rename = "bearerFormat")]
        bearer_format: Option<String>,
    },
    Oauth2 {
        flows: OauthFlows,
    },
    OpenIdConnect {
        open_id_connect_url: String,
    },
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityScheme {
    #[serde(flatten)]
    _type: SecurityType,
    description: Option<String>,
}

// todo should be enum
#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthFlows {
    implicit: Option<OauthFlow>,
    password: Option<OauthFlow>,
    client_credentials: Option<OauthFlow>,
    authorization_code: Option<OauthFlow>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthFlow {
    authorization_url: String,
    token_url: Option<String>,
    refresh_url: Option<String>,
    scopes: BTreeMap<String, String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecurityRequirement {
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

            pass!{Responses, r##"
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

            pass!{Response, r###"
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

            pass!{Response, r###"
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
