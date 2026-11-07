//! OpenAPI 3.0 schema keyword conversion. This edits one schema node only;
//! callers choose structural traversal, dialect policy and processing limits.
use crate::{Schema, SchemaType};
use serde_json::Value;

fn invalid(message: &'static str) -> serde_json::Error {
    <serde_json::Error as serde::de::Error>::custom(message)
}

impl Schema {
    /// Convert the 3.0 schema keyword forms represented by this model to 3.1.
    /// Does not recurse, resolve references, or inspect literal instance data.
    pub fn upgrade_3_0(&mut self) -> Result<(), serde_json::Error> {
        let fallback = self.extras.remove("nullable");
        let nullable = self
            .nullable
            .take()
            .unwrap_or(fallback == Some(Value::Bool(true)));
        if nullable {
            if let Some(kind) = self._type.take() {
                self._type = Some(match kind {
                    SchemaType::Single(kind) => SchemaType::Multiple(vec![kind, "null".into()]),
                    SchemaType::Multiple(mut kinds) => {
                        if !kinds.iter().any(|kind| kind == "null") {
                            kinds.push("null".into());
                        }
                        SchemaType::Multiple(kinds)
                    }
                });
            } else if self.extras.contains_key("type") {
                return Err(invalid("invalid nullable schema type"));
            }
        }
        for (exclusive, inclusive) in [
            ("exclusiveMinimum", "minimum"),
            ("exclusiveMaximum", "maximum"),
        ] {
            match self.extras.get(exclusive) {
                Some(Value::Bool(true)) => {
                    let value = self
                        .extras
                        .remove(inclusive)
                        .filter(Value::is_number)
                        .ok_or_else(|| invalid("missing numeric exclusive boundary"))?;
                    self.extras.insert(exclusive.into(), value);
                }
                Some(Value::Bool(false)) => {
                    self.extras.remove(exclusive);
                }
                _ => {}
            }
        }
        if let Some(example) = self.extras.remove("example") {
            self.extras
                .entry("examples".into())
                .or_insert_with(|| Value::Array(vec![example]));
        }
        let format = self
            .format
            .as_deref()
            .or_else(|| self.extras.get("format").and_then(Value::as_str));
        let key = match format {
            Some("binary") => {
                self.content_media_type = Some("application/octet-stream".into());
                "contentMediaType"
            }
            Some("byte" | "base64") => {
                self.content_encoding = Some("base64".into());
                "contentEncoding"
            }
            _ => return Ok(()),
        };
        self.format = None;
        self.extras.remove("format");
        self.extras.remove(key);
        Ok(())
    }
}
