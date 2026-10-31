//! Field-preserving edits for document metadata objects.
use super::{Decode, Field, Fields};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Consumer policy for metadata edits. Keys are visited lexically before their values.
/// Returning true from `key` replaces that value literally, including null.
pub trait MetadataPatch {
    type Error;
    fn key(&mut self, key: &str) -> Result<bool, Self::Error>;
    fn replacement(&mut self, value: &impl Serialize) -> Result<(), Self::Error>;
    fn raw(&mut self, target: &mut Value, patch: &Value) -> Result<(), Self::Error>;
    /// Schemas are replaced unless the consumer supplies a structural merge policy.
    fn schema(
        &mut self,
        target: &mut crate::SchemaValue,
        patch: &crate::SchemaValue,
    ) -> Result<(), Self::Error> {
        self.replacement(patch)?;
        target.clone_from(patch);
        Ok(())
    }
}

pub(crate) trait MetadataValue: Clone + Serialize + Default {
    fn patch<P: MetadataPatch>(&mut self, patch: &Self, policy: &mut P) -> Result<(), P::Error>;
}
impl MetadataValue for String {
    fn patch<P: MetadataPatch>(&mut self, patch: &Self, policy: &mut P) -> Result<(), P::Error> {
        policy.replacement(patch)?;
        self.clone_from(patch);
        Ok(())
    }
}

pub(crate) fn field<T: MetadataValue + Decode, P: MetadataPatch>(
    target: &mut Field<T>,
    patch: &Field<T>,
    literal: bool,
    policy: &mut P,
) -> Result<(), P::Error> {
    match patch {
        Field::Missing => {}
        _ if literal => {
            policy.replacement(patch)?;
            target.clone_from(patch);
        }
        Field::Invalid(Value::Null) => *target = Field::Missing,
        Field::Value(patch) => {
            if !matches!(target, Field::Value(_)) {
                let previous = std::mem::take(target);
                *target = Field::Value(match previous {
                    Field::Invalid(value) => T::decode(value).unwrap_or_default(),
                    _ => T::default(),
                });
            }
            if let Field::Value(value) = target {
                value.patch(patch, policy)?;
            }
        }
        Field::Invalid(patch) => {
            let previous = std::mem::take(target);
            let mut value = match previous {
                Field::Value(value) => serde_json::to_value(value).expect("metadata serializes"),
                Field::Invalid(value) => value,
                Field::Missing => Value::Null,
            };
            policy.raw(&mut value, patch)?;
            *target = Field::from_value(value);
        }
    }
    Ok(())
}

pub(crate) fn extra<P: MetadataPatch>(
    target: &mut BTreeMap<String, Value>,
    key: &str,
    patch: &Value,
    literal: bool,
    policy: &mut P,
) -> Result<(), P::Error> {
    if literal {
        policy.replacement(patch)?;
        target.insert(key.into(), patch.clone());
    } else if patch.is_null() {
        target.remove(key);
    } else {
        policy.raw(target.entry(key.into()).or_insert(Value::Null), patch)?;
    }
    Ok(())
}

impl MetadataValue for crate::SchemaValue {
    fn patch<P: MetadataPatch>(&mut self, patch: &Self, policy: &mut P) -> Result<(), P::Error> {
        policy.schema(self, patch)
    }
}

impl<T: Clone + Serialize> MetadataValue for Vec<Field<T>> {
    fn patch<P: MetadataPatch>(&mut self, patch: &Self, policy: &mut P) -> Result<(), P::Error> {
        policy.replacement(patch)?;
        self.clone_from(patch);
        Ok(())
    }
}

impl<T: MetadataValue + Decode> MetadataValue for Fields<T> {
    fn patch<P: MetadataPatch>(&mut self, patch: &Self, policy: &mut P) -> Result<(), P::Error> {
        for (key, value) in patch {
            let literal = policy.key(key)?;
            field(self.entry(key.clone()).or_default(), value, literal, policy)?;
            if self.get(key).is_some_and(Field::is_missing) {
                self.remove(key);
            }
        }
        Ok(())
    }
}

impl MetadataValue for BTreeMap<String, Vec<String>> {
    fn patch<P: MetadataPatch>(&mut self, patch: &Self, policy: &mut P) -> Result<(), P::Error> {
        for (key, value) in patch {
            policy.key(key)?;
            policy.replacement(value)?;
            self.insert(key.clone(), value.clone());
        }
        Ok(())
    }
}
