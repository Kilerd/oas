//! Conservative owned-allocation estimates for source-cache accounting.
//!
//! Estimates exclude the value's own inline storage. Containers account for their
//! allocated inline slots once, then add their children's heap estimates. These
//! are capacity estimates, not allocator measurements or process RSS.
use crate::lossless_component::Field;
use crate::{SchemaCount, SchemaDependency, SchemaItems, SchemaType, SchemaValue};
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;
use std::mem::{align_of, size_of};

/// Estimate owned heap bytes, excluding `size_of_val(self)`, with saturating arithmetic.
pub trait HeapSize {
    fn heap_size(&self) -> usize;
}

impl HeapSize for String {
    fn heap_size(&self) -> usize {
        self.capacity()
    }
}

impl HeapSize for bool {
    fn heap_size(&self) -> usize {
        0
    }
}

impl HeapSize for Number {
    fn heap_size(&self) -> usize {
        // serde_json's default number representation stores integers/floats inline.
        0
    }
}

impl<T: HeapSize> HeapSize for Box<T> {
    fn heap_size(&self) -> usize {
        size_of::<T>().saturating_add((**self).heap_size())
    }
}

impl<T: HeapSize> HeapSize for Option<T> {
    fn heap_size(&self) -> usize {
        self.as_ref().map_or(0, HeapSize::heap_size)
    }
}

impl<T: HeapSize> HeapSize for Field<T> {
    fn heap_size(&self) -> usize {
        match self {
            Self::Missing => 0,
            Self::Value(value) => value.heap_size(),
            Self::Invalid(value) => value.heap_size(),
        }
    }
}

impl<T: HeapSize> HeapSize for Vec<T> {
    fn heap_size(&self) -> usize {
        self.iter().fold(
            self.capacity().saturating_mul(size_of::<T>()),
            |total, value| total.saturating_add(value.heap_size()),
        )
    }
}

fn map_storage<K, V>(len: usize) -> usize {
    // std BTreeMap nodes have 11 key/value slots and at most 12 child pointers.
    // Non-root nodes contain at least five entries. Reserve one root even for an
    // empty map: removing its final entry may retain the allocation. Include an
    // internal-node header and alignment slack for every node as an upper estimate.
    let node = size_of::<K>()
        .saturating_add(size_of::<V>())
        .saturating_mul(11)
        .saturating_add(16usize.saturating_mul(size_of::<usize>()))
        .saturating_add(align_of::<K>())
        .saturating_add(align_of::<V>());
    (len / 5).saturating_add(1).saturating_mul(node)
}

impl<K: HeapSize, V: HeapSize> HeapSize for BTreeMap<K, V> {
    fn heap_size(&self) -> usize {
        self.iter()
            .fold(map_storage::<K, V>(self.len()), |total, (key, value)| {
                total
                    .saturating_add(key.heap_size())
                    .saturating_add(value.heap_size())
            })
    }
}

impl HeapSize for Map<String, Value> {
    fn heap_size(&self) -> usize {
        self.iter().fold(
            map_storage::<String, Value>(self.len()),
            |total, (key, value)| {
                total
                    .saturating_add(key.heap_size())
                    .saturating_add(value.heap_size())
            },
        )
    }
}

impl HeapSize for Value {
    fn heap_size(&self) -> usize {
        match self {
            Self::Null | Self::Bool(_) => 0,
            Self::Number(value) => value.heap_size(),
            Self::String(value) => value.heap_size(),
            Self::Array(values) => values.heap_size(),
            Self::Object(values) => values.heap_size(),
        }
    }
}

impl HeapSize for SchemaCount {
    fn heap_size(&self) -> usize {
        self.as_number().heap_size()
    }
}

impl HeapSize for SchemaType {
    fn heap_size(&self) -> usize {
        match self {
            Self::Single(value) => value.heap_size(),
            Self::Multiple(values) => values.heap_size(),
        }
    }
}

impl HeapSize for SchemaValue {
    fn heap_size(&self) -> usize {
        match self {
            Self::Boolean(_) => 0,
            Self::Object(value) => value.heap_size(),
            Self::Invalid(value) => value.heap_size(),
        }
    }
}

impl HeapSize for SchemaItems {
    fn heap_size(&self) -> usize {
        match self {
            Self::Schema(value) => value.heap_size(),
            Self::Tuple(values) => values.heap_size(),
        }
    }
}

impl HeapSize for SchemaDependency {
    fn heap_size(&self) -> usize {
        match self {
            Self::Schema(value) => value.heap_size(),
            Self::Required(values) => values.heap_size(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lossless_component::{Decode, PathItem};
    use serde_json::json;

    #[test]
    fn owned_capacity_excludes_inline_storage_and_counts_boxed_values_once() {
        let mut text = String::with_capacity(64);
        text.push_str("hello");
        let capacity = text.capacity();
        assert_eq!(text.heap_size(), capacity);
        let boxed = Box::new(Some(text));
        assert_eq!(boxed.heap_size(), size_of::<Option<String>>() + capacity);
        let mut values = Vec::with_capacity(4);
        values.push(boxed);
        assert_eq!(
            values.heap_size(),
            values.capacity() * size_of::<Box<Option<String>>>()
                + size_of::<Option<String>>()
                + capacity
        );
        let invalid: Field<String> = Field::Invalid(json!(["literal"]));
        assert!(invalid.heap_size() >= size_of::<Value>() + "literal".len());
    }

    #[test]
    fn sparse_typed_paths_account_for_allocated_slots_and_nested_children() {
        let empty = PathItem::decode(json!({})).unwrap();
        let mut paths = BTreeMap::new();
        paths.insert("/empty".to_owned(), Field::Value(empty));
        assert!(paths.heap_size() >= size_of::<Field<PathItem>>() + "/empty".len());
        let populated = PathItem::decode(json!({"get":{"responses":{"200":{
            "description":"ok", "content":{"application/json":{"schema":{
                "properties":{"name":{"type":"string"}},"x-note":"retained"
            }}}
        }}}}))
        .unwrap();
        assert!(populated.heap_size() > paths["/empty"].heap_size());
        paths.insert("/data".into(), Field::Value(populated));
        assert!(paths.heap_size() >= 2 * size_of::<Field<PathItem>>());
    }
}
