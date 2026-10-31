# OpenAPI 3.2 streaming and the 0.3 API

Version 0.3 adds the schema representation needed for OpenAPI 3.1/3.2 and typed
streaming media types. It does **not** claim complete OpenAPI 3.1 or 3.2 model
coverage. The remaining differences are listed below.

## Compatibility policy

- `OpenAPIV3::new(info)` and `builders::api(title, version)` still emit `3.0.0`.
  Opt in with `OpenAPIV3::new_v3_1(info)` or `OpenAPIV3::new_v3_2(info)`.
- Existing `Schema` constructors and `Referenceable` schema helpers remain
  available. Schema-bearing builders accept them, as well as `SchemaValue`,
  `Schema`, and booleans. `Schema::boolean()` still means `{"type":"boolean"}`;
  `SchemaValue::Boolean(false)` means the schema that accepts no values.
- Existing 3.0 JSON, including `nullable`, boolean `exclusiveMinimum` /
  `exclusiveMaximum`, and references, keeps its representation. The regression
  suite includes the existing 3.0 example documents.
- Choosing a document version does not convert schemas, enforce a dialect, or
  validate a document. For example, adding `itemSchema` to a document whose version
  is still `3.0.0` produces an invalid 3.0 document. Callers must select the version
  and migrate the keywords together.
