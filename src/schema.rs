//! Structural schema access without walking instance data or resolving references.
use crate::{Schema, SchemaDependency, SchemaItems, SchemaValue};
use serde::{Deserialize, Deserializer, Serialize};

/// A nonnegative integer count, retaining integer-valued JSON number forms.
///
/// Values are bounded by `u64`. `1.0` and `1e0` are accepted as integers; their
/// parsed JSON number representation is retained when serialized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SchemaCount(serde_json::Number);

impl SchemaCount {
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

// Missing keywords use Option::default; present null must not erase a type error.
pub(crate) fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

impl Schema {
    /// Visit immediate schema children in a deterministic order, stopping on error.
    ///
    /// This method does not recurse or follow `$ref`. The caller chooses whether to
    /// descend and can charge depth/work before each descent. Literal payloads in
    /// `extras` (including examples, defaults and extension data) are never visited.
    /// A visitor error leaves edits to earlier children in place.
    pub fn try_for_each_child_mut<E>(
        &mut self,
        mut visit: impl FnMut(&mut SchemaValue) -> Result<(), E>,
    ) -> Result<(), E> {
        for children in [
            &mut self.properties,
            &mut self.pattern_properties,
            &mut self.dependent_schemas,
            &mut self.defs,
            &mut self.definitions,
        ]
        .into_iter()
        .flatten()
        {
            for child in children.values_mut() {
                visit(child)?;
            }
        }
        for children in [
            &mut self.all_of,
            &mut self.any_of,
            &mut self.one_of,
            &mut self.prefix_items,
        ]
        .into_iter()
        .flatten()
        {
            for child in children {
                visit(child)?;
            }
        }
        for child in [
            &mut self.content_schema,
            &mut self.additional_properties,
            &mut self.unevaluated_properties,
            &mut self.property_names,
            &mut self.not,
            &mut self.if_schema,
            &mut self.then_schema,
            &mut self.else_schema,
            &mut self.additional_items,
            &mut self.unevaluated_items,
            &mut self.contains,
        ]
        .into_iter()
        .flatten()
        {
            visit(child)?;
        }
        match &mut self.items {
            Some(SchemaItems::Schema(child)) => visit(child)?,
            Some(SchemaItems::Tuple(children)) => {
                for child in children {
                    visit(child)?;
                }
            }
            None => {}
        }
        for dependency in self
            .dependencies
            .iter_mut()
            .flat_map(|map| map.values_mut())
        {
            if let SchemaDependency::Schema(child) = dependency {
                visit(child)?;
            }
        }
        Ok(())
    }
}
