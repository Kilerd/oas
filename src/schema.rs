//! Structural schema access without walking instance data or resolving references.
use crate::json_pointer::Step;
use crate::{Schema, SchemaDependency, SchemaItems, SchemaValue};
use serde::{Deserialize, Deserializer, Serialize};

pub(crate) mod fields;
use fields::schema_fields;

#[cfg(feature = "lossless-schema-keywords")]
mod lossless;
#[cfg(feature = "lossless-schema-keywords")]
pub use lossless::{OwnedSchemaField, SchemaField, SchemaFieldMut, SchemaKeywordKind};

/// A nonnegative integer count, retaining integer-valued JSON number forms.
///
/// Values are bounded by `u64`. `1.0` and `1e0` are accepted as integers; their
/// parsed JSON number representation is retained when serialized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SchemaCount(serde_json::Number);

impl SchemaCount {
    /// The retained JSON number, for consumers with a representation-specific policy.
    pub fn as_number(&self) -> &serde_json::Number {
        &self.0
    }

    pub fn value(&self) -> u64 {
        self.0
            .as_u64()
            .unwrap_or_else(|| self.0.as_f64().expect("a count is a JSON number") as u64)
    }
}

impl From<u64> for SchemaCount {
    fn from(value: u64) -> Self {
        Self(value.into())
    }
}

impl<'de> Deserialize<'de> for SchemaCount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let number = serde_json::Number::deserialize(deserializer)?;
        let integral = number.as_u64().is_some()
            || number.as_f64().is_some_and(|value| {
                value >= 0.0 && value < u64::MAX as f64 && value.fract() == 0.0
            });
        if !integral {
            return Err(serde::de::Error::custom(
                "expected a nonnegative integer count within u64",
            ));
        }
        Ok(Self(number))
    }
}

// Missing fields default to None; present values parse as T (JSON payload null stays Some).
pub(crate) fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

// Child shapes are shared by all vocabulary entries of the same kind. Scalars,
// metadata and literal payloads have no schema child to visit or look up.
macro_rules! visit_typed_children {
    (legacy_child, $($args:tt)*) => { visit_typed_children!(child, $($args)*) };
    (child, $field:expr, $key:expr, $visit:ident, $iter:ident, $values:ident, $borrow:ident) => {
        visit_children($key, $field.$iter().map(Box::$borrow), &mut $visit)?;
    };
    (schema_map, $field:expr, $key:expr, $visit:ident, $iter:ident, $values:ident, $borrow:ident) => {
        visit_children($key, $field.$iter().flat_map(|map| map.$values()), &mut $visit)?;
    };
    (schema_array, $field:expr, $key:expr, $visit:ident, $iter:ident, $values:ident, $borrow:ident) => {
        visit_children($key, $field.$iter().flatten(), &mut $visit)?;
    };
    (items, $field:expr, $key:expr, $visit:ident, $iter:ident, $values:ident, $borrow:ident) => {
        match $field.$borrow() {
            Some(SchemaItems::Schema(child)) => $visit($key, child)?,
            Some(SchemaItems::Tuple(children)) => visit_children($key, children, &mut $visit)?,
            None => {}
        }
    };
    (dependencies, $field:expr, $key:expr, $visit:ident, $iter:ident, $values:ident, $borrow:ident) => {
        visit_children($key, $field.$iter().flat_map(|map| map.$values()).filter_map(|dependency| {
            match dependency {
                SchemaDependency::Schema(child) => Some(child.$borrow()),
                SchemaDependency::Required(_) => None,
            }
        }), &mut $visit)?;
    };
    ($other:ident, $($args:tt)*) => {};
}

