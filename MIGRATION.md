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

## Rust API migration

`Schema` remains the object form, with `extras` retaining unmodeled keywords and
nested schemas as JSON values. `SchemaValue` is the object-or-boolean form used by
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
`$defs`, `$dynamicRef`, applicators, numeric bounds, `const`, and `examples`, are
retained in `Schema.extras`. Nested schemas there retain objects, booleans, null
unions, and references. Put keywords with dedicated fields in those fields;
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
keys and other keys deserialize into `Callback.extras`. This classifies entries
without validating runtime expression syntax. Malformed values at typed keys
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
| 3.1 metadata/references | `Info.summary`, `License.identifier`, non-schema Reference Object `summary` / `description`. The existing generic `Referenceable<T>` parser can treat references to all-optional objects (e.g. headers) as inline data; the new schema positions avoid that wrapper. |
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
