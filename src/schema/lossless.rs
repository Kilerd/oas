//! Compatibility decoding for keywords that were opaque before structural typing.
//! Values move into either typed fields or extras; no original subtree is cached.
use crate::{Schema, SchemaDependency, SchemaItems, SchemaValue};
use serde::{
    de::DeserializeOwned, ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
enum Origin {
    LegacyTyped,
    FormerlyOpaque,
}

// One vocabulary drives decoding, serialization, effective ownership and counting.
// Legacy fields retain their old admission only at previously typed boundaries.
macro_rules! schema_fields {
    ($apply:ident) => {
        $apply! {
            legacy {
                _type => "type": scalar,
                _ref => "$ref": scalar,
                format => "format": scalar,
                nullable => "nullable": scalar,
                description => "description": scalar,
                content_media_type => "contentMediaType": scalar,
                content_encoding => "contentEncoding": scalar,
                content_schema => "contentSchema": legacy_child
            }
            structural {
                properties => "properties": schema_map,
                pattern_properties => "patternProperties": schema_map,
                required => "required": strings,
                min_properties => "minProperties": scalar,
                max_properties => "maxProperties": scalar,
                additional_properties => "additionalProperties": child,
                unevaluated_properties => "unevaluatedProperties": child,
                property_names => "propertyNames": child,
                all_of => "allOf": schema_array,
                any_of => "anyOf": schema_array,
                one_of => "oneOf": schema_array,
                not => "not": child,
                if_schema => "if": child,
                then_schema => "then": child,
                else_schema => "else": child,
                dependent_required => "dependentRequired": string_map,
                dependent_schemas => "dependentSchemas": schema_map,
                dependencies => "dependencies": dependencies,
                defs => "$defs": schema_map,
                definitions => "definitions": schema_map,
                items => "items": items,
                prefix_items => "prefixItems": schema_array,
                additional_items => "additionalItems": child,
                unevaluated_items => "unevaluatedItems": child,
                contains => "contains": child
            }
        }
    };
}

// Origin only changes the old recursively typed contentSchema field.
macro_rules! decoder {
    (legacy_child, $origin:expr) => {
        |value| match $origin {
            Origin::LegacyTyped => legacy_child(value),
            Origin::FormerlyOpaque => child(value),
        }
    };
    (scalar, $origin:expr) => {
        scalar
    };
}

macro_rules! implement_fields {
    (legacy { $($old:ident => $old_key:literal: $old_decode:ident),* }
     structural { $($field:ident => $key:literal: $decode:ident),* }) => {
        impl Schema {
            fn decode(mut fields: Map<String, Value>, origin: Origin) -> Result<Self, serde_json::Error> {
                let mut schema = Self::default();
                $(decode_legacy(&mut schema.$old, &mut fields, $old_key, origin,
                    decoder!($old_decode, origin))?;)*
                $(decode_new(&mut schema.$field, &mut fields, $key, $decode);)*
                schema.extras = fields.into_iter().collect();
                Ok(schema)
            }

            fn owns_keyword(&self, key: &str) -> bool {
                [$(($old_key, self.$old.is_some()),)* $(($key, self.$field.is_some()),)*]
                    .into_iter().any(|(keyword, present)| keyword == key && present)
            }

            /// Count emitted entries without traversing children or serializing values.
            /// Typed fields take precedence over retained extras of the same name.
            pub fn keyword_count(&self) -> usize {
                let typed = [$(self.$old.is_some(),)* $(self.$field.is_some(),)*]
                    .into_iter().filter(|present| *present).count();
                typed + self.extras.keys().filter(|key| !self.owns_keyword(key)).count()
            }

        }

        impl Serialize for Schema {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                let mut map = serializer.serialize_map(Some(self.keyword_count()))?;
                $(serialize_optional(&mut map, $old_key, &self.$old)?;)*
                $(serialize_optional(&mut map, $key, &self.$field)?;)*
                for (key, value) in &self.extras {
                    if !self.owns_keyword(key) { map.serialize_entry(key, value)?; }
                }
                map.end()
            }
        }
    };
}
schema_fields!(implement_fields);

fn serialize_optional<M: SerializeMap, T: Serialize>(
    map: &mut M,
    key: &str,
    value: &Option<T>,
) -> Result<(), M::Error> {
    if let Some(value) = value {
        map.serialize_entry(key, value)?;
    }
    Ok(())
}

impl<'de> Deserialize<'de> for Schema {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::decode(Map::deserialize(deserializer)?, Origin::LegacyTyped)
            .map_err(serde::de::Error::custom)
    }
}

fn decode_legacy<T>(
    slot: &mut Option<T>,
    fields: &mut Map<String, Value>,
    key: &str,
    origin: Origin,
    decode: impl FnOnce(Value) -> Result<T, Value>,
) -> Result<(), serde_json::Error> {
    let Some(value) = fields.remove(key) else {
        return Ok(());
    };
    if matches!(origin, Origin::FormerlyOpaque) {
        promote(slot, fields, key, value, decode);
        return Ok(());
    }
    if value.is_null() {
        return Ok(());
    }
    *slot = Some(decode(value).map_err(|_| {
        <serde_json::Error as serde::de::Error>::custom(format!("invalid schema keyword {key}"))
    })?);
    Ok(())
}