// The keyed counterpart of `visit_typed_children`: the same children in the same
// order, each with the step that locates it under its schema.
macro_rules! visit_keyed_children {
    (legacy_child, $($args:tt)*) => { visit_keyed_children!(child, $($args)*) };
    (child, $field:expr, $key:expr, $visit:ident, $iter:ident, $borrow:ident) => {
        for child in $field.$iter().map(Box::$borrow) {
            $visit(Step::Token($key), child)?;
        }
    };
    (schema_map, $field:expr, $key:expr, $visit:ident, $iter:ident, $borrow:ident) => {
        for (name, child) in $field.$iter().flat_map(|map| map.$iter()) {
            $visit(Step::Entry { field: $key, key: name }, child)?;
        }
    };
    (schema_array, $field:expr, $key:expr, $visit:ident, $iter:ident, $borrow:ident) => {
        for (index, child) in $field.$iter().flatten().enumerate() {
            $visit(Step::Element { field: $key, index }, child)?;
        }
    };
    (items, $field:expr, $key:expr, $visit:ident, $iter:ident, $borrow:ident) => {
        match $field.$borrow() {
            Some(SchemaItems::Schema(child)) => $visit(Step::Token($key), child)?,
            Some(SchemaItems::Tuple(children)) => {
                for (index, child) in children.$iter().enumerate() {
                    $visit(Step::Element { field: $key, index }, child)?;
                }
            }
            None => {}
        }
    };
    (dependencies, $field:expr, $key:expr, $visit:ident, $iter:ident, $borrow:ident) => {
        for (name, dependency) in $field.$iter().flat_map(|map| map.$iter()) {
            if let SchemaDependency::Schema(child) = dependency {
                $visit(Step::Entry { field: $key, key: name }, child)?;
            }
        }
    };
    ($other:ident, $($args:tt)*) => {};
}

macro_rules! schema_child {
    (legacy_child, $($args:tt)*) => { schema_child!(child, $($args)*) };
    (child, $field:expr, $tail:ident) => {
        $field.as_deref().map(|child| (child, $tail))
    };
    (schema_map, $field:expr, $tail:ident) => {{
        let (key, rest) = $tail.split_first()?;
        Some(($field.as_ref()?.get(key.as_ref())?, rest))
    }};
    (schema_array, $field:expr, $tail:ident) => {{
        let (index, rest) = $tail.split_first()?;
        Some(($field.as_ref()?.get(index.as_ref().parse::<usize>().ok()?)?, rest))
    }};
    (items, $field:expr, $tail:ident) => {
        match $field.as_ref()? {
            SchemaItems::Schema(child) => Some((child.as_ref(), $tail)),
            SchemaItems::Tuple(children) => {
                let (index, rest) = $tail.split_first()?;
                Some((children.get(index.as_ref().parse::<usize>().ok()?)?, rest))
            }
        }
    };
    (dependencies, $field:expr, $tail:ident) => {{
        let (key, rest) = $tail.split_first()?;
        match $field.as_ref()?.get(key.as_ref())? {
            SchemaDependency::Schema(child) => Some((child.as_ref(), rest)),
            SchemaDependency::Required(_) => None,
        }
    }};
    ($other:ident, $($args:tt)*) => { None };
}

