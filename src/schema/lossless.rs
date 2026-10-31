//! Compatibility decoding for keywords that were opaque before structural typing.
//! Values move into either typed fields or extras; no original subtree is cached.
use crate::{Schema, SchemaDependency, SchemaItems, SchemaValue};
use serde::{
    de::DeserializeOwned, ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// The structural shape declared for a schema keyword, independent of its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaKeywordKind {
    Single,
    Map,
    Array,
    Items,
    Dependencies,
    ExternalDocs,
}

/// A structural field borrowed without interpreting its contents or references.
/// Containers remain intact so callers can preserve their own visitation policy.
pub enum SchemaFieldMut<'a> {
    Single(&'a mut SchemaValue),
    Map(&'a mut BTreeMap<String, SchemaValue>),
    Array(&'a mut Vec<SchemaValue>),
    Items(&'a mut SchemaItems),
    Dependencies(&'a mut BTreeMap<String, SchemaDependency>),
    ExternalDocs(&'a mut crate::lossless_component::ExternalDocumentation),
    Invalid(&'a mut Value),
}

/// A structural field borrowed without cloning or interpreting its children.
pub enum SchemaField<'a> {
    Single(&'a SchemaValue),
    Map(&'a BTreeMap<String, SchemaValue>),
    Array(&'a Vec<SchemaValue>),
    Items(&'a SchemaItems),
    Dependencies(&'a BTreeMap<String, SchemaDependency>),
    ExternalDocs(&'a crate::lossless_component::ExternalDocumentation),
    Invalid(&'a Value),
}

macro_rules! structural_field {
    ($kind:ident, $field:expr) => {
        structural_field!($kind, $field, SchemaFieldMut, as_mut, as_deref_mut)
    };
    (legacy_child, $($args:tt)*) => { structural_field!(child, $($args)*) };
    (child, $field:expr, $view:ident, $borrow:ident, $deref:ident) => { $field.$deref().map($view::Single) };
    (schema_map, $field:expr, $view:ident, $borrow:ident, $deref:ident) => { $field.$borrow().map($view::Map) };
    (schema_array, $field:expr, $view:ident, $borrow:ident, $deref:ident) => { $field.$borrow().map($view::Array) };
    (items, $field:expr, $view:ident, $borrow:ident, $deref:ident) => { $field.$borrow().map($view::Items) };
    (dependencies, $field:expr, $view:ident, $borrow:ident, $deref:ident) => { $field.$borrow().map($view::Dependencies) };
    (external_docs, $field:expr, $view:ident, $borrow:ident, $deref:ident) => { $field.$borrow().map($view::ExternalDocs) };
    ($other:ident, $field:expr, $view:ident, $borrow:ident, $deref:ident) => { None::<$view<'_>> };
}

/// An owned structural field moved out of a schema without cloning its subtree.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum OwnedSchemaField {
    Single(Box<SchemaValue>),
    Map(BTreeMap<String, SchemaValue>),
    Array(Vec<SchemaValue>),
    Items(SchemaItems),
    Dependencies(BTreeMap<String, SchemaDependency>),
    ExternalDocs(Box<crate::lossless_component::ExternalDocumentation>),
    Invalid(Value),
}

macro_rules! take_structural_field {
    (legacy_child, $field:expr) => {
        take_structural_field!(child, $field)
    };
    (child, $field:expr) => {
        $field.take().map(OwnedSchemaField::Single)
    };
    (schema_map, $field:expr) => {
        $field.take().map(OwnedSchemaField::Map)
    };
    (schema_array, $field:expr) => {
        $field.take().map(OwnedSchemaField::Array)
    };
    (items, $field:expr) => {
        $field.take().map(OwnedSchemaField::Items)
    };
    (dependencies, $field:expr) => {
        $field.take().map(OwnedSchemaField::Dependencies)
    };
    (external_docs, $field:expr) => {
        $field
            .take()
            .map(Box::new)
            .map(OwnedSchemaField::ExternalDocs)
    };
    ($other:ident, $field:expr) => {
        None::<OwnedSchemaField>
    };
}

macro_rules! put_structural_field {
    (legacy_child, $($args:tt)*) => { put_structural_field!(child, $($args)*) };
    (child, $($args:tt)*) => { put_structural_field!(@typed Single, value => value, $($args)*) };
    (schema_map, $($args:tt)*) => { put_structural_field!(@typed Map, value => value, $($args)*) };
    (schema_array, $($args:tt)*) => { put_structural_field!(@typed Array, value => value, $($args)*) };
    (items, $($args:tt)*) => { put_structural_field!(@typed Items, value => value, $($args)*) };
    (dependencies, $($args:tt)*) => { put_structural_field!(@typed Dependencies, value => value, $($args)*) };
    (external_docs, $($args:tt)*) => { put_structural_field!(@typed ExternalDocs, value => *value, $($args)*) };
    (@typed $variant:ident, $value:ident => $convert:expr,
        $slot:expr, $extras:expr, $key:expr, $field:expr) => {
        match $field {
            OwnedSchemaField::$variant($value) => {
                $slot = Some($convert);
                $extras.remove($key);
                Ok(())
            }
            OwnedSchemaField::Invalid(value) => {
                $slot = None;
                $extras.insert($key.into(), value);
                Ok(())
            }
            field => Err(field),
        }
    };
    ($other:ident, $slot:expr, $extras:expr, $key:expr, $field:expr) => { Err($field) };
}

macro_rules! structural_kind {
    (legacy_child) => {
        structural_kind!(child)
    };
    (child) => {
        Some(SchemaKeywordKind::Single)
    };
    (schema_map) => {
        Some(SchemaKeywordKind::Map)
    };
    (schema_array) => {
        Some(SchemaKeywordKind::Array)
    };
    (items) => {
        Some(SchemaKeywordKind::Items)
    };
    (dependencies) => {
        Some(SchemaKeywordKind::Dependencies)
    };
    (external_docs) => {
        Some(SchemaKeywordKind::ExternalDocs)
    };
    ($other:ident) => {
        None::<SchemaKeywordKind>
    };
}

macro_rules! is_structural {
    ($kind:ident) => {
        structural_kind!($kind).is_some()
    };
}

macro_rules! patch_metadata_field {
    (string_map, $slot:expr, $patch:expr, $extras:expr, $key:expr, $policy:expr) => {{
        if $slot.is_none() && $extras.get($key).is_some_and(Value::is_object) {
            if let Some(patch) = &$patch {
                let mut raw = $extras.remove($key).expect("raw object checked");
                let fields = raw.as_object_mut().expect("raw object checked");
                for (name, value) in patch {
                    $policy.key(name)?;
                    $policy.replacement(value)?;
                    fields.insert(name.clone(), Value::Array(value.iter().cloned().map(Value::String).collect()));
                }
                match string_map(raw) {
                    Ok(value) => $slot = Some(value),
                    Err(value) => { $extras.insert($key.into(), value); }
                }
                return Ok(true);
            }
        }
        patch_metadata_field!(@object $slot, $patch, $extras, $key, $policy)
    }};
    (component, $slot:expr, $patch:expr, $extras:expr, $key:expr, $policy:expr) => {{
        if $slot.is_none() {
            if let Some(raw) = $extras.remove($key) {
                match component(raw) {
                    Ok(value) => $slot = Some(value),
                    Err(value) => { $extras.insert($key.into(), value); }
                }
            }
        }
        patch_metadata_field!(@object $slot, $patch, $extras, $key, $policy)
    }};
    (@object $slot:expr, $patch:expr, $extras:expr, $key:expr, $policy:expr) => {{
        if let Some(patch) = &$patch {
            let target = $slot.get_or_insert_with(Default::default);
            crate::lossless_component::metadata::MetadataValue::patch(target, patch, $policy)?;
            $extras.remove($key);
            return Ok(true);
        }
        Ok(false)
    }};
    ($other:ident, $($args:tt)*) => { Ok(false) };
}

#[derive(Clone, Copy)]
enum Origin {
    LegacyTyped,
    FormerlyOpaque,
}

use super::fields::schema_fields;

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
     structural { $($(#[$field_attr:meta])* $field:ident => $key:literal: $decode:ident),* }) => {
        impl Schema {
            /// Classify a declared structural keyword without inspecting or validating a value.
            ///
            /// `schema` is structural only when the caller supplies `native_wrapper = true`.
            /// Unknown keywords, scalar metadata and literal payloads return `None`.
            /// `Items` may hold one schema or an array; `Dependencies` maps names to
            /// schemas or required-name arrays. The caller retains traversal and admission policy.
            pub fn structural_keyword_kind(key: &str, native_wrapper: bool) -> Option<SchemaKeywordKind> {
                if key == "schema" && !native_wrapper {
                    return None;
                }
                match key {
                    $($old_key => structural_kind!($old_decode),)*
                    $($key => structural_kind!($decode),)*
                    _ => None,
                }
            }

            fn decode(mut fields: Map<String, Value>, origin: Origin) -> Result<Self, serde_json::Error> {
                let mut schema = Self::default();
                let literal_schema = if fields.contains_key("schemaFormat") {
                    None
                } else {
                    fields.remove("schema")
                };
                $(decode_legacy(&mut schema.$old, &mut fields, $old_key, origin,
                    decoder!($old_decode, origin))?;)*
                $(decode_new(&mut schema.$field, &mut fields, $key, $decode);)*
                if let Some(value) = literal_schema {
                    fields.insert("schema".into(), value);
                }
                schema.extras = fields.into_iter().collect();
                Ok(schema)
            }

            fn check_legacy_root(&self) -> Result<(), serde_json::Error> {
                $(check_legacy_field(&self.$old, self.extras.get($old_key))?;)*
                if let Some(child) = &self.content_schema {
                    child.check_legacy_root()?;
                }
                Ok(())
            }

            fn admit_legacy_root(&mut self) -> Result<(), serde_json::Error> {
                $(if let Some(value) = self.extras.remove($old_key) {
                    if self.$old.is_none() && !value.is_null() {
                        self.$old = Some(decoder!($old_decode, Origin::LegacyTyped)(value)
                            .map_err(|_| <serde_json::Error as serde::de::Error>::custom(
                                concat!("invalid schema keyword ", $old_key)))?);
                    }
                })*
                if let Some(child) = self.content_schema.take() {
                    self.content_schema = Some(Box::new(child.admit_legacy_root()?));
                }
                Ok(())
            }

            /// Visit present structural fields in lexical keyword order.
            ///
            /// Lossless invalid fields are yielded at the same position as typed
            /// fields. Typed assignments shadow extras. Literal data is skipped;
            /// no recursion, reference resolution or admission is performed.
            pub fn try_for_each_structural_field_mut<E>(
                &mut self,
                mut visit: impl FnMut(&'static str, SchemaFieldMut<'_>) -> Result<(), E>,
            ) -> Result<(), E> {
                let wrapper = self.schema_format.is_some() || self.extras.contains_key("schemaFormat");
                let mut keywords = [$(($old_key, is_structural!($old_decode)),)*
                                     $(($key, is_structural!($decode)),)*];
                keywords.sort_unstable_by_key(|(keyword, _)| *keyword);
                for (keyword, structural) in keywords {
                    if !structural || (keyword == "schema" && !wrapper) { continue; }
                    let typed = match keyword {
                        $($old_key => structural_field!($old_decode, self.$old),)*
                        $($key => structural_field!($decode, self.$field),)*
                        _ => unreachable!("keyword comes from the schema vocabulary"),
                    };
                    if let Some(field) = typed {
                        visit(keyword, field)?;
                    } else if let Some(value) = self.extras.get_mut(keyword) {
                        visit(keyword, SchemaFieldMut::Invalid(value))?;
                    }
                }
                Ok(())
            }

            /// Borrow a structural field using the caller's context for `schema`.
            /// Retained raw values, including a formerly literal `schema`, are borrowed as `Invalid`.
            pub fn structural_field(&self, key: &str, schema_is_structural: bool) -> Option<SchemaField<'_>> {
                if key == "schema" && !schema_is_structural {
                    return None;
                }
                let typed = match key {
                    $($old_key if is_structural!($old_decode) =>
                        structural_field!($old_decode, self.$old, SchemaField, as_ref, as_deref),)*
                    $($key if is_structural!($decode) =>
                        structural_field!($decode, self.$field, SchemaField, as_ref, as_deref),)*
                    _ => return None,
                };
                typed.or_else(|| self.extras.get(key).map(SchemaField::Invalid))
            }

            /// Move structural fields out in lexical keyword order.
            ///
            /// Scalars and literal payloads remain in this schema. A moved typed
            /// field also removes its shadowed fallback, so serializing the remainder
            /// cannot emit the same key again. Invalid structural fields move as-is.
            pub fn take_structural_fields(&mut self) -> BTreeMap<&'static str, OwnedSchemaField> {
                let wrapper = self.schema_format.is_some() || self.extras.contains_key("schemaFormat");
                self.take_structural_fields_with_schema(wrapper)
            }

            /// Move structural fields using the caller's context for the `schema` keyword.
            ///
            /// When enabled, a formerly literal `schema` value is decoded once from its
            /// owned value. Other keywords use the same rules as `take_structural_fields`.
            pub fn take_structural_fields_with_schema(
                &mut self,
                schema_is_structural: bool,
            ) -> BTreeMap<&'static str, OwnedSchemaField> {
                if schema_is_structural && self.schema.is_none() {
                    if let Some(value) = self.extras.remove("schema") {
                        match child(value) {
                            Ok(schema) => self.schema = Some(schema),
                            Err(value) => { self.extras.insert("schema".into(), value); }
                        }
                    }
                }
                let mut result = BTreeMap::new();
                $(take_owned_field(&mut result, &mut self.extras, $old_key,
                    is_structural!($old_decode), take_structural_field!($old_decode, self.$old));)*
                $(if $key != "schema" || schema_is_structural {
                    take_owned_field(&mut result, &mut self.extras, $key,
                        is_structural!($decode), take_structural_field!($decode, self.$field));
                })*
                result
            }

            /// Put back a structural field, removing any shadowed fallback.
            /// Unknown keys or mismatched variants return the field without modifying this schema.
            pub fn put_structural_field(
                &mut self,
                key: &str,
                field: OwnedSchemaField,
            ) -> Result<(), OwnedSchemaField> {
                match key {
                    $($old_key => put_structural_field!($old_decode, self.$old, self.extras, key, field),)*
                    $($key => put_structural_field!($decode, self.$field, self.extras, key, field),)*
                    _ => Err(field),
                }
            }

            /// Borrow effective keyword names in lexical order, without duplicating shadowed extras.
            pub fn keyword_names(&self) -> Vec<&str> {
                let mut names: Vec<&str> = [$(($old_key, self.$old.is_some()),)*
                    $(($key, self.$field.is_some()),)*]
                    .into_iter().filter_map(|(key, present)| present.then_some(key)).collect();
                names.extend(self.extras.keys().filter(|key| !self.owns_keyword(key)).map(String::as_str));
                names.sort_unstable();
                names
            }

            /// Patch a typed object metadata keyword, returning false for other fields.
            /// The caller handles raw fallback values and charges the outer keyword.
            pub fn patch_metadata_keyword<P: crate::lossless_component::MetadataPatch>(
                &mut self, key: &str, patch: &Self, policy: &mut P,
            ) -> Result<bool, P::Error> {
                match key {
                    $($old_key => patch_metadata_field!($old_decode, self.$old, patch.$old, self.extras, key, policy),)*
                    $($key => patch_metadata_field!($decode, self.$field, patch.$field, self.extras, key, policy),)*
                    _ => Ok(false),
                }
            }

            /// Borrow the raw fallback only when no typed field owns this keyword.
            pub fn raw_keyword(&self, key: &str) -> Option<&Value> {
                if self.owns_keyword(key) { None } else { self.extras.get(key) }
            }

            /// Replace one keyword from an owned raw value, promoting valid structure once.
            pub fn set_raw_keyword(&mut self, key: &str, value: Value) {
                self.remove_keyword(key);
                if key == "schema" && self.schema_format.is_none() && !self.extras.contains_key("schemaFormat") {
                    self.extras.insert(key.into(), value);
                    return;
                }
                match key {
                    $($old_key => match decoder!($old_decode, Origin::FormerlyOpaque)(value) {
                        Ok(value) => self.$old = Some(value),
                        Err(value) => { self.extras.insert(key.into(), value); }
                    },)*
                    $($key => match $decode(value) {
                        Ok(value) => self.$field = Some(value),
                        Err(value) => { self.extras.insert(key.into(), value); }
                    },)*
                    _ => { self.extras.insert(key.into(), value); }
                }
            }

            /// Serialize one effective field by borrowing its typed value or fallback.
            /// A missing field returns `None` without invoking the serializer.
            pub fn serialize_keyword<S: Serializer>(
                &self,
                key: &str,
                serializer: S,
            ) -> Result<Option<S::Ok>, S::Error> {
                match key {
                    $($old_key => if let Some(value) = &self.$old {
                        return value.serialize(serializer).map(Some);
                    },)*
                    $($key => if let Some(value) = &self.$field {
                        return value.serialize(serializer).map(Some);
                    },)*
                    _ => {}
                }
                self.extras.get(key).map(|value| value.serialize(serializer)).transpose()
            }

            /// Test whether the effective value is an explicit JSON null; absent fields are false.
            pub fn keyword_is_null(&self, key: &str) -> bool {
                !self.owns_keyword(key) && self.extras.get(key).is_some_and(Value::is_null)
            }

            /// Remove both the typed value and its fallback, returning whether either was present.
            pub fn remove_keyword(&mut self, key: &str) -> bool {
                let typed = match key {
                    $($old_key => self.$old.take().is_some(),)*
                    $($key => self.$field.take().is_some(),)*
                    _ => false,
                };
                self.extras.remove(key).is_some() || typed
            }

            /// Move an effective field from another schema, discarding shadowed fallbacks.
            /// An absent source field leaves the destination unchanged.
            pub fn move_keyword_from(&mut self, key: &str, source: &mut Self) -> bool {
                match key {
                    $($old_key => if let Some(value) = source.$old.take() {
                        self.$old = Some(value);
                        self.extras.remove(key);
                        source.extras.remove(key);
                        return true;
                    },)*
                    $($key => if let Some(value) = source.$field.take() {
                        self.$field = Some(value);
                        self.extras.remove(key);
                        source.extras.remove(key);
                        return true;
                    },)*
                    _ => {}
                }
                if let Some(value) = source.extras.remove(key) {
                    self.remove_keyword(key);
                    self.extras.insert(key.into(), value);
                    true
                } else {
                    false
                }
            }

            /// Clone one effective field from a borrowed source, discarding the destination fallback.
            /// An absent source field leaves the destination unchanged.
            pub fn clone_keyword_from(&mut self, key: &str, source: &Self) -> bool {
                match key {
                    $($old_key => if let Some(value) = &source.$old {
                        self.$old = Some(value.clone());
                        self.extras.remove(key);
                        return true;
                    },)*
                    $($key => if let Some(value) = &source.$field {
                        self.$field = Some(value.clone());
                        self.extras.remove(key);
                        return true;
                    },)*
                    _ => {}
                }
                if let Some(value) = source.extras.get(key) {
                    self.remove_keyword(key);
                    self.extras.insert(key.into(), value.clone());
                    true
                } else {
                    false
                }
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

fn check_legacy_field<T: DeserializeOwned>(
    typed: &Option<T>,
    fallback: Option<&Value>,
) -> Result<(), serde_json::Error> {
    if typed.is_none() {
        if let Some(value) = fallback.filter(|value| !value.is_null()) {
            T::deserialize(value)?;
        }
    }
    Ok(())
}

fn take_owned_field(
    fields: &mut BTreeMap<&'static str, OwnedSchemaField>,
    extras: &mut BTreeMap<String, Value>,
    key: &'static str,
    structural: bool,
    typed: Option<OwnedSchemaField>,
) {
    if !structural {
        return;
    }
    let fallback = extras.remove(key);
    if let Some(field) = typed.or_else(|| fallback.map(OwnedSchemaField::Invalid)) {
        fields.insert(key, field);
    }
}

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

fn external_docs(value: Value) -> Result<crate::lossless_component::ExternalDocumentation, Value> {
    component(value)
}

fn component<T: crate::lossless_component::Decode>(value: Value) -> Result<T, Value> {
    T::decode(value)
}

fn is_schema(value: &Value) -> bool {
    value.is_object() || value.is_boolean()
}

impl SchemaValue {
    // Used only to choose the inline candidate of a Referenceable before moving it.
    pub(crate) fn check_legacy_root(&self) -> Result<(), serde_json::Error> {
        match self {
            Self::Object(schema) => schema.check_legacy_root(),
            Self::Boolean(_) => Ok(()),
            Self::Invalid(_) => Err(<serde_json::Error as serde::de::Error>::custom(
                "expected an object or boolean schema",
            )),
        }
    }

    /// Apply the ordinary decoder's legacy-field admission to an owned schema.
    ///
    /// This removes null legacy fields and rejects malformed legacy values without
    /// serializing the schema. Only `contentSchema` recursively crosses the same
    /// legacy boundary; other structural children keep their lossless admission.
    pub fn admit_legacy_root(mut self) -> Result<Self, serde_json::Error> {
        match &mut self {
            Self::Object(schema) => schema.admit_legacy_root()?,
            Self::Boolean(_) => {}
            Self::Invalid(_) => {
                return Err(<serde_json::Error as serde::de::Error>::custom(
                    "expected an object or boolean schema",
                ))
            }
        }
        Ok(self)
    }

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
    if is_schema(&value) {
        SchemaValue::from_lossless_value(value)
            .expect("opaque schema fields are retained losslessly")
    } else {
        SchemaValue::Invalid(value)
    }
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
    Ok(fields
        .into_iter()
        .map(|(key, value)| (key, opaque(value)))
        .collect())
}

fn schema_array(value: Value) -> Result<Vec<SchemaValue>, Value> {
    let Value::Array(values) = value else {
        return Err(value);
    };
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
    Ok(fields
        .into_iter()
        .map(|(key, value)| {
            let dependency = if is_strings(&value) {
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
