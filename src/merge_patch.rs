//! RFC 7386 JSON Merge Patch.
//!
//! [`merge`] and [`merge_with`] patch a [`Value`]. With `lossless-schema-keywords`,
//! `MergePatch` is a `MetadataPatch` policy that applies the same algorithm
//! to typed nodes through their `patch_metadata` methods, and
//! `SchemaValue::merge_patch` merges typed schemas. Typed results serialize exactly as the JSON merge of their
//! serialized forms would.
use serde_json::{Map, Value};

#[cfg(feature = "lossless-schema-keywords")]
use crate::lossless_component::MetadataPatch;

/// Apply `patch` to `target` (RFC 7386, section 2): an object patch merges
/// member by member and a `null` member removes the target member; any other
/// patch replaces the target.
///
/// ```
/// use serde_json::json;
/// let mut target = json!({"a": "b", "c": {"d": "e", "f": "g"}});
/// oas::merge_patch::merge(&mut target, &json!({"a": "z", "c": {"f": null}}));
/// assert_eq!(target, json!({"a": "z", "c": {"d": "e"}}));
/// ```
pub fn merge(target: &mut Value, patch: &Value) {
    let result = merge_with(
        target,
        patch,
        &mut || Ok::<(), std::convert::Infallible>(()),
    );
    match result {
        Ok(()) => {}
        Err(never) => match never {},
    }
}

/// [`merge`], calling `charge` before each patched value is applied, the
/// patch itself included. A removed member is not charged. An error stops the
/// merge and leaves the members merged before it in place.
pub fn merge_with<E>(
    target: &mut Value,
    patch: &Value,
    charge: &mut (impl FnMut() -> Result<(), E> + ?Sized),
) -> Result<(), E> {
    charge()?;
    let Value::Object(patch) = patch else {
        target.clone_from(patch);
        return Ok(());
    };
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    let Value::Object(target) = target else {
        unreachable!("the target was made an object")
    };
    for (key, value) in patch {
        if value.is_null() {
            target.remove(key);
        } else {
            merge_with(
                target.entry(key.clone()).or_insert(Value::Null),
                value,
                charge,
            )?;
        }
    }
    Ok(())
}

/// The RFC 7386 policy for `patch_metadata`: every key merges (none is copied
/// literally), `null` removes, objects merge recursively, and other values,
/// including arrays, replace. Schemas merge with
/// [`SchemaValue::merge_patch_with`](crate::SchemaValue::merge_patch_with).
///
/// `charge` is called once per key and once per patched value, so a caller
/// can bound the work of a merge; [`MergePatch::new`] charges nothing.
///
/// ```
/// use oas::lossless_asyncapi::Message;
/// use oas::lossless_component::Decode;
/// use oas::merge_patch::MergePatch;
/// use serde_json::json;
///
/// let mut message = Message::decode(json!({"name": "a", "bindings": {"ws": {"method": "GET"}}})).unwrap();
/// let patch = Message::decode(json!({"name": null, "bindings": {"ws": {"bindingVersion": "0.1.0"}}})).unwrap();
/// message.patch_metadata(&patch, &mut MergePatch::new()).unwrap();
/// assert_eq!(
///     serde_json::to_value(&message).unwrap(),
///     json!({"bindings": {"ws": {"method": "GET", "bindingVersion": "0.1.0"}}})
/// );
/// ```
#[cfg(feature = "lossless-schema-keywords")]
pub struct MergePatch<F, E = std::convert::Infallible> {
    charge: F,
    error: std::marker::PhantomData<fn() -> E>,
}

#[cfg(feature = "lossless-schema-keywords")]
impl MergePatch<fn() -> Result<(), std::convert::Infallible>> {
    /// A policy that never fails.
    pub fn new() -> Self {
        Self::charging(|| Ok(()))
    }
}

#[cfg(feature = "lossless-schema-keywords")]
impl Default for MergePatch<fn() -> Result<(), std::convert::Infallible>> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "lossless-schema-keywords")]
impl<F: FnMut() -> Result<(), E>, E> MergePatch<F, E> {
    /// A policy that calls `charge` for every key and patched value and stops
    /// at its first error.
    pub fn charging(charge: F) -> Self {
        Self {
            charge,
            error: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "lossless-schema-keywords")]
impl<F: FnMut() -> Result<(), E>, E> MetadataPatch for MergePatch<F, E> {
    type Error = E;

    fn key(&mut self, _key: &str) -> Result<bool, E> {
        (self.charge)()?;
        Ok(false)
    }

    fn replacement(&mut self, _value: &impl serde::Serialize) -> Result<(), E> {
        (self.charge)()
    }

    fn raw(&mut self, target: &mut Value, patch: &Value) -> Result<(), E> {
        merge_with(target, patch, &mut self.charge)
    }

    fn schema(
        &mut self,
        target: &mut crate::SchemaValue,
        patch: &crate::SchemaValue,
    ) -> Result<(), E> {
        target.merge_patch_with(patch, &mut self.charge)
    }
}
