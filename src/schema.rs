//! Structural schema access without walking instance data or resolving references.
use crate::{Schema, SchemaDependency, SchemaItems, SchemaValue};
use serde::{Deserialize, Deserializer, Serialize};

#[cfg(feature = "lossless-schema-keywords")]
mod lossless;

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

impl Schema {
    /// Count present keyword entries, including extras and explicitly empty collections.
    ///
    /// Does not visit children or serialize payloads. In strict mode, duplicate
    /// typed and extra entries count separately, as in its derived serializer.
    /// With `lossless-schema-keywords`, a typed field shadows the matching extra.
    #[cfg(not(feature = "lossless-schema-keywords"))]
    pub fn keyword_count(&self) -> usize {
        self.extras.len()
            + [
                self._type.is_some(),
                self._ref.is_some(),
                self.format.is_some(),
                self.nullable.is_some(),
                self.description.is_some(),
                self.content_media_type.is_some(),
                self.content_encoding.is_some(),
                self.content_schema.is_some(),
                self.properties.is_some(),
                self.pattern_properties.is_some(),
                self.required.is_some(),
                self.min_properties.is_some(),
                self.max_properties.is_some(),
                self.additional_properties.is_some(),
                self.unevaluated_properties.is_some(),
                self.property_names.is_some(),
                self.all_of.is_some(),
                self.any_of.is_some(),
                self.one_of.is_some(),
                self.not.is_some(),
                self.if_schema.is_some(),
                self.then_schema.is_some(),
                self.else_schema.is_some(),
                self.dependent_required.is_some(),
                self.dependent_schemas.is_some(),
                self.dependencies.is_some(),
                self.defs.is_some(),
                self.definitions.is_some(),
                self.items.is_some(),
                self.prefix_items.is_some(),
                self.additional_items.is_some(),
                self.unevaluated_items.is_some(),
                self.contains.is_some(),
            ]
            .into_iter()
            .filter(|present| *present)
            .count()
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

    /// Visit immediate children with the keyword containing each child.
    ///
    /// Keywords and map keys are visited lexically; array children retain their
    /// original order. The keyword lets callers select dialect-specific traversal.
    /// Returning `Ok(())` without editing a child skips it. This does not recurse,
    /// resolve references, or visit literal payloads and unknown vocabulary.
    pub fn try_for_each_child_mut_with_keyword<E>(
        &mut self,
        mut visit: impl FnMut(&'static str, &mut SchemaValue) -> Result<(), E>,
    ) -> Result<(), E> {
        visit_children(
            "$defs",
            self.defs.iter_mut().flat_map(|m| m.values_mut()),
            &mut visit,
        )?;
        visit_children(
            "additionalItems",
            self.additional_items.iter_mut().map(Box::as_mut),
            &mut visit,
        )?;
        visit_children(
            "additionalProperties",
            self.additional_properties.iter_mut().map(Box::as_mut),
            &mut visit,
        )?;
        visit_children("allOf", self.all_of.iter_mut().flatten(), &mut visit)?;
        visit_children("anyOf", self.any_of.iter_mut().flatten(), &mut visit)?;
        visit_children(
            "contains",
            self.contains.iter_mut().map(Box::as_mut),
            &mut visit,
        )?;
        visit_children(
            "contentSchema",
            self.content_schema.iter_mut().map(Box::as_mut),
            &mut visit,
        )?;
        visit_children(
            "definitions",
            self.definitions.iter_mut().flat_map(|m| m.values_mut()),
            &mut visit,
        )?;
        visit_children(
            "dependencies",
            self.dependencies
                .iter_mut()
                .flat_map(|m| m.values_mut())
                .filter_map(|dependency| match dependency {
                    SchemaDependency::Schema(child) => Some(child.as_mut()),
                    SchemaDependency::Required(_) => None,
                }),
            &mut visit,
        )?;
        visit_children(
            "dependentSchemas",
            self.dependent_schemas
                .iter_mut()
                .flat_map(|m| m.values_mut()),
            &mut visit,
        )?;
        visit_children(
            "else",
            self.else_schema.iter_mut().map(Box::as_mut),
            &mut visit,
        )?;
        visit_children("if", self.if_schema.iter_mut().map(Box::as_mut), &mut visit)?;
        match &mut self.items {
            Some(SchemaItems::Schema(child)) => visit("items", child)?,
            Some(SchemaItems::Tuple(children)) => visit_children("items", children, &mut visit)?,
            None => {}
        }
        visit_children("not", self.not.iter_mut().map(Box::as_mut), &mut visit)?;
        visit_children("oneOf", self.one_of.iter_mut().flatten(), &mut visit)?;
        visit_children(
            "patternProperties",
            self.pattern_properties
                .iter_mut()
                .flat_map(|m| m.values_mut()),
            &mut visit,
        )?;
        visit_children(
            "prefixItems",
            self.prefix_items.iter_mut().flatten(),
            &mut visit,
        )?;
        visit_children(
            "properties",
            self.properties.iter_mut().flat_map(|m| m.values_mut()),
            &mut visit,
        )?;
        visit_children(
            "propertyNames",
            self.property_names.iter_mut().map(Box::as_mut),
            &mut visit,
        )?;
        visit_children(
            "then",
            self.then_schema.iter_mut().map(Box::as_mut),
            &mut visit,
        )?;
        visit_children(
            "unevaluatedItems",
            self.unevaluated_items.iter_mut().map(Box::as_mut),
            &mut visit,
        )?;
        visit_children(
            "unevaluatedProperties",
            self.unevaluated_properties.iter_mut().map(Box::as_mut),
            &mut visit,
        )
    }
}

fn visit_children<'a, E>(
    keyword: &'static str,
    children: impl IntoIterator<Item = &'a mut SchemaValue>,
    visit: &mut impl FnMut(&'static str, &mut SchemaValue) -> Result<(), E>,
) -> Result<(), E> {
    for child in children {
        visit(keyword, child)?;
    }
    Ok(())
}
