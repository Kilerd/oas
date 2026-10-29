//! Security-related types for OpenAPI 3.0 specifications.
//!
//! This module contains all types related to authentication and authorization
//! in OpenAPI specifications, including security schemes, OAuth flows, and security requirements.

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

use crate::types::ParameterIn;

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
        flows: OauthFlows,
    },
    OpenIdConnect {
        /// OpenId Connect URL to discover OAuth2 configuration values.
        #[serde(rename = "openIdConnectUrl")]
        open_id_connect_url: String,
    },
}

/// Defines a security scheme that can be used by the operations.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct SecurityScheme {
    #[serde(flatten)]
    pub _type: SecurityType,
    /// A short description for security scheme.
    pub description: Option<String>,
    /// Extension fields and other unmodeled properties.
    #[serde(flatten)]
    pub extras: BTreeMap<String, serde_json::Value>,
}

impl<'de> Deserialize<'de> for SecurityScheme {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Fields {
            #[serde(flatten)]
            _type: SecurityType,
            description: Option<String>,
            #[serde(flatten)]
            extras: BTreeMap<String, serde_json::Value>,
        }

        let Fields {
            _type,
            description,
            mut extras,
        } = Fields::deserialize(deserializer)?;
        // A flattened internally tagged enum leaves its fields available to the
        // flattened map. Keep one owner so typed edits cannot emit stale values.
        let modeled_fields: &[&str] = match &_type {
            SecurityType::ApiKey { .. } => &["type", "name", "in"],
            SecurityType::Http { .. } => &["type", "scheme", "bearerFormat"],
            SecurityType::Oauth2 { .. } => &["type", "flows"],
            SecurityType::OpenIdConnect { .. } => &["type", "openIdConnectUrl"],
        };
        for field in modeled_fields {
            extras.remove(*field);
        }
        Ok(Self {
            _type,
            description,
            extras,
        })
    }
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
    /// Extension fields and other unmodeled properties.
    #[serde(flatten)]
    pub extras: BTreeMap<String, serde_json::Value>,
}

/// Configuration details for a supported OAuth flow.
///
/// URL requirements depend on the containing field in [`OauthFlows`]. This model
/// preserves documents without validating flow-specific required fields.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthFlow {
    /// Authorization URL; required for implicit and authorization code flows.
    pub authorization_url: Option<String>,
    /// Token URL; required for password, client credentials, and authorization code flows.
    pub token_url: Option<String>,
    /// The URL to be used for obtaining refresh tokens. This MUST be in the form of a URL.
    pub refresh_url: Option<String>,
    /// The available scopes for the OAuth2 security scheme.
    pub scopes: BTreeMap<String, String>,
    /// Extension fields and other unmodeled properties.
    #[serde(flatten)]
    pub extras: BTreeMap<String, serde_json::Value>,
}

/// Lists the required security schemes to execute this operation.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecurityRequirement {
    #[serde(flatten)]
    pub data: BTreeMap<String, Vec<String>>,
}
