# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.3-dev] - 2026-11-03

### Added
- `json_pointer::Step` locates a child under its parent: a field, a map entry with its key, or an array element with its index. It writes an escaped pointer suffix and pushes unescaped tokens.
- Keyed child visitors: `try_for_each_child_at` on the lossless OpenAPI and AsyncAPI `Node` / `NodeMut`, and `Schema::try_for_each_child_at` / `try_for_each_child_mut_at`. They visit the same children in the same order as the unkeyed visitors, whose behavior is unchanged.
- `merge_patch::merge` and `merge_with` apply an RFC 7386 JSON Merge Patch to a `serde_json::Value`; `merge_with` charges each patched value to a caller callback.
- With `lossless-schema-keywords`: `merge_patch::MergePatch`, an RFC 7386 `MetadataPatch` policy for `patch_metadata`, and `SchemaValue::merge_patch` / `merge_patch_with`, a typed schema merge whose result serializes as the JSON merge and keeps the typed placement of a fresh lossless decode.
- With `lossless-schema-keywords`: typed `properties` / `required` access on `Schema` that keeps malformed values. `properties_map`, `properties_map_mut`, `properties_map_or_insert`, `required_names`, `required_names_mut`, `required_names_or_insert` and `insert_required_name` return the effective typed value, or `Err` with a malformed value retained in `extras`, which they never replace.
- With `lossless-schema-keywords`: AsyncAPI traits merge without value round trips. `From<MessageTrait> for Message` and `From<OperationTrait> for Operation` convert a trait to its object, and `Message::merge_traits` / `Operation::merge_traits` apply resolved traits in declaration order with the object's own fields last, using any `MetadataPatch` policy (`MergePatch` for RFC 7386).

## [0.4.2-dev] - 2026-11-02

### Added
- Optional `lossless-schema-keywords` parsing retains formerly opaque values while exposing valid typed fields, including schemas embedded in operations. Default parsing remains strict.
- `Schema::keyword_count()` counts present entries without serializing child payloads. With the optional feature, typed fields override colliding extras in serialization and counting.
- Lossless AsyncAPI 3.0 structure owners (`Document`, `Server`, `ServerVariable`, `Channel`, `Operation`, `OperationTrait`, `OperationReply`, `OperationReplyAddress`, `Message`, `MessageTrait`, `SecurityScheme`, `OauthFlows`, `OauthFlow`, `Reference`) with node traversal, reference access and pointer lookup. `Channel.address` keeps an explicit `null` apart from a missing address.
- Lossless OpenAPI `Document`, `Components` and `Paths` owners. They round-trip losslessly, let one visitor walk a document from its root, and support `SchemaLookup` from the root.

### Changed
- AsyncAPI `Components` declares every 3.0 component group. `Components::take_from` moves the newly declared groups out of the source object, and `Node` / `NodeMut` gain variants for the new owners.
- Lossless `Callback` decoding treats every key other than `x-*`, `$ref`, `summary` and `description` as a Path Item, so constant callback URLs without braces are traversed. Admission into the strict model is unchanged.
- OpenAPI `Node` / `NodeMut` gain `Document`, `Components` and `Paths` variants; exhaustive matches must handle them.

### Fixed
- Metadata patches keep the previous field value, and its untouched siblings, when a `MetadataPatch` policy rejects an update.

This prerelease is the first package that contains the `lossless-schema-keywords` feature and the `lossless_component` / `lossless_asyncapi` modules.

## [0.4.1-dev] - 2026-10-30

### Added
- Typed structural schema fields for properties, required members, object counts, compositions, conditionals, dependencies, definitions, and array applicators.
- Shallow mutable schema visitors, including keyword-aware traversal in lexical order. Callers retain control of recursion, dialect selection, and reference resolution.
- `SchemaCount` preserves integer-valued JSON number forms and exposes both the count and its number representation.