- This is a **source-breaking 0.3 release** for direct field access, struct
  literals, and explicitly typed schema collections. Applications requiring the
  unchanged 0.2 API should remain on `oas = "0.2"`.
  [Version 0.3.0](https://crates.io/crates/oas/0.3.0) was published on 2026-09-14.

## 0.4.0-dev: Tag builder change

`Tag::with_description` now consumes an existing tag, matching the other
`with_*` builders. Replace `Tag::with_description(name, description)` with
`Tag::simple(name).with_description(description)`. The two-argument
`Tag::new(name, Some(description))` constructor remains available. This is a Rust
source change; serialized tags keep the same fields and values. Updating an
existing tag's description preserves its name and external documentation.

## 0.4.0-dev: OAuth flow storage

`SecurityType::Oauth2::flows` is now `Box<OauthFlows>` to reduce the inline size
of `SecurityType`. This is a source-breaking change after 0.3.0; the OAuth JSON
representation is unchanged.

- Construct or replace the field with `Box::new(flows)` instead of `flows`:
  `SecurityType::Oauth2 { flows: Box::new(flows) }`.
- Destructuring `SecurityType::Oauth2 { flows }` now yields `Box<OauthFlows>`.
  Use `let flows: OauthFlows = *flows;` when an owned, unboxed value is needed.
- Borrowed field access continues to dereference automatically; use
  `flows.as_ref()` or `flows.as_mut()` when an explicit reference is needed.

## 0.4.1-dev: typed structural schema keywords

`Schema` now owns properties, required members, object counts, composition and
conditional schemas, dependencies, definitions, and array applicators as typed
fields. Use those fields instead of inserting their JSON keys into `extras`.
Complete struct literals need the new fields or `..Schema::default()`.

```rust
use oas::Schema;
use std::collections::BTreeMap;

let mut header = Schema {
    properties: Some(BTreeMap::from([
        ("model".into(), Schema::string().into()),
    ])),
    required: Some(vec!["model".into()]),
    ..Schema::object()
};
header.properties.as_mut().unwrap().remove("model");
header.required.as_mut().unwrap().retain(|name| name != "model");
```

`None` means absent; `Some` of an empty collection preserves an explicit empty
keyword. Newly modeled keywords reject explicit null and incorrect shapes during
parsing, including nested children. They were previously opaque JSON. Object
counts use `SchemaCount`, which retains integer-valued JSON number forms such as
`1.0` and exposes their `u64` value through `value()`. Construct edited counts with
`SchemaCount::from(count)` or `count.into()`. Negative, fractional, and out-of-range
values are rejected.
This is structural parsing, not dialect validation or schema evaluation.

`SchemaItems` represents either a single schema or a legacy tuple.
`SchemaDependency` represents either required property names or a legacy schema
dependency. Neither representation converts between JSON Schema dialects.
`default`, `const`, examples, and unknown vocabulary remain literal JSON in
`extras`, including explicit null. Existing optional scalar fields keep their
previous null behavior; this change is not a general lossless document parser.

`Schema::try_for_each_child_mut` exposes immediate schema children in lexical
keyword order. `try_for_each_child_mut_with_keyword` also supplies the containing
keyword so callers can select dialect-specific children. Map keys are lexical;
array elements retain input order. `SchemaCount::as_number()` exposes the retained
number representation when a caller needs to distinguish integral number forms.
Callers control recursion, work/depth limits, and reference resolution.
The visitor never descends into literal data in `extras`; on error it stops and
retains earlier edits. The method follows structural children only, not references.

### Retaining formerly opaque schema keywords

The optional `lossless-schema-keywords` Cargo feature exposes valid structural
fields while retaining values that cannot be represented without changing them:

```toml
[dependencies]
oas = { version = "=0.4.1-dev", features = ["lossless-schema-keywords"] }
```

This feature is available from the source revision containing this section; use
that revision until a package containing it is released. It is disabled by default.
Cargo feature unification applies the parsing policy to all uses of this crate in
the dependency graph, including schemas embedded in `Operation` and `MediaType`.

At positions already typed before structural fields were added, existing scalar
errors and null elision remain unchanged. For example, root `type: 7` is rejected
and root `description: null` disappears. Within a formerly opaque child such as
`properties.name`, those same values remain in `extras` with the typed field empty.
Valid object and boolean children become typed children; malformed structural
containers remain intact in `extras`. Defaults, examples, and unknown vocabulary
remain literal data. Retention does not establish schema validity.

A typed assignment overrides an extra with the same keyword during serialization
and `keyword_count()`. Clearing that assignment exposes the retained fallback;
remove both entries to delete the keyword. The shallow child visitor visits typed
children only. `keyword_count()` counts present serialized entries without
serializing payloads; with the feature disabled, typed/extra collisions retain
the existing duplicate-entry behavior and should be avoided.

With this feature enabled, use `Schema::required_contains` and
`retain_required_names` to inspect or edit the effective `required` value.
`remove_dependent_required` removes a trigger from the effective `dependentRequired`
map, including a trigger whose value is malformed; `retain_dependent_required_names`
filters string members of its dependency arrays without removing trigger keys.
These methods handle typed collections and retained malformed arrays/maps. They
preserve non-string array members, non-array dependency values, and explicit null;
a non-array `required` or non-object `dependentRequired` is left unchanged.

`normalize_required_and_type_sets_by_key` sorts and deduplicates effective
`required` and array-valued `type` collections using the caller's JSON ordering
key, including malformed members. Scalar/non-array values and literal payloads
such as examples are untouched. All these methods follow serialization precedence:
a populated typed field (including an empty collection) shadows its same-key extra.
They do not edit shadowed extras; clearing the typed field exposes that fallback.
Remove both representations when deleting the entire keyword.

Consumers that isolate failures per operation can use the feature-gated
`lossless_component` model. Its `Field<T>` distinguishes missing, typed, and
malformed values without rejecting unrelated fields. Select and edit the operation
before consuming it through `Operation::admit`; ordinary model deserialization
continues to enforce the existing admission rules.

With `lossless-schema-keywords`, admitted `oas::Callback` now has
`reference: lossless_component::Field<String>`. Read and edit `$ref` through this
field instead of `Callback.extras`. Complete struct literals must initialize it
with `Field::Missing` when absent. Standalone Callback decoding preserves strings
as `Field::Value` and null or other malformed values as `Field::Invalid`; missing
references stay missing. Serialization preserves these values. This field is not
available when the feature is disabled, where `$ref` retains its extras behavior.

Callback admission preserves the same field in inline `Referenceable::Data`
callbacks, including callbacks with expression children. Reference candidate
precedence is unchanged: a valid pure reference selects `Reference`, and a valid
reference can remain the fallback when inline children cannot be admitted. Do not
convert mixed callbacks into references merely because `$ref` is present; doing so
would discard their inline editing surface. Avoid a duplicate `$ref` in admitted
`Callback.extras`: unlike Schema's effective-key methods, Callback serialization
does not suppress manually inserted duplicate keys.

`SchemaValue::from_lossless_value` retains malformed structural children as
`SchemaValue::Invalid`; exhaustive matches must handle this feature-gated variant.
The method still rejects a non-object, non-boolean root. Use `admit_legacy_root`
when the selected root must satisfy the older typed-boundary rules. Structural
field views expose both typed children and malformed fields; callers retain
control of dialect policy, recursion limits, reference resolution, and diagnostics.
`remove_keyword` removes both a typed keyword and its retained fallback.

The feature also exports `lossless_asyncapi` owners for parameters, correlations,
WebSocket bindings, tags, documentation, and their component collections. Borrowed
and mutable node views traverse structural fields without serializing or cloning
owners; examples and unknown vocabulary remain literal JSON. These types do not
validate complete AsyncAPI documents or resolve references. `MetadataPatch` lets
consumers define work charges, literal-field handling, and schema merge policy;
`patch_keyword_from` applies one present field when callers need to interleave
metadata with other fields.

`lossless_asyncapi::Metadata::owns_keyword` reports declared fields independently
of presence. `Metadata::take_from` and `Components::take_from` move only declared
fields accepted by the caller's predicate out of a JSON object. Rejected and
unknown keys remain in that object; returned owners have empty `extras`. Use these
methods when partitioning a larger envelope instead of maintaining a second list
of the owner's fields.

`Metadata::copy_keyword_from` copies a present declared field literally, including
explicit null, and calls only `MetadataPatch::replacement`; the caller charges its
key. `patch_or_insert_keyword_from` calls `MetadataPatch::key` first, then copies a
missing target's first non-null value literally. Existing values use the recursive
patch policy; non-literal null deletes the field. Both methods ignore absent and
undeclared source keys and leave the source unchanged. Existing `patch_keyword_from`
keeps its merge-from-empty behavior. Field storage, serialized values, heap
accounting, and `NodeLookup` results are unchanged; descriptive scalar values can
still contain arbitrary JSON in `extras`.

`Components::append_rekeyed` consumes source collections and passes each group and
owned entry name to a renaming callback. Empty or missing source collections do
not create target groups. Within each declared group, source names are visited
lexically; later inserts replace a target entry at the same renamed key, including
malformed individual entries. Invalid collection containers in either owner and
nonempty source `extras` return an error before any callback or target mutation.
Target extras remain untouched. Call this before publishing a larger aggregate
when its error must leave that aggregate unchanged; the callback is infallible.

`json_pointer::tokens` and `json_pointer::escape` handle RFC 6901 string-form
pointers and tokens. URI fragment decoding, array-index admission, and target
lookup remain the caller's responsibility. They do not fetch external content.

Use `SchemaValue` for schema deserialization. `Referenceable<Schema>` remains a
legacy compatibility type: inline parse failures can select a Reference Object,
which now retains siblings rather than validating them as schemas. Migrating
schema deserialization to `SchemaValue` rejects malformed schema shapes. Existing
`Referenceable::schema_ref` constructors remain usable through `.into()`; the
conversion promotes siblings only when their JSON values survive unchanged.

## Rust API migration

`Schema` remains the object form, with `extras` retaining unmodeled keywords and
literal payloads as JSON values. `SchemaValue` is the object-or-boolean form used by
`Components.schemas`, `Parameter.schema`, `Header.schema`, `MediaType.schema`,
`MediaType.item_schema`, and `Schema.content_schema`. Schema references are objects
with a `$ref` keyword, so sibling constraints and annotations are preserved.

| 0.2 usage | 0.3 usage |
| --- | --- |
| `schema._type = Some("string".to_string())` | `schema._type = Some("string".into())`, or `Schema::string()` |
| Complete `Schema { ... }` literal | Add `..Schema::default()` for the new reference/content fields |
| Complete `MediaType { ... }` literal | Add `item_schema: None`, or `..MediaType::default()` |
| Assign `Some(Referenceable::schema_ref("User"))` to a schema field | Append `.into()` to the reference, or use `Some(Schema::reference("#/components/schemas/User").into())` |
| `BTreeMap<String, Referenceable<Schema>>` assigned to `Components.schemas` | Use `BTreeMap<String, SchemaValue>`; alternatively pass the old map to `Components::with_schemas` |
| Inspect schema fields through `Referenceable::as_data()` | Use `SchemaValue::as_object()` / `as_object_mut()` and handle `SchemaValue::Boolean` |
| Inspect a schema reference with `as_reference()` | Inspect the object's `_ref`; it may also have sibling keywords |

`with_schemas` now accepts a generic map of convertible schema values. An empty
map may need a type annotation, e.g. `BTreeMap::<String, SchemaValue>::new()`.
Mixed object/boolean collections should also use `SchemaValue` explicitly.

Existing constructor calls continue to work:

```rust
use oas::{MediaType, Referenceable, Schema};

let legacy = MediaType::new().with_schema(Referenceable::string_schema());
let modern = MediaType::new().with_schema(Schema::string().with_types(["string", "null"]));
let unrestricted = MediaType::new().with_schema(true);
let stream = MediaType::new().with_item_schema(Schema::reference("#/components/schemas/Event"));
```

## Non-schema references

`Referenceable<T>` reads an object containing only `$ref` and optional `summary` /
`description` as `Reference`, including in nested header, example, and response
fields. Other object shapes first try the inline `Data(T)` parser; if that fails,
a reference with additional properties is accepted and those properties are
retained in `Reference.extras`. For example, a parameter reference with `x-note`
but no inline `name` / `in` remains readable. The extras map preserves arbitrary
JSON values for document editing; those fields are not assigned additional
OpenAPI Reference Object semantics. This fallback preserves the existing inline
interpretation of schema siblings and generic payloads such as an Example with
`$ref` and `value`. Schema positions use `SchemaValue`, where `$ref` and arbitrary
sibling keywords remain part of the schema object.

`Reference` has public `summary`, `description`, and `extras` fields. Complete
struct literals and exhaustive destructuring must account for the new fields;
use `Reference::new(path)` for a pure reference, or initialize both optional fields
to `None` and `extras` to an empty map. Put `$ref`, `summary`, and `description` in
their typed fields, not in `extras`.
`Referenceable::reference` and `component_ref` keep emitting pure `$ref` objects.
The generic Serde bound remains `T: Deserialize<'de>`, including borrowed inline
data; no `DeserializeOwned`, `Serialize`, or `'static` bound is required.

For legacy `Referenceable<Schema>` deserialization, the standard Reference Object
shape now selects `Reference` rather than `Data`; schema objects with additional
keywords still select `Data`. Converting either variant into `SchemaValue` retains
its fields. Use `SchemaValue` directly when inspecting schema objects.

`From<Reference> for SchemaValue` uses the existing Schema deserializer to promote
siblings such as `type`, `format`, and `contentSchema` into typed fields when their
JSON values round-trip unchanged. This lets ordinary typed edits behave like a
direct schema read. Reference siblings need not be valid schema values: if parsing
fails or would change a value (for example, omit `format: null`), the infallible
conversion retains all siblings in `extras` instead, including ones that could
have been promoted. That fallback preserves content but does not normalize schema
fields. To edit a sibling after fallback, remove or replace its `extras` entry;
setting the corresponding typed field to `None` does not remove the retained key.

This parsing rule does not validate documents or resolve references. Malformed
reference candidates may still fall back to `Data(T)`; objects with additional
fields follow that inline type's existing handling of unknown fields. The new
metadata fields expect strings: inputs previously accepted by ignoring malformed
`summary` / `description` values may now fail deserialization.

## Migrating schema semantics

For OpenAPI 3.1/3.2, replace a 3.0 nullable primitive with a type union:

```rust
use oas::Schema;

let mut old = Schema::string();
old.nullable = Some(true);

let new = Schema::string().with_types(["string", "null"]);
```

For a nullable reference, use `anyOf` containing the referenced schema and
`{"type":"null"}`. Adding a `type` union beside `$ref` does not bypass constraints
in the referenced schema. Review `enum` and composed schemas when migrating.
Convert exclusive bounds to numeric values (for example, `minimum: 0` plus
`exclusiveMinimum: true` becomes `exclusiveMinimum: 0`); prefer `examples` over
`example` and review binary schemas. These transformations are intentionally
explicit. See the [official 3.0 to 3.1 migration guide](https://learn.openapis.org/upgrading/v3.0-to-v3.1.html).

JSON Schema 2020-12 keywords without dedicated fields, including `$schema`, `$id`,
`$dynamicRef`, numeric bounds other than object counts, `const`, and `examples`,
are retained in `Schema.extras`. Structural schema children use the typed fields
described above. Put keywords with dedicated fields in those fields;
do not duplicate them in `extras`.

## SSE

Use `MediaType.item_schema` for each parsed event. `MediaType.schema` continues to
describe the complete body; both fields can coexist. In `text/event-stream`,
`event`, `id`, and `data` are strings, and `retry` is an integer. Describe JSON
inside `data` as a string with `contentMediaType: application/json` and a
`contentSchema`. Content annotations do not perform JSON decoding or validation
in this crate. See the [OpenAPI SSE rules](https://spec.openapis.org/oas/v3.2.0.html#special-considerations-for-server-sent-events).

Run `cargo run --example sse` for a constructed document. The round-trip fixture is
[examples/v3.2/json/sse.json](examples/v3.2/json/sse.json).

## Document editing and extensions

The following objects retain unmodeled fields in public
`extras: BTreeMap<String, serde_json::Value>` maps:

- `Info`, `Contact`, `License`, `Server`, `ServerVariable`, and `Tag`;
- `PathItem`, `Operation`, `ExternalDocumentation`, and `Parameter`;
- `RequestBody`, `MediaType`, `Encoding`, `Response`, `Header`, `Link`, and `Example`;
- `Components`, `Responses`, and `Callback` containers;
- `SecurityScheme`, `OauthFlows`, and `OauthFlow`.

This includes `x-*` extensions and unknown fields without that prefix; retention
does not make them valid OpenAPI fields.
Values retain their JSON structure, including arrays, objects, and null. Typed
edits such as changing `Operation.summary` or removing `PathItem.post` do not
restore old values or deleted operations. The existing root and Schema `extras`
behavior is unchanged.

Existing constructors, defaults, and builders initialize the maps empty, emitting
no additional JSON fields. Constructor and builder calls need no changes.
Complete struct literals for every type listed above must add
`extras: BTreeMap::new()` (or a populated map), which is a **source-breaking**
change. `Components` literals also need `path_items: None` (or a populated map).
`Components.path_items: Option<BTreeMap<String, PathItem>>` exposes reusable
OpenAPI 3.1 path items for typed edits; values formerly stored under
`Components.extras["pathItems"]` now deserialize into that field.
`Link.operation_id` is now `Option<String>`: wrap existing values in `Some(...)`;
use `None` for `operationRef` or documentation-only links. This allows retention
without enforcing link validity or the mutual exclusion of its operation fields.
`OauthFlow.authorization_url` is now `Option<String>`: wrap existing URLs in
`Some(...)`, and use `None` for password and client credentials flows. Implicit
flows use `authorizationUrl`; password and client credentials use `tokenUrl`;
authorization code uses both. The shared flow model preserves these shapes
without enforcing flow-specific URL requirements. Complete security scheme,
flow-container, and flow literals also need `extras: BTreeMap::new()`.
Struct update syntax can instead reuse an existing constructor, where available:

```rust
use oas::{Operation, Response, Responses};

let operation = Operation {
    summary: Some("List items".into()),
    ..Operation::new(Responses::new())
};
let response = Response {
    description: "Success".into(),
    ..Response::new("Success")
};
```

Use the typed member for a modeled field; do not insert keys such as `summary`,
`responses`, or `description` into that object's `extras`, as duplicate JSON keys
can result. Extensions belong to the object that owns them, not to a root-level
copy of `paths`.

Components container metadata, including objects with literal `$ref` keys, is
retained as ordinary JSON rather than parsed as a component reference. Removing
a typed schema or another component does not restore it from container metadata.

Responses separates `default` and HTTP status keys (`100`–`599`, `1XX`–`5XX`)
from container metadata. Other keys now deserialize into `Responses.extras`
rather than `Responses.data`. Callback keys containing `{` deserialize into
`Callback.data` as Path Items, including URL-prefixed expressions; all `x-*`
keys and other keys deserialize into `Callback.extras`, except that `$ref` uses
`Callback.reference` when `lossless-schema-keywords` is enabled, as described above.
This classifies entries without validating runtime expression syntax. Malformed values at typed keys
still fail deserialization. Metadata is never interpreted as a reference or
traversed as a response/path item. Deleting typed entries does not restore them.
Keep status/expression keys in `data`, and other keys in `extras`; duplicate keys
across these maps can produce duplicate JSON fields.

These extras maps retain unmodeled object fields; they do not determine
`Referenceable<T>` variant selection or reference semantics. Retention is still
limited: standalone Discriminator/XML objects lack extras maps; the root `paths`
map does not support arbitrary container metadata. Optional JSON payloads (`Parameter.example`, `Header.example`,
`MediaType.example`, `Example.value`, and `Link.request_body`) preserve explicit
null as `Some(serde_json::Value::Null)`; missing fields deserialize to `None`.
Setting these typed members to `None` removes them from serialized output. Their
Rust field types and constructor signatures are unchanged. Other modeled optional
fields can still normalize explicit null to absence.
For lossless edits across unsupported objects, keep the original
`serde_json::Value` and write back only the fields being changed.

## Remaining model differences

This inventory compares the existing model to OpenAPI 3.1/3.2. Field preservation
is limited as described above; successful parsing alone does not guarantee a
lossless round trip outside the supported subset.

| Area | Remaining work |
| --- | --- |
| 3.1 document structure | Optional `paths`, typed `webhooks` and `jsonSchemaDialect`. Root `extras` retains unknown fields, but `paths` is still required. |
| 3.1 metadata | `Info.summary`, `License.identifier`. |
| 3.1 security | `mutualTLS`; flow-specific URL requirements are not validated. |
| 3.1 operations/links | Optional operation `responses`; Link operation-field mutual exclusion is not validated. |
| 3.2 paths/parameters | `query`, `additionalOperations`, `in: querystring`. |
| 3.2 media/encoding | Reusable `Components.mediaTypes` and references in content maps; `prefixEncoding`, `itemEncoding`, and nested encoding fields. |
| 3.2 metadata/examples | Server `name`, tag `summary` / `parent` / `kind`, example `dataValue` / `serializedValue`. Root `$self` is retained only through `extras`. |
| 3.2 security | Device authorization flow, `oauth2MetadataUrl`, security scheme `deprecated`. |
| 3.2 discriminator/XML | New discriminator/XML fields lack typed models; schema-level values are preserved through `extras`. The legacy standalone `Discriminator` also retains its misspelled `maapping` field. |
| Cross-version | Full extension preservation, required-field and mutual-exclusion validation, reference resolution, and JSON Schema evaluation. |

See the [3.2 specification](https://spec.openapis.org/oas/v3.2.0.html) and
[3.1 to 3.2 guide](https://learn.openapis.org/upgrading/v3.1-to-v3.2.html) for the
complete language changes. Gotcha's generator/dependency migration and SSE
annotations are separate work; this crate describes neither WebSocket message
contracts nor AsyncAPI documents.