fn decode_new<T>(
    slot: &mut Option<T>,
    fields: &mut Map<String, Value>,
    key: &str,
    decode: impl FnOnce(Value) -> Result<T, Value>,
) {
    if let Some(value) = fields.remove(key) {
        promote(slot, fields, key, value, decode);
    }
}

fn promote<T>(
    slot: &mut Option<T>,
    fields: &mut Map<String, Value>,
    key: &str,
    value: Value,
    decode: impl FnOnce(Value) -> Result<T, Value>,
) {
    match decode(value) {
        Ok(typed) => *slot = Some(typed),
        Err(value) => {
            fields.insert(key.into(), value);
        }
    }
}

// Only scalar vocabulary uses borrowed serde parsing. Structural containers below
// are moved after shallow shape checks; no typed/raw subtree pair is constructed.
fn scalar<T: DeserializeOwned>(value: Value) -> Result<T, Value> {
    T::deserialize(&value).map_err(|_| value)
}

fn is_schema(value: &Value) -> bool {
    value.is_object() || value.is_boolean()
}

impl SchemaValue {
    /// Decode an object or boolean schema from a previously untyped boundary.
    ///
    /// Unlike `Deserialize`, this retains null or malformed legacy keywords in
    /// `Schema::extras`, including nested `contentSchema` values. Valid fields
    /// become typed; this does not validate a schema or resolve references.
    /// Available with `lossless-schema-keywords`.
    pub fn from_lossless_value(value: Value) -> Result<Self, serde_json::Error> {
        match value {
            Value::Bool(value) => Ok(Self::Boolean(value)),
            Value::Object(fields) => Schema::decode(fields, Origin::FormerlyOpaque).map(Into::into),
            _ => Err(<serde_json::Error as serde::de::Error>::custom(
                "expected an object or boolean schema",
            )),
        }
    }
}

fn opaque(value: Value) -> SchemaValue {
    SchemaValue::from_lossless_value(value).expect("schema shape checked before consuming")
}

fn child(value: Value) -> Result<Box<SchemaValue>, Value> {
    if !is_schema(&value) {
        return Err(value);
    }
    Ok(Box::new(opaque(value)))
}

fn legacy_child(value: Value) -> Result<Box<SchemaValue>, Value> {
    // Legacy contentSchema is the only recursively typed position in 0.4.0.
    // Its errors are fatal at legacy boundaries; opaque decoding uses child instead.
    match value {
        Value::Bool(value) => Ok(Box::new(value.into())),
        Value::Object(fields) => Schema::decode(fields, Origin::LegacyTyped)
            .map(|schema| Box::new(schema.into()))
            .map_err(|_| Value::Null),
        value => Err(value),
    }
}

fn schema_map(value: Value) -> Result<BTreeMap<String, SchemaValue>, Value> {
    let Value::Object(fields) = value else {
        return Err(value);
    };
    if !fields.values().all(is_schema) {
        return Err(Value::Object(fields));
    }
    Ok(fields
        .into_iter()
        .map(|(key, value)| (key, opaque(value)))
        .collect())
}

fn schema_array(value: Value) -> Result<Vec<SchemaValue>, Value> {
    let Value::Array(values) = value else {
        return Err(value);
    };
    if !values.iter().all(is_schema) {
        return Err(Value::Array(values));
    }
    Ok(values.into_iter().map(opaque).collect())
}

fn is_strings(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|values| values.iter().all(Value::is_string))
}

fn strings(value: Value) -> Result<Vec<String>, Value> {
    if !is_strings(&value) {
        return Err(value);
    }
    let Value::Array(values) = value else {
        unreachable!()
    };
    Ok(values
        .into_iter()
        .map(|value| match value {
            Value::String(value) => value,
            _ => unreachable!("string shape checked before consuming"),
        })
        .collect())
}

fn string_map(value: Value) -> Result<BTreeMap<String, Vec<String>>, Value> {
    let Value::Object(fields) = value else {
        return Err(value);
    };
    if !fields.values().all(is_strings) {
        return Err(Value::Object(fields));
    }
    Ok(fields
        .into_iter()
        .map(|(key, value)| (key, strings(value).unwrap()))
        .collect())
}

fn dependencies(value: Value) -> Result<BTreeMap<String, SchemaDependency>, Value> {
    let Value::Object(fields) = value else {
        return Err(value);
    };
    if !fields
        .values()
        .all(|value| is_schema(value) || is_strings(value))
    {
        return Err(Value::Object(fields));
    }
    Ok(fields
        .into_iter()
        .map(|(key, value)| {
            let dependency = if value.is_array() {
                SchemaDependency::Required(strings(value).unwrap())
            } else {
                SchemaDependency::Schema(Box::new(opaque(value)))
            };
            (key, dependency)
        })
        .collect())
}

fn items(value: Value) -> Result<SchemaItems, Value> {
    if value.is_array() {
        return schema_array(value).map(SchemaItems::Tuple);
    }
    child(value).map(SchemaItems::Schema)
}