### Changed
- Structural schema keywords move from `extras` to typed fields. Explicit empty collections remain present; null and malformed structural values now fail parsing. Complete schema literals need the new fields or a default update.
- Examples and guides edit schema fields directly. Reference-to-schema conversion moves retained siblings without cloning them; explicit-null payload fields share one presence parser.

See [MIGRATION.md](MIGRATION.md) for selective-parsing requirements and field migration. This prerelease does not add deferred parsing, document validation, or reference resolution.

## [0.4.0-dev] - 2026-10-29

### Added
- Retained-object metadata preservation, typed reusable Path Items, and explicit-null payload preservation during typed edits.
- Non-schema Reference Object annotations and unknown siblings, with lossless conversion into schema fields where possible.
- Metadata retention on security schemes and OAuth flows, including password and client credentials flows without an authorization URL.

### Changed
- Complete object and reference literals require the new metadata fields; `Link.operation_id` and `OauthFlow.authorization_url` are optional.
- `SecurityType::Oauth2` stores boxed flows. `Tag::with_description` is a consuming builder that retains existing metadata.
- Security serialization gives current typed fields precedence over colliding retained metadata.
- Formatting and warning-free Clippy checks apply across all targets.

See [MIGRATION.md](MIGRATION.md) for source migration details and remaining model limitations. This is a prerelease; it does not add document validation or reference resolution.

## [0.3.0] - 2026-09-14

### Added
- OpenAPI 3.2 `MediaType.item_schema` and its builder, independent of the whole-body schema.
- `SchemaValue` for object/boolean schemas and `SchemaType` for single types/type unions.
- Schema references with sibling keywords and typed `contentMediaType`, `contentEncoding`, and `contentSchema`.
- Explicit OpenAPI 3.1/3.2 document constructors, an SSE example, round-trip/compatibility tests, and CI.
- [Migration policy and remaining 3.1/3.2 model differences](MIGRATION.md); coverage remains partial.

### Changed
- Schema-bearing fields now use `SchemaValue`; schema builders continue accepting legacy `Referenceable<Schema>` helpers.
- `Schema._type` uses `Option<SchemaType>`; schema and media-type literals need the new fields or defaults.
- The package version advances to 0.3.0 for these source-breaking changes. Existing document constructors still emit 3.0.0, and legacy nullable/exclusive-bound values are not rewritten.

## [0.2.0] - 2025-08-20

### Added
- **Comprehensive convenience constructors** for all major types (`new()` methods with fluent APIs)
- **Builder pattern support** with `OperationBuilder` for complex operation construction
- **Quick builders module** (`builders::api`, `builders::get`, `builders::post`, etc.)
- **Referenceable helper methods** for common patterns:
  - Type-specific constructors (`query_param`, `path_param`, `schema_ref`)
  - Chainable methods (`with_schema`, `with_description`, `with_content`)
- **Schema shortcuts** (`Schema::string()`, `Schema::integer()`, etc.)
- **Default implementations** for appropriate types
- **Extensive documentation** with examples for all public APIs
- **Comprehensive README.md** with usage examples and API reference
- **Usage guide** (`USAGE_GUIDE.md`) with detailed tutorials
- **Example files** demonstrating various usage patterns

### Changed
- **Improved ergonomics** - significantly reduced boilerplate code required
- **Enhanced type safety** - better inference and cleaner APIs
- **Documentation coverage** - every public item now has comprehensive docs

### Technical Details
- All convenience methods maintain 100% backward compatibility
- Builder patterns use move semantics for efficient chaining
- Examples compile and run successfully
- All documentation tests pass

This release represents a major usability improvement while maintaining full API compatibility.

## [0.1.1] - Previous Release
- Basic OpenAPI 3.0 support
- Core serialization/deserialization functionality

## [0.1.0] - Initial Release  
- Initial implementation of OpenAPI 3.0 specification in Rust
- Basic struct definitions and serde support
