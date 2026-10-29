//! Security-related types for OpenAPI 3.0 specifications.
//!
//! This module contains all types related to authentication and authorization
//! in OpenAPI specifications, including security schemes, OAuth flows, and security requirements.

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

use crate::parameter_location::ParameterIn;

/// The type of the security scheme.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum SecurityType {
    ApiKey {
        /// The name of the header, query parameter or cookie.
        name: String,
        /// The location of the API key. Valid values are `query`, `header or `cookie`.
        #[serde(rename = "in")]
        _in: ParameterIn,
    },
    Http {
        /// The name of the HTTP Authorization scheme.
        scheme: String,
        /// A hint to the client to identify how the bearer token is formatted.
        #[serde(rename = "bearerFormat")]
        bearer_format: Option<String>,
    },
    Oauth2 {
        /// An object containing configuration information for the flow types supported.
        flows: Box<OauthFlows>,
    },
    OpenIdConnect {
        /// OpenId Connect URL to discover OAuth2 configuration values.
        #[serde(rename = "openIdConnectUrl")]
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

/// Allows configuration of the supported OAuth Flows.
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
    /// The token URL to be used for this flow. This MUST be in the form of a URL.
    pub token_url: Option<String>,
    /// The URL to be used for obtaining refresh tokens. This MUST be in the form of a URL.
    pub refresh_url: Option<String>,
    /// The available scopes for the OAuth2 security scheme.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_authorization_code_preserves_wire_format() {
        let scheme = SecurityScheme {
            _type: SecurityType::Oauth2 {
                flows: Box::new(OauthFlows {
                    implicit: None,
                    password: None,
                    client_credentials: None,
                    authorization_code: Some(OauthFlow {
                        authorization_url: "https://example.com/oauth/authorize".into(),
                        token_url: Some("https://example.com/oauth/token".into()),
                        refresh_url: Some("https://example.com/oauth/refresh".into()),
                        scopes: BTreeMap::from([
                            ("read:items".into(), "Read items".into()),
                            ("write:items".into(), "Write items".into()),
                        ]),
                    }),
                }),
            },
            description: Some("OAuth authorization code".into()),
        };
        let expected = concat!(
            r#"{"type":"oauth2","flows":{"authorizationCode":{"authorizationUrl":"https://example.com/oauth/authorize","#,
            r#""tokenUrl":"https://example.com/oauth/token","refreshUrl":"https://example.com/oauth/refresh","#,
            r#""scopes":{"read:items":"Read items","write:items":"Write items"}}},"description":"OAuth authorization code"}"#,
        );
        assert_eq!(serde_json::to_string(&scheme).unwrap(), expected);
        let decoded: SecurityScheme = serde_json::from_str(expected).unwrap();
        assert_eq!(serde_json::to_string(&decoded).unwrap(), expected);
    }
}
