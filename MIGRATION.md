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

## Unreleased: typed structural schema keywords

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

`Schema::try_for_each_child_mut` exposes immediate schema children in deterministic
order. Callers control recursion, work/depth limits, and reference resolution.
The visitor never descends into literal data in `extras`; on error it stops and
retains earlier edits. The method follows structural children only, not references.

Consumers that isolate failures per operation must select the relevant subtree
before parsing it. Eagerly parsing a complete document can reject unrelated
operations, and pruning private fields after parsing can reject content that was
previously removed first. This release does not supply deferred document parsing.

Use `SchemaValue` for schema deserialization. The legacy
`Referenceable<Schema>` untagged parser can fall back to a Reference Object after
inline parsing fails, discarding siblings on `main`; stricter schema fields make
that fallback reachable for newly malformed children. Migrating deserialization
to `SchemaValue` rejects those malformed shapes instead. Existing
`Referenceable::schema_ref` constructors remain usable through `.into()`.
Consumers retaining legacy reference deserialization should pair this change with
[the metadata/reference preservation change](https://github.com/Kilerd/oas/pull/4).

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

## Remaining model differences

This inventory compares the existing model to OpenAPI 3.1/3.2. Unknown fields on
most non-schema objects are currently discarded; successful parsing alone does
not guarantee a lossless round trip outside the supported subset.

| Area | Remaining work |
| --- | --- |
| 3.1 document structure | Optional `paths`, typed `webhooks` and `jsonSchemaDialect`, `Components.pathItems`. Root `extras` retains unknown fields, but `paths` is still required. |
| 3.1 metadata/references | `Info.summary`, `License.identifier`, non-schema Reference Object `summary` / `description`. The existing generic `Referenceable<T>` parser can treat references to all-optional objects (e.g. headers) as inline data; the new schema positions avoid that wrapper. |
| 3.1 security | `mutualTLS`; audit OAuth flow required fields (the legacy model requires `authorizationUrl` for every flow). |
| 3.1 operations/links | Optional operation `responses`; `Link.operationId` remains required even when `operationRef` is used. |
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