macro_rules! implement_schema_access {
    (legacy { $($old:ident => $old_key:literal: $old_kind:ident),* }
     structural { $($(#[$attr:meta])* $field:ident => $key:literal: $kind:ident),* }) => {
        implement_schema_access!(@fields
            $($old => $old_key: $old_kind,)*
            $($(#[$attr])* $field => $key: $kind,)*
        );
    };
    (@fields $($(#[$attr:meta])* $field:ident => $key:literal: $kind:ident,)*) => {
        impl Schema {
            fn child_at_tokens<'a, 't, T: AsRef<str>>(
                &'a self,
                keyword: &str,
                tail: &'t [T],
            ) -> Option<(&'a SchemaValue, &'t [T])> {
                if !self.is_schema_child_keyword(keyword) { return None; }
                match keyword {
                    $($(#[$attr])* $key => schema_child!($kind, self.$field, tail),)*
                    _ => None,
                }
            }

            /// Count present entries, including extras and explicitly empty collections.
            /// Typed and extra entries count separately, as in the derived serializer.
            #[cfg(not(feature = "lossless-schema-keywords"))]
            pub fn keyword_count(&self) -> usize {
                self.extras.len() + [ $($(#[$attr])* self.$field.is_some(),)* ]
                    .into_iter().filter(|present| *present).count()
            }

            /// Borrow immediate schema children in lexical keyword order, stopping on error.
            /// Map keys are lexical and arrays retain their order. Literal payloads and
            /// lossless fallback fields are skipped; this does not recurse or resolve references.
            pub fn try_for_each_child_with_keyword<E>(
                &self,
                mut visit: impl FnMut(&'static str, &SchemaValue) -> Result<(), E>,
            ) -> Result<(), E> {
                let mut keywords = [$($(#[$attr])* $key,)*];
                keywords.sort_unstable();
                for keyword in keywords {
                    if !self.is_schema_child_keyword(keyword) { continue; }
                    match keyword {
                        $($(#[$attr])* $key => {
                            visit_typed_children!($kind, self.$field, $key, visit, iter, values, as_ref);
                        },)*
                        _ => unreachable!("keyword comes from the schema vocabulary"),
                    }
                }
                Ok(())
            }

            /// Visit immediate children with their containing keyword, in lexical order.
            /// Array children retain their order. An error leaves prior edits in place.
            /// This does not recurse, resolve references or visit literal/fallback values.
            pub fn try_for_each_child_mut_with_keyword<E>(
                &mut self,
                mut visit: impl FnMut(&'static str, &mut SchemaValue) -> Result<(), E>,
            ) -> Result<(), E> {
                let mut keywords = [$($(#[$attr])* $key,)*];
                keywords.sort_unstable();
                for keyword in keywords {
                    if !self.is_schema_child_keyword(keyword) { continue; }
                    match keyword {
                        $($(#[$attr])* $key => {
                            visit_typed_children!($kind, self.$field, $key, visit, iter_mut, values_mut, as_mut);
                        },)*
                        _ => unreachable!("keyword comes from the schema vocabulary"),
                    }
                }
                Ok(())
            }

            /// Borrow immediate schema children with the [`Step`] that locates each
            /// one: `/keyword` for a single child, `/keyword/name` for a map entry and
            /// `/keyword/index` for an array element. Children and their order are
            /// those of [`Schema::try_for_each_child_with_keyword`], whose keyword is
            /// [`Step::name`]. This does not recurse or resolve references.
            pub fn try_for_each_child_at<E>(
                &self,
                mut visit: impl FnMut(Step<'_>, &SchemaValue) -> Result<(), E>,
            ) -> Result<(), E> {
                let mut keywords = [$($(#[$attr])* $key,)*];
                keywords.sort_unstable();
                for keyword in keywords {
                    if !self.is_schema_child_keyword(keyword) { continue; }
                    match keyword {
                        $($(#[$attr])* $key => {
                            visit_keyed_children!($kind, self.$field, $key, visit, iter, as_ref);
                        },)*
                        _ => unreachable!("keyword comes from the schema vocabulary"),
                    }
                }
                Ok(())
            }

            /// The editable form of [`Schema::try_for_each_child_at`]. An error
            /// leaves edits to earlier children in place.
            pub fn try_for_each_child_mut_at<E>(
                &mut self,
                mut visit: impl FnMut(Step<'_>, &mut SchemaValue) -> Result<(), E>,
            ) -> Result<(), E> {
                let mut keywords = [$($(#[$attr])* $key,)*];
                keywords.sort_unstable();
                for keyword in keywords {
                    if !self.is_schema_child_keyword(keyword) { continue; }
                    match keyword {
                        $($(#[$attr])* $key => {
                            visit_keyed_children!($kind, self.$field, $key, visit, iter_mut, as_mut);
                        },)*
                        _ => unreachable!("keyword comes from the schema vocabulary"),
                    }
                }
                Ok(())
            }
        }

        #[cfg(feature = "lossless-schema-keywords")]
        impl crate::HeapSize for Schema {
            fn heap_size(&self) -> usize {
                let mut size = crate::HeapSize::heap_size(&self.extras);
                $($(#[$attr])* {
                    size = size.saturating_add(crate::HeapSize::heap_size(&self.$field));
                })*
                size
            }
        }
    };
}
schema_fields!(implement_schema_access);

impl SchemaValue {
    /// Borrow a structural schema descendant using already decoded path tokens.
    ///
    /// An empty path returns this schema. Map keys are used verbatim; array tokens
    /// parse as `usize`. Literal payloads, unknown vocabulary and malformed fields
    /// retained in lossless extras are not schema children. This does not decode a
    /// JSON Pointer, follow references, or traverse an external document.
    pub fn schema_at_tokens<T: AsRef<str>>(&self, tokens: &[T]) -> Option<&Self> {
        let mut current = self;
        let mut tokens = tokens;
        while let Some((keyword, tail)) = tokens.split_first() {
            let schema = current.as_object()?;
            let (child, rest) = schema.child_at_tokens(keyword.as_ref(), tail)?;
            current = child;
            tokens = rest;
        }
        Some(current)
    }
}

impl Schema {
    fn is_schema_child_keyword(&self, keyword: &str) -> bool {
        #[cfg(feature = "lossless-schema-keywords")]
        if keyword == "schema" {
            return self.schema_format.is_some() || self.extras.contains_key("schemaFormat");
        }
        let _ = keyword;
        true
    }

    /// Visit immediate schema children in lexical keyword order, stopping on error.
    ///
    /// This method does not recurse or follow `$ref`. The caller chooses whether to
    /// descend and can charge depth/work before each descent. Literal payloads in
    /// `extras` (including examples, defaults and extension data) are never visited.
    /// A visitor error leaves edits to earlier children in place.
    pub fn try_for_each_child_mut<E>(
        &mut self,
        mut visit: impl FnMut(&mut SchemaValue) -> Result<(), E>,
    ) -> Result<(), E> {
        self.try_for_each_child_mut_with_keyword(|_, child| visit(child))
    }
}

fn visit_children<T, E>(
    keyword: &'static str,
    children: impl IntoIterator<Item = T>,
    visit: &mut impl FnMut(&'static str, T) -> Result<(), E>,
) -> Result<(), E> {
    for child in children {
        visit(keyword, child)?;
    }
    Ok(())
}

/// Strict (default-feature) schema decoding.
///
/// The derived deserializer buffered the whole schema object into Serde's
/// `Content` at every nesting level, because `Schema` flattens its extras and
/// its child enums are untagged. A schema nested 100 levels deep therefore
/// copied its leaf subtree once per level. Decoding the map once with
/// `serde_json` and moving children out of it keeps the cost linear in the
/// input size; no node is copied per nesting level.
#[cfg(not(feature = "lossless-schema-keywords"))]
mod strict {
    use crate::{Schema, SchemaCount, SchemaDependency, SchemaItems, SchemaValue};
    use serde::de::{self, DeserializeOwned};
    use serde::{Deserialize, Deserializer};
    use serde_json::{Map, Value};
    use std::collections::BTreeMap;

    impl<'de> Deserialize<'de> for Schema {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            decode(Map::<String, Value>::deserialize(deserializer)?)
        }
    }

    fn decode<E>(mut fields: Map<String, Value>) -> Result<Schema, E>
    where
        E: de::Error,
    {
        let mut schema = Schema::default();
        // Legacy fields treat JSON null as absent, like serde's `Option<T>`.
        schema._type = legacy(fields.remove("type"), "type")?;
        schema._ref = legacy(fields.remove("$ref"), "$ref")?;
        schema.format = legacy(fields.remove("format"), "format")?;
        schema.nullable = legacy(fields.remove("nullable"), "nullable")?;
        schema.description = legacy(fields.remove("description"), "description")?;
        schema.content_media_type = legacy(fields.remove("contentMediaType"), "contentMediaType")?;
        schema.content_encoding = legacy(fields.remove("contentEncoding"), "contentEncoding")?;
        schema.content_schema = legacy(fields.remove("contentSchema"), "contentSchema")?;
        // Structural fields reject a present null or malformed value.
        schema.properties = present(fields.remove("properties"), "properties", schema_map)?;
        schema.pattern_properties = present(
            fields.remove("patternProperties"),
            "patternProperties",
            schema_map,
        )?;
        schema.required = present(fields.remove("required"), "required", strings)?;
        schema.min_properties = present(fields.remove("minProperties"), "minProperties", count)?;
        schema.max_properties = present(fields.remove("maxProperties"), "maxProperties", count)?;
        schema.additional_properties = present(
            fields.remove("additionalProperties"),
            "additionalProperties",
            boxed_schema,
        )?;
        schema.unevaluated_properties = present(
            fields.remove("unevaluatedProperties"),
            "unevaluatedProperties",
            boxed_schema,
        )?;
        schema.property_names = present(
            fields.remove("propertyNames"),
            "propertyNames",
            boxed_schema,
        )?;
        schema.all_of = present(fields.remove("allOf"), "allOf", schema_array)?;
        schema.any_of = present(fields.remove("anyOf"), "anyOf", schema_array)?;
        schema.one_of = present(fields.remove("oneOf"), "oneOf", schema_array)?;
        schema.not = present(fields.remove("not"), "not", boxed_schema)?;
        schema.if_schema = present(fields.remove("if"), "if", boxed_schema)?;
        schema.then_schema = present(fields.remove("then"), "then", boxed_schema)?;
        schema.else_schema = present(fields.remove("else"), "else", boxed_schema)?;
        schema.dependent_required = present(
            fields.remove("dependentRequired"),
            "dependentRequired",
            string_map,
        )?;
        schema.dependent_schemas = present(
            fields.remove("dependentSchemas"),
            "dependentSchemas",
            schema_map,
        )?;
        schema.dependencies = present(fields.remove("dependencies"), "dependencies", dependencies)?;
        schema.defs = present(fields.remove("$defs"), "$defs", schema_map)?;
        schema.definitions = present(fields.remove("definitions"), "definitions", schema_map)?;
        schema.items = present(fields.remove("items"), "items", items)?;
        schema.prefix_items = present(fields.remove("prefixItems"), "prefixItems", schema_array)?;
        schema.additional_items = present(
            fields.remove("additionalItems"),
            "additionalItems",
            boxed_schema,
        )?;
        schema.unevaluated_items = present(
            fields.remove("unevaluatedItems"),
            "unevaluatedItems",
            boxed_schema,
        )?;
        schema.contains = present(fields.remove("contains"), "contains", boxed_schema)?;
        schema.extras = fields.into_iter().collect();
        Ok(schema)
    }

    fn legacy<T, E>(value: Option<Value>, key: &str) -> Result<Option<T>, E>
    where
        T: DeserializeOwned,
        E: de::Error,
    {
        match value {
            None | Some(Value::Null) => Ok(None),
            Some(value) => T::deserialize(value)
                .map(Some)
                .map_err(|_| E::custom(format!("invalid schema keyword {key}"))),
        }
    }

    fn present<T, E>(
        value: Option<Value>,
        key: &str,
        parse: impl FnOnce(Value) -> Result<T, E>,
    ) -> Result<Option<T>, E>
    where
        E: de::Error,
    {
        match value {
            None => Ok(None),
            Some(value) => parse(value)
                .map(Some)
                .map_err(|_| E::custom(format!("invalid schema keyword {key}"))),
        }
    }

    fn schema_value<E: de::Error>(value: Value) -> Result<SchemaValue, E> {
        match value {
            Value::Bool(value) => Ok(SchemaValue::Boolean(value)),
            Value::Object(fields) => {
                decode(fields).map(|schema| SchemaValue::Object(Box::new(schema)))
            }
            _ => Err(E::custom("expected a schema object or boolean")),
        }
    }

    fn boxed_schema<E: de::Error>(value: Value) -> Result<Box<SchemaValue>, E> {
        schema_value(value).map(Box::new)
    }

    fn schema_map<E: de::Error>(value: Value) -> Result<BTreeMap<String, SchemaValue>, E> {
        match value {
            Value::Object(fields) => fields
                .into_iter()
                .map(|(key, value)| Ok((key, schema_value(value)?)))
                .collect(),
            _ => Err(E::custom("expected an object of schemas")),
        }
    }

    fn schema_array<E: de::Error>(value: Value) -> Result<Vec<SchemaValue>, E> {
        match value {
            Value::Array(values) => values.into_iter().map(schema_value).collect(),
            _ => Err(E::custom("expected an array of schemas")),
        }
    }

    fn strings<E: de::Error>(value: Value) -> Result<Vec<String>, E> {
        match value {
            Value::Array(values) => values
                .into_iter()
                .map(|value| match value {
                    Value::String(value) => Ok(value),
                    _ => Err(E::custom("expected an array of strings")),
                })
                .collect(),
            _ => Err(E::custom("expected an array of strings")),
        }
    }

    fn string_map<E: de::Error>(value: Value) -> Result<BTreeMap<String, Vec<String>>, E> {
        match value {
            Value::Object(fields) => fields
                .into_iter()
                .map(|(key, value)| Ok((key, strings(value)?)))
                .collect(),
            _ => Err(E::custom("expected an object of string arrays")),
        }
    }

    fn count<E: de::Error>(value: Value) -> Result<SchemaCount, E> {
        SchemaCount::deserialize(value)
            .map_err(|_| E::custom("expected a nonnegative integer count"))
    }

    fn items<E: de::Error>(value: Value) -> Result<SchemaItems, E> {
        match value {
            Value::Array(values) => Ok(SchemaItems::Tuple(
                values
                    .into_iter()
                    .map(schema_value)
                    .collect::<Result<_, E>>()?,
            )),
            value => Ok(SchemaItems::Schema(Box::new(schema_value(value)?))),
        }
    }

    fn dependencies<E: de::Error>(value: Value) -> Result<BTreeMap<String, SchemaDependency>, E> {
        match value {
            Value::Object(fields) => fields
                .into_iter()
                .map(|(key, value)| {
                    let dependency = match value {
                        Value::Array(values) => {
                            SchemaDependency::Required(strings(Value::Array(values))?)
                        }
                        value => SchemaDependency::Schema(Box::new(schema_value(value)?)),
                    };
                    Ok((key, dependency))
                })
                .collect(),
            _ => Err(E::custom("expected an object of dependencies")),
        }
    }
}
