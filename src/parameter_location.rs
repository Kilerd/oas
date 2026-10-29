use serde::{Deserialize, Serialize};

/// The location of the parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterIn {
    Query,
    Header,
    Path,
    Cookie,
}

#[cfg(test)]
mod tests {
    use super::ParameterIn;

    /// Moving the shared enum must preserve both published import paths and wire values.
    #[test]
    fn parameter_locations_keep_public_paths_and_wire_values() {
        for (location, expected) in [
            (ParameterIn::Query, "\"query\""),
            (ParameterIn::Header, "\"header\""),
            (ParameterIn::Path, "\"path\""),
            (ParameterIn::Cookie, "\"cookie\""),
        ] {
            let through_types: crate::types::ParameterIn = location;
            let through_root: crate::ParameterIn = through_types;
            assert_eq!(
                serde_json::to_string(&through_root).unwrap(),
                expected,
                "parameter locations must keep their wire spelling, otherwise documents change after an internal move"
            );
            let decoded: crate::types::ParameterIn = serde_json::from_str(expected).unwrap();
            assert_eq!(
                std::mem::discriminant(&decoded),
                std::mem::discriminant(&through_root),
                "decoding must select the same location, otherwise parameter and security models disagree"
            );
        }
    }
}
