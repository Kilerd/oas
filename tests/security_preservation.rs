use oas::{SecurityScheme, SecurityType};
use serde_json::json;

/// Regression: password and client credentials flows failed without authorizationUrl.
#[test]
fn all_oauth_flows_round_trip_applicable_urls() {
    for (name, flow) in [
        (
            "implicit",
            json!({"authorizationUrl": "https://example.com/authorize", "scopes": {}}),
        ),
        (
            "password",
            json!({"tokenUrl": "https://example.com/token", "scopes": {}}),
        ),
        (
            "clientCredentials",
            json!({"tokenUrl": "https://example.com/token", "scopes": {"read": "Read"}}),
        ),
        (
            "authorizationCode",
            json!({"authorizationUrl": "https://example.com/authorize", "tokenUrl": "https://example.com/token", "scopes": {"read": "Read"}}),
        ),
    ] {
        let input = json!({"type": "oauth2", "flows": {name: flow}});
        let scheme: SecurityScheme = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(scheme).unwrap(), input,
            "must preserve applicable URLs for {name}, otherwise ordinary OAuth documents cannot round trip");
    }
}

/// Regression: typed security readback dropped scheme, flow-container, and flow metadata.
#[test]
fn security_metadata_survives_round_trip_and_typed_edits() {
    for mut input in [
        json!({"type": "apiKey", "name": "X-API-Key", "in": "header"}),
        json!({"type": "http", "scheme": "bearer", "bearerFormat": "JWT"}),
        json!({"type": "openIdConnect", "openIdConnectUrl": "https://example.com/.well-known/openid-configuration"}),
        json!({"type": "oauth2", "flows": {
            "authorizationCode": {
                "authorizationUrl": "https://example.com/authorize",
                "tokenUrl": "https://example.com/token",
                "refreshUrl": "https://example.com/refresh",
                "scopes": {"read": "Read"},
                "x-flow": {"null": null, "enabled": false, "$ref": "literal"}
            },
            "x-flows": {"note": "flow container metadata"}
        }}),
    ] {
        input["description"] = json!("Original description");
        input["vendorMetadata"] = json!({"$ref": "literal", "enabled": false});
        input["x-scheme"] = json!({"order": [2, 1], "nested": {"empty": {}}});
        let mut scheme: SecurityScheme = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(&scheme).unwrap(), input,
            "must preserve all three metadata layers, otherwise typed security readback loses document content");
        if let SecurityType::Oauth2 { flows } = &mut scheme._type {
            flows.authorization_code.as_mut().unwrap().refresh_url = None;
            let mut expected = input.clone();
            expected["flows"]["authorizationCode"]
                .as_object_mut()
                .unwrap()
                .remove("refreshUrl");
            assert_eq!(serde_json::to_value(&scheme).unwrap(), expected,
                "must remove a typed refresh URL while retaining metadata, otherwise flattened fields restore stale URLs");
            let SecurityType::Oauth2 { flows } = &mut scheme._type else {
                unreachable!()
            };
            flows.authorization_code = None;
            expected["flows"]
                .as_object_mut()
                .unwrap()
                .remove("authorizationCode");
            assert_eq!(serde_json::to_value(&scheme).unwrap(), expected,
                "must remove a typed flow while retaining container metadata, otherwise flattened fields restore stale flows");
        }
        // Replacing the typed variant must not retain its old fields as extras.
        scheme._type = SecurityType::Http {
            scheme: "basic".into(),
            bearer_format: None,
        };
        scheme.description = Some("Edited description".into());
        let wire = serde_json::to_string(&scheme).unwrap();
        assert_eq!(wire, concat!(
            r#"{"type":"http","scheme":"basic","description":"Edited description","vendorMetadata":{"$ref":"literal","enabled":false},"#,
            r#""x-scheme":{"nested":{"empty":{}},"order":[2,1]}}"#),
            "must emit each current field once in stable compact JSON, otherwise flattened enum fields duplicate stale data");
        assert_eq!(
            serde_json::to_value(scheme).unwrap(),
            json!({
                "type": "http", "scheme": "basic", "description": "Edited description",
                "x-scheme": input["x-scheme"], "vendorMetadata": input["vendorMetadata"]
            }),
            "must retain metadata and only the replacement variant fields, otherwise changing security type restores old fields"
        );
    }
}
